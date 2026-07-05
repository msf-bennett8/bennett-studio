//! P2P Signaling Server — STUB for future WebRTC implementation
//!
//! This module provides a WebSocket endpoint for ICE candidate exchange.
//! It is NOT active when P2P is disabled (default).
//!
//! When enabled, clients connect here to negotiate direct peer-to-peer
//! connections before falling back to TCP relay.

use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::State,
    response::IntoResponse,
    routing::get,
    Router,
};
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::info;

/// Signaling message between peers
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum SignalingMessage {
    /// Register as a host for a share
    RegisterHost {
        share_id: String,
        host_id: String,
    },
    /// Request connection to a share (from guest)
    RequestConnection {
        share_id: String,
        guest_id: String,
    },
    /// ICE candidate from host or guest
    IceCandidate {
        share_id: String,
        peer_id: String,
        candidate: String,
    },
    /// Connection established
    Connected {
        share_id: String,
    },
    /// Error
    Error {
        message: String,
    },
}

/// Shared signaling state
pub struct SignalingState {
    /// Broadcast channel for ICE candidates
    tx: broadcast::Sender<SignalingMessage>,
}

impl SignalingState {
    pub fn new() -> Arc<Self> {
        let (tx, _rx) = broadcast::channel(1024);
        Arc::new(Self { tx })
    }
}

/// WebSocket handler for signaling
pub async fn signaling_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<SignalingState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<SignalingState>) {
    info!("New signaling connection");

    let mut rx = state.tx.subscribe();

    // Forward broadcast messages to this client
    let forward_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            let json = match serde_json::to_string(&msg) {
                Ok(j) => j,
                Err(_) => continue,
            };

            if socket.send(Message::Text(json)).await.is_err() {
                break;
            }
        }
    });

    // TODO: Read messages from client, handle registration/ICE exchange
    // For now, just keep connection alive
    tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
    let _ = forward_task.abort();
}

/// Create signaling router
pub fn signaling_router(state: Arc<SignalingState>) -> Router {
    Router::new()
        .route("/signaling", get(signaling_handler))
        .with_state(state)
}
