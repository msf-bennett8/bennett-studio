//! Standalone P2P Sidecar Proxy binary
//! 
//! Usage:
//!   bennett-p2p-proxy --share-url "https://share.bennett.studio/db/AG5BECGUT9?t=...&ice=..."
//!                     --http-bind 127.0.0.1:8080
//!                     --mysql-bind 127.0.0.1:3307

use clap::Parser;
use std::net::SocketAddr;
use tracing::info;

mod sidecar;
use sidecar::{HttpSidecar, MySqlSidecar};
use relay::transport::{IceCandidates, TransportFactory};

#[derive(Debug, Parser)]
#[command(name = "bennett-p2p-proxy")]
#[command(about = "Bennett Studio P2P Sidecar Proxy")]
#[command(version)]
struct ProxyConfig {
    /// Share URL with ICE candidates
    #[arg(long, env = "BENNETT_SHARE_URL")]
    share_url: String,

    /// HTTP proxy bind address
    #[arg(long, default_value = "127.0.0.1:8080", env = "BENNETT_HTTP_BIND")]
    http_bind: SocketAddr,

    /// MySQL proxy bind address
    #[arg(long, default_value = "127.0.0.1:3307", env = "BENNETT_MYSQL_BIND")]
    mysql_bind: SocketAddr,

    /// Log level
    #[arg(long, default_value = "info", env = "RUST_LOG")]
    log_level: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = ProxyConfig::parse();

    tracing_subscriber::fmt()
        .with_env_filter(&config.log_level)
        .with_target(true)
        .init();

    info!("Bennett P2P Sidecar Proxy starting");

    // Parse share URL to extract code, token, and ICE candidates
    let (code, token, remote_ice) = parse_share_url(&config.share_url)?;

    info!(share_code = %code, "Parsed share URL");

    // Create P2P transport (client mode — connects to remote engine)
    let transport = TransportFactory::create_p2p_client(remote_ice, Some(code))
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create P2P transport: {}", e))?;

    info!(transport = transport.name(), "P2P transport ready");

    // Start HTTP sidecar
    let http_sidecar = HttpSidecar::new(config.http_bind, transport.clone());
    let http_handle = tokio::spawn(async move {
        if let Err(e) = http_sidecar.run().await {
            tracing::error!("HTTP sidecar error: {}", e);
        }
    });

    // Start MySQL sidecar
    let mysql_sidecar = MySqlSidecar::new(config.mysql_bind, transport);
    let mysql_handle = tokio::spawn(async move {
        if let Err(e) = mysql_sidecar.run().await {
            tracing::error!("MySQL sidecar error: {}", e);
        }
    });

    info!(http = %config.http_bind, mysql = %config.mysql_bind, "Sidecar proxies listening");
    info!("Laravel .env: DB_HOST=127.0.0.1 DB_PORT={}", config.mysql_bind.port());
    info!("HTTP API: http://{}/bennett.v1.QueryService/ExecuteQuery", config.http_bind);

    // Wait for both
    tokio::select! {
        _ = http_handle => {},
        _ = mysql_handle => {},
    }

    Ok(())
}

/// Parse share URL: https://share.bennett.studio/db/AG5BECGUT9?t=JWT&ice=BASE64
fn parse_share_url(url: &str) -> anyhow::Result<(String, String, IceCandidates)> {
    // Extract code from path
    let code = url
        .split("/db/")
        .nth(1)
        .and_then(|s| s.split('?').next())
        .ok_or_else(|| anyhow::anyhow!("Invalid share URL: missing code"))?
        .to_string();

    // Extract token from query string
    let token = url
        .split("t=")
        .nth(1)
        .and_then(|s| s.split('&').next())
        .ok_or_else(|| anyhow::anyhow!("Invalid share URL: missing token"))?
        .to_string();

    // Extract ICE from query string
    let ice_b64 = url
        .split("ice=")
        .nth(1)
        .and_then(|s| s.split('&').next())
        .ok_or_else(|| anyhow::anyhow!("Invalid share URL: missing ICE candidates"))?
        .to_string();

    let ice = IceCandidates::from_base64(&ice_b64)
        .map_err(|e| anyhow::anyhow!("Invalid ICE candidates: {}", e))?;

    Ok((code, token, ice))
}
