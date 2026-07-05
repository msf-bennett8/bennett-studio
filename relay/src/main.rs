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

    // Gather ICE candidates if requested
    if config.gather_ice {
        match transport::ice::gather_ice_candidates().await {
            Ok(candidates) => {
                // Output base64-encoded ICE for easy embedding in URLs
                println!("{}", candidates.to_base64());
                return Ok(());
            }
            Err(e) => {
                eprintln!("Failed to gather ICE candidates: {}", e);
                std::process::exit(1);
            }
        }
    }

    // Create transport (pooled TCP, or P2P)
    let transport: Arc<dyn transport::Transport> = if config.enable_p2p {
        info!("P2P transport enabled");

        // Firebase signaling mode (best UX)
        let p2p_transport = if config.use_firebase {
            let firebase_url = config.firebase_url.clone()
                .ok_or_else(|| anyhow::anyhow!("--firebase-url required for Firebase signaling"))?;
            let signaling = signaling::firebase::FirebaseSignaling::new(firebase_url);

            if config.p2p_mode == "engine" {
                // Engine mode: create room, upload ICE, wait for client
                let room_code = config.share_code.clone()
                    .unwrap_or_else(signaling::firebase::generate_room_code);

                let local_ice = transport::ice::gather_ice_candidates().await
                    .map_err(|e| anyhow::anyhow!("ICE gathering failed: {}", e))?;

                signaling.create_room(&room_code, &local_ice).await
                    .map_err(|e| anyhow::anyhow!("Failed to create signaling room: {}", e))?;

                info!(room = %room_code, "Firebase room created. Share code with client.");
                info!("Client should run: --enable-p2p --use-firebase --share-code {} --firebase-url {}", room_code, config.firebase_url.as_ref().unwrap());

                // Wait for client's ICE
                let client_ice = signaling.poll_for_client(&room_code, 300).await
                    .map_err(|e| anyhow::anyhow!("Failed to get client ICE: {}", e))?;

                signaling.mark_connected(&room_code).await.ok();

                transport::TransportFactory::create_p2p_server(local_ice, Some(room_code)).await
            } else if config.p2p_mode == "client" {
                // Client mode: join room by code, upload our ICE, get engine's ICE
                let room_code = config.share_code.clone()
                    .ok_or_else(|| anyhow::anyhow!("--share-code required for Firebase client mode"))?;

                let local_ice = transport::ice::gather_ice_candidates().await
                    .map_err(|e| anyhow::anyhow!("ICE gathering failed: {}", e))?;

                let engine_ice = signaling.join_room(&room_code, &local_ice).await
                    .map_err(|e| anyhow::anyhow!("Failed to join room: {}", e))?;

                transport::TransportFactory::create_p2p_client(engine_ice, Some(room_code)).await
            } else {
                return Err(anyhow::anyhow!("--p2p-mode must be 'engine' or 'client'"));
            }
        } else if let Some(remote_ice_b64) = &config.remote_ice {
            // Manual ICE mode (fallback, no server)
            let remote_ice = transport::ice::IceCandidates::from_base64(remote_ice_b64)
                .map_err(|e| anyhow::anyhow!("Invalid remote ICE: {}", e))?;
            transport::TransportFactory::create_p2p_client(remote_ice, config.share_code.clone()).await
        } else {
            // Server mode: gather our ICE and wait for connections
            let local_ice = transport::ice::gather_ice_candidates().await
                .map_err(|e| anyhow::anyhow!("ICE gathering failed: {}", e))?;
            info!("P2P server ICE: {}", serde_json::to_string_pretty(&local_ice).unwrap());
            transport::TransportFactory::create_p2p_server(local_ice, config.share_code.clone()).await
        };

        p2p_transport.map_err(|e| anyhow::anyhow!("P2P transport init failed: {}", e))?
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
