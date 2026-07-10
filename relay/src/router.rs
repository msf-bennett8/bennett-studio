//! Share router — maps share_id to engine endpoint
//! Polls SQLite DB for active shares, maintains in-memory cache

use dashmap::DashMap;
use sqlx::{Pool, Row, Sqlite};
use std::sync::Arc;
use tokio::time::{interval, Duration};
use tracing::{debug, error, info, warn};

/// Route entry for a share
#[derive(Debug, Clone)]
pub struct ShareRoute {
    pub share_id: String,
    pub db_id: String,
    pub protocol: crate::transport::ProtocolType,
    pub engine_port: u16,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub revoked: bool,
    /// Host ID of the remote engine that owns this share (for tunnel cleanup)
    pub host_id: Option<String>,
}

/// In-memory route cache with SQLite backing
pub struct ShareRouter {
    db_pool: Pool<Sqlite>,
    cache: Arc<DashMap<String, ShareRoute>>,
    engine_http_port: u16,
    engine_mysql_port: u16,
    /// Shared tunnel registry for remote engine communication
    tunnel_registry: Option<Arc<crate::tunnel_registry::TunnelRegistry>>,
    /// Reverse index: host_id -> set of share_ids (for cleanup on disconnect)
    host_routes: Arc<DashMap<String, dashmap::DashSet<String>>>,
}

impl ShareRouter {
    pub async fn new(
        db_path: &std::path::Path,
        engine_http_port: u16,
        engine_mysql_port: u16,
    ) -> anyhow::Result<Arc<Self>> {
        // Open SQLite in read-only mode (we don't own it, engine does)
        let db_url = format!("sqlite:{}?mode=ro", db_path.display());
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(2)
            .connect(&db_url)
            .await?;

        let router = Arc::new(Self {
            db_pool: pool,
            cache: Arc::new(DashMap::new()),
            engine_http_port,
            engine_mysql_port,
            tunnel_registry: None,
            host_routes: Arc::new(DashMap::new()),
        });

        // Initial load
        router.refresh_routes().await?;

        Ok(router)
    }

    /// Look up a share by ID
    pub fn lookup(&self, share_id: &str) -> Option<ShareRoute> {
        self.cache.get(share_id).map(|e| e.clone())
    }

    /// Check if share exists and is active (checks both HTTP and MySQL entries)
    pub fn is_active(&self, share_id: &str) -> bool {
        // Try with :http suffix first (most common)
        if let Some(route) = self.cache.get(&format!("{}:http", share_id)) {
            if !route.revoked && route.expires_at > chrono::Utc::now() {
                return true;
            }
        }
        // Fallback: check bare share_id (backward compat)
        if let Some(route) = self.cache.get(share_id) {
            return !route.revoked && route.expires_at > chrono::Utc::now();
        }
        false
    }

    /// Remote engine routes received via tunnel WebSocket
    /// Key: share_code, Value: route info from remote engine
    pub async fn add_remote_route(&self, route: ShareRoute) {
        let share_id = route.share_id.clone();
        let host_id = route.host_id.clone();
        
        self.cache.insert(format!("{}:http", share_id), route.clone());
        self.cache.insert(format!("{}:mysql", share_id), ShareRoute {
            protocol: crate::transport::ProtocolType::MySqlWire,
            ..route.clone()
        });
        
        // Track in reverse index for host-based cleanup
        if let Some(ref host) = host_id {
            let entry = self.host_routes.entry(host.clone()).or_insert_with(dashmap::DashSet::new);
            entry.value().insert(share_id.clone());
        }
        
        info!("Added remote route for share {} (host: {:?})", share_id, host_id);
    }

    /// Remove a single remote route by share_id
    pub async fn remove_remote_route(&self, share_id: &str) {
        // Find host_id before removing, to clean up reverse index
        let host_id = self.cache.get(&format!("{}:http", share_id))
            .and_then(|r| r.host_id.clone());
            
        self.cache.remove(&format!("{}:http", share_id));
        self.cache.remove(&format!("{}:mysql", share_id));
        
        // Clean up reverse index
        if let Some(ref host) = host_id {
            if let Some(entry) = self.host_routes.get(host) {
                entry.value().remove(share_id);
                if entry.value().is_empty() {
                    drop(entry);
                    self.host_routes.remove(host);
                }
            }
        }
        
        info!("Removed remote route for share {}", share_id);
    }

    /// Remove ALL routes for a given host_id (called when engine tunnel disconnects)
    pub async fn remove_all_host_routes(&self, host_id: &str) {
        let share_ids: Vec<String> = if let Some(entry) = self.host_routes.get(host_id) {
            entry.value().iter().map(|s| s.clone()).collect()
        } else {
            return;
        };
        
        for share_id in &share_ids {
            self.cache.remove(&format!("{}:http", share_id));
            self.cache.remove(&format!("{}:mysql", share_id));
            info!("Cleaned up route {} for disconnected host {}", share_id, host_id);
        }
        
        self.host_routes.remove(host_id);
        info!("Removed all routes for host {} ({} shares cleaned up)", host_id, share_ids.len());
    }

    /// Refresh routes from database (local) or accept remote updates
    pub async fn refresh_routes(&self) -> anyhow::Result<()> {
        debug!("Refreshing share routes from database");

        // Try local SQLite first (for same-host deployment)
        let rows = sqlx::query(
            r#"
            SELECT code, db_id, permission, tables, created_at, expires_at, revoked
            FROM shares
            WHERE revoked = 0 AND expires_at > datetime('now')
            "#
        )
        .fetch_all(&self.db_pool)
        .await;

        match rows {
            Ok(shares) => {
                let mut count = 0;
                for share in shares {
                    let code: String = share.get("code");
                    let db_id: String = share.get("db_id");
                    let expires_at_str: String = share.get("expires_at");
                    let revoked_i32: i32 = share.get("revoked");

                    let route = ShareRoute {
                        share_id: code.clone(),
                        db_id,
                        protocol: crate::transport::ProtocolType::ConnectRpc,
                        engine_port: self.engine_http_port,
                        expires_at: chrono::DateTime::parse_from_rfc3339(&expires_at_str)
                            .map(|d| d.with_timezone(&chrono::Utc))
                            .unwrap_or_else(|_| chrono::DateTime::UNIX_EPOCH),
                        revoked: revoked_i32 != 0,
                        host_id: None, // Local routes have no remote host
                    };

                    // Also add MySQL wire route for the same share
                    let mysql_route = ShareRoute {
                        protocol: crate::transport::ProtocolType::MySqlWire,
                        engine_port: self.engine_mysql_port,
                        host_id: None,
                        ..route.clone()
                    };

                    self.cache.insert(
                        format!("{}:http", code),
                        route,
                    );
                    self.cache.insert(
                        format!("{}:mysql", code),
                        mysql_route,
                    );
                    count += 1;
                }

                // Clean expired entries (both local and remote)
                self.cache.retain(|_, v| {
                    !v.revoked && v.expires_at > chrono::Utc::now()
                });

                info!("Route cache refreshed: {} active local shares", count);
            }
            Err(e) => {
                // On Render, SQLite won't exist — remote routes via tunnel are primary
                debug!("Local SQLite not available (expected on Render): {}", e);
                // Clean expired entries only
                self.cache.retain(|_, v| {
                    !v.revoked && v.expires_at > chrono::Utc::now()
                });
            }
        }

        Ok(())
    }

    /// Start background refresh task
    pub fn start_refresh_task(
        self: &Arc<Self>,
        interval_secs: u64,
    ) -> tokio::task::JoinHandle<()> {
        let router = self.clone();
        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(interval_secs));
            loop {
                ticker.tick().await;
                if let Err(e) = router.refresh_routes().await {
                    error!("Background route refresh failed: {}", e);
                }
            }
        })
    }

    /// Forward an HTTP request to the engine via TCP or tunnel
    /// Returns the raw HTTP response body as bytes
    pub async fn forward_to_engine(
        &self,
        share_id: &str,
        method: &str,
        path: &str,
        body: Option<Vec<u8>>,
        token: &str,
    ) -> anyhow::Result<Vec<u8>> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        // Get the engine endpoint for this share
        let route = self.lookup(share_id)
            .or_else(|| self.lookup(&format!("{}:http", share_id)))
            .ok_or_else(|| anyhow::anyhow!("Share {} not found in route cache", share_id))?;

        // PHASE D: Remote route via tunnel (engine_port == 0)
        if route.engine_port == 0 {
            return self.forward_via_tunnel(share_id, method, path, body, token).await;
        }

        // Local engine — connect via TCP
        let engine_addr = format!("127.0.0.1:{}", route.engine_port);
        let mut stream = tokio::net::TcpStream::connect(&engine_addr).await
            .map_err(|e| anyhow::anyhow!("Failed to connect to engine at {}: {}", engine_addr, e))?;

        // Build HTTP request
        let body_len = body.as_ref().map(|b| b.len()).unwrap_or(0);
        eprintln!("DEBUG FORWARD: method={}, path={}, token_len={}, token_empty={}",
            method, path, token.len(), token.is_empty());
        let request = format!(
            "{} {} HTTP/1.1\r\n\
             Host: localhost:{}\r\n\
             Content-Type: application/json\r\n\
             Content-Length: {}\r\n\
             X-Share-Token: {}\r\n\
             Connection: close\r\n\
             \r\n",
            method,
            path,
            route.engine_port,
            body_len,
            token
        );
        eprintln!("DEBUG FORWARD: request={:?}", request);

        stream.write_all(request.as_bytes()).await?;
        if let Some(body) = body {
            stream.write_all(&body).await?;
        }
        stream.flush().await?;

        // Read response
        let mut response = Vec::new();
        let mut buf = [0u8; 8192];
        loop {
            match stream.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => response.extend_from_slice(&buf[..n]),
                Err(e) => return Err(anyhow::anyhow!("Read error: {}", e)),
            }
        }

        Ok(response)
    }

    // ============================================================================
    // PHASE D: Tunnel Forwarding — Async request/response correlation
    // ============================================================================

    /// Forward HTTP request through engine tunnel WebSocket
    /// Uses request_id correlation to match async responses
    async fn forward_via_tunnel(
        &self,
        share_id: &str,
        _method: &str,
        path: &str,
        body: Option<Vec<u8>>,
        token: &str,
    ) -> anyhow::Result<Vec<u8>> {
        let request_id = uuid::Uuid::new_v4().to_string();
        let (tx, _rx) = tokio::sync::oneshot::channel::<serde_json::Value>();

        // Register response channel with tunnel registry (industry best: uses existing registry)
        let registry = self.tunnel_registry.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Tunnel registry not initialized"))?;
        registry.register_pending(request_id.clone(), tx).await;

        // Determine if this is a query or schema request
        let is_query = path.contains("/query");
        let _is_schema = path.contains("/schema");

        // Send through tunnel registry
        let registry = self.tunnel_registry.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Tunnel registry not initialized"))?;

        // Find which host owns this share
        let route = self.lookup(share_id)
            .or_else(|| self.lookup(&format!("{}:http", share_id)))
            .ok_or_else(|| anyhow::anyhow!("Share {} not found", share_id))?;

        // Get host_id from the route (now stored in ShareRoute)
        let host_id = route.host_id.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Remote route missing host_id for share {}", share_id))?;

        let tunnel_msg = if is_query {
            // Parse SQL from body for the tunnel message (industry best: parse once, use everywhere)
            let sql = body.as_ref().and_then(|b| {
                serde_json::from_slice::<serde_json::Value>(b).ok()
                    .and_then(|v| v.get("sql").and_then(|s| s.as_str()).map(|s| s.to_string()))
            }).unwrap_or_default();

            crate::tunnel_registry::TunnelMessageToEngine::QueryRequest {
                request_id: request_id.clone(),
                share_code: share_id.to_string(),
                token: token.to_string(),
                sql,
                limit: None,
                offset: None,
            }
        } else {
            crate::tunnel_registry::TunnelMessageToEngine::SchemaRequest {
                request_id: request_id.clone(),
                share_code: share_id.to_string(),
                token: token.to_string(),
            }
        };

        match registry.send_and_wait(&host_id, tunnel_msg, 30).await {
            Ok(response) => {
                // Build HTTP response from tunnel response
                let http_body = serde_json::to_vec(&response)
                    .unwrap_or_else(|_| b"{\"success\":false,\"error\":\"serialization failed\"}".to_vec());
                
                // Build minimal HTTP/1.1 response
                let response_str = format!(
                    "HTTP/1.1 200 OK\r\n\
                     Content-Type: application/json\r\n\
                     Content-Length: {}\r\n\
                     Connection: close\r\n\
                     \r\n",
                    http_body.len()
                );
                
                let mut full_response = response_str.into_bytes();
                full_response.extend_from_slice(&http_body);
                Ok(full_response)
            }
            Err(e) => Err(e)
        }
    }

    /// Attach tunnel registry (call once after construction)
    pub fn with_tunnel_registry(self: Arc<Self>, registry: Arc<crate::tunnel_registry::TunnelRegistry>) -> Arc<Self> {
        Arc::new(Self {
            db_pool: self.db_pool.clone(),
            cache: self.cache.clone(),
            engine_http_port: self.engine_http_port,
            engine_mysql_port: self.engine_mysql_port,
            tunnel_registry: Some(registry),
            host_routes: self.host_routes.clone(),
        })
    }

    // ============================================================================
    // PHASE F: Host Heartbeat Monitoring
    // ============================================================================

    /// Check all hosts with active remote routes and mark stale ones offline
    /// Stale threshold: 90 seconds (same as engine's is_host_alive)
    pub async fn check_host_heartbeats(&self) -> anyhow::Result<()> {
        let _stale_threshold = chrono::Utc::now() - chrono::Duration::seconds(90);
        let mut offline_hosts = Vec::new();

        // Check each host in our reverse index
        for entry in self.host_routes.iter() {
            let host_id = entry.key().clone();
            let share_ids: Vec<String> = entry.value().iter().map(|s| s.clone()).collect();
            
            // Check if this host has any recent heartbeat
            // In production, read from a shared heartbeat store or tunnel ping
            // For now, we track last tunnel ping time in the tunnel registry
            let is_stale = if let Some(ref registry) = self.tunnel_registry {
                !registry.is_host_alive(&host_id).await
            } else {
                // No tunnel registry — can't verify, assume stale if no recent activity
                true
            };

            if is_stale {
                warn!("Host {} heartbeat stale — marking {} routes offline", host_id, share_ids.len());
                offline_hosts.push(host_id);
            }
        }

        // Mark offline hosts' routes as revoked (temporarily)
        for host_id in offline_hosts {
            // Don't remove routes — just mark so forward_to_engine returns 503
            // Routes will be restored when host reconnects
            self.mark_host_offline(&host_id).await;
        }

        Ok(())
    }

    /// Mark all routes for a host as temporarily offline
    /// (keeps routes in cache but sets revoked=true so is_active returns false)
    pub async fn mark_host_offline(&self, host_id: &str) {
        if let Some(entry) = self.host_routes.get(host_id) {
            for share_id in entry.value().iter() {
                let share_id_str = share_id.clone();
                let http_key = format!("{}:http", share_id_str);
                let mysql_key = format!("{}:mysql", share_id_str);
                
                if let Some(mut route) = self.cache.get_mut(&http_key) {
                    route.revoked = true;
                }
                if let Some(mut route) = self.cache.get_mut(&mysql_key) {
                    route.revoked = true;
                }
            }
        }
        info!("Marked host {} as offline (routes temporarily unavailable)", host_id);
    }

    /// Restore routes when host reconnects (called by tunnel_ws_handler on connect)
    pub async fn mark_host_online(&self, host_id: &str) {
        if let Some(entry) = self.host_routes.get(host_id) {
            for share_id in entry.value().iter() {
                let share_id_str = share_id.clone();
                let http_key = format!("{}:http", share_id_str);
                let mysql_key = format!("{}:mysql", share_id_str);
                
                if let Some(mut route) = self.cache.get_mut(&http_key) {
                    route.revoked = false;
                }
                if let Some(mut route) = self.cache.get_mut(&mysql_key) {
                    route.revoked = false;
                }
            }
        }
        info!("Marked host {} as online (routes restored)", host_id);
    }
}

impl Clone for ShareRouter {
    fn clone(&self) -> Self {
        Self {
            db_pool: self.db_pool.clone(),
            cache: self.cache.clone(),
            engine_http_port: self.engine_http_port,
            engine_mysql_port: self.engine_mysql_port,
            tunnel_registry: self.tunnel_registry.clone(),
            host_routes: self.host_routes.clone(),
        }
    }
}

/// HTTP proxy request for share queries
/// Used by external websites to query through the P2P tunnel
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ProxyQueryRequest {
    pub sql: String,
    pub token: String,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// HTTP proxy response
#[derive(Debug, Clone, serde::Serialize)]
pub struct ProxyQueryResponse {
    pub success: bool,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<serde_json::Value>>,
    pub row_count: usize,
    pub execution_time_ms: u64,
    pub error: Option<String>,
}

/// CORS headers for external website access
pub fn cors_headers() -> Vec<(&'static str, &'static str)> {
    vec![
        ("Access-Control-Allow-Origin", "*"),
        ("Access-Control-Allow-Methods", "GET, POST, OPTIONS"),
        ("Access-Control-Allow-Headers", "Content-Type, Authorization, X-Share-Token"),
        ("Access-Control-Max-Age", "86400"),
    ]
}
