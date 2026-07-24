//! SQLite-backed share session storage
//! Stores active shares, guest sessions, and revoked tokens
//! Uses TTL cleanup with background janitor

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite, Row};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, error};

/// Share record in database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareRecord {
    pub code: String,
    pub db_id: String,
    pub host_id: String,
    /// Host IP address for guest direct connection
    pub host: Option<String>,
    /// Host port for guest direct connection
    pub port: Option<u16>,
    pub token_jti: String,
    /// Full JWT token for wire protocol validation
    pub token: Option<String>,
    pub permission: String,
    pub tables: String, // JSON array
    pub cols: Option<String>, // JSON object
    pub rls: Option<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub revoked: bool,
    pub guest_count: i32,
    pub pinned: bool,
    /// Base64 ICE candidates for P2P connections
    pub ice: Option<String>,
}

/// Guest session record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuestSession {
    pub id: String,
    pub share_code: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub connected_at: DateTime<Utc>,
    pub last_active: DateTime<Utc>,
    pub query_count: i32,
}

/// Revoked token record (for immediate revocation)
#[derive(Debug, Clone)]
pub struct RevokedToken {
    pub jti: String,
    pub revoked_at: DateTime<Utc>,
    pub reason: String,
}

/// API key record (durable, no expiry until explicitly revoked)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyRecord {
    pub id: String,
    pub key_hash: String,
    pub db_id: String,
    pub name: String,
    pub permission: String,
    pub tables: String,
    pub cols: Option<String>,
    pub rls: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub revoked: bool,
    /// Max rows per query — independent of share defaults
    pub max_rows: i32,
    /// Query timeout in seconds — independent of share defaults
    pub timeout_secs: i32,
    /// Wire-protocol (MySQL/Postgres) username, if wire access is enabled
    pub wire_username: Option<String>,
    /// sha256 hash of the wire-protocol password — never store plaintext
    pub wire_password_hash: Option<String>,
}

/// Share store with SQLite backend
pub struct ShareStore {
    pool: Pool<Sqlite>,
    // In-memory cache for fast revocation checks
    revoked_cache: Arc<RwLock<dashmap::DashMap<String, DateTime<Utc>>>>,
}

impl ShareStore {
    /// Initialize share store with SQLite connection
    pub async fn new(db_path: &str) -> anyhow::Result<Arc<Self>> {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(db_path)
            .await?;
        
        // Create tables
        Self::init_schema(&pool).await?;
        
        let store = Arc::new(Self {
            pool,
            revoked_cache: Arc::new(RwLock::new(dashmap::DashMap::new())),
        });
        
        // Start background janitor
        let store_clone = store.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(300)); // 5 min
            loop {
                interval.tick().await;
                if let Err(e) = store_clone.cleanup_expired().await {
                    error!("Share store cleanup error: {}", e);
                }
            }
        });
        
        info!("Share store initialized");
        Ok(store)
    }
    
    async fn init_schema(pool: &Pool<Sqlite>) -> anyhow::Result<()> {
        // Use raw_sql for multiple statements (sqlx 0.8+)
        sqlx::raw_sql(
            r#"
            CREATE TABLE IF NOT EXISTS shares (
                code TEXT PRIMARY KEY,
                db_id TEXT NOT NULL,
                host_id TEXT NOT NULL,
                host TEXT,
                port INTEGER,
                token_jti TEXT NOT NULL UNIQUE,
                token TEXT,
                permission TEXT NOT NULL DEFAULT 'ro',
                tables TEXT NOT NULL DEFAULT '["*"]',
                cols TEXT,
                rls TEXT,
                created_at TEXT NOT NULL,
                expires_at TEXT NOT NULL,
                revoked INTEGER NOT NULL DEFAULT 0,
                guest_count INTEGER NOT NULL DEFAULT 0,
                pinned INTEGER NOT NULL DEFAULT 0,
                ice TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_shares_db_id ON shares(db_id);
            CREATE INDEX IF NOT EXISTS idx_shares_expires ON shares(expires_at);
            CREATE INDEX IF NOT EXISTS idx_shares_revoked ON shares(revoked);

            CREATE TABLE IF NOT EXISTS guest_sessions (
                id TEXT PRIMARY KEY,
                share_code TEXT NOT NULL,
                ip_address TEXT,
                user_agent TEXT,
                connected_at TEXT NOT NULL,
                last_active TEXT NOT NULL,
                query_count INTEGER NOT NULL DEFAULT 0,
                FOREIGN KEY (share_code) REFERENCES shares(code) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_guests_share ON guest_sessions(share_code);
            CREATE INDEX IF NOT EXISTS idx_guests_last_active ON guest_sessions(last_active);

            CREATE TABLE IF NOT EXISTS revoked_tokens (
                jti TEXT PRIMARY KEY,
                revoked_at TEXT NOT NULL,
                reason TEXT NOT NULL DEFAULT 'host_revoked'
            );

            CREATE INDEX IF NOT EXISTS idx_revoked_jti ON revoked_tokens(jti);

            CREATE TABLE IF NOT EXISTS kv_store (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS host_heartbeats (
                host_id TEXT PRIMARY KEY,
                last_beat TEXT NOT NULL,
                ip_address TEXT,
                port INTEGER,
                version TEXT
            );

            CREATE TABLE IF NOT EXISTS api_keys (
                id TEXT PRIMARY KEY,
                key_hash TEXT NOT NULL UNIQUE,
                db_id TEXT NOT NULL,
                name TEXT NOT NULL,
                permission TEXT NOT NULL DEFAULT 'ro',
                tables TEXT NOT NULL DEFAULT '["*"]',
                cols TEXT,
                rls TEXT,
                created_at TEXT NOT NULL,
                last_used_at TEXT,
                revoked INTEGER NOT NULL DEFAULT 0,
                max_rows INTEGER NOT NULL DEFAULT 1000,
                timeout_secs INTEGER NOT NULL DEFAULT 30,
                wire_username TEXT,
                wire_password_hash TEXT UNIQUE
            );

            CREATE INDEX IF NOT EXISTS idx_api_keys_hash ON api_keys(key_hash);
            CREATE INDEX IF NOT EXISTS idx_api_keys_db_id ON api_keys(db_id);
            CREATE INDEX IF NOT EXISTS idx_api_keys_wire_password_hash ON api_keys(wire_password_hash);
            "#
        )
        .execute(pool)
        .await?;

        // Migration: Add token column to existing shares table (for upgrades)
        let _ = sqlx::query("ALTER TABLE shares ADD COLUMN token TEXT")
            .execute(pool)
            .await;

        // Migration: Add ice column to existing shares table (for P2P)
        let _ = sqlx::query("ALTER TABLE shares ADD COLUMN ice TEXT")
            .execute(pool)
            .await;

        // Migration: Add per-key limits to existing api_keys table (for upgrades)
        let _ = sqlx::query("ALTER TABLE api_keys ADD COLUMN max_rows INTEGER NOT NULL DEFAULT 1000")
            .execute(pool)
            .await;
        let _ = sqlx::query("ALTER TABLE api_keys ADD COLUMN timeout_secs INTEGER NOT NULL DEFAULT 30")
            .execute(pool)
            .await;

        // Migration: Add wire-protocol credentials to existing api_keys table (for upgrades)
        let _ = sqlx::query("ALTER TABLE api_keys ADD COLUMN wire_username TEXT")
            .execute(pool)
            .await;
        let _ = sqlx::query("ALTER TABLE api_keys ADD COLUMN wire_password_hash TEXT")
            .execute(pool)
            .await;

        Ok(())
    }
    
    /// Create a new share record
    pub async fn create_share(&self, record: &ShareRecord) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            INSERT INTO shares (code, db_id, host_id, host, port, token_jti, token, permission, tables, cols, rls, created_at, expires_at, revoked, guest_count, pinned, ice)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&record.code)
        .bind(&record.db_id)
        .bind(&record.host_id)
        .bind(record.host.as_ref())
        .bind(record.port.map(|p| p as i32))
        .bind(&record.token_jti)
        .bind(record.token.as_ref())
        .bind(&record.permission)
        .bind(&record.tables)
        .bind(record.cols.as_ref())
        .bind(record.rls.as_ref())
        .bind(record.created_at.to_rfc3339())
        .bind(record.expires_at.to_rfc3339())
        .bind(record.revoked as i32)
        .bind(record.guest_count)
        .bind(record.pinned as i32)
        .bind(record.ice.as_ref())
        .execute(&self.pool)
        .await?;
        
        info!("Created share {} for db {}", record.code, record.db_id);
        Ok(())
    }
    
    /// Get share by code
    pub async fn get_share(&self, code: &str) -> anyhow::Result<Option<ShareRecord>> {
        let row = sqlx::query("SELECT * FROM shares WHERE code = ?")
            .bind(code)
            .fetch_optional(&self.pool)
            .await?;
        
        Ok(row.map(|r| Self::row_to_share(r)))
    }
    
    /// Get share by JTI (token ID)
    pub async fn get_share_by_jti(&self, jti: &str) -> anyhow::Result<Option<ShareRecord>> {
        let row = sqlx::query("SELECT * FROM shares WHERE token_jti = ?")
            .bind(jti)
            .fetch_optional(&self.pool)
            .await?;
        
        Ok(row.map(|r| Self::row_to_share(r)))
    }
    
    /// List all active shares for a database
    pub async fn list_shares_by_db(&self, db_id: &str) -> anyhow::Result<Vec<ShareRecord>> {
        let rows = sqlx::query(
            "SELECT * FROM shares WHERE db_id = ? AND revoked = 0 AND expires_at > ? ORDER BY created_at DESC"
        )
        .bind(db_id)
        .bind(Utc::now().to_rfc3339())
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Self::row_to_share).collect())
    }

    /// List ALL shares for a database (including revoked and expired) — for admin view
    pub async fn list_all_shares_by_db(&self, db_id: &str) -> anyhow::Result<Vec<ShareRecord>> {
        let rows = sqlx::query(
            "SELECT * FROM shares WHERE db_id = ? ORDER BY created_at DESC"
        )
        .bind(db_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Self::row_to_share).collect())
    }
    
    /// Revoke a share by code (host action)
    pub async fn revoke_share(&self, code: &str, reason: &str) -> anyhow::Result<bool> {
        let result = sqlx::query(
            "UPDATE shares SET revoked = 1 WHERE code = ?"
        )
        .bind(code)
        .execute(&self.pool)
        .await?;
        
        if result.rows_affected() > 0 {
            // Also add to revoked_tokens for immediate invalidation
            if let Ok(Some(share)) = self.get_share(code).await {
                let jti = share.token_jti;
                sqlx::query("INSERT OR REPLACE INTO revoked_tokens (jti, revoked_at, reason) VALUES (?, ?, ?)")
                    .bind(&jti)
                    .bind(Utc::now().to_rfc3339())
                    .bind(reason)
                    .execute(&self.pool)
                    .await?;
                
                // Add to in-memory cache
                self.revoked_cache.write().await.insert(jti, Utc::now());
            }
            
            info!("Revoked share {}", code);
            Ok(true)
        } else {
            Ok(false)
        }
    }
    
    /// Check if a token JTI is revoked
    pub async fn is_revoked(&self, jti: &str) -> bool {
        // Check in-memory cache first
        if self.revoked_cache.read().await.contains_key(jti) {
            return true;
        }
        
        // Check database
        match sqlx::query("SELECT 1 FROM revoked_tokens WHERE jti = ?")
            .bind(jti)
            .fetch_optional(&self.pool)
            .await
        {
            Ok(Some(_)) => {
                // Add to cache for next time
                self.revoked_cache.write().await.insert(jti.to_string(), Utc::now());
                true
            }
            _ => false,
        }
    }
    
    /// Record guest connection
    pub async fn record_guest_connect(&self, share_code: &str, ip: Option<String>, ua: Option<String>) -> anyhow::Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now();
        
        sqlx::query(
            "INSERT INTO guest_sessions (id, share_code, ip_address, user_agent, connected_at, last_active, query_count) VALUES (?, ?, ?, ?, ?, ?, 0)"
        )
        .bind(&id)
        .bind(share_code)
        .bind(ip)
        .bind(ua)
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await?;
        
        // Increment guest count
        sqlx::query("UPDATE shares SET guest_count = guest_count + 1 WHERE code = ?")
            .bind(share_code)
            .execute(&self.pool)
            .await?;
        
        Ok(id)
    }
    
    /// Record guest activity
    pub async fn record_guest_activity(&self, session_id: &str) -> anyhow::Result<()> {
        sqlx::query("UPDATE guest_sessions SET last_active = ?, query_count = query_count + 1 WHERE id = ?")
            .bind(Utc::now().to_rfc3339())
            .bind(session_id)
            .execute(&self.pool)
            .await?;
        
        Ok(())
    }
    
    /// List all guest sessions across all shares (for the Guests settings view)
    pub async fn list_all_guest_sessions(&self) -> anyhow::Result<Vec<GuestSession>> {
        let rows = sqlx::query("SELECT * FROM guest_sessions ORDER BY connected_at DESC")
            .fetch_all(&self.pool)
            .await?;
        Ok(rows.into_iter().map(Self::row_to_guest).collect())
    }

    fn row_to_guest(row: sqlx::sqlite::SqliteRow) -> GuestSession {
        GuestSession {
            id: row.get("id"),
            share_code: row.get("share_code"),
            ip_address: row.get("ip_address"),
            user_agent: row.get("user_agent"),
            connected_at: DateTime::parse_from_rfc3339(&row.get::<String, _>("connected_at"))
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            last_active: DateTime::parse_from_rfc3339(&row.get::<String, _>("last_active"))
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            query_count: row.get("query_count"),
        }
    }

    /// Disconnect guest
    pub async fn record_guest_disconnect(&self, session_id: &str) -> anyhow::Result<()> {
        // Delete guest session and decrement count
        let share_code: Option<String> = sqlx::query("SELECT share_code FROM guest_sessions WHERE id = ?")
            .bind(session_id)
            .fetch_optional(&self.pool)
            .await?
            .map(|r| r.get("share_code"));
        
        if let Some(code) = share_code {
            sqlx::query("DELETE FROM guest_sessions WHERE id = ?")
                .bind(session_id)
                .execute(&self.pool)
                .await?;
            
            sqlx::query("UPDATE shares SET guest_count = MAX(0, guest_count - 1) WHERE code = ?")
                .bind(&code)
                .execute(&self.pool)
                .await?;
        }
        
        Ok(())
    }

        /// Toggle pin status for a share
    pub async fn toggle_pin_share(&self, code: &str) -> anyhow::Result<bool> {
        // Get current pin status
        let current = sqlx::query("SELECT pinned FROM shares WHERE code = ?")
            .bind(code)
            .fetch_optional(&self.pool)
            .await?;

        if let Some(row) = current {
            let pinned: i32 = row.get("pinned");
            let new_pinned = if pinned == 0 { 1 } else { 0 };

            let result = sqlx::query("UPDATE shares SET pinned = ? WHERE code = ?")
                .bind(new_pinned)
                .bind(code)
                .execute(&self.pool)
                .await?;

            if result.rows_affected() > 0 {
                info!("Toggled pin for share {} to {}", code, new_pinned);
                Ok(true)
            } else {
                Ok(false)
            }
        } else {
            Ok(false)
        }
    }

        /// Record host heartbeat
    pub async fn record_heartbeat(&self, host_id: &str, ip: Option<String>, port: Option<u16>, version: &str) -> anyhow::Result<()> {
        sqlx::query(
            "INSERT INTO host_heartbeats (host_id, last_beat, ip_address, port, version)
             VALUES (?, ?, ?, ?, ?)
             ON CONFLICT(host_id) DO UPDATE SET
             last_beat = excluded.last_beat,
             ip_address = excluded.ip_address,
             port = excluded.port,
             version = excluded.version"
        )
        .bind(host_id)
        .bind(Utc::now().to_rfc3339())
        .bind(ip)
        .bind(port.map(|p| p as i32))
        .bind(version)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Check if host is alive (heartbeat within last 90 seconds)
    pub async fn is_host_alive(&self, host_id: &str) -> anyhow::Result<bool> {
        let cutoff = (Utc::now() - Duration::seconds(90)).to_rfc3339();
        
        let row = sqlx::query(
            "SELECT 1 FROM host_heartbeats WHERE host_id = ? AND last_beat > ?"
        )
        .bind(host_id)
        .bind(&cutoff)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.is_some())
    }

    /// Get host info
    pub async fn get_host_info(&self, host_id: &str) -> anyhow::Result<Option<(String, Option<u16>)>> {
        let row = sqlx::query(
            "SELECT ip_address, port FROM host_heartbeats WHERE host_id = ?"
        )
        .bind(host_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| {
            let ip: String = r.get("ip_address");
            let port: Option<i32> = r.get("port");
            (ip, port.map(|p| p as u16))
        }))
    }

    /// Get all unique host_ids from active (non-revoked, non-expired) shares
    pub async fn get_all_active_host_ids(&self) -> anyhow::Result<Vec<String>> {
        let now = Utc::now().to_rfc3339();
        
        let rows = sqlx::query(
            "SELECT DISTINCT host_id FROM shares WHERE revoked = 0 AND expires_at > ?"
        )
        .bind(&now)
        .fetch_all(&self.pool)
        .await?;

        let host_ids: Vec<String> = rows
            .into_iter()
            .map(|r| r.get::<String, _>("host_id"))
            .collect();

        Ok(host_ids)
    }

    /// Get stored host ID (for stable relay tunnel identity)
    pub async fn get_host_id(&self) -> anyhow::Result<Option<String>> {
        let row = sqlx::query("SELECT value FROM kv_store WHERE key = 'host_id'")
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|r| r.get::<String, _>("value")))
    }

    /// Store host ID for stable relay tunnel identity
    pub async fn set_host_id(&self, host_id: &str) -> anyhow::Result<()> {
        sqlx::query(
            "INSERT INTO kv_store (key, value) VALUES ('host_id', ?)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value"
        )
        .bind(host_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // ========================================================================
    // API Keys — durable, long-lived credentials (no expiry, revocable)
    // ========================================================================

    pub async fn create_api_key(&self, record: &ApiKeyRecord) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            INSERT INTO api_keys (id, key_hash, db_id, name, permission, tables, cols, rls, created_at, last_used_at, revoked, max_rows, timeout_secs, wire_username, wire_password_hash)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&record.id)
        .bind(&record.key_hash)
        .bind(&record.db_id)
        .bind(&record.name)
        .bind(&record.permission)
        .bind(&record.tables)
        .bind(record.cols.as_ref())
        .bind(record.rls.as_ref())
        .bind(record.created_at.to_rfc3339())
        .bind(record.last_used_at.map(|d| d.to_rfc3339()))
        .bind(record.revoked as i32)
        .bind(record.max_rows)
        .bind(record.timeout_secs)
        .bind(record.wire_username.as_ref())
        .bind(record.wire_password_hash.as_ref())
        .execute(&self.pool)
        .await?;

        info!("Created API key {} ({}) for db {}", record.id, record.name, record.db_id);
        Ok(())
    }

    pub async fn get_api_key_by_hash(&self, key_hash: &str) -> anyhow::Result<Option<ApiKeyRecord>> {
        let row = sqlx::query("SELECT * FROM api_keys WHERE key_hash = ? AND revoked = 0")
            .bind(key_hash)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(Self::row_to_api_key))
    }

    /// Lookup by wire-protocol password hash (for the MySQL/Postgres wire
    /// proxy — Phase 3). Caller should additionally verify the record's
    /// wire_username matches what the client presented.
    pub async fn get_api_key_by_wire_password_hash(&self, wire_password_hash: &str) -> anyhow::Result<Option<ApiKeyRecord>> {
        let row = sqlx::query("SELECT * FROM api_keys WHERE wire_password_hash = ? AND revoked = 0")
            .bind(wire_password_hash)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(Self::row_to_api_key))
    }

    pub async fn list_api_keys_by_db(&self, db_id: &str) -> anyhow::Result<Vec<ApiKeyRecord>> {
        let rows = sqlx::query("SELECT * FROM api_keys WHERE db_id = ? ORDER BY created_at DESC")
            .bind(db_id)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows.into_iter().map(Self::row_to_api_key).collect())
    }

    /// Re-sync helper for relay reconnect (mirrors list_all_active() for shares)
    pub async fn list_all_active_api_keys(&self) -> anyhow::Result<Vec<ApiKeyRecord>> {
        let rows = sqlx::query("SELECT * FROM api_keys WHERE revoked = 0 ORDER BY created_at DESC")
            .fetch_all(&self.pool)
            .await?;
        Ok(rows.into_iter().map(Self::row_to_api_key).collect())
    }

    /// Permanently delete an API key (irreversible — distinct from revoke,
    /// which keeps the row for history). Returns the key_hash if found, so
    /// the caller can clear it from the relay's routing table too.
    pub async fn hard_delete_api_key(&self, id: &str) -> anyhow::Result<Option<String>> {
        let row = sqlx::query("SELECT key_hash FROM api_keys WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        let key_hash: Option<String> = row.map(|r| r.get("key_hash"));

        if key_hash.is_some() {
            sqlx::query("DELETE FROM api_keys WHERE id = ?")
                .bind(id)
                .execute(&self.pool)
                .await?;
            info!("Hard deleted API key {}", id);
        }
        Ok(key_hash)
    }

    pub async fn revoke_api_key(&self, id: &str) -> anyhow::Result<Option<String>> {
        let row = sqlx::query("SELECT key_hash FROM api_keys WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        let key_hash: Option<String> = row.map(|r| r.get("key_hash"));

        if key_hash.is_some() {
            sqlx::query("UPDATE api_keys SET revoked = 1 WHERE id = ?")
                .bind(id)
                .execute(&self.pool)
                .await?;
            info!("Revoked API key {}", id);
        }
        Ok(key_hash)
    }

    pub async fn touch_api_key(&self, key_hash: &str) -> anyhow::Result<()> {
        sqlx::query("UPDATE api_keys SET last_used_at = ? WHERE key_hash = ?")
            .bind(Utc::now().to_rfc3339())
            .bind(key_hash)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    fn row_to_api_key(row: sqlx::sqlite::SqliteRow) -> ApiKeyRecord {
        ApiKeyRecord {
            id: row.get("id"),
            key_hash: row.get("key_hash"),
            db_id: row.get("db_id"),
            name: row.get("name"),
            permission: row.get("permission"),
            tables: row.get("tables"),
            cols: row.get("cols"),
            rls: row.get("rls"),
            created_at: DateTime::parse_from_rfc3339(&row.get::<String, _>("created_at"))
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            last_used_at: row.get::<Option<String>, _>("last_used_at")
                .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                .map(|d| d.with_timezone(&Utc)),
            revoked: row.get::<i32, _>("revoked") != 0,
            max_rows: row.get("max_rows"),
            timeout_secs: row.get("timeout_secs"),
            wire_username: row.get("wire_username"),
            wire_password_hash: row.get("wire_password_hash"),
        }
    }

    /// Cleanup stale heartbeats (> 7 days)
    pub async fn cleanup_stale_heartbeats(&self) -> anyhow::Result<u64> {
        let cutoff = (Utc::now() - Duration::days(7)).to_rfc3339();
        
        let result = sqlx::query("DELETE FROM host_heartbeats WHERE last_beat < ?")
            .bind(&cutoff)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }
    
    /// List all active (non-revoked, non-expired) shares
    pub async fn list_all_active(&self) -> anyhow::Result<Vec<ShareRecord>> {
        let now = Utc::now().to_rfc3339();

        let rows = sqlx::query(
            "SELECT * FROM shares WHERE revoked = 0 AND expires_at > ? ORDER BY created_at DESC"
        )
        .bind(&now)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Self::row_to_share).collect())
    }

    /// Hard delete a share from the database (permanent removal)
    pub async fn hard_delete_share(&self, code: &str) -> anyhow::Result<bool> {
        let result = sqlx::query("DELETE FROM shares WHERE code = ?")
            .bind(code)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() > 0 {
            // Also clean up related guest sessions (CASCADE should handle this, but be explicit)
            let _ = sqlx::query("DELETE FROM guest_sessions WHERE share_code = ?")
                .bind(code)
                .execute(&self.pool)
                .await;
            
            // Remove from revoked tokens if present
            if let Ok(Some(share)) = self.get_share(code).await {
                let _ = sqlx::query("DELETE FROM revoked_tokens WHERE jti = ?")
                    .bind(&share.token_jti)
                    .execute(&self.pool)
                    .await;
            }

            info!("Hard deleted share {}", code);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Cleanup expired shares and stale sessions
    pub async fn cleanup_expired(&self) -> anyhow::Result<()> {
        let now = Utc::now().to_rfc3339();
        
        // Mark expired shares as revoked
        let expired = sqlx::query("UPDATE shares SET revoked = 1 WHERE expires_at < ? AND revoked = 0")
            .bind(&now)
            .execute(&self.pool)
            .await?;
        
        // Delete old guest sessions (inactive for > 24h)
        let cutoff = (Utc::now() - Duration::hours(24)).to_rfc3339();
        let stale = sqlx::query("DELETE FROM guest_sessions WHERE last_active < ?")
            .bind(&cutoff)
            .execute(&self.pool)
            .await?;
        
        // Clean old revoked tokens (> 30 days)
        let old_cutoff = (Utc::now() - Duration::days(30)).to_rfc3339();
        let old = sqlx::query("DELETE FROM revoked_tokens WHERE revoked_at < ?")
            .bind(&old_cutoff)
            .execute(&self.pool)
            .await?;
        
        // Cleanup stale heartbeats
        let _ = self.cleanup_stale_heartbeats().await;

        if expired.rows_affected() > 0 || stale.rows_affected() > 0 || old.rows_affected() > 0 {
            info!("Cleaned up {} expired shares, {} stale sessions, {} old tokens", 
                expired.rows_affected(), stale.rows_affected(), old.rows_affected());
        }
        
        Ok(())
    }
    
    fn row_to_share(row: sqlx::sqlite::SqliteRow) -> ShareRecord {
        ShareRecord {
            code: row.get("code"),
            db_id: row.get("db_id"),
            host_id: row.get("host_id"),
            host: row.get("host"),
            port: row.get::<Option<i32>, _>("port").map(|p| p as u16),
            token_jti: row.get("token_jti"),
            token: row.get("token"),
            permission: row.get("permission"),
            tables: row.get("tables"),
            cols: row.get("cols"),
            rls: row.get("rls"),
            created_at: DateTime::parse_from_rfc3339(&row.get::<String, _>("created_at"))
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            expires_at: DateTime::parse_from_rfc3339(&row.get::<String, _>("expires_at"))
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            revoked: row.get::<i32, _>("revoked") != 0,
            guest_count: row.get("guest_count"),
            pinned: row.get::<i32, _>("pinned") != 0,
            ice: row.get("ice"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_share_store() {
        let store = ShareStore::new("sqlite::memory:").await.unwrap();
        
        let record = ShareRecord {
            code: "ACQPFDAQ7P".to_string(),
            db_id: "db-123".to_string(),
            host_id: "host-abc".to_string(),
            host: Some("192.168.1.100".to_string()),
            port: Some(3001),
            token_jti: "jti-123".to_string(),
            token: None,
            permission: "ro".to_string(),
            tables: r#"["*"]"#.to_string(),
            cols: None,
            rls: None,
            created_at: Utc::now(),
            expires_at: Utc::now() + Duration::hours(24),
            revoked: false,
            guest_count: 0,
            ice: None,
        };
        
        store.create_share(&record).await.unwrap();
        
        let fetched = store.get_share("ACQPFDAQ7P").await.unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().code, "ACQPFDAQ7P");
    }
}
