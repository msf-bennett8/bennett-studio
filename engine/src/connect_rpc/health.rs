//! Connect-RPC health check

use axum::response::Response;

/// Simple health check for Connect-RPC
pub async fn connect_health() -> Response {
    let body = serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
    });

    Response::builder()
        .status(axum::http::StatusCode::OK)
        .header(axum::http::header::CONTENT_TYPE, "application/json")
        .body(axum::body::Body::from(body.to_string()))
        .unwrap()
}
