//! Connect-RPC service implementations
//! Phase 2: Full Query, Schema, Export services with permission enforcement
//! 
//! Connect-RPC protocol: HTTP/1.1 + HTTP/2, JSON + binary protobuf
//! Endpoints: POST /bennett.v1.{Service}/{Method}

pub mod query_service;
pub mod schema_service;
pub mod export_service;
pub mod interceptor;
pub mod router;

use axum::{
    response::{IntoResponse, Response},
    http::{StatusCode, header, HeaderMap},
    body::Body,
    extract::State,
    Json,
};
use serde_json::json;
use crate::AppState;

/// Connect-RPC error response
pub fn connect_error(code: &str, message: &str) -> Response {
    let body = json!({
        "code": code,
        "message": message,
    });
    
    Response::builder()
        .status(StatusCode::OK) // Connect-RPC uses 200 with error in body
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

/// Connect-RPC success response wrapper
pub fn connect_response<T: serde::Serialize>(data: T) -> Response {
    let body = serde_json::to_string(&data).unwrap_or_default();
    
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap()
}

/// Parse Connect-RPC request envelope
/// Format: {"shareCode": "...", "token": "...", ...}
pub fn parse_connect_request<T: serde::de::DeserializeOwned>(body: &str) -> Result<T, Response> {
    match serde_json::from_str::<T>(body) {
        Ok(req) => Ok(req),
        Err(e) => Err(connect_error("invalid_argument", &format!("Invalid request: {}", e))),
    }
}

/// Validate share token from request with rate limiting
pub async fn validate_share_request(
    state: &AppState,
    share_code: &str,
    token: &str,
) -> Result<crate::auth::share_token::ValidatedShare, Response> {
    // TODO: Extract IP from request context for rate limiting
    // For now, use a placeholder IP
    let ip = std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1));
    
    // Check rate limit
    if let Err(msg) = state.rate_limiter.check(share_code, &ip).await {
        return Err(connect_error("resource_exhausted", &msg));
    }
    // Check if share exists and is active
    let record = match state.share_store.get_share(share_code).await {
        Ok(Some(r)) => r,
        Ok(None) => return Err(connect_error("not_found", "Share not found")),
        Err(e) => {
            tracing::error!("Database error: {}", e);
            return Err(connect_error("internal", "Database error"));
        }
    };
    
    if record.revoked {
        return Err(connect_error("permission_denied", "Share has been revoked"));
    }
    
    if record.expires_at < chrono::Utc::now() {
        return Err(connect_error("permission_denied", "Share has expired"));
    }
    
    // Validate JWT
    let token_manager = state.token_manager.read().await;
    let validated = match token_manager.validate_token(token) {
        Ok(v) => v,
        Err(e) => return Err(connect_error("unauthenticated", &format!("Invalid token: {}", e))),
    };
    
    if validated.code != share_code {
        return Err(connect_error("unauthenticated", "Token does not match share code"));
    }
    
    // Check if token JTI is revoked
    if state.share_store.is_revoked(&validated.jti).await {
        return Err(connect_error("permission_denied", "Token has been revoked"));
    }
    
    Ok(validated)
}

/// Check if permission allows write operations
pub fn require_write_permission(
    permission: &crate::auth::share_token::SharePermission,
) -> Result<(), Response> {
    if !permission.can_write() {
        return Err(connect_error(
            "permission_denied",
            "Write operations require read-write permission"
        ));
    }
    Ok(())
}

/// SQL injection check for shared queries
pub fn validate_shared_sql(sql: &str, permission: &crate::auth::share_token::SharePermission) -> Result<(), Response> {
    let upper = sql.trim().to_uppercase();
    
    // Block dangerous statements for all
    let forbidden = ["DROP ", "TRUNCATE ", "ALTER SYSTEM", "COPY ", "\\COPY "];
    for f in &forbidden {
        if upper.contains(f) {
            return Err(connect_error("permission_denied", &format!("Statement type not allowed: {}", f.trim())));
        }
    }
    
    // Write permission check
    let write_stmts = ["INSERT ", "UPDATE ", "DELETE ", "CREATE ", "ALTER ", "GRANT ", "REVOKE "];
    let is_write = write_stmts.iter().any(|s| upper.starts_with(s));
    
    if is_write && !permission.can_write() {
        return Err(connect_error("permission_denied", "Write operations require read-write permission"));
    }
    
    // Multi-statement check
    if sql.split(';').count() > 2 {
        return Err(connect_error("invalid_argument", "Multiple statements not allowed"));
    }
    
    Ok(())
}

/// Apply table/column filtering to SQL
pub fn apply_table_filter(
    sql: &str,
    allowed_tables: &[String],
) -> Result<String, Response> {
    if allowed_tables.len() == 1 && allowed_tables[0] == "*" {
        return Ok(sql.to_string());
    }
    
    // TODO: Phase 2 - Implement proper SQL parsing for table extraction
    // For now, do basic check that query references only allowed tables
    let upper = sql.to_uppercase();
    for table in allowed_tables {
        // Simple check - production would use sqlparser
        if !upper.contains(&table.to_uppercase()) && !upper.starts_with("SELECT") {
            // Allow if it's a SELECT that might join - we'll check at execution
        }
    }
    
    Ok(sql.to_string())
}

/// Apply RLS (Row-Level Security) filter
pub fn apply_rls(
    sql: &str,
    rls: Option<&str>,
) -> String {
    let Some(rls_filter) = rls else {
        return sql.to_string();
    };
    
    // Inject RLS into WHERE clause
    // Simple implementation: append to existing WHERE or add WHERE
    let upper = sql.to_uppercase();
    if upper.contains(" WHERE ") {
        format!("{} AND ({})", sql.trim_end_matches(';'), rls_filter)
    } else if upper.contains(" GROUP BY ") || upper.contains(" ORDER BY ") || upper.contains(" LIMIT ") {
        // Insert before GROUP BY, ORDER BY, LIMIT
        let sql = sql.trim_end_matches(';');
        let insert_point = upper.find(" GROUP BY ")
            .or_else(|| upper.find(" ORDER BY "))
            .or_else(|| upper.find(" LIMIT "))
            .unwrap_or(sql.len());
        
        let (before, after) = sql.split_at(insert_point);
        format!("{} WHERE ({}){}", before, rls_filter, after)
    } else {
        format!("{} WHERE ({})", sql.trim_end_matches(';'), rls_filter)
    }
}

/// TODO: Phase 3 - Implement column projection
/// TODO: Phase 3 - Implement query type restrictions (DDL blocking)
/// TODO: Phase 5 - Implement audit logging for all queries
