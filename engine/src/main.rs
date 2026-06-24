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
    
    // Initialize health check start time
    bennett_engine::api::health::init_start_time();

    let state = match AppState::new().await {
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
                    "http://localhost:3002".parse().unwrap(),
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
                    axum::http::header::HeaderName::from_static("x-share-code"),
                    axum::http::header::HeaderName::from_static("x-share-token"),
                    axum::http::header::HeaderName::from_static("x-requested-with"),
                ])
                .allow_credentials(true),
        )
        .with_state(state.clone());

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

    // Start gRPC server on a port derived from engine port (engine_port + 100)
    // This ensures no conflict even with dynamic port allocation
    let grpc_port = std::env::var("BENNETT_GRPC_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or_else(|| {
            let base_port = port + 100; // Always offset from actual engine port
            for offset in 0..10 {
                let port = base_port + offset;
                if std::net::TcpListener::bind(("0.0.0.0", port)).is_ok() {
                    return port;
                }
            }
            panic!("No available gRPC port found in range {}-{}", base_port, base_port + 9);
        });

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("Bennett Engine starting on http://{}", addr);
    info!("gRPC server on port {}", grpc_port);
    info!("Docker runtime: connected");
    info!("API endpoints:");
    info!("  GET    /api/databases");
    info!("  POST   /api/databases");
    info!("  GET    /api/databases/:id");
    info!("  PUT    /api/databases/:id");
    info!("  DELETE /api/databases/:id");
    info!("  POST   /api/databases/:id/start");
    info!("  POST   /api/databases/:id/stop");
    
    let grpc_state = state.clone();
    tokio::spawn(async move {
        if let Err(e) = bennett_engine::grpc::start_grpc_server(grpc_state, grpc_port).await {
            tracing::error!("gRPC server error: {}", e);
        }
    });
    
    info!("gRPC server starting on port {}", grpc_port);
    
    // Start wire protocol proxy (Phase 5)
    let proxy_port = std::env::var("BENNETT_WIRE_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or_else(|| {
            let base_port = 13307;
            for offset in 0..10 {
                let port = base_port + offset;
                if std::net::TcpListener::bind(("0.0.0.0", port)).is_ok() {
                    return port;
                }
            }
            panic!("No available port found in range 13307-13316");
        });
    
    let proxy_state = state.clone();
    tokio::spawn(async move {
        let proxy = bennett_engine::sharing::proxy::WireProxyServer::new(proxy_state, proxy_port);
        if let Err(e) = proxy.start().await {
            tracing::error!("Wire protocol proxy error: {}", e);
        }
    });
    
    info!("Wire protocol proxy starting on port {}", proxy_port);
    info!("MySQL: mysql -h host -P {} -u bennett_SHARECODE -p", proxy_port);
    info!("PostgreSQL: psql -h host -p {} -U bennett_SHARECODE", proxy_port);
    info!("API endpoints:");
    info!("gRPC-Web enabled for browser clients");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    
    // Graceful shutdown with SIGTERM/SIGINT
    let shutdown = tokio::spawn(async move {
        let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to create SIGTERM handler");
        let mut sigint = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())
            .expect("Failed to create SIGINT handler");
        
        tokio::select! {
            _ = sigterm.recv() => {
                info!("Received SIGTERM, starting graceful shutdown...");
            }
            _ = sigint.recv() => {
                info!("Received SIGINT, starting graceful shutdown...");
            }
        }
        
        // Drain connections
        info!("Draining connections...");
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        info!("Shutdown complete");
    });
    
    let server = axum::serve(listener, app)
        .with_graceful_shutdown(async {
            shutdown.await.ok();
        });
    
    if let Err(e) = server.await {
        tracing::error!("Server error: {}", e);
    }
}
