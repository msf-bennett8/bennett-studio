use axum::Router;
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use tracing::info;

use bennett_engine::{
    api::routes,
    AppState,
};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env().add_directive("bennett_engine=debug".parse().unwrap()))
        .init();

    let state = match AppState::new() {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("Failed to initialize engine: {}", e);
            tracing::error!("Make sure Docker daemon is running");
            std::process::exit(1);
        }
    };

    // Verify Docker is accessible
    if let Err(e) = state.docker.verify().await {
        tracing::error!("Docker verification failed: {}", e);
        std::process::exit(1);
    }

    // Discover existing Bennett containers on startup
    info!("Scanning for existing Bennett database containers...");
    match state.docker.list_bennett_containers().await {
        Ok(containers) => {
            let mut db = state.databases.lock().unwrap();
            for container in containers {
                // Avoid duplicates (in case somehow already in state)
                if !db.iter().any(|d| d.id == container.id) {
                    info!(
                        "Discovered existing database: {} (id: {}, type: {}, port: {}, status: {:?})",
                        container.name, container.id, container.db_type, container.port, container.status
                    );
                    db.push(container);
                }
            }
            info!("Loaded {} existing database(s) from Docker", db.len());
        }
        Err(e) => {
            tracing::warn!("Failed to scan for existing containers: {}", e);
        }
    }

    let app = Router::new()
        .merge(routes())
        .layer(
            CorsLayer::new()
                .allow_origin([
                    "http://localhost:5173".parse().unwrap(),
                    "http://localhost:5174".parse().unwrap(),
                    "http://localhost:3000".parse().unwrap(),
                    "http://localhost:3001".parse().unwrap(),
                    "tauri://localhost".parse().unwrap(),
                ])
                .allow_methods([
                    axum::http::Method::GET,
                    axum::http::Method::POST,
                    axum::http::Method::PUT,
                    axum::http::Method::DELETE,
                ])
                .allow_headers([
                    axum::http::header::CONTENT_TYPE,
                    axum::http::header::AUTHORIZATION,
                ]),
        )
        .with_state(state);

    let port = std::env::var("BENNETT_ENGINE_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or_else(|| {
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
    info!("Docker runtime: connected");
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
