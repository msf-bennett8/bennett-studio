pub mod http;

use axum::{
    routing::{get, post, put, delete},
    Router,
};
use crate::AppState;

pub use http::*;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/databases", get(http::list_databases))
        .route("/api/databases", post(http::create_database))
        .route("/api/databases/:id", get(http::get_database))
        .route("/api/databases/:id", put(http::update_database))
        .route("/api/databases/:id", delete(http::delete_database))
        .route("/api/databases/:id/start", post(http::start_database))
        .route("/api/databases/:id/stop", post(http::stop_database))
        .route("/api/health", get(http::health_check))
}
