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
use crate::models::database::DatabaseInstance;
use crate::sharing::multiplex::wire_bridge::{WireBridgeRegistry, WireFrameSender};
use crate::sharing::multiplex::wire_frame::{decode_wire_frame, WIRE_FRAME_TYPE_DATA};

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
    /// New API key registered — notify relay so it can route bearer-token requests
    ApiKeyRegistered {
        key_hash: String,
        db_id: String,
        permission: String,
    },
    /// API key revoked — notify relay to stop routing it
    ApiKeyRevoked {
        key_hash: String,
    },
    /// API key query request from relay (external app via /api/v1/query)
    ApiKeyQueryRequest {
        request_id: String,
        key_hash: String,
        sql: String,
        limit: Option<i32>,
        offset: Option<i32>,
    },
    /// API key query response back to relay
    ApiKeyQueryResponse {
        request_id: String,
        result: serde_json::Value,
    },
    /// API key schema request from relay
    ApiKeySchemaRequest {
        request_id: String,
        key_hash: String,
    },
    /// API key schema response back to relay
    ApiKeySchemaResponse {
        request_id: String,
        schema: serde_json::Value,
    },
    /// Open a new tunneled wire-protocol (MySQL/Postgres) byte stream —
    /// sent from relay to engine when a client's wire handshake completes.
    /// Bulk data itself travels as raw binary WebSocket frames, not this
    /// JSON message; this only carries stream lifecycle + auth.
    WireStreamOpen {
        stream_id: String,
        wire_username: String,
        wire_password_hash: String,
    },
    /// Engine confirms a wire stream was opened successfully
    WireStreamOpened {
        stream_id: String,
    },
    /// Engine reports a wire stream failed to open (bad credentials, db unreachable, etc.)
    WireStreamError {
        stream_id: String,
        message: String,
    },
    /// Either side signals a tunneled wire stream has ended
    WireStreamClose {
        stream_id: String,
    },
}

/// Relay tunnel client state
pub struct RelayTunnelClient {
    relay_url: String,
    host_id: String,
    token_manager: Arc<RwLock<ShareTokenManager>>,
    share_store: Arc<ShareStore>,
    connection_manager: Option<Arc<tokio::sync::Mutex<crate::control_plane::connection::manager::ConnectionManager>>>,
    databases: Option<Arc<std::sync::Mutex<Vec<DatabaseInstance>>>>,
    rate_limiter: Option<Arc<crate::rate_limit::RateLimitService>>,
    ws_tx: Option<mpsc::UnboundedSender<TunnelMessage>>,
    connected: bool,
    /// Registry of active tunneled wire-protocol (MySQL/Postgres) byte streams
    wire_registry: Arc<WireBridgeRegistry>,
    /// Sends already-framed binary payloads out over the tunnel (engine -> relay)
    wire_out_tx: Option<WireFrameSender>,
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
            databases: None,
            rate_limiter: None,
            ws_tx: None,
            connected: false,
            wire_registry: WireBridgeRegistry::new(),
            wire_out_tx: None,
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

    /// Attach the databases list so tunnel schema responses can include real name/type
    pub fn with_databases(
        mut self,
        databases: Arc<std::sync::Mutex<Vec<DatabaseInstance>>>,
    ) -> Self {
        self.databases = Some(databases);
        self
    }

    /// Attach the engine's rate limiter for defense-in-depth on API key
    /// queries (in addition to the relay's own per-key rate limiting)
    pub fn with_rate_limiter(
        mut self,
        rate_limiter: Arc<crate::rate_limit::RateLimitService>,
    ) -> Self {
        self.rate_limiter = Some(rate_limiter);
        self
    }

    /// Attach the sender used to push tunneled wire-protocol binary frames
    /// out to the relay (Phase 2: MySQL/Postgres byte-stream tunneling)
    pub fn with_wire_out_tx(mut self, wire_out_tx: WireFrameSender) -> Self {
        self.wire_out_tx = Some(wire_out_tx);
        self
    }

    /// Start the tunnel — connects and maintains connection with auto-reconnect
    /// `tx`/`rx` are the SAME channel returned to the caller (start_relay_tunnel),
    /// so external sends (notify_share_created, etc.) actually reach the socket.
    pub async fn run(
        &mut self,
        tx: mpsc::UnboundedSender<TunnelMessage>,
        mut rx: mpsc::UnboundedReceiver<TunnelMessage>,
        mut wire_out_rx: mpsc::UnboundedReceiver<Vec<u8>>,
    ) -> anyhow::Result<()> {
        self.ws_tx = Some(tx.clone());

        let mut reconnect_delay = Duration::from_secs(1);
        let max_reconnect_delay = Duration::from_secs(60);

        loop {
            match self.connect_and_maintain(&tx, &mut rx, &mut wire_out_rx).await {
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

    async fn connect_and_maintain(
        &mut self,
        tx: &mpsc::UnboundedSender<TunnelMessage>,
        rx: &mut mpsc::UnboundedReceiver<TunnelMessage>,
        wire_out_rx: &mut mpsc::UnboundedReceiver<Vec<u8>>,
    ) -> anyhow::Result<()> {
        // Support both URL formats: with or without /ws/tunnel prefix
        let base = if self.relay_url.ends_with("/ws/tunnel") {
            self.relay_url.clone()
        } else if self.relay_url.ends_with("/tunnel") {
            self.relay_url.replace("/tunnel", "/ws/tunnel")
        } else {
            format!("{}/ws/tunnel", self.relay_url.trim_end_matches('/'))
        };
        let ws_url = format!("{}/{}", base, self.host_id);
        info!("Connecting to relay tunnel: {}", ws_url);

        let (ws_stream, _) = tokio_tungstenite::connect_async(&ws_url).await
            .map_err(|e| anyhow::anyhow!("WebSocket connect failed: {}", e))?;

        let (mut write, mut read) = ws_stream.split();

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

        // Re-sync all active shares from persistent store so relay learns about them after reconnect
        if let Ok(shares) = self.share_store.list_all_active().await {
            for share in shares {
                if share.revoked || share.expires_at < chrono::Utc::now() {
                    continue;
                }
                let code = share.code.clone();
                let msg = TunnelMessage::ShareCreated {
                    code: share.code,
                    db_id: share.db_id,
                    permission: share.permission,
                    expires_at: share.expires_at.timestamp(),
                };
                if let Err(e) = tx.send(msg) {
                    warn!("Failed to re-sync share to relay: {}", e);
                } else {
                    info!("Re-synced share {} to relay after reconnect", code);
                }
            }
        }

        // Re-sync all active API keys so the relay can route bearer-token
        // requests immediately after reconnect (same pattern as shares above)
        if let Ok(keys) = self.share_store.list_all_active_api_keys().await {
            for key in keys {
                let msg = TunnelMessage::ApiKeyRegistered {
                    key_hash: key.key_hash.clone(),
                    db_id: key.db_id.clone(),
                    permission: key.permission.clone(),
                };
                if let Err(e) = tx.send(msg) {
                    warn!("Failed to re-sync API key to relay: {}", e);
                } else {
                    info!("Re-synced API key '{}' to relay after reconnect", key.name);
                }
            }
        }

        // Heartbeat interval
        let mut heartbeat = interval(Duration::from_secs(30));

        loop {
            tokio::select! {
                // Incoming messages from relay
                msg = read.next() => {
                    match msg {
                        Some(Ok(tokio_tungstenite::tungstenite::Message::Text(text))) => {
                            if let Ok(tunnel_msg) = serde_json::from_str::<TunnelMessage>(&text) {
                                self.handle_message(tunnel_msg, tx).await;
                            }
                        }
                        Some(Ok(tokio_tungstenite::tungstenite::Message::Binary(data))) => {
                            if let Some((msg_type, stream_id, payload)) = decode_wire_frame(&data) {
                                if msg_type == WIRE_FRAME_TYPE_DATA {
                                    if !self.wire_registry.route_inbound(stream_id, payload.to_vec()) {
                                        debug!("Wire frame for unknown/closed stream {}", stream_id);
                                    }
                                }
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

                // Outgoing wire-protocol binary frames (Phase 2: tunneled MySQL/Postgres bytes)
                Some(frame) = wire_out_rx.recv() => {
                    if let Err(e) = write.send(tokio_tungstenite::tungstenite::Message::Binary(frame)).await {
                        warn!("Failed to send wire binary frame to relay: {}", e);
                        self.connected = false;
                        return Err(anyhow::anyhow!("Wire binary send failed: {}", e));
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
            TunnelMessage::ApiKeyQueryRequest { request_id, key_hash, sql, limit, offset } => {
                debug!("Tunnel API key query request: {}", request_id);

                let result = if let Some(ref cm) = self.connection_manager {
                    self.execute_tunnel_api_key_query(cm, &key_hash, &sql, limit, offset).await
                } else {
                    Err(anyhow::anyhow!("ConnectionManager not attached to tunnel client"))
                };

                let response = match result {
                    Ok(data) => TunnelMessage::ApiKeyQueryResponse { request_id, result: data },
                    Err(e) => TunnelMessage::ApiKeyQueryResponse {
                        request_id,
                        result: serde_json::json!({
                            "success": false,
                            "error": format!("API key query failed: {}", e)
                        }),
                    },
                };
                let _ = tx.send(response);
            }
            TunnelMessage::ApiKeySchemaRequest { request_id, key_hash } => {
                debug!("Tunnel API key schema request: {}", request_id);

                let result = if let Some(ref cm) = self.connection_manager {
                    self.execute_tunnel_api_key_schema(cm, &key_hash).await
                } else {
                    Err(anyhow::anyhow!("ConnectionManager not attached to tunnel client"))
                };

                let response = match result {
                    Ok(data) => TunnelMessage::ApiKeySchemaResponse { request_id, schema: data },
                    Err(e) => TunnelMessage::ApiKeySchemaResponse {
                        request_id,
                        schema: serde_json::json!({
                            "success": false,
                            "error": format!("API key schema failed: {}", e)
                        }),
                    },
                };
                let _ = tx.send(response);
            }
            TunnelMessage::WireStreamOpen { stream_id, wire_username, wire_password_hash } => {
                debug!("Wire stream open request: {}", stream_id);

                let databases = match &self.databases {
                    Some(d) => d.clone(),
                    None => {
                        let _ = tx.send(TunnelMessage::WireStreamError {
                            stream_id,
                            message: "Engine databases not attached to tunnel client".to_string(),
                        });
                        return;
                    }
                };
                let wire_out_tx = match &self.wire_out_tx {
                    Some(w) => w.clone(),
                    None => {
                        let _ = tx.send(TunnelMessage::WireStreamError {
                            stream_id,
                            message: "Engine wire-out channel not attached".to_string(),
                        });
                        return;
                    }
                };

                let share_store = self.share_store.clone();
                let registry = self.wire_registry.clone();
                let tx = tx.clone();
                let stream_id_for_result = stream_id.clone();

                tokio::spawn(async move {
                    match crate::sharing::multiplex::wire_bridge::open_wire_stream(
                        share_store, databases, registry, stream_id.clone(),
                        wire_username, wire_password_hash, wire_out_tx,
                    ).await {
                        Ok(_) => {
                            let _ = tx.send(TunnelMessage::WireStreamOpened { stream_id: stream_id_for_result });
                        }
                        Err(e) => {
                            let _ = tx.send(TunnelMessage::WireStreamError { stream_id: stream_id_for_result, message: e });
                        }
                    }
                });
            }
            TunnelMessage::WireStreamClose { stream_id } => {
                debug!("Wire stream close: {}", stream_id);
                self.wire_registry.remove(&stream_id);
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

        // Look up the real database name/type so the guest UI shows
        // e.g. "MyShop AGMVFDINWM" instead of the raw db_id
        let (db_name, db_type) = if let Some(dbs) = &self.databases {
            let dbs = dbs.lock().unwrap();
            dbs.iter()
                .find(|d| d.id == record.db_id)
                .map(|d| (d.name.clone(), d.db_type.clone()))
                .unwrap_or_else(|| (record.db_id.clone(), "unknown".to_string()))
        } else {
            (record.db_id.clone(), "unknown".to_string())
        };

        // Match the shape the frontend expects from the direct-engine path:
        // { success, data: { tables, databaseName, databaseType } }
        Ok(serde_json::json!({
            "success": true,
            "data": {
                "tables": tables,
                "databaseName": db_name,
                "databaseType": db_type,
            }
        }))
    }

    /// Execute a query authenticated by a durable API key (not a share JWT).
    /// Applies the key's own row limit and timeout — independent of the
    /// defaults used for ephemeral shares.
    async fn execute_tunnel_api_key_query(
        &self,
        cm: &Arc<tokio::sync::Mutex<crate::control_plane::connection::manager::ConnectionManager>>,
        key_hash: &str,
        sql: &str,
        limit: Option<i32>,
        _offset: Option<i32>,
    ) -> anyhow::Result<serde_json::Value> {
        let key = self.share_store.get_api_key_by_hash(key_hash).await
            .map_err(|e| anyhow::anyhow!("API key lookup failed: {}", e))?
            .ok_or_else(|| anyhow::anyhow!("Invalid or revoked API key"))?;

        // Engine-side rate limit — defense in depth behind the relay's own
        // per-key limiter. Keyed by key_hash; per-IP diversity isn't visible
        // at this hop since requests arrive over the shared relay tunnel.
        if let Some(ref limiter) = self.rate_limiter {
            let dummy_ip: std::net::IpAddr = std::net::IpAddr::from([0, 0, 0, 0]);
            if let Err(e) = limiter.check(key_hash, &dummy_ip).await {
                return Err(anyhow::anyhow!(e));
            }
        }

        if let Err(e) = crate::api::http::validate_sql(sql) {
            return Err(anyhow::anyhow!(e));
        }

        let permission = crate::auth::share_token::SharePermission::from_str(&key.permission);
        if !permission.can_write() && !sql.trim().to_uppercase().starts_with("SELECT") {
            return Err(anyhow::anyhow!("Write not permitted for this API key"));
        }

        // Apply this key's own row limit (independent of share defaults)
        let requested_limit = limit.unwrap_or(key.max_rows).clamp(1, key.max_rows.max(1));
        let is_select = sql.trim().to_uppercase().starts_with("SELECT") || sql.trim().to_uppercase().starts_with("WITH");
        let final_sql = if is_select && !sql.to_uppercase().contains("LIMIT") {
            format!("{} LIMIT {}", sql, requested_limit)
        } else {
            sql.to_string()
        };

        let timeout = std::time::Duration::from_secs(key.timeout_secs.max(1) as u64);
        let conn = cm.lock().await;
        let result = tokio::time::timeout(timeout, conn.execute(&key.db_id, &final_sql))
            .await
            .map_err(|_| anyhow::anyhow!("Query timed out after {}s", key.timeout_secs))?
            .map_err(|e| anyhow::anyhow!("Query execution failed: {}", e))?;
        drop(conn);

        let _ = self.share_store.touch_api_key(key_hash).await;

        Ok(serde_json::json!({
            "success": true,
            "columns": result.columns,
            "rows": result.rows,
            "row_count": result.row_count,
            "last_insert_id": result.last_insert_id,
        }))
    }

    /// Get schema authenticated by a durable API key
    async fn execute_tunnel_api_key_schema(
        &self,
        cm: &Arc<tokio::sync::Mutex<crate::control_plane::connection::manager::ConnectionManager>>,
        key_hash: &str,
    ) -> anyhow::Result<serde_json::Value> {
        let key = self.share_store.get_api_key_by_hash(key_hash).await
            .map_err(|e| anyhow::anyhow!("API key lookup failed: {}", e))?
            .ok_or_else(|| anyhow::anyhow!("Invalid or revoked API key"))?;

        let conn = cm.lock().await;
        let tables = conn.get_schema(&key.db_id).await
            .map_err(|e| anyhow::anyhow!("Schema fetch failed: {}", e))?;
        drop(conn);

        let (db_name, db_type) = if let Some(dbs) = &self.databases {
            let dbs = dbs.lock().unwrap();
            dbs.iter()
                .find(|d| d.id == key.db_id)
                .map(|d| (d.name.clone(), d.db_type.clone()))
                .unwrap_or_else(|| (key.db_id.clone(), "unknown".to_string()))
        } else {
            (key.db_id.clone(), "unknown".to_string())
        };

        Ok(serde_json::json!({
            "success": true,
            "data": {
                "tables": tables,
                "databaseName": db_name,
                "databaseType": db_type,
            }
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
    databases: Option<Arc<std::sync::Mutex<Vec<DatabaseInstance>>>>,
    rate_limiter: Option<Arc<crate::rate_limit::RateLimitService>>,
) -> anyhow::Result<mpsc::UnboundedSender<TunnelMessage>> {
    let (tx, rx) = mpsc::unbounded_channel::<TunnelMessage>();
    // Phase 2: dedicated channel for tunneled wire-protocol binary frames
    // (MySQL/Postgres bytes), kept separate from the JSON control channel
    // above so bulk transfer never touches JSON/base64 encoding.
    let (wire_out_tx, wire_out_rx) = mpsc::unbounded_channel::<Vec<u8>>();

    let mut client = RelayTunnelClient::new(
        relay_url,
        host_id,
        token_manager,
        share_store,
    ).with_wire_out_tx(wire_out_tx);

    // Attach ConnectionManager if provided
    if let Some(cm) = connection_manager {
        client = client.with_connection_manager(cm);
    }

    // Attach databases list so schema responses include real name/type
    if let Some(dbs) = databases {
        client = client.with_databases(dbs);
    }

    // Attach rate limiter for API-key query throttling (defense in depth)
    if let Some(rl) = rate_limiter {
        client = client.with_rate_limiter(rl);
    }

    let tx_for_caller = tx.clone();

    // Spawn connection loop — pass the REAL tx/rx pair in, not a disconnected one
    tokio::spawn(async move {
        if let Err(e) = client.run(tx, rx, wire_out_rx).await {
            error!("Relay tunnel task ended: {}", e);
        }
    });

    // Return sender for external use (create_share, revoke_share, etc.)
    Ok(tx_for_caller)
}

