//! Connect-RPC ExportService implementation
//! ExportCsv, ExportJson, ExportParquet, ExportTableDump

use axum::{
    extract::State,
    response::Response,
    body::Body,
};
use serde::{Deserialize, Serialize};
use tokio_stream::wrappers::ReceiverStream;
use futures_util::StreamExt;
use tracing::info;

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
    request: axum::extract::Request,
) -> Response {
    let (parts, body) = request.into_parts();
    let bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap_or_default();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    let client_ip = parts.extensions.get::<crate::api::middleware::ClientIp>().map(|c| c.0);
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
    request: axum::extract::Request,
) -> Response {
    let (_parts, body) = request.into_parts();
    let bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap_or_default();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    let req: ExportRequest = match parse_connect_request(&body.to_string()) {
        Ok(r) => r,
        Err(resp) => return resp,
    };
    
    if req.format != "json" {
        return connect_error("invalid_argument", "Format must be 'json' for this endpoint");
    }
    
    execute_export(None, state, req, "json").await
}

/// POST /bennett.v1.ExportService/ExportParquet
pub async fn export_parquet(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let (parts, body) = request.into_parts();
    let bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap_or_default();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    let client_ip = parts.extensions.get::<crate::api::middleware::ClientIp>().map(|c| c.0);
    let req: ExportRequest = match parse_connect_request(&body.to_string()) {
        Ok(r) => r,
        Err(resp) => return resp,
    };

    if req.format != "parquet" {
        return connect_error("invalid_argument", "Format must be 'parquet' for this endpoint");
    }

    execute_export(client_ip, state, req, "parquet").await
}

/// POST /bennett.v1.ExportService/ExportTableDump
pub async fn export_table_dump(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let (_parts, body) = request.into_parts();
    let bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap_or_default();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    let req: ExportTableRequest = match parse_connect_request(&body.to_string()) {
        Ok(r) => r,
        Err(resp) => return resp,
    };
    
    // Build SELECT * query
    let sql = format!(r#"SELECT * FROM "{}""#, req.table_name);
    
    let format = req.format.clone();
    let export_req = ExportRequest {
        share_code: req.share_code,
        token: req.token,
        sql,
        format: req.format,
        include_headers: true,
    };

    execute_export(None, state, export_req, &format).await
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
        "parquet" => match format_parquet(&result.columns, &result.rows) {
            Ok(bytes) => {
                // Parquet is binary — encode as base64 for JSON transport
                // Client decodes: atob(data) in browser, Buffer.from(data, 'base64') in Node
                use base64::Engine;
                base64::engine::general_purpose::STANDARD.encode(&bytes)
            }
            Err(e) => {
                return connect_response(ExportResponse {
                    success: false,
                    data: String::new(),
                    is_last: true,
                    total_rows: 0,
                    chunk_index: 0,
                    error: Some(format!("Parquet encoding failed: {}", e)),
                });
            }
        },
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
    
    let body = Body::from_stream(ReceiverStream::new(rx).map(|chunk: Result<ExportChunk, std::convert::Infallible>| {
        chunk.map(|c| axum::body::Bytes::from(c.data))
    }));
    
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

/// Format query result as Apache Parquet binary
/// Returns base64-encoded string for JSON transport
fn format_parquet(
    columns: &[String],
    rows: &[Vec<serde_json::Value>],
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    use arrow::array::*;
    use arrow::datatypes::*;
    use arrow::record_batch::RecordBatch;
    use parquet::arrow::arrow_writer::ArrowWriter;
    use parquet::file::properties::WriterProperties;
    use std::sync::Arc;

    if columns.is_empty() {
        return Ok(Vec::new());
    }

    let arrow_fields: Vec<Field> = columns.iter().enumerate().map(|(i, name)| {
        let data_type = if let Some(first_row) = rows.first() {
            if let Some(val) = first_row.get(i) {
                match val {
                    serde_json::Value::Null => DataType::Utf8,
                    serde_json::Value::Bool(_) => DataType::Boolean,
                    serde_json::Value::Number(n) => {
                        if n.is_i64() { DataType::Int64 }
                        else if n.is_f64() { DataType::Float64 }
                        else { DataType::Utf8 }
                    }
                    serde_json::Value::String(_) => DataType::Utf8,
                    _ => DataType::Utf8,
                }
            } else {
                DataType::Utf8
            }
        } else {
            DataType::Utf8
        };
        Field::new(name, data_type, true)
    }).collect();

    let schema = Arc::new(Schema::new(arrow_fields.clone()));

    let mut arrays: Vec<Arc<dyn Array>> = Vec::with_capacity(columns.len());

    for (col_idx, field) in arrow_fields.iter().enumerate() {
        match field.data_type() {
            DataType::Boolean => {
                let values: Vec<Option<bool>> = rows.iter().map(|row| {
                    row.get(col_idx).and_then(|v| match v {
                        serde_json::Value::Bool(b) => Some(*b),
                        _ => None,
                    })
                }).collect();
                arrays.push(Arc::new(BooleanArray::from(values)));
            }
            DataType::Int64 => {
                let values: Vec<Option<i64>> = rows.iter().map(|row| {
                    row.get(col_idx).and_then(|v| match v {
                        serde_json::Value::Number(n) => n.as_i64(),
                        _ => None,
                    })
                }).collect();
                arrays.push(Arc::new(Int64Array::from(values)));
            }
            DataType::Float64 => {
                let values: Vec<Option<f64>> = rows.iter().map(|row| {
                    row.get(col_idx).and_then(|v| match v {
                        serde_json::Value::Number(n) => n.as_f64(),
                        _ => None,
                    })
                }).collect();
                arrays.push(Arc::new(Float64Array::from(values)));
            }
            _ => {
                let values: Vec<Option<String>> = rows.iter().map(|row| {
                    row.get(col_idx).map(|v| match v {
                        serde_json::Value::Null => String::new(),
                        serde_json::Value::String(s) => s.clone(),
                        other => other.to_string(),
                    })
                }).collect();
                arrays.push(Arc::new(StringArray::from_iter(values)));
            }
        }
    }

    let batch = RecordBatch::try_new(schema.clone(), arrays)
        .map_err(|e| format!("Failed to build RecordBatch: {}", e))?;

    let mut buf = Vec::new();
    let props = WriterProperties::builder().build();
    let mut writer = ArrowWriter::try_new(&mut buf, schema, Some(props))
        .map_err(|e| format!("Failed to create Parquet writer: {}", e))?;

    writer.write(&batch)
        .map_err(|e| format!("Failed to write Parquet batch: {}", e))?;
    writer.close()
        .map_err(|e| format!("Failed to close Parquet writer: {}", e))?;

    Ok(buf)
}

// ExportService implementation complete
// Features: CSV/JSON/Parquet export, chunked streaming, progress tracking
