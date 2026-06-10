use axum::Router;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

use bennett_engine::{
    api::routes,
    models::database::{DatabaseInstance, DatabaseStatus},
    AppState,
};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env().add_directive("bennett_engine=debug".parse().unwrap()))
        .init();

    let state = AppState {
        databases: Arc::new(Mutex::new(vec![
            DatabaseInstance {
                id: "1".to_string(),
                name: "local-postgres".to_string(),
                db_type: "postgres".to_string(),
                version: "16.2".to_string(),
                status: DatabaseStatus::Running,
                port: 5433,
                size: "245 MB".to_string(),
                created_at: "2024-06-10".to_string(),
                container_id: Some("pg-16-local".to_string()),
            },
            DatabaseInstance {
                id: "2".to_string(),
                name: "dev-mysql".to_string(),
                db_type: "mysql".to_string(),
                version: "8.0".to_string(),
                status: DatabaseStatus::Stopped,
                port: 3307,
                size: "128 MB".to_string(),
                created_at: "2024-06-09".to_string(),
                container_id: None,
            },
        ])),
    };

    let app = Router::new()
        .merge(routes())
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .with_state(state);

    let port = std::env::var("BENNETT_ENGINE_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or_else(|| {
            // Try 3001 first, fallback to 3002, 3003, etc.
            let base_port = 3001;
            for offset in 0..10 {
                let port = base_port + offset;
                if std::net::TcpListener::bind(("0.0.0.0", port)).is_ok() {
                    return port;
                }
            }
            panic!("No available port found in range 3001-3010");
        });

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("Bennett Engine starting on http://{}", addr);
    info!("API endpoints:");
    info!("  GET    /api/databases");
    info!("  POST   /api/databases");
    info!("  GET    /api/databases/:id");
    info!("  PUT    /api/databases/:id");
    info!("  DELETE /api/databases/:id");
    info!("  POST   /api/databases/:id/start");
    info!("  POST   /api/databases/:id/stop");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
