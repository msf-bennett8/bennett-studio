//! Firebase Realtime Database signaling
//! Free P2P ICE exchange using Firebase Spark plan (no server needed)
//!
//! Setup:
//! 1. Create project at firebase.google.com
//! 2. Enable Realtime Database
//! 3. Set BENNETT_FIREBASE_URL or --firebase-url
//! 4. (Optional) Add Firebase Auth rules for security

use std::time::Duration;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::transport::ice::IceCandidates;

/// Firebase signaling client
pub struct FirebaseSignaling {
    base_url: String,
    client: reqwest::Client,
}

/// Room data structure in Firebase
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SignalingRoom {
    /// Engine's ICE candidates
    pub engine_ice: Option<IceCandidates>,
    /// Client's ICE candidates
    pub client_ice: Option<IceCandidates>,
    /// Room creation timestamp (for TTL cleanup)
    pub created_at: i64,
    /// Connection state: "waiting", "connecting", "connected", "closed"
    pub state: String,
}

impl FirebaseSignaling {
    pub fn new(base_url: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_default();
        
        Self { base_url, client }
    }

    /// Create a room as engine (host) and upload our ICE
    pub async fn create_room(&self, room_code: &str, engine_ice: &IceCandidates) -> Result<(), SignalingError> {
        let room = SignalingRoom {
            engine_ice: Some(engine_ice.clone()),
            client_ice: None,
            created_at: chrono::Utc::now().timestamp(),
            state: "waiting".to_string(),
        };
        let url = format!("{}/rooms/{}.json", self.base_url.trim_end_matches('/'), room_code);
        
        info!(room = %room_code, url = %url, "Creating Firebase room");
        
        let resp = self.client.put(&url)
            .json(&room)
            .send()
            .await
            .map_err(|e| {
                tracing::error!(error = %e, url = %url, "Firebase PUT failed");
                SignalingError::Network(e)
            })?;
        
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            tracing::error!(status = %status, body = %body, url = %url, "Firebase returned error");
            return Err(SignalingError::Http(
                reqwest::Error::from(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("HTTP {}: {}", status, body)
                ))
            ));
        }

        info!(room = %room_code, "Room created, waiting for client");
        Ok(())
    }

    /// Poll room for client's ICE (engine side)
    pub async fn poll_for_client(&self, room_code: &str, timeout_secs: u64) -> Result<IceCandidates, SignalingError> {
        let url = format!("{}/rooms/{}.json", self.base_url.trim_end_matches('/'), room_code);
        let deadline = std::time::Instant::now() + Duration::from_secs(timeout_secs);

        info!(room = %room_code, timeout = timeout_secs, "Polling for client ICE");

        while std::time::Instant::now() < deadline {
            let resp = self.client.get(&url)
                .send()
                .await
                .map_err(|e| SignalingError::Network(e))?;

            if resp.status().is_success() {
                let room: Option<SignalingRoom> = resp.json().await
                    .map_err(|e| SignalingError::Parse(e.to_string()))?;

                if let Some(room) = room {
                    if let Some(client_ice) = room.client_ice {
                        info!(room = %room_code, "Client ICE received");
                        return Ok(client_ice);
                    }
                }
            }

            tokio::time::sleep(Duration::from_secs(1)).await;
        }

        Err(SignalingError::Timeout)
    }

    /// Join room as client, upload our ICE, get engine's ICE
    pub async fn join_room(&self, room_code: &str, client_ice: &IceCandidates) -> Result<IceCandidates, SignalingError> {
        let url = format!("{}/rooms/{}.json", self.base_url.trim_end_matches('/'), room_code);
        
        info!(room = %room_code, "Joining Firebase signaling room");

        // First, get engine's ICE
        let resp = self.client.get(&url)
            .send()
            .await
            .map_err(|e| SignalingError::Network(e))?;

        if !resp.status().is_success() {
            return Err(SignalingError::RoomNotFound);
        }

        let room: SignalingRoom = resp.json().await
            .map_err(|e| SignalingError::Parse(e.to_string()))?;

        let engine_ice = room.engine_ice.ok_or(SignalingError::NoEngineIce)?;

        // Upload our ICE as client
        let patch = serde_json::json!({
            "client_ice": client_ice,
            "state": "connecting"
        });

        self.client.patch(&url)
            .json(&patch)
            .send()
            .await
            .map_err(|e| SignalingError::Network(e))?
            .error_for_status()
            .map_err(|e| SignalingError::Http(e))?;

        info!(room = %room_code, "Joined room, engine ICE received");
        Ok(engine_ice)
    }

    /// Mark room as connected (both sides)
    pub async fn mark_connected(&self, room_code: &str) -> Result<(), SignalingError> {
        let url = format!("{}/rooms/{}.json", self.base_url.trim_end_matches('/'), room_code);
        let patch = serde_json::json!({ "state": "connected" });
        
        self.client.patch(&url)
            .json(&patch)
            .send()
            .await
            .map_err(|e| SignalingError::Network(e))?
            .error_for_status()
            .map_err(|e| SignalingError::Http(e))?;

        Ok(())
    }

    /// Close and delete room
    pub async fn close_room(&self, room_code: &str) -> Result<(), SignalingError> {
        let url = format!("{}/rooms/{}.json", self.base_url.trim_end_matches('/'), room_code);
        
        self.client.delete(&url)
            .send()
            .await
            .map_err(|e| SignalingError::Network(e))?
            .error_for_status()
            .map_err(|e| SignalingError::Http(e))?;

        info!(room = %room_code, "Room closed");
        Ok(())
    }
}

/// Generate a short human-readable room code
/// Format: ABC-123 (easy to type, read over phone, etc.)
pub fn generate_room_code() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    
    const CONSONANTS: &[u8] = b"BCDFGHJKLMNPQRSTVWXZ";
    const VOWELS: &[u8] = b"AEIOU";
    const DIGITS: &[u8] = b"23456789"; // No 0, 1 (confusable with O, I)
    
    let mut code = String::with_capacity(7);
    
    // CVC-NNN pattern: easy to pronounce, hard to mistake
    code.push(CONSONANTS[rng.gen_range(0..CONSONANTS.len())] as char);
    code.push(VOWELS[rng.gen_range(0..VOWELS.len())] as char);
    code.push(CONSONANTS[rng.gen_range(0..CONSONANTS.len())] as char);
    code.push('-');
    code.push(DIGITS[rng.gen_range(0..DIGITS.len())] as char);
    code.push(DIGITS[rng.gen_range(0..DIGITS.len())] as char);
    code.push(DIGITS[rng.gen_range(0..DIGITS.len())] as char);
    
    code
}

#[derive(Debug)]
pub enum SignalingError {
    Network(reqwest::Error),
    Http(reqwest::Error),
    Parse(String),
    Timeout,
    RoomNotFound,
    NoEngineIce,
}

impl std::fmt::Display for SignalingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SignalingError::Network(e) => write!(f, "Network error: {}", e),
            SignalingError::Http(e) => write!(f, "HTTP error: {}", e),
            SignalingError::Parse(s) => write!(f, "Parse error: {}", s),
            SignalingError::Timeout => write!(f, "Signaling timeout"),
            SignalingError::RoomNotFound => write!(f, "Room not found"),
            SignalingError::NoEngineIce => write!(f, "Engine has not uploaded ICE yet"),
        }
    }
}

impl std::error::Error for SignalingError {}
