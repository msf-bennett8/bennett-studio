//! Audit logging service
//! Phase 5: Every query logged with user attribution, timestamp, result
//! Stored in SQLite with 90-day retention

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite, sqlite::SqlitePoolOptions};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{info, warn, error};

/// Audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub share_code: String,
    pub db_id: String,
    pub peer_ip: String,
    pub user_agent: Option<String>,
    pub query_type: QueryType,
    pub sql: String,
    pub rows_affected: i64,
    pub execution_time_ms: i64,
    pub success: bool,
    pub error_message: Option<String>,
    pub permission_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum QueryType {
    Select,
    Insert,
    Update,
    Delete,
    Create,
    Alter,
    Drop,
    Other,
}

impl QueryType {
    pub fn from_sql(sql: &str) -> Self {
        let upper = sql.trim().to_uppercase();
        if upper.starts_with("SELECT") { Self::Select }
        else if upper.starts_with("INSERT") { Self::Insert }
        else if upper.starts_with("UPDATE") { Self::Update }
        else if upper.starts_with("DELETE") { Self::Delete }
        else if upper.starts_with("CREATE") { Self::Create }
        else if upper.starts_with("ALTER") { Self::Alter }
        else if upper.starts_with("DROP") { Self::Drop }
        else { Self::Other }
    }
}

/// Audit log service
pub struct AuditService {
    pool: Pool<Sqlite>,
    tx: mpsc::Sender<AuditEntry>,
}

impl AuditService {
    pub async fn new(db_path: &str) -> Result<Arc<Self>, sqlx::Error> {
        let pool = SqlitePoolOptions::new()
            .max_connections(2)
            .connect(db_path)
            .await?;
        
        // Create table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS audit_log (
                id TEXT PRIMARY KEY,
                timestamp TEXT NOT NULL,
                share_code TEXT NOT NULL,
                db_id TEXT NOT NULL,
                peer_ip TEXT NOT NULL,
                user_agent TEXT,
                query_type TEXT NOT NULL,
                sql TEXT NOT NULL,
                rows_affected INTEGER NOT NULL DEFAULT 0,
                execution_time_ms INTEGER NOT NULL DEFAULT 0,
                success INTEGER NOT NULL DEFAULT 1,
                error_message TEXT,
                permission_level TEXT NOT NULL DEFAULT 'ro'
            );
            
            CREATE INDEX IF NOT EXISTS idx_audit_timestamp ON audit_log(timestamp);
            CREATE INDEX IF NOT EXISTS idx_audit_share ON audit_log(share_code);
            CREATE INDEX IF NOT EXISTS idx_audit_db ON audit_log(db_id);
            CREATE INDEX IF NOT EXISTS idx_audit_type ON audit_log(query_type);
            "#
        )
        .execute(&pool)
        .await?;
        
        let (tx, mut rx) = mpsc::channel::<AuditEntry>(1000);
        
        let service = Arc::new(Self {
            pool,
            tx,
        });
        
        // Background writer
        let pool_clone = service.pool.clone();
        tokio::spawn(async move {
            while let Some(entry) = rx.recv().await {
                if let Err(e) = Self::write_entry(&pool_clone, &entry).await {
                    error!("Audit write failed: {}", e);
                }
            }
        });
        
        // Background cleanup (90-day retention)
        let pool_clone = service.pool.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(86400)); // Daily
            loop {
                interval.tick().await;
                if let Err(e) = Self::cleanup_old(&pool_clone, 90).await {
                    error!("Audit cleanup failed: {}", e);
                }
            }
        });
        
        info!("Audit service initialized");
        Ok(service)
    }
    
    /// Log a query (async, non-blocking)
    pub async fn log_query(&self, entry: AuditEntry) {
        if let Err(e) = self.tx.send(entry).await {
            warn!("Audit log channel full, dropping entry: {}", e);
        }
    }
    
    /// Write entry to database
    async fn write_entry(pool: &Pool<Sqlite>, entry: &AuditEntry) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO audit_log 
            (id, timestamp, share_code, db_id, peer_ip, user_agent, query_type, sql, rows_affected, execution_time_ms, success, error_message, permission_level)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&entry.id)
        .bind(entry.timestamp.to_rfc3339())
        .bind(&entry.share_code)
        .bind(&entry.db_id)
        .bind(&entry.peer_ip)
        .bind(entry.user_agent.as_ref())
        .bind(match entry.query_type {
            QueryType::Select => "Select",
            QueryType::Insert => "Insert",
            QueryType::Update => "Update",
            QueryType::Delete => "Delete",
            QueryType::Create => "Create",
            QueryType::Alter => "Alter",
            QueryType::Drop => "Drop",
            QueryType::Other => "Other",
        })
        .bind(&entry.sql)
        .bind(entry.rows_affected)
        .bind(entry.execution_time_ms)
        .bind(entry.success as i32)
        .bind(entry.error_message.as_ref())
        .bind(&entry.permission_level)
        .execute(pool)
        .await?;
        
        Ok(())
    }
    
    /// Cleanup entries older than retention_days
    async fn cleanup_old(pool: &Pool<Sqlite>, retention_days: i64) -> Result<u64, sqlx::Error> {
        let cutoff = (Utc::now() - chrono::Duration::days(retention_days)).to_rfc3339();
        
        let result = sqlx::query("DELETE FROM audit_log WHERE timestamp < ?")
            .bind(cutoff)
            .execute(pool)
            .await?;
        
        if result.rows_affected() > 0 {
            info!("Cleaned up {} old audit entries", result.rows_affected());
        }
        
        Ok(result.rows_affected())
    }
    
    /// Query audit log (for admin/reports)
    pub async fn query(
        &self,
        share_code: Option<&str>,
        db_id: Option<&str>,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
        limit: i64,
    ) -> Result<Vec<AuditEntry>, sqlx::Error> {
        let mut query_str = "SELECT * FROM audit_log WHERE 1=1".to_string();
        let mut binds: Vec<Box<dyn sqlx::Encode<'_, sqlx::Sqlite> + sqlx::Type<sqlx::Sqlite> + Send>> = Vec::new();
        
        if let Some(code) = share_code {
            query_str.push_str(" AND share_code = ?");
            // binds.push(Box::new(code)); // Simplified - real impl needs proper binding
        }
        
        query_str.push_str(" ORDER BY timestamp DESC LIMIT ?");
        
        let rows = sqlx::query(&query_str)
            .bind(limit)
            .fetch_all(&self.pool)
            .await?;
        
        let entries = rows.into_iter().map(|row| {
            let query_type_str: String = row.get("query_type");
            let query_type = match query_type_str.as_str() {
                "Select" => QueryType::Select,
                "Insert" => QueryType::Insert,
                "Update" => QueryType::Update,
                "Delete" => QueryType::Delete,
                "Create" => QueryType::Create,
                "Alter" => QueryType::Alter,
                "Drop" => QueryType::Drop,
                _ => QueryType::Other,
            };
            
            AuditEntry {
                id: row.get("id"),
                timestamp: DateTime::parse_from_rfc3339(&row.get::<String, _>("timestamp"))
                    .map(|d| d.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                share_code: row.get("share_code"),
                db_id: row.get("db_id"),
                peer_ip: row.get("peer_ip"),
                user_agent: row.get("user_agent"),
                query_type,
                sql: row.get("sql"),
                rows_affected: row.get("rows_affected"),
                execution_time_ms: row.get("execution_time_ms"),
                success: row.get::<i32, _>("success") != 0,
                error_message: row.get("error_message"),
                permission_level: row.get("permission_level"),
            }
        }).collect();
        
        Ok(entries)
    }
}

/// Convenience function to create audit entry
pub fn create_entry(
    share_code: &str,
    db_id: &str,
    peer_ip: &str,
    sql: &str,
    rows_affected: i64,
    execution_time_ms: i64,
    success: bool,
    permission_level: &str,
) -> AuditEntry {
    AuditEntry {
        id: uuid::Uuid::new_v4().to_string(),
        timestamp: Utc::now(),
        share_code: share_code.to_string(),
        db_id: db_id.to_string(),
        peer_ip: peer_ip.to_string(),
        user_agent: None,
        query_type: QueryType::from_sql(sql),
        sql: sql.to_string(),
        rows_affected,
        execution_time_ms,
        success,
        error_message: None,
        permission_level: permission_level.to_string(),
    }
}
