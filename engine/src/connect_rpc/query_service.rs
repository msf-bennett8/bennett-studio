//! Connect-RPC QueryService implementation
//! ExecuteQuery, StreamQuery, ExecuteWrite

use axum::{
    extract::State,
    response::{IntoResponse, Response},
    http::StatusCode,
    body::Body,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
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
    Json(body): Json<serde_json::Value>,
) -> Response {
    let req: ExecuteQueryRequest = match parse_connect_request(&body.to_string()) {
        Ok(r) => r,
        Err(resp) => return resp,
    };
    
    let start = std::time::Instant::now();
    
    // Validate share and token
    let validated = match validate_share_request(&state, &req.share_code, &req.token).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    
    // Validate SQL
    if let Err(resp) = validate_shared_sql(&req.sql, &validated.permission) {
        return resp;
    }
    
    // Apply RLS
    let sql = apply_rls(&req.sql, validated.rls.as_deref());
    
    // Limit check
    let limit = req.limit.clamp(1, 10000);
    
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
    
    // Auto-connect
    {
        let mut conn = state.connections.lock().await;
        if !conn.is_connected(&db_instance.id) {
            if let Err(e) = conn.connect(&db_instance).await {
                error!("Connection failed for {}: {}", db_instance.id, e);
                return connect_error("unavailable", "Database connection failed");
            }
        }
    }
    
    // Check query cache for SELECT queries
    let is_select = sql.trim().to_uppercase().starts_with("SELECT") || sql.trim().to_uppercase().starts_with("WITH");
    let cache_key = format!("{}:{}", db_instance.id, final_sql);
    
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
    
    // Execute query with LIMIT appended
    let final_sql = if !sql.to_uppercase().contains("LIMIT") {
        format!("{} LIMIT {}", sql, limit)
    } else {
        sql
    };
    
    let result = {
        let conn = state.connections.lock().await;
        match conn.execute(&db_instance.id, &final_sql).await {
            Ok(r) => r,
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
        }
    };
    
    // Convert to JSON values
    let rows: Vec<Vec<serde_json::Value>> = result.rows;
    
    let elapsed = start.elapsed().as_millis() as i64;
    
    // Record guest activity
    // TODO: Link to session ID from context
    // Store in cache
    if is_select {
        let tables = vec![]; // TODO: Extract tables from query
        state.query_cache.put(&db_instance.id, &final_sql, Some(&req.share_code), result.clone(), tables).await;
    }
    
    // Audit log
    if let Some(audit) = &state.audit_service {
        let entry = crate::audit::create_entry(
            &req.share_code,
            &validated.db_id,
            "127.0.0.1", // TODO: Extract real IP
            &req.sql,
            result.row_count as i64,
            elapsed as i64,
            true,
            validated.permission.as_str(),
        );
        let _ = audit.log_query(entry).await;
    }
    
    info!(
        "Query executed on share {}: {} rows in {}ms",
        req.share_code, result.row_count, elapsed
    );
    
    connect_response(ExecuteQueryResponse {
        success: true,
        columns: result.columns,
        rows,
        row_count: result.row_count as i32,
        execution_time_ms: elapsed,
        error: None,
    })
}

/// POST /bennett.v1.QueryService/ExecuteWrite
pub async fn execute_write(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    let req: ExecuteWriteRequest = match parse_connect_request(&body.to_string()) {
        Ok(r) => r,
        Err(resp) => return resp,
    };
    
    let start = std::time::Instant::now();
    
    // Validate share and token
    let validated = match validate_share_request(&state, &req.share_code, &req.token).await {
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
    
    // Auto-connect
    {
        let mut conn = state.connections.lock().await;
        if !conn.is_connected(&db_instance.id) {
            if let Err(e) = conn.connect(&db_instance).await {
                return connect_error("unavailable", &format!("Connection failed: {}", e));
            }
        }
    }
    
    // Execute write
    let result = {
        let conn = state.connections.lock().await;
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
        last_insert_id: None, // TODO: Extract from result
        execution_time_ms: elapsed,
        error: None,
    })
}

/// TODO: Phase 2 - Implement StreamQuery for large result sets
/// TODO: Phase 2 - Implement query result caching
/// TODO: Phase 3 - Implement query plan analysis
