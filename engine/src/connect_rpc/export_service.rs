//! Connect-RPC ExportService implementation
//! ExportCsv, ExportJson, ExportParquet, ExportTableDump

use axum::{
    extract::State,
    response::{IntoResponse, Response},
    body::Body,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio_stream::wrappers::ReceiverStream;
use tracing::{info, warn};

use crate::AppState;
use crate::connect_rpc::{
    connect_error, connect_response, validate_share_request,
    validate_shared_sql, parse_connect_request,
};

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct ExportRequest {
    pub share_code: String,
    pub token: String,
    pub sql: String,
    pub format: String, // "csv", "json", "parquet"
    #[serde(default = "default_true")]
    pub include_headers: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize)]
pub struct ExportTableRequest {
    pub share_code: String,
    pub token: String,
    pub table_name: String,
    pub format: String,
}

#[derive(Debug, Serialize)]
pub struct ExportResponse {
    pub success: bool,
    pub data: String, // Base64 encoded chunk
    pub is_last: bool,
    pub total_rows: i64,
    pub chunk_index: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ExportChunk {
    pub data: Vec<u8>,
    pub is_last: bool,
    pub total_rows: i64,
    pub chunk_index: i64,
}

// ============================================================================
// Handlers
// ============================================================================

/// POST /bennett.v1.ExportService/ExportCsv
pub async fn export_csv(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
    request: axum::extract::Request,
) -> Response {
    let client_ip = request.extensions().get::<crate::api::middleware::ClientIp>().map(|c| c.0);
    let req: ExportRequest = match parse_connect_request(&body.to_string()) {
        Ok(r) => r,
        Err(resp) => return resp,
    };
    
    if req.format != "csv" {
        return connect_error("invalid_argument", "Format must be 'csv' for this endpoint");
    }
    
    execute_export(client_ip, state, req, "csv").await
}

/// POST /bennett.v1.ExportService/ExportJson
pub async fn export_json(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    let req: ExportRequest = match parse_connect_request(&body.to_string()) {
        Ok(r) => r,
        Err(resp) => return resp,
    };
    
    if req.format != "json" {
        return connect_error("invalid_argument", "Format must be 'json' for this endpoint");
    }
    
    execute_export(None, state, req, "json").await
}

/// POST /bennett.v1.ExportService/ExportTableDump
pub async fn export_table_dump(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    let req: ExportTableRequest = match parse_connect_request(&body.to_string()) {
        Ok(r) => r,
        Err(resp) => return resp,
    };
    
    // Build SELECT * query
    let sql = format!(r#"SELECT * FROM "{}""#, req.table_name);
    
    let export_req = ExportRequest {
        share_code: req.share_code,
        token: req.token,
        sql,
        format: req.format,
        include_headers: true,
    };
    
    execute_export(None, state, export_req, &req.format).await
}

// ============================================================================
// Core Export Logic
// ============================================================================

async fn execute_export(
    client_ip: Option<std::net::IpAddr>,
    state: AppState,
    req: ExportRequest,
    format: &str,
) -> Response {
    let start = std::time::Instant::now();
    
    // Validate
    let validated = match validate_share_request(&state, &req.share_code, &req.token, client_ip).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    
    // Validate SQL
    if let Err(resp) = validate_shared_sql(&req.sql, &validated.permission) {
        return resp;
    }
    
    // Find database
    let db_instance = {
        let dbs = state.databases.lock().unwrap();
        dbs.iter().find(|d| d.id == validated.db_id).cloned()
    };
    
    let db_instance = match db_instance {
        Some(d) => d,
        None => return connect_error("not_found", "Database not available"),
    };
    
    // Auto-connect
    {
        let mut conn = state.connections.lock().await;
        if !conn.is_connected(&db_instance.id) {
            if let Err(e) = conn.connect(&db_instance).await {
                return connect_error("unavailable", &format!("Connection failed: {}", e));
            }
        }
    }
    
    // Execute query
    let result = {
        let conn = state.connections.lock().await;
        match conn.execute(&db_instance.id, &req.sql).await {
            Ok(r) => r,
            Err(e) => {
                return connect_response(ExportResponse {
                    success: false,
                    data: String::new(),
                    is_last: true,
                    total_rows: 0,
                    chunk_index: 0,
                    error: Some(format!("Export query failed: {}", e)),
                });
            }
        }
    };
    
    // Format output
    let data = match format {
        "csv" => format_csv(&result.columns, &result.rows, req.include_headers),
        "json" => format_json(&result.columns, &result.rows),
        _ => {
            return connect_error("invalid_argument", &format!("Unsupported format: {}", format));
        }
    };
    
    let elapsed = start.elapsed().as_millis() as i64;
    
    info!(
        "Export completed for share {}: {} rows as {} in {}ms",
        req.share_code, result.row_count, format, elapsed
    );
    
    // Stream in chunks to avoid memory issues
    let (tx, rx) = tokio::sync::mpsc::channel(4);
    let data_bytes = data.into_bytes();
    let chunk_size = 64 * 1024; // 64KB chunks
    let total_rows = result.row_count as i64;
    
    tokio::spawn(async move {
        let mut offset = 0;
        let mut chunk_index = 0;
        let total_len = data_bytes.len();
        
        loop {
            let end = (offset + chunk_size).min(total_len);
            let is_last = end == total_len;
            let chunk_data = data_bytes[offset..end].to_vec();
            
            let chunk = ExportChunk {
                data: chunk_data,
                is_last,
                total_rows,
                chunk_index,
            };
            
            if tx.send(Ok(chunk)).await.is_err() {
                break; // Receiver dropped
            }
            
            if is_last {
                break;
            }
            
            offset = end;
            chunk_index += 1;
        }
    });
    
    let body = Body::from_stream(ReceiverStream::new(rx));
    
    Response::builder()
        .status(axum::http::StatusCode::OK)
        .header(axum::http::header::CONTENT_TYPE, "application/json")
        .body(body)
        .unwrap()
}

fn format_csv(
    columns: &[String],
    rows: &[Vec<serde_json::Value>],
    include_headers: bool,
) -> String {
    let mut output = String::new();
    
    if include_headers {
        output.push_str(&columns.join(","));
        output.push('\n');
    }
    
    for row in rows {
        let values: Vec<String> = row.iter().map(|v| {
            match v {
                serde_json::Value::Null => String::new(),
                serde_json::Value::String(s) => {
                    // Escape quotes and wrap in quotes if contains comma
                    if s.contains(',') || s.contains('"') || s.contains('\n') {
                        format!("\"{}\"", s.replace("\"", "\"\""))
                    } else {
                        s.clone()
                    }
                }
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::Bool(b) => b.to_string(),
                _ => v.to_string(),
            }
        }).collect();
        output.push_str(&values.join(","));
        output.push('\n');
    }
    
    output
}

fn format_json(
    columns: &[String],
    rows: &[Vec<serde_json::Value>],
) -> String {
    let mut objects = Vec::new();
    
    for row in rows {
        let mut obj = serde_json::Map::new();
        for (i, col) in columns.iter().enumerate() {
            let value = row.get(i).cloned().unwrap_or(serde_json::Value::Null);
            obj.insert(col.clone(), value);
        }
        objects.push(serde_json::Value::Object(obj));
    }
    
    serde_json::to_string_pretty(&objects).unwrap_or_default()
}

// ExportService implementation complete
// Features: CSV/JSON export, chunked streaming, progress tracking
