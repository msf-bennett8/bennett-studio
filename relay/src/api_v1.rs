//! Public API Gateway — /api/v1/*
//! Stable REST surface for external apps, authenticated with durable API
//! keys (Authorization: Bearer bnt_live_...) instead of short-lived share
//! JWTs. Same tunnel transport as shares. Rate limited per-key (primary
//! defense — durable keys have no expiry) and per-IP for pre-auth requests.

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use sha2::{Digest, Sha256};
use std::net::IpAddr;
use tracing::warn;

use crate::ProxyApiState;

fn hash_key(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    hasher.finalize().iter().map(|b| format!("{:02x}", b)).collect()
}

fn extract_bearer(headers: &HeaderMap) -> Option<String> {
    let auth = headers.get(axum::http::header::AUTHORIZATION)?.to_str().ok()?;
    auth.strip_prefix("Bearer ").map(|s| s.to_string())
}

/// Extract client IP, preferring proxy headers (Render sits behind Cloudflare)
fn extract_client_ip(headers: &HeaderMap) -> IpAddr {
    for header in ["cf-connecting-ip", "x-forwarded-for", "x-real-ip"] {
        if let Some(v) = headers.get(header).and_then(|v| v.to_str().ok()) {
            let first = v.split(',').next().unwrap_or(v).trim();
            if let Ok(ip) = first.parse::<IpAddr>() {
                return ip;
            }
        }
    }
    IpAddr::from([0, 0, 0, 0])
}

#[derive(Debug, serde::Deserialize)]
pub struct ApiV1QueryRequest {
    pub sql: String,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

pub async fn api_v1_query(
    State(state): State<ProxyApiState>,
    headers: HeaderMap,
    Json(req): Json<ApiV1QueryRequest>,
) -> impl IntoResponse {
    let client_ip = extract_client_ip(&headers);
    if let Err(e) = state.rate_limiter.check_ip(client_ip) {
        return (StatusCode::TOO_MANY_REQUESTS, Json(serde_json::json!({ "success": false, "error": e })));
    }

    let key = match extract_bearer(&headers) {
        Some(k) if k.starts_with("bnt_live_") => k,
        _ => return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({
            "success": false, "error": "Missing or malformed Authorization: Bearer bnt_live_... header"
        }))),
    };
    let key_hash = hash_key(&key);

    let host_id = match state.api_key_registry.resolve(&key_hash) {
        Some(h) => h,
        None => return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({
            "success": false, "error": "Invalid or revoked API key"
        }))),
    };

    if let Err(e) = state.rate_limiter.check_key(&key_hash).await {
        return (StatusCode::TOO_MANY_REQUESTS, Json(serde_json::json!({ "success": false, "error": e })));
    }

    let request_id = uuid::Uuid::new_v4().to_string();
    let msg = crate::tunnel_registry::TunnelMessageToEngine::ApiKeyQueryRequest {
        request_id, key_hash, sql: req.sql, limit: req.limit, offset: req.offset,
    };

    match state.tunnel_registry.send_and_wait(&host_id, msg, 30).await {
        Ok(result) => (StatusCode::OK, Json(result)),
        Err(e) => {
            warn!("api/v1/query tunnel error: {}", e);
            (StatusCode::BAD_GATEWAY, Json(serde_json::json!({
                "success": false, "error": format!("Engine unreachable: {}", e)
            })))
        }
    }
}

pub async fn api_v1_schema(
    State(state): State<ProxyApiState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let client_ip = extract_client_ip(&headers);
    if let Err(e) = state.rate_limiter.check_ip(client_ip) {
        return (StatusCode::TOO_MANY_REQUESTS, Json(serde_json::json!({ "success": false, "error": e })));
    }

    let key = match extract_bearer(&headers) {
        Some(k) if k.starts_with("bnt_live_") => k,
        _ => return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({
            "success": false, "error": "Missing or malformed Authorization header"
        }))),
    };
    let key_hash = hash_key(&key);

    let host_id = match state.api_key_registry.resolve(&key_hash) {
        Some(h) => h,
        None => return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({
            "success": false, "error": "Invalid or revoked API key"
        }))),
    };

    if let Err(e) = state.rate_limiter.check_key(&key_hash).await {
        return (StatusCode::TOO_MANY_REQUESTS, Json(serde_json::json!({ "success": false, "error": e })));
    }

    let request_id = uuid::Uuid::new_v4().to_string();
    let msg = crate::tunnel_registry::TunnelMessageToEngine::ApiKeySchemaRequest { request_id, key_hash };

    match state.tunnel_registry.send_and_wait(&host_id, msg, 30).await {
        Ok(result) => (StatusCode::OK, Json(result)),
        Err(e) => {
            warn!("api/v1/schema tunnel error: {}", e);
            (StatusCode::BAD_GATEWAY, Json(serde_json::json!({
                "success": false, "error": format!("Engine unreachable: {}", e)
            })))
        }
    }
}

pub async fn api_v1_health() -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({
        "service": "bennett-api-gateway", "status": "ok", "version": "v1"
    })))
}
