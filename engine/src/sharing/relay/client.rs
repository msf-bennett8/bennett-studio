//! Engine → Relay Tunnel Client
//! Maintains persistent WebSocket connection to public relay (Render)
//! so external websites can reach this engine through the relay.

use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio::time::{interval, Duration};
use tracing::{info, warn, error, debug};
use serde::{Deserialize, Serialize};
use futures_util::{StreamExt, SinkExt};

use crate::sharing::share_store::ShareStore;
use crate::auth::share_token::ShareTokenManager;

/// Tunnel message types between engine and relay
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TunnelMessage {
    /// Engine registers itself with relay
    Register {
        host_id: String,
        version: String,
        capabilities: Vec<String>,
    },
    /// Heartbeat ping
    Ping {
        timestamp: i64,
    },
    /// Heartbeat pong
    Pong {
        timestamp: i64,
    },
    /// New share created — notify relay
    ShareCreated {
        code: String,
        db_id: String,
        permission: String,
        expires_at: i64,
    },
    /// Share revoked — notify relay
    ShareRevoked {
        code: String,
    },
    /// Query request from relay (external website)
    QueryRequest {
        request_id: String,
        share_code: String,
        token: String,
        sql: String,
        limit: Option<i32>,
        offset: Option<i32>,
    },
    /// Query response back to relay
    QueryResponse {
        request_id: String,
        result: serde_json::Value,
    },
    /// Schema request from relay
    SchemaRequest {
        request_id: String,
        share_code: String,
        token: String,
    },
    /// Schema response back to relay
    SchemaResponse {
        request_id: String,
        schema: serde_json::Value,
    },
}

/// Relay tunnel client state
pub struct RelayTunnelClient {
    relay_url: String,
    host_id: String,
    token_manager: Arc<RwLock<ShareTokenManager>>,
    share_store: Arc<ShareStore>,
    connection_manager: Option<Arc<tokio::sync::Mutex<crate::control_plane::connection::manager::ConnectionManager>>>,
    ws_tx: Option<mpsc::UnboundedSender<TunnelMessage>>,
    connected: bool,
}

impl RelayTunnelClient {
    pub fn new(
        relay_url: String,
        host_id: String,
        token_manager: Arc<RwLock<ShareTokenManager>>,
        share_store: Arc<ShareStore>,
    ) -> Self {
        Self {
            relay_url,
            host_id,
            token_manager,
            share_store,
            connection_manager: None,
            ws_tx: None,
            connected: false,
        }
    }

    /// Attach a ConnectionManager for query execution (call after new, before run)
    pub fn with_connection_manager(
        mut self,
        cm: Arc<tokio::sync::Mutex<crate::control_plane::connection::manager::ConnectionManager>>,
    ) -> Self {
        self.connection_manager = Some(cm);
        self
    }

    /// Start the tunnel — connects and maintains connection with auto-reconnect
    pub async fn run(&mut self) -> anyhow::Result<()> {
        let mut reconnect_delay = Duration::from_secs(1);
        let max_reconnect_delay = Duration::from_secs(60);

        loop {
            match self.connect_and_maintain().await {
                Ok(_) => {
                    info!("Relay tunnel closed gracefully");
                    reconnect_delay = Duration::from_secs(1);
                }
                Err(e) => {
                    warn!("Relay tunnel error: {}. Reconnecting in {}s...", e, reconnect_delay.as_secs());
                    tokio::time::sleep(reconnect_delay).await;
                    reconnect_delay = std::cmp::min(reconnect_delay * 2, max_reconnect_delay);
                }
            }
        }
    }

    async fn connect_and_maintain(&mut self) -> anyhow::Result<()> {
        let ws_url = format!("{}/ws/tunnel/{}", self.relay_url, self.host_id);
        info!("Connecting to relay tunnel: {}", ws_url);

        let (ws_stream, _) = tokio_tungstenite::connect_async(&ws_url).await
            .map_err(|e| anyhow::anyhow!("WebSocket connect failed: {}", e))?;

        let (mut write, mut read) = ws_stream.split();
        let (tx, mut rx) = mpsc::unbounded_channel::<TunnelMessage>();

        self.ws_tx = Some(tx.clone());
        self.connected = true;

        // Send registration
        let register_msg = TunnelMessage::Register {
            host_id: self.host_id.clone(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            capabilities: vec!["query".to_string(), "schema".to_string(), "write".to_string()],
        };
        let _ = write.send(tokio_tungstenite::tungstenite::Message::Text(
            serde_json::to_string(&register_msg).unwrap()
        )).await;

        info!("Relay tunnel connected and registered");

        // Heartbeat interval
        let mut heartbeat = interval(Duration::from_secs(30));

        loop {
            tokio::select! {
                // Incoming messages from relay
                msg = read.next() => {
                    match msg {
                        Some(Ok(tokio_tungstenite::tungstenite::Message::Text(text))) => {
                            if let Ok(tunnel_msg) = serde_json::from_str::<TunnelMessage>(&text) {
                                self.handle_message(tunnel_msg, &tx).await;
                            }
                        }
                        Some(Ok(tokio_tungstenite::tungstenite::Message::Close(_))) | None => {
                            warn!("Relay WebSocket closed");
                            self.connected = false;
                            return Err(anyhow::anyhow!("WebSocket closed"));
                        }
                        Some(Err(e)) => {
                            warn!("Relay WebSocket error: {}", e);
                            self.connected = false;
                            return Err(anyhow::anyhow!("WebSocket error: {}", e));
                        }
                        _ => {}
                    }
                }

                // Outgoing messages to relay
                Some(msg) = rx.recv() => {
                    let text = serde_json::to_string(&msg).unwrap();
                    if let Err(e) = write.send(tokio_tungstenite::tungstenite::Message::Text(text)).await {
                        warn!("Failed to send to relay: {}", e);
                        self.connected = false;
                        return Err(anyhow::anyhow!("Send failed: {}", e));
                    }
                }

                // Heartbeat
                _ = heartbeat.tick() => {
                    let ping = TunnelMessage::Ping {
                        timestamp: chrono::Utc::now().timestamp(),
                    };
                    let text = serde_json::to_string(&ping).unwrap();
                    if let Err(e) = write.send(tokio_tungstenite::tungstenite::Message::Text(text)).await {
                        warn!("Heartbeat failed: {}", e);
                        self.connected = false;
                        return Err(anyhow::anyhow!("Heartbeat failed: {}", e));
                    }
                }
            }
        }
    }

    async fn handle_message(
        &self,
        msg: TunnelMessage,
        tx: &mpsc::UnboundedSender<TunnelMessage>,
    ) {
        match msg {
            TunnelMessage::Pong { timestamp } => {
                let latency = chrono::Utc::now().timestamp() - timestamp;
                debug!("Relay heartbeat latency: {}ms", latency);
            }
            TunnelMessage::QueryRequest { request_id, share_code, token, sql, limit, offset } => {
                debug!("Tunnel query request: {} for share {}", request_id, share_code);
                
                let result = if let Some(ref cm) = self.connection_manager {
                    // Execute via ConnectionManager
                    self.execute_tunnel_query(cm, &share_code, &token, &sql, limit, offset).await
                } else {
                    Err(anyhow::anyhow!("ConnectionManager not attached to tunnel client"))
                };

                let response = match result {
                    Ok(data) => TunnelMessage::QueryResponse {
                        request_id,
                        result: data,
                    },
                    Err(e) => TunnelMessage::QueryResponse {
                        request_id,
                        result: serde_json::json!({
                            "success": false,
                            "error": format!("Tunnel query failed: {}", e)
                        }),
                    },
                };
                let _ = tx.send(response);
            }
            TunnelMessage::SchemaRequest { request_id, share_code, token } => {
                debug!("Tunnel schema request: {} for share {}", request_id, share_code);
                
                let result = if let Some(ref cm) = self.connection_manager {
                    self.execute_tunnel_schema(cm, &share_code, &token).await
                } else {
                    Err(anyhow::anyhow!("ConnectionManager not attached to tunnel client"))
                };

                let response = match result {
                    Ok(data) => TunnelMessage::SchemaResponse {
                        request_id,
                        schema: data,
                    },
                    Err(e) => TunnelMessage::SchemaResponse {
                        request_id,
                        schema: serde_json::json!({
                            "success": false,
                            "error": format!("Tunnel schema failed: {}", e)
                        }),
                    },
                };
                let _ = tx.send(response);
            }
            _ => {
                debug!("Unhandled tunnel message: {:?}", msg);
            }
        }
    }

    /// Execute a query via ConnectionManager for tunnel requests
    async fn execute_tunnel_query(
        &self,
        cm: &Arc<tokio::sync::Mutex<crate::control_plane::connection::manager::ConnectionManager>>,
        share_code: &str,
        token: &str,
        sql: &str,
        _limit: Option<i32>,
        _offset: Option<i32>,
    ) -> anyhow::Result<serde_json::Value> {
        // Validate token
        let validated = {
            let tm = self.token_manager.read().await;
            tm.validate_token(token)
                .map_err(|e| anyhow::anyhow!("Invalid token: {}", e))?
        };

        if validated.code != share_code {
            return Err(anyhow::anyhow!("Token code mismatch"));
        }

        // Get share record
        let record = self.share_store.get_share(share_code).await
            .map_err(|e| anyhow::anyhow!("Share lookup failed: {}", e))?
            .ok_or_else(|| anyhow::anyhow!("Share not found"))?;

        if record.revoked {
            return Err(anyhow::anyhow!("Share revoked"));
        }
        if record.expires_at < chrono::Utc::now() {
            return Err(anyhow::anyhow!("Share expired"));
        }

        // Check permissions
        if validated.permission.as_str() == "ro" && !sql.trim().to_uppercase().starts_with("SELECT") {
            return Err(anyhow::anyhow!("Write not permitted on read-only share"));
        }

        // Execute via ConnectionManager
        let conn = cm.lock().await;
        let result = conn.execute(&record.db_id, sql).await
            .map_err(|e| anyhow::anyhow!("Query execution failed: {}", e))?;

        Ok(serde_json::json!({
            "success": true,
            "columns": result.columns,
            "rows": result.rows,
            "row_count": result.row_count,
            "last_insert_id": result.last_insert_id,
        }))
    }

    /// Get schema via ConnectionManager for tunnel requests
    async fn execute_tunnel_schema(
        &self,
        cm: &Arc<tokio::sync::Mutex<crate::control_plane::connection::manager::ConnectionManager>>,
        share_code: &str,
        token: &str,
    ) -> anyhow::Result<serde_json::Value> {
        let validated = {
            let tm = self.token_manager.read().await;
            tm.validate_token(token)
                .map_err(|e| anyhow::anyhow!("Invalid token: {}", e))?
        };

        if validated.code != share_code {
            return Err(anyhow::anyhow!("Token code mismatch"));
        }

        let record = self.share_store.get_share(share_code).await
            .map_err(|e| anyhow::anyhow!("Share lookup failed: {}", e))?
            .ok_or_else(|| anyhow::anyhow!("Share not found"))?;

        if record.revoked {
            return Err(anyhow::anyhow!("Share revoked"));
        }
        if record.expires_at < chrono::Utc::now() {
            return Err(anyhow::anyhow!("Share expired"));
        }

        let conn = cm.lock().await;
        let tables = conn.get_schema(&record.db_id).await
            .map_err(|e| anyhow::anyhow!("Schema fetch failed: {}", e))?;

        Ok(serde_json::json!({
            "success": true,
            "tables": tables,
            "database_name": record.db_id,
        }))
    }

    /// Notify relay that a new share was created
    pub async fn notify_share_created(&self, code: &str, db_id: &str, permission: &str, expires_at: i64) {
        if let Some(tx) = &self.ws_tx {
            let msg = TunnelMessage::ShareCreated {
                code: code.to_string(),
                db_id: db_id.to_string(),
                permission: permission.to_string(),
                expires_at,
            };
            let _ = tx.send(msg);
        }
    }

    /// Notify relay that a share was revoked
    pub async fn notify_share_revoked(&self, code: &str) {
        if let Some(tx) = &self.ws_tx {
            let msg = TunnelMessage::ShareRevoked {
                code: code.to_string(),
            };
            let _ = tx.send(msg);
        }
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }
}

/// Start tunnel as background task
/// NOTE: Call this from main.rs where AppState (and thus ConnectionManager) is available
pub async fn start_relay_tunnel(
    relay_url: String,
    host_id: String,
    token_manager: Arc<RwLock<ShareTokenManager>>,
    share_store: Arc<ShareStore>,
    connection_manager: Option<Arc<tokio::sync::Mutex<crate::control_plane::connection::manager::ConnectionManager>>>,
) -> anyhow::Result<mpsc::UnboundedSender<TunnelMessage>> {
    let (tx, _rx) = mpsc::unbounded_channel::<TunnelMessage>();

    let mut client = RelayTunnelClient::new(
        relay_url,
        host_id,
        token_manager,
        share_store,
    );

    // Attach ConnectionManager if provided
    if let Some(cm) = connection_manager {
        client = client.with_connection_manager(cm);
    }

    // Spawn connection loop
    tokio::spawn(async move {
        if let Err(e) = client.run().await {
            error!("Relay tunnel task ended: {}", e);
        }
    });

    // Return sender for external use
    Ok(tx)
}

