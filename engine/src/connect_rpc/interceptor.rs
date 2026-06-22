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
/// TODO: Phase 4 - Implement token bucket rate limiter
pub async fn rate_limit_interceptor(
    State(_state): State<AppState>,
    req: Request,
    next: Next,
) -> Response {
    // TODO: Phase 4 - Check rate limits per share_code/IP
    next.run(req).await
}

/// Audit logging interceptor
/// TODO: Phase 5 - Log all queries with user attribution
pub async fn audit_interceptor(
    State(_state): State<AppState>,
    req: Request,
    next: Next,
) -> Response {
    let path = req.uri().path().to_string();
    let start = std::time::Instant::now();
    
    let response = next.run(req).await;
    
    let elapsed = start.elapsed().as_millis();
    info!("Connect-RPC {} completed in {}ms", path, elapsed);
    
    response
}
