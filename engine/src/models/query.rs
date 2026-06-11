use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct ExecuteQueryRequest {
    pub sql: String,
    pub database_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<serde_json::Value>>,
    pub row_count: usize,
    pub execution_time_ms: u64,
}
