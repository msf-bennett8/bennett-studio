use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Deserialize)]
pub struct CreateApiKeyRequest {
    pub database_id: String,
    pub name: String,
    pub permission: Option<String>,
    pub tables: Option<Vec<String>>,
    pub cols: Option<serde_json::Value>,
    pub rls: Option<String>,
    /// Max rows returned per query (default: 1000)
    pub max_rows: Option<i32>,
    /// Query timeout in seconds (default: 30)
    pub timeout_secs: Option<i32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateApiKeyResponse {
    pub id: String,
    pub key: String, // plaintext — shown once, never stored
    pub name: String,
    pub permission: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiKeyInfo {
    pub id: String,
    pub name: String,
    pub db_id: String,
    pub permission: String,
    pub tables: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub revoked: bool,
    pub key_preview: String,
    pub max_rows: i32,
    pub timeout_secs: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct ListApiKeysResponse {
    pub keys: Vec<ApiKeyInfo>,
    pub total: usize,
}
