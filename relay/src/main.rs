//! Bennett Relay Server
//! Public-facing TLS proxy for Bennett Studio database shares
//!
//! Architecture:
//! - TLS termination on 0.0.0.0:443
//! - Protocol detection (HTTP vs MySQL wire)
//! - Share ID extraction from URL path or MySQL username
//! - Route lookup in SQLite (engine's share store)
//! - Bidirectional TCP proxy to local engine
//!
//! Future: P2P transport fallback via WebRTC/QUIC

mod config;
mod health;
mod multiplexer;
mod router;
mod server;
mod signaling;
mod transport;

use clap::Parser;
use std::sync::Arc;
use tracing::{error, info};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse configuration
    let config = config::RelayConfig::parse();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(&config.log_level)
        .with_target(true)
        .init();

    info!(
        version = env!("CARGO_PKG_VERSION"),
        bind = %config.bind,
        "Bennett Relay Server starting"
    );

    // Resolve database path
    let db_path = config.resolve_db_path();

    // Initialize share router (reads engine's SQLite DB)
    let router = router::ShareRouter::new(
        &db_path,
        config.engine_http.port(),
        config.engine_mysql.port(),
    )
    .await
    .map_err(|e| {
        error!("Failed to initialize share router: {}", e);
        e
    })?;

    // Start background route refresh
    let _refresh_handle = router.start_refresh_task(config.route_refresh);

    // Create transport (TCP to local engine, or P2P stub)
    let transport: Arc<dyn transport::Transport> = if config.enable_p2p {
        info!("P2P transport enabled (stub)");
        transport::TransportFactory::create_p2p_stub()
    } else {
        info!("TCP transport active");
        transport::TransportFactory::create_tcp(
            config.engine_http,
            config.engine_mysql,
        )
    };

    // Start health monitor
    let _health_handle = health::HealthMonitor::start(
        router.clone(),
        transport.clone(),
        config.health_interval,
    );

    // Initialize and run relay server
    let relay = server::RelayServer::new(config, router, transport).await?;

    info!("Relay server ready");
    relay.run().await?;

    Ok(())
}
