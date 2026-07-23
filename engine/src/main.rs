use axum::Router;
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use tracing::info;
use bennett_engine::sharing::share_store::ShareStore;
use bennett_engine::sharing::p2p_listener::start_p2p_listener;

use bennett_engine::{
    api::routes,
    AppState,
};

#[tokio::main]
async fn main() {
    // Install rustls crypto provider for WebSocket TLS (relay tunnel)
    let _ = rustls::crypto::ring::default_provider().install_default();

    // Load .env file if present (for local development)
    // Production uses actual environment variables
    if let Err(e) = dotenvy::dotenv() {
        tracing::debug!(".env file not found or invalid: {}", e);
    } else {
        tracing::info!("Loaded environment from .env file");
    }

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

    // Start host heartbeat background task — beats for ALL active shares
    let heartbeat_store = state.share_store.clone();
    let heartbeat_ip = bennett_engine::utils::net::detect_lan_ip().unwrap_or_else(|| "127.0.0.1".to_string());
    let heartbeat_port = std::env::var("BENNETT_ENGINE_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3001u16);

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
        loop {
            interval.tick().await;

            // Get all active shares and send heartbeat for each unique host_id
            // This ensures heartbeats match the host_id stored in share records
            match heartbeat_store.get_all_active_host_ids().await {
                Ok(host_ids) => {
                    for host_id in host_ids {
                        if let Err(e) = heartbeat_store.record_heartbeat(
                            &host_id,
                            Some(heartbeat_ip.clone()),
                            Some(heartbeat_port),
                            env!("CARGO_PKG_VERSION"),
                        ).await {
                            tracing::warn!("Heartbeat failed for {}: {}", host_id, e);
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to get active host IDs for heartbeat: {}", e);
                }
            }
        }
    });

    // PHASE 6: Start relay tunnel for remote engine → relay communication
    // This allows the Render relay to forward traffic when P2P fails
    let relay_url = std::env::var("BENNETT_RELAY_URL")
        .unwrap_or_else(|_| {
            tracing::warn!("BENNETT_RELAY_URL not set — tunnel disabled, P2P only mode");
            String::new()
        });

    if !relay_url.is_empty() {
        // Use stable host ID from share store, or generate and persist
        let host_id = match state.share_store.get_host_id().await.ok().flatten() {
            Some(id) => id,
            None => {
                let new_id = format!("host-{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap_or("unknown"));
                if let Err(e) = state.share_store.set_host_id(&new_id).await {
                    tracing::warn!("Failed to persist host_id: {}", e);
                } else {
                    tracing::info!("Generated and persisted new host_id: {}", new_id);
                }
                new_id
            }
        };
        let token_manager_clone = state.token_manager.clone();
        let share_store_clone = state.share_store.clone();
        let connection_manager_clone = state.connections.clone();
        let tunnel_tx_clone = state.tunnel_tx.clone();
        let state_databases_clone = state.databases.clone();
        let rate_limiter_clone = state.rate_limiter.clone();

        tokio::spawn(async move {
            use bennett_engine::sharing::relay::start_relay_tunnel;

            match start_relay_tunnel(
                relay_url,
                host_id,
                token_manager_clone,
                share_store_clone,
                Some(connection_manager_clone),
                Some(state_databases_clone),
                Some(rate_limiter_clone),
            ).await {
                Ok(tx) => {
                    tracing::info!("Relay tunnel established — engine reachable via relay fallback");
                    // Store tunnel sender in AppState so create_share can notify relay
                    let mut tunnel_lock = tunnel_tx_clone.write().await;
                    *tunnel_lock = Some(tx);
                    drop(tunnel_lock);
                    tracing::info!("Tunnel sender stored in AppState — share notifications enabled");
                    loop {
                        tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
                    }
                }
                Err(e) => {
                    tracing::warn!("Relay tunnel failed: {}", e);
                }
            }
        });
    } else {
        tracing::info!("Relay tunnel not configured — running in P2P-only mode");
    }

    // PHASE 6: Start P2P listener for direct browser connections via Firebase signaling
    let p2p_db_path = std::env::var("BENNETT_DATA_DIR")
        .map(|d| std::path::PathBuf::from(d).join("shares.db"))
        .unwrap_or_else(|_| std::path::PathBuf::from("./data/shares.db"));
    let p2p_token_manager = state.token_manager.clone();
    let p2p_share_store = state.share_store.clone();
    let p2p_conn_manager = state.connections.clone();

    tokio::spawn(async move {
        use bennett_engine::sharing::p2p_listener::start_p2p_listener;
        
        if let Err(e) = start_p2p_listener(
            p2p_db_path,
            p2p_token_manager,
            p2p_share_store,
            p2p_conn_manager,
        ).await {
            tracing::error!("P2P listener error: {}", e);
        }
    });

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
                    "https://share-bennett-studio.vercel.app".parse().unwrap(),
                    "https://bennett-relay.onrender.com".parse().unwrap(),
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

    // Resurrect connection pools for databases with active shares
    info!("Resurrecting connection pools for active shares...");
    {
        let db_list = {
            let db = state.databases.lock().unwrap();
            db.clone()
        };
        
        for instance in &db_list {
            match state.share_store.list_shares_by_db(&instance.id).await {
                Ok(shares) if !shares.is_empty() => {
                    info!("Database {} has {} active share(s), pre-connecting...", instance.name, shares.len());
                    let mut conn = state.connections.lock().await;
                    if let Err(e) = conn.connect(instance).await {
                        tracing::warn!("Failed to resurrect connection for {}: {}", instance.name, e);
                    } else {
                        info!("Connection pool resurrected for {}", instance.name);
                    }
                }
                Ok(_) => {
                    // No active shares, skip
                }
                Err(e) => {
                    tracing::warn!("Failed to list shares for {}: {}", instance.id, e);
                }
            }
        }
    }
}
