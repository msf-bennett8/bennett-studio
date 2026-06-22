use axum::{
    extract::{ConnectInfo, Request},
    middleware::Next,
    response::Response,
};
use std::net::{IpAddr, SocketAddr};
use tracing::info;

/// Extension key for client IP
#[derive(Clone, Copy, Debug)]
pub struct ClientIp(pub IpAddr);

/// Extract real client IP from headers or connection info
pub async fn client_ip_middleware(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request: Request,
    next: Next,
) -> Response {
    let ip = extract_client_ip(&request, addr.ip());
    
    let mut request = request;
    request.extensions_mut().insert(ClientIp(ip));
    
    next.run(request).await
}

/// Extract IP from X-Forwarded-For, X-Real-IP, or direct connection
fn extract_client_ip(request: &Request, direct_ip: IpAddr) -> IpAddr {
    // Check X-Forwarded-For (common behind reverse proxy)
    if let Some(forwarded) = request.headers().get("x-forwarded-for") {
        if let Ok(forwarded_str) = forwarded.to_str() {
            // X-Forwarded-For: client, proxy1, proxy2
            if let Some(first) = forwarded_str.split(',').next() {
                if let Ok(ip) = first.trim().parse::<IpAddr>() {
                    return ip;
                }
            }
        }
    }
    
    // Check X-Real-IP (nginx)
    if let Some(real_ip) = request.headers().get("x-real-ip") {
        if let Ok(real_ip_str) = real_ip.to_str() {
            if let Ok(ip) = real_ip_str.parse::<IpAddr>() {
                return ip;
            }
        }
    }
    
    // Check CF-Connecting-IP (Cloudflare)
    if let Some(cf_ip) = request.headers().get("cf-connecting-ip") {
        if let Ok(cf_ip_str) = cf_ip.to_str() {
            if let Ok(ip) = cf_ip_str.parse::<IpAddr>() {
                return ip;
            }
        }
    }
    
    // Fallback to direct connection IP
    direct_ip
}

/// Request logger middleware (existing)
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

/// JWT/API key validation middleware
/// Validates Bearer token in Authorization header
pub async fn auth_check(req: Request, next: Next) -> Response {
    let auth_header = req.headers().get("authorization");
    
    match auth_header {
        Some(header) => {
            if let Ok(header_str) = header.to_str() {
                if header_str.starts_with("Bearer ") {
                    let token = &header_str[7..];
                    // Validate token format (basic check)
                    if token.len() > 10 && token.contains('.') {
                        // Pass to next middleware
                        // Full JWT validation happens in handlers
                        return next.run(req).await;
                    }
                }
            }
            // Invalid auth header
            Response::builder()
                .status(axum::http::StatusCode::UNAUTHORIZED)
                .body(axum::body::Body::from(r#"{"code":"unauthenticated","message":"Invalid or missing Bearer token"}"#))
                .unwrap()
        }
        None => {
            // No auth header - allow for public endpoints
            // In production, require auth for all endpoints
            next.run(req).await
        }
    }
}
