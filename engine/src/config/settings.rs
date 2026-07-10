// Settings management//! Engine settings — loaded from environment variables
//! Industry best: single source of truth, validated at startup

use serde::Deserialize;
use std::net::SocketAddr;

#[derive(Debug, Clone, Deserialize)]
pub struct EngineSettings {
    pub http_port: u16,
    pub p2p_port: u16,
    pub relay_url: String,
    pub firebase_url: String,
    pub jwt_secret: String,
    pub vault_key: String,
    pub database_url: String,
    pub log_level: String,
    pub grpc_port: Option<u16>,
    pub wire_port: Option<u16>,
}

impl EngineSettings {
    /// Load from environment with sensible defaults
    pub fn from_env() -> anyhow::Result<Self> {
        let http_port = std::env::var("BENNETT_HTTP_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(3001);

        let p2p_port = std::env::var("BENNETT_P2P_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(56882);

        let relay_url = std::env::var("BENNETT_RELAY_URL")
            .unwrap_or_else(|_| "wss://bennett-relay.onrender.com/ws/tunnel".to_string());

        let firebase_url = std::env::var("BENNETT_FIREBASE_URL")
            .unwrap_or_else(|_| "https://bennett-p2p-signaling-default-rtdb.europe-west1.firebasedatabase.app/".to_string());

        let jwt_secret = std::env::var("BENNETT_JWT_SECRET")
            .unwrap_or_else(|_| "dev-secret-change-in-production".to_string());

        let vault_key = std::env::var("BENNETT_VAULT_KEY")
            .unwrap_or_else(|_| "dev-vault-key-change-in-production".to_string());

        let database_url = std::env::var("BENNETT_DATABASE_URL")
            .unwrap_or_else(|_| {
                let home = dirs::home_dir()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| ".".to_string());
                format!("sqlite:{}/.bennett/data/engine.db", home)
            });

        let log_level = std::env::var("RUST_LOG")
            .unwrap_or_else(|_| "info".to_string());

        let grpc_port = std::env::var("BENNETT_GRPC_PORT")
            .ok()
            .and_then(|p| p.parse().ok());

        let wire_port = std::env::var("BENNETT_WIRE_PORT")
            .ok()
            .and_then(|p| p.parse().ok());

        // Validate critical secrets in production
        if jwt_secret.len() < 32 && std::env::var("BENNETT_PRODUCTION").is_ok() {
            return Err(anyhow::anyhow!("BENNETT_JWT_SECRET must be at least 32 characters in production"));
        }

        Ok(Self {
            http_port,
            p2p_port,
            relay_url,
            firebase_url,
            jwt_secret,
            vault_key,
            database_url,
            log_level,
            grpc_port,
            wire_port,
        })
    }

    pub fn http_addr(&self) -> SocketAddr {
        SocketAddr::from(([0, 0, 0, 0], self.http_port))
    }

    pub fn is_production(&self) -> bool {
        std::env::var("BENNETT_PRODUCTION").is_ok()
            || self.relay_url.contains("onrender.com")
            || self.relay_url.contains("vercel.app")
    }
}
