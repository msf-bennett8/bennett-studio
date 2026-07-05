//! Share router — maps share_id to engine endpoint
//! Polls SQLite DB for active shares, maintains in-memory cache

use dashmap::DashMap;
use sqlx::{Pool, Sqlite};
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

    /// Check if share exists and is active
    pub fn is_active(&self, share_id: &str) -> bool {
        match self.cache.get(share_id) {
            Some(route) => !route.revoked && route.expires_at > chrono::Utc::now(),
            None => false,
        }
    }

    /// Refresh routes from database
    pub async fn refresh_routes(&self) -> anyhow::Result<()> {
        debug!("Refreshing share routes from database");

        let rows = sqlx::query!(
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
                    let route = ShareRoute {
                        share_id: share.code.clone(),
                        db_id: share.db_id,
                        protocol: crate::transport::ProtocolType::ConnectRpc,
                        engine_port: self.engine_http_port,
                        expires_at: share.expires_at.parse().unwrap_or_else(|_| {
                            chrono::DateTime::UNIX_EPOCH
                        }),
                        revoked: share.revoked != 0,
                    };

                    // Also add MySQL wire route for the same share
                    let mysql_route = ShareRoute {
                        protocol: crate::transport::ProtocolType::MySqlWire,
                        engine_port: self.engine_mysql_port,
                        ..route.clone()
                    };

                    self.cache.insert(
                        format!("{}:http", share.code),
                        route,
                    );
                    self.cache.insert(
                        format!("{}:mysql", share.code),
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
