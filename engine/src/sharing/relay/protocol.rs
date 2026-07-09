//! Relay Protocol — Message serialization for engine-relay tunnel
//! Shared between engine client and relay server tunnel handler

use serde::{Deserialize, Serialize};

/// Protocol version for compatibility
pub const PROTOCOL_VERSION: &str = "1.0.0";

/// All messages use JSON with this envelope
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolEnvelope {
    pub version: String,
    pub timestamp: i64,
    pub payload: TunnelPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TunnelPayload {
    Register(RegisterPayload),
    Heartbeat(HeartbeatPayload),
    Query(QueryPayload),
    QueryResult(QueryResultPayload),
    Schema(SchemaPayload),
    SchemaResult(SchemaResultPayload),
    Error(ErrorPayload),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterPayload {
    pub host_id: String,
    pub version: String,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatPayload {
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryPayload {
    pub request_id: String,
    pub share_code: String,
    pub token: String,
    pub sql: String,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResultPayload {
    pub request_id: String,
    pub success: bool,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<serde_json::Value>>,
    pub row_count: usize,
    pub execution_time_ms: u64,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaPayload {
    pub request_id: String,
    pub share_code: String,
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaResultPayload {
    pub request_id: String,
    pub success: bool,
    pub tables: Vec<serde_json::Value>,
    pub database_name: String,
    pub database_type: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorPayload {
    pub request_id: String,
    pub code: String,
    pub message: String,
}

/// Build a protocol envelope
pub fn envelope(payload: TunnelPayload) -> ProtocolEnvelope {
    ProtocolEnvelope {
        version: PROTOCOL_VERSION.to_string(),
        timestamp: chrono::Utc::now().timestamp(),
        payload,
    }
}
