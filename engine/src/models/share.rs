use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Legacy share session (kept for backward compatibility)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareSession {
    pub id: String,
    pub database_id: String,
    pub token: String,
    pub expires_at: String,
    pub read_only: bool,
}

/// Phase 1: Full share link with JWT and permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareLink {
    pub code: String,
    pub url: String,
    pub db_id: String,
    pub db_name: String,
    pub db_type: String,
    pub permission: String,
    pub tables: Vec<String>,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub guest_count: i32,
    pub pinned: bool,
    pub status: ShareStatus,
    pub ice: Option<String>, // Base64 ICE candidates for P2P (optional)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ShareStatus {
    Active,
    Expired,
    Revoked,
}

/// Request to create a share
#[derive(Debug, Clone, Deserialize)]
pub struct CreateShareRequest {
    pub database_id: String,
    pub permission: Option<String>,
    pub tables: Option<Vec<String>>,
    pub cols: Option<serde_json::Value>,
    pub rls: Option<String>,
    pub duration_hours: Option<i64>,
}

/// Share creation response
#[derive(Debug, Clone, Serialize)]
pub struct CreateShareResponse {
    pub code: String,
    pub url: String,
    pub token: String,
    pub expires_at: DateTime<Utc>,
    pub ice: Option<String>, // Base64 ICE candidates for P2P (optional)
    pub sig: Option<String>, // Short signaling code for Firebase P2P (optional)
}

/// Share validation request (from guest)
#[derive(Debug, Clone, Deserialize)]
pub struct ValidateShareRequest {
    pub code: String,
    pub token: String,
}

/// Share validation response
#[derive(Debug, Clone, Serialize)]
pub struct ValidateShareResponse {
    pub valid: bool,
    pub code: String,
    pub db_id: String,
    pub permission: String,
    pub tables: Vec<String>,
    pub expires_at: DateTime<Utc>,
    pub host_online: bool,
}

/// Revoke share request
#[derive(Debug, Clone, Deserialize)]
pub struct RevokeShareRequest {
    pub code: String,
    pub reason: Option<String>,
}

/// List shares response
#[derive(Debug, Clone, Serialize)]
pub struct ListSharesResponse {
    pub shares: Vec<ShareLink>,
    pub total: usize,
}
