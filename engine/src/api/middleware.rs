use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
};
use tracing::info;

pub async fn request_logger(req: Request, next: Next) -> Response {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let start = std::time::Instant::now();
    
    let response = next.run(req).await;
    
    let duration = start.elapsed();
    info!(
        "{} {} -> {} in {:?}",
        method,
        uri,
        response.status().as_u16(),
        duration
    );
    
    response
}

// Stub for future auth middleware
pub async fn auth_check(req: Request, next: Next) -> Response {
    // TODO: Implement JWT/API key validation
    next.run(req).await
}
