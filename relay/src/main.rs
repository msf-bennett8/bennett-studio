//! Bennett Relay Server
//! Public-facing TLS proxy for Bennett Studio database shares
//!
//! Architecture:
//! - TLS termination on 0.0.0.0:443
//! - Protocol detection (HTTP vs MySQL wire)
//! - Share ID extraction from URL path or MySQL username
//! - Route lookup in SQLite (engine's share store)
//! - Bidirectional TCP proxy to local engine
//! - Graceful shutdown on SIGTERM/SIGINT
//!
//! Future: P2P transport fallback via WebRTC/QUIC

mod config;
mod health;
mod metrics;
mod multiplexer;
mod router;
mod server;
mod signaling;
mod transport;

use clap::Parser;
use std::sync::Arc;
use tokio::signal;
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

    // Install rustls crypto provider (required by rustls 0.23)
    let _ = tokio_rustls::rustls::crypto::aws_lc_rs::default_provider()
        .install_default();

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

    // Create transport (pooled TCP, or P2P stub)
    let transport: Arc<dyn transport::Transport> = if config.enable_p2p {
        info!("P2P transport enabled (stub)");
        transport::TransportFactory::create_p2p_stub()
    } else {
        info!("Pooled TCP transport active (connection pooling + splice)");
        transport::TransportFactory::create_pooled_tcp(
            config.engine_http,
            config.engine_mysql,
            config.max_conn_per_share,
        )
    };

    // Start health monitor
    let _health_handle = health::HealthMonitor::start(
        router.clone(),
        transport.clone(),
        config.health_interval,
    );

    // Start metrics HTTP endpoint (separate from TLS relay)
    let _metrics_handle = metrics::start_metrics_server(config.bind.port() + 1000).await;

    // Shutdown channel
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

    // Spawn signal handler
    let shutdown_tx_clone = shutdown_tx.clone();
    tokio::spawn(async move {
        let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to register SIGTERM handler");
        let mut sigint = signal::unix::signal(signal::unix::SignalKind::interrupt())
            .expect("Failed to register SIGINT handler");

        tokio::select! {
            _ = sigterm.recv() => {
                info!("Received SIGTERM, initiating graceful shutdown");
            }
            _ = sigint.recv() => {
                info!("Received SIGINT, initiating graceful shutdown");
            }
        }

        let _ = shutdown_tx_clone.send(true);
    });

    // Also handle Ctrl+C for Windows/non-Unix
    #[cfg(not(unix))]
    {
        let shutdown_tx_clone = shutdown_tx.clone();
        tokio::spawn(async move {
            if let Err(e) = tokio::signal::ctrl_c().await {
                warn!("Failed to listen for ctrl-c: {}", e);
                return;
            }
            info!("Received Ctrl+C, initiating graceful shutdown");
            let _ = shutdown_tx_clone.send(true);
        });
    }

    // Initialize and run relay server
    let relay = server::RelayServer::new(config, router, transport).await?;

    info!("Relay server ready — waiting for connections");
    
    // Run server with shutdown support
    relay.run(shutdown_rx).await?;

    info!("Relay server shutdown complete");
    Ok(())
}
