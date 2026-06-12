pub mod http;
pub mod websocket;

use axum::{
    routing::{get, post, put, delete},
    Router,
};
use crate::AppState;

pub use http::*;
pub use websocket::*;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/databases", get(http::list_databases))
        .route("/api/databases", post(http::create_database))
        .route("/api/databases/:id", get(http::get_database))
        .route("/api/databases/:id", put(http::update_database))
        .route("/api/databases/:id", delete(http::delete_database))
        .route("/api/databases/:id/start", post(http::start_database))
        .route("/api/databases/:id/stop", post(http::stop_database))
        .route("/api/databases/:id/query", post(http::execute_query))
        .route("/api/databases/:id/schema", get(http::get_schema))
        .route("/api/databases/:id/data", post(http::get_table_data))
        .route("/api/databases/:id/rows/update", post(http::update_row))
        .route("/api/databases/:id/rows/delete", post(http::delete_row))
        .route("/api/databases/:id/columns", post(http::get_table_columns))
        .route("/api/databases/:id/rows/insert", post(http::insert_row))
        .route("/api/databases/:id/ws", get(websocket::ws_handler))
        .route("/api/health", get(http::health_check))
}
