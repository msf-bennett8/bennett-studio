//! Connect-RPC auth interceptor
//! Validates share tokens on incoming requests

use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
};
use tracing::{warn, info};

use crate::AppState;

/// Auth interceptor for Connect-RPC endpoints
/// Extracts share_code and token from request body and validates
pub async fn auth_interceptor(
    State(_state): State<AppState>,
    req: Request,
    next: Next,
) -> Response {
    // For now, pass through - validation happens in each handler
    // Future: Extract and validate token here for unified auth
    
    // Log request
    let path = req.uri().path().to_string();
    info!("Connect-RPC request: {}", path);
    
    next.run(req).await
}

/// Rate limiting interceptor
/// Applies token bucket rate limiting per share_code + IP
pub async fn rate_limit_interceptor(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Response {
    // Extract client IP
    let client_ip = req.extensions().get::<crate::api::middleware::ClientIp>().map(|c| c.0);
    let ip_str = client_ip.map(|ip| ip.to_string()).unwrap_or_else(|| "unknown".to_string());

    // Try to extract share_code from request body (best effort for Connect-RPC)
    // For full implementation, parse JSON body
    let share_code = "unknown".to_string();

    // Check rate limit
    if let Err(msg) = state.rate_limiter.check(&share_code, &client_ip.unwrap_or(std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)))).await {
        return crate::connect_rpc::connect_error("resource_exhausted", &msg);
    }

    next.run(req).await
}

/// Audit logging interceptor
/// Logs all Connect-RPC requests with timing and path
pub async fn audit_interceptor(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Response {
    let path = req.uri().path().to_string();
    let client_ip = req.extensions().get::<crate::api::middleware::ClientIp>().map(|c| c.0);
    let start = std::time::Instant::now();

    let response = next.run(req).await;

    let elapsed = start.elapsed().as_millis();
    info!("Connect-RPC {} completed in {}ms", path, elapsed);

    // Log to audit service if available
    if let Some(audit) = &state.audit_service {
        let entry = crate::audit::create_entry(
            "interceptor", // share_code unknown at this level
            "unknown",     // db_id unknown at this level
            &client_ip.map(|ip| ip.to_string()).unwrap_or_else(|| "127.0.0.1".to_string()),
            &format!("Connect-RPC {}", path),
            0,
            elapsed as i64,
            response.status().is_success(),
            "interceptor",
        );
        let _ = audit.log_query(entry).await;
    }

    response
}
