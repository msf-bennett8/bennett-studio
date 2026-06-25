//! Connect-RPC QueryService implementation
//! ExecuteQuery, StreamQuery, ExecuteWrite

use axum::{
    extract::State,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use tracing::{info, warn, error};

use crate::AppState;
use crate::connect_rpc::{
    connect_error, connect_response, validate_share_request,
    validate_shared_sql, require_write_permission, apply_rls,
    parse_connect_request,
};

// ============================================================================
// Request/Response Types (JSON envelope for Connect-RPC)
// ============================================================================

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecuteQueryRequest {
    pub share_code: String,
    pub token: String,
    pub sql: String,
    #[serde(default = "default_limit")]
    pub limit: i32,
    #[serde(default)]
    pub offset: i32,
}

fn default_limit() -> i32 {
    1000
}

#[derive(Debug, Serialize)]
pub struct ExecuteQueryResponse {
    pub success: bool,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<serde_json::Value>>,
    pub row_count: i32,
    pub execution_time_ms: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecuteWriteRequest {
    pub share_code: String,
    pub token: String,
    pub sql: String,
    #[serde(default)]
    pub parameters: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct ExecuteWriteResponse {
    pub success: bool,
    pub rows_affected: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_insert_id: Option<String>,
    pub execution_time_ms: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

// ============================================================================
// Handlers
// ============================================================================

/// POST /bennett.v1.QueryService/ExecuteQuery
pub async fn execute_query(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let (parts, body) = request.into_parts();
    let bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap_or_default();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    let client_ip = parts.extensions.get::<crate::api::middleware::ClientIp>().map(|c| c.0);
    let headers = parts.headers;
    
    let req: ExecuteQueryRequest = match parse_connect_request(&body.to_string()) {
        Ok(r) => r,
        Err(resp) => return resp,
    };
    
    let start = std::time::Instant::now();
    
    // Extract client IP from request extensions (set by middleware)
    // Validate share and token
    let validated = match validate_share_request(&state, &req.share_code, &req.token, client_ip).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    
    // Validate SQL
    if let Err(resp) = validate_shared_sql(&req.sql, &validated.permission) {
        return resp;
    }
    
    // Apply RLS
    let sql = apply_rls(&req.sql, validated.rls.as_deref());
    tracing::info!(
        "Share {} SQL pipeline: original={:?}, rls={:?}, after_rls={:?}",
        req.share_code, req.sql, validated.rls, sql
    );
    
    // Limit check
    let limit = req.limit.clamp(1, 10000);
    tracing::info!(
        "Share {} validated: db_id={}, permission={:?}, tables={:?}, cols={:?}",
        req.share_code, validated.db_id, validated.permission, validated.tables, validated.cols
    );

    // Find database
    let db_instance = {
        let dbs = state.databases.lock().unwrap();
        dbs.iter().find(|d| d.id == validated.db_id).cloned()
    };
    
    let db_instance = match db_instance {
        Some(d) => d,
        None => {
            warn!("Database {} not found for share {}", validated.db_id, req.share_code);
            return connect_error("not_found", "Database not available");
        }
    };
    
    // Acquire connection lock once, handle auto-connect + cache + execute atomically
    let (result, is_select, final_sql) = {
        let mut conn = state.connections.lock().await;
        if !conn.is_connected(&db_instance.id) {
            if let Err(e) = conn.connect(&db_instance).await {
                error!("Connection failed for {}: {}", db_instance.id, e);
                return connect_error("unavailable", "Database connection failed");
            }
        }
        
        // Execute with LIMIT
        let final_sql = if !sql.to_uppercase().contains("LIMIT") {
            format!("{} LIMIT {}", sql, limit)
        } else {
            sql.clone()
        };
        tracing::info!("Share {} final SQL: {:?}", req.share_code, final_sql);

        // Check query cache for SELECT queries
        let is_select = sql.trim().to_uppercase().starts_with("SELECT") || sql.trim().to_uppercase().starts_with("WITH");
        
        if is_select {
            if let Some(cached) = state.query_cache.get(&db_instance.id, &final_sql, Some(&req.share_code)).await {
                let elapsed = start.elapsed().as_millis() as i64;
                return connect_response(ExecuteQueryResponse {
                    success: true,
                    columns: cached.columns,
                    rows: cached.rows,
                    row_count: cached.row_count as i32,
                    execution_time_ms: elapsed,
                    error: None,
                });
            }
        }
        
        let result = match conn.execute(&db_instance.id, &final_sql).await {
            Ok(r) => {
                tracing::info!(
                    "Share {} query executed: columns={:?}, rows={}, row_count={}",
                    req.share_code, r.columns, r.rows.len(), r.row_count
                );
                r
            }
            Err(e) => {
                warn!("Query failed: {}", e);
                return connect_response(ExecuteQueryResponse {
                    success: false,
                    columns: vec![],
                    rows: vec![],
                    row_count: 0,
                    execution_time_ms: start.elapsed().as_millis() as i64,
                    error: Some(format!("Query failed: {}", e)),
                });
            }
        };
        
        (result, is_select, final_sql)
    };
    
    // Extract table name from SQL for table-specific column projection
    let tables = crate::control_plane::query::cache::QueryCache::extract_tables(&final_sql);
    let primary_table = tables.first().map(|s| s.as_str());

    // Apply column projection based on share permissions
    tracing::info!(
        "Share {} before projection: columns={:?}, rows={}, primary_table={:?}",
        req.share_code, result.columns, result.rows.len(), primary_table
    );
    let (filtered_columns, filtered_rows) = crate::connect_rpc::project_columns(
        &result.columns,
        &result.rows,
        &validated.cols,
        primary_table,
    );
    tracing::info!(
        "Share {} after projection: columns={:?}, rows={}",
        req.share_code, filtered_columns, filtered_rows.len()
    );
    
    let elapsed = start.elapsed().as_millis() as i64;
    
    // Store in cache
    if is_select {
        let tables = crate::control_plane::query::cache::QueryCache::extract_tables(&final_sql);
        state.query_cache.put(&db_instance.id, &final_sql, Some(&req.share_code), result.clone(), tables).await;
    }
    
    // Audit log with session context
    if let Some(audit) = &state.audit_service {
        let mut entry = crate::audit::create_entry(
            &req.share_code,
            &validated.db_id,
            &client_ip.map(|ip| ip.to_string()).unwrap_or_else(|| "127.0.0.1".to_string()),
            &req.sql,
            result.row_count as i64,
            elapsed as i64,
            true,
            validated.permission.as_str(),
        );
        // Session ID from request headers if available
        if let Some(session_id) = headers.get("x-session-id").and_then(|h| h.to_str().ok()) {
            entry.user_agent = Some(format!("session:{}", session_id));
        }
        let _ = audit.log_query(entry).await;
    }
    
    info!(
        "Query executed on share {}: {} rows in {}ms",
        req.share_code, result.row_count, elapsed
    );
    
    connect_response(ExecuteQueryResponse {
        success: true,
        columns: filtered_columns,
        rows: filtered_rows,
        row_count: result.row_count as i32,
        execution_time_ms: elapsed,
        error: None,
    })
}

/// POST /bennett.v1.QueryService/ExecuteWrite
pub async fn execute_write(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let (parts, body) = request.into_parts();
    let bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap_or_default();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    let client_ip = parts.extensions.get::<crate::api::middleware::ClientIp>().map(|c| c.0);

    let req: ExecuteWriteRequest = match parse_connect_request(&body.to_string()) {
        Ok(r) => r,
        Err(resp) => return resp,
    };
    
    let start = std::time::Instant::now();
    
    // Validate share and token
    let validated = match validate_share_request(&state, &req.share_code, &req.token, client_ip).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    
    // Require write permission
    if let Err(resp) = require_write_permission(&validated.permission) {
        return resp;
    }
    
    // Validate SQL (stricter for writes)
    if let Err(resp) = validate_shared_sql(&req.sql, &validated.permission) {
        return resp;
    }
    
    // Apply RLS to write
    let sql = apply_rls(&req.sql, validated.rls.as_deref());
    
    // Find database
    let db_instance = {
        let dbs = state.databases.lock().unwrap();
        dbs.iter().find(|d| d.id == validated.db_id).cloned()
    };
    
    let db_instance = match db_instance {
        Some(d) => d,
        None => return connect_error("not_found", "Database not available"),
    };
    
    // Acquire connection lock once for auto-connect + execute
    let result = {
        let mut conn = state.connections.lock().await;
        if !conn.is_connected(&db_instance.id) {
            if let Err(e) = conn.connect(&db_instance).await {
                return connect_error("unavailable", &format!("Connection failed: {}", e));
            }
        }
        
        match conn.execute(&db_instance.id, &sql).await {
            Ok(r) => r,
            Err(e) => {
                return connect_response(ExecuteWriteResponse {
                    success: false,
                    rows_affected: 0,
                            last_insert_id: None,
                    execution_time_ms: start.elapsed().as_millis() as i64,
                    error: Some(format!("Write failed: {}", e)),
                });
            }
        }
    };
    
    let elapsed = start.elapsed().as_millis() as i64;
    
    info!(
        "Write executed on share {}: {} rows in {}ms",
        req.share_code, result.row_count, elapsed
    );
    
    connect_response(ExecuteWriteResponse {
        success: true,
        rows_affected: result.row_count as i64,
        last_insert_id: result.last_insert_id.clone(),
        execution_time_ms: elapsed,
        error: None,
    })
}

/// POST /bennett.v1.QueryService/StreamQuery
/// Stream query results in chunks using SSE for browser compatibility
pub async fn stream_query(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Response {
    let (parts, body) = request.into_parts();
    let bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap_or_default();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    let client_ip = parts.extensions.get::<crate::api::middleware::ClientIp>().map(|c| c.0);
    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct StreamQueryRequest {
        share_code: String,
        token: String,
        sql: String,
        #[serde(default = "default_chunk_size")]
        chunk_size: i32,
        #[serde(default = "default_max_chunks")]
        max_chunks: i32,
    }

    fn default_chunk_size() -> i32 { 1000 }
    fn default_max_chunks() -> i32 { 100 }

    let req: StreamQueryRequest = match parse_connect_request(&body.to_string()) {
        Ok(r) => r,
        Err(resp) => return resp,
    };

    let validated = match validate_share_request(&state, &req.share_code, &req.token, client_ip).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    if let Err(resp) = validate_shared_sql(&req.sql, &validated.permission) {
        return resp;
    }

    let sql = apply_rls(&req.sql, validated.rls.as_deref());
    let chunk_size = req.chunk_size.clamp(1, 10000);
    let max_chunks = req.max_chunks.clamp(1, 100);

    let db_instance = {
        let dbs = state.databases.lock().unwrap();
        dbs.iter().find(|d| d.id == validated.db_id).cloned()
    };

    let db_instance = match db_instance {
        Some(d) => d,
        None => return connect_error("not_found", "Database not available"),
    };

    // Acquire connection lock once, hold for entire operation
    let result = {
        let mut conn = state.connections.lock().await;
        if !conn.is_connected(&db_instance.id) {
            if let Err(e) = conn.connect(&db_instance).await {
                return connect_error("unavailable", &format!("Connection failed: {}", e));
            }
        }

        // Execute with LIMIT to control chunk size
        let limited_sql = format!("{} LIMIT {}", sql, chunk_size * max_chunks);
        match conn.execute(&db_instance.id, &limited_sql).await {
            Ok(r) => r,
            Err(e) => {
                return connect_response(ExecuteQueryResponse {
                    success: false,
                    columns: vec![],
                    rows: vec![],
                    row_count: 0,
                    execution_time_ms: 0,
                    error: Some(format!("Query failed: {}", e)),
                });
            }
        }
    };

    // Apply column projection
    let tables = crate::control_plane::query::cache::QueryCache::extract_tables(&sql);
    let primary_table = tables.first().map(|s| s.as_str());
    let (filtered_columns, filtered_rows) = crate::connect_rpc::project_columns(
        &result.columns,
        &result.rows,
        &validated.cols,
        primary_table,
    );

    // Stream via SSE
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<axum::response::sse::Event, std::convert::Infallible>>(16);
    let total_rows = filtered_rows.len() as i32;
    let columns = filtered_columns.clone();

    tokio::spawn(async move {
        let chunk_size = chunk_size as usize;
        let chunks = filtered_rows.chunks(chunk_size);

        let chunk_count = chunks.len();
        for (index, chunk) in chunks.enumerate().take(max_chunks as usize) {
            let chunk_data = serde_json::json!({
                "chunk_index": index,
                "columns": &columns,
                "rows": chunk,
                "row_count": chunk.len(),
                "is_last": index == chunk_count - 1 || index == max_chunks as usize - 1,
            });

            let event = axum::response::sse::Event::default()
                .data(chunk_data.to_string());

            if tx.send(Ok(event)).await.is_err() {
                break; // Client disconnected
            }
        }

        // Send completion event
        let _ = tx.send(Ok(
            axum::response::sse::Event::default()
                .event("complete")
                .data(serde_json::json!({ "total_rows": total_rows }).to_string())
        )).await;
    });

    let stream = tokio_stream::wrappers::ReceiverStream::new(rx);

    axum::response::Sse::new(stream)
        .keep_alive(axum::response::sse::KeepAlive::default())
        .into_response()
}

// POST /bennett.v1.QueryService/StreamQuery
// Streaming query execution complete
// Features: SSE chunked streaming, column projection, RLS enforcement, backpressure handling
