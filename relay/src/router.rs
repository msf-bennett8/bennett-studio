//! Share router — maps share_id to engine endpoint
//! Polls SQLite DB for active shares, maintains in-memory cache

use dashmap::DashMap;
use sqlx::{Pool, Row, Sqlite};
use std::sync::Arc;
use tokio::time::{interval, Duration};
use tracing::{debug, error, info};

/// Route entry for a share
#[derive(Debug, Clone)]
pub struct ShareRoute {
    pub share_id: String,
    pub db_id: String,
    pub protocol: crate::transport::ProtocolType,
    pub engine_port: u16,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub revoked: bool,
}

/// In-memory route cache with SQLite backing
pub struct ShareRouter {
    db_pool: Pool<Sqlite>,
    cache: Arc<DashMap<String, ShareRoute>>,
    engine_http_port: u16,
    engine_mysql_port: u16,
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

    /// Refresh routes from database
    pub async fn refresh_routes(&self) -> anyhow::Result<()> {
        debug!("Refreshing share routes from database");

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
                    };

                    // Also add MySQL wire route for the same share
                    let mysql_route = ShareRoute {
                        protocol: crate::transport::ProtocolType::MySqlWire,
                        engine_port: self.engine_mysql_port,
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

                // Clean expired entries
                self.cache.retain(|_, v| {
                    !v.revoked && v.expires_at > chrono::Utc::now()
                });

                info!("Route cache refreshed: {} active shares", count);
            }
            Err(e) => {
                error!("Failed to refresh routes: {}", e);
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

    /// Forward an HTTP request to the engine via TCP
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

        // Connect to engine HTTP port via TCP (relay and engine run on same host)
        let engine_addr = format!("127.0.0.1:{}", route.engine_port);
        let mut stream = tokio::net::TcpStream::connect(&engine_addr).await
            .map_err(|e| anyhow::anyhow!("Failed to connect to engine at {}: {}", engine_addr, e))?;

        // Build HTTP request
        let body_len = body.as_ref().map(|b| b.len()).unwrap_or(0);
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
}

impl Clone for ShareRouter {
    fn clone(&self) -> Self {
        Self {
            db_pool: self.db_pool.clone(),
            cache: self.cache.clone(),
            engine_http_port: self.engine_http_port,
            engine_mysql_port: self.engine_mysql_port,
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
