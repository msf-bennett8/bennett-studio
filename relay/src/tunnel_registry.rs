//! Shared tunnel registry for async request/response correlation
//! Bridges ShareRouter (which decides WHERE to send) with tunnel_ws_handler (which HAS the sender)

use std::collections::HashMap;
use tokio::sync::{mpsc, oneshot, RwLock};
use std::sync::Arc;
use tracing::{debug, warn};

/// Message types sent TO the engine through the tunnel
#[derive(Debug, Clone)]
pub enum TunnelMessageToEngine {
    QueryRequest {
        request_id: String,
        share_code: String,
        token: String,
        sql: String,
        limit: Option<i32>,
        offset: Option<i32>,
    },
    SchemaRequest {
        request_id: String,
        share_code: String,
        token: String,
    },
    Ping,
}

/// Response FROM the engine through the tunnel
#[derive(Debug, Clone)]
pub enum TunnelMessageFromEngine {
    QueryResponse {
        request_id: String,
        result: serde_json::Value,
    },
    SchemaResponse {
        request_id: String,
        schema: serde_json::Value,
    },
    Pong,
}

/// Global registry shared between router and WebSocket handler
#[derive(Default)]
pub struct TunnelRegistry {
    /// host_id -> sender to tunnel WebSocket
    tunnels: RwLock<HashMap<String, mpsc::UnboundedSender<TunnelMessageToEngine>>>,
    /// request_id -> oneshot sender for HTTP response correlation
    pending: RwLock<HashMap<String, oneshot::Sender<serde_json::Value>>>,
}

impl TunnelRegistry {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Register a tunnel sender when engine WebSocket connects
    pub async fn register_tunnel(&self, host_id: String, tx: mpsc::UnboundedSender<TunnelMessageToEngine>) {
        let mut tunnels = self.tunnels.write().await;
        tunnels.insert(host_id, tx);
        debug!("Registered tunnel for host: {}", host_id);
    }

    /// Remove tunnel when engine disconnects
    pub async fn unregister_tunnel(&self, host_id: &str) {
        let mut tunnels = self.tunnels.write().await;
        tunnels.remove(host_id);
        debug!("Unregistered tunnel for host: {}", host_id);
    }

    /// Register a pending HTTP request waiting for tunnel response
    pub async fn register_pending(&self, request_id: String, tx: oneshot::Sender<serde_json::Value>) {
        let mut pending = self.pending.write().await;
        pending.insert(request_id, tx);
    }

    /// Complete a pending request with engine response
    pub async fn complete_request(&self, request_id: &str, response: serde_json::Value) {
        let mut pending = self.pending.write().await;
        if let Some(tx) = pending.remove(request_id) {
            let _ = tx.send(response);
        } else {
            warn!("No pending request for tunnel response: {}", request_id);
        }
    }

    /// Send a message through tunnel to a specific host, return receiver for response
    pub async fn send_and_wait(
        &self,
        host_id: &str,
        msg: TunnelMessageToEngine,
        timeout_secs: u64,
    ) -> anyhow::Result<serde_json::Value> {
        let tunnels = self.tunnels.read().await;
        let tx = tunnels.get(host_id)
            .ok_or_else(|| anyhow::anyhow!("No tunnel for host: {}", host_id))?
            .clone();
        drop(tunnels);

        let request_id = match &msg {
            TunnelMessageToEngine::QueryRequest { request_id, .. } => request_id.clone(),
            TunnelMessageToEngine::SchemaRequest { request_id, .. } => request_id.clone(),
            _ => uuid::Uuid::new_v4().to_string(),
        };

        let (rx_tx, rx_rx) = oneshot::channel();
        self.register_pending(request_id.clone(), rx_tx).await;

        // Send to tunnel
        tx.send(msg).map_err(|e| anyhow::anyhow!("Tunnel send failed: {}", e))?;

        // Wait for response with timeout
        match tokio::time::timeout(
            tokio::time::Duration::from_secs(timeout_secs),
            rx_rx
        ).await {
            Ok(Ok(response)) => Ok(response),
            Ok(Err(_)) => Err(anyhow::anyhow!("Tunnel response channel closed")),
            Err(_) => {
                // Cleanup on timeout
                let mut pending = self.pending.write().await;
                pending.remove(&request_id);
                Err(anyhow::anyhow!("Tunnel request timeout after {}s", timeout_secs))
            }
        }
    }

    /// Get list of connected host_ids
    pub async fn connected_hosts(&self) -> Vec<String> {
        let tunnels = self.tunnels.read().await;
        tunnels.keys().cloned().collect()
    }

    /// Check if a host has an active tunnel connection
    /// (used by health monitor for heartbeat staleness detection)
    pub async fn is_host_alive(&self, host_id: &str) -> bool {
        let tunnels = self.tunnels.read().await;
        tunnels.contains_key(host_id)
    }

    /// Update last activity timestamp for a host (called on any tunnel message)
    pub async fn touch_host(&self, host_id: &str) {
        // In production, track last_activity per host
        // For now, tunnel existence implies alive
        debug!("Host {} tunnel activity", host_id);
    }
}
