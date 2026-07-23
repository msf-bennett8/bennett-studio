pub mod activity;
pub mod api_keys;
pub mod engine_info;
pub mod guests;
pub mod health;
pub mod http;
pub mod middleware;
pub mod websocket;
pub mod websocket_buffer;
pub mod sharing;
// pub mod connect_rpc; // Module is at crate root: src/connect_rpc/mod.rs

use axum::{
    routing::{get, post, put, delete},
    Router,
};
use crate::AppState;

pub use http::*;
pub use websocket::*;
pub use sharing::*;

use axum::response::Response;
use axum::body::Body;
use axum::http::{StatusCode, header};

/// Prometheus metrics endpoint
pub async fn metrics_endpoint() -> Response {
    let registry = crate::telemetry::metrics::init_metrics();
    let body = registry.export_prometheus().await;
    
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(Body::from(body))
        .unwrap()
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .layer(axum::middleware::from_fn(middleware::client_ip_middleware))
        .route("/api/databases", get(http::list_databases))
        .route("/api/databases", post(http::create_database))
        .route("/api/databases/discover", post(http::discover_local_databases))
        .route("/api/databases/:id", get(http::get_database))
        .route("/api/databases/:id", put(http::update_database))
        .route("/api/databases/:id", delete(http::delete_database))
        .route("/api/databases/:id/start", post(http::start_database))
        .route("/api/databases/:id/stop", post(http::stop_database))
        .route("/api/databases/:id/unlock", post(http::unlock_database))
        .route("/api/databases/:id/status", get(http::get_database_status))
        .route("/api/databases/:id/env-scan", get(http::scan_env_files))
        .route("/api/databases/:id/query", post(http::execute_query))
        .route("/api/databases/:id/schema", get(http::get_schema))
        .route("/api/databases/:id/data", post(http::get_table_data))
        .route("/api/databases/:id/rows/update", post(http::update_row))
        .route("/api/databases/:id/rows/delete", post(http::delete_row))
        .route("/api/databases/:id/columns", post(http::get_table_columns))
        .route("/api/databases/:id/rows/insert", post(http::insert_row))
        .route("/api/databases/:id/ws", get(websocket::ws_handler))
        // Phase 1: Share endpoints
        .route("/api/shares", post(sharing::create_share))
        .route("/api/shares", get(sharing::list_shares))
        .route("/api/shares/:code", get(sharing::get_share_info))
        .route("/api/shares/:code", delete(sharing::revoke_share))
        .route("/api/shares/:code/permanent", delete(sharing::delete_share))
        .route("/api/shares/:code/pin", post(sharing::toggle_pin_share))
        .route("/api/shares/:code/schema", get(sharing::get_share_schema))
        .route("/api/shares/:code/query", post(sharing::execute_share_query))
        .route("/api/shares/:code/validate", post(sharing::validate_share))
        .route("/api/shares/:code/resolve", get(sharing::resolve_share))
        // API Keys — durable credentials for external apps
        .route("/api/keys", post(api_keys::create_api_key))
        .route("/api/keys", get(api_keys::list_api_keys))
        .route("/api/keys/:id", delete(api_keys::revoke_api_key))
        // General settings — real host/engine identity
        .route("/api/engine/info", get(engine_info::get_engine_info))
        // Activity settings — real audit log
        .route("/api/activity", get(activity::list_activity).delete(activity::clear_activity))
        // Guests settings — real connected-guest sessions
        .route("/api/guests", get(guests::list_guests))
        .route("/api/guests/:id", delete(guests::disconnect_guest))
        .route("/api/health", get(crate::api::health::comprehensive_health_check))
        // Notes endpoints
        .route("/api/notes", get(http::list_notes))
        .route("/api/notes", post(http::create_note))
        .route("/api/notes/search", get(http::search_notes))
        .route("/api/notes/sync", post(http::sync_notes))
        .route("/api/notes/:id", get(http::get_note))
        .route("/api/notes/:id", put(http::update_note))
        .route("/api/notes/:id", delete(http::delete_note))
        .route("/metrics", get(metrics_endpoint))
        // Phase 2: Connect-RPC full service endpoints
        .route("/bennett.v1.HealthService/Check", post(crate::connect_rpc::health::connect_health))
        .merge(crate::connect_rpc::router::connect_routes())
}
