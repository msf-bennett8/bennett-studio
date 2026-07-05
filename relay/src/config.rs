//! Relay server configuration
//! Loads from CLI args, env vars, or config file

use clap::Parser;
use std::net::SocketAddr;
use std::path::PathBuf;

/// Bennett Relay Server — Public-facing TLS proxy for database shares
#[derive(Debug, Clone, Parser)]
#[command(name = "bennett-relay")]
#[command(about = "Bennett Studio Public Relay Server")]
#[command(version)]
pub struct RelayConfig {
    /// Bind address for public TLS listener
    #[arg(long, default_value = "0.0.0.0:443", env = "BENNETT_RELAY_BIND")]
    pub bind: SocketAddr,

    /// Path to SQLite database (engine's share store)
    #[arg(long, default_value = "~/.bennett/data/shares.db", env = "BENNETT_RELAY_DB_PATH")]
    pub db_path: String,

    /// Path to TLS certificate directory
    #[arg(long, default_value = "./certs", env = "BENNETT_RELAY_CERT_DIR")]
    pub cert_dir: PathBuf,

    /// Engine Connect-RPC host:port
    #[arg(long, default_value = "127.0.0.1:3001", env = "BENNETT_RELAY_ENGINE_HTTP")]
    pub engine_http: SocketAddr,

    /// Engine MySQL wire proxy host:port
    #[arg(long, default_value = "127.0.0.1:13307", env = "BENNETT_RELAY_ENGINE_MYSQL")]
    pub engine_mysql: SocketAddr,

    /// Health check interval (seconds)
    #[arg(long, default_value_t = 30, env = "BENNETT_RELAY_HEALTH_INTERVAL")]
    pub health_interval: u64,

    /// Route refresh interval (seconds)
    #[arg(long, default_value_t = 5, env = "BENNETT_RELAY_ROUTE_REFRESH")]
    pub route_refresh: u64,

    /// Maximum concurrent connections per share
    #[arg(long, default_value_t = 100, env = "BENNETT_RELAY_MAX_CONN_PER_SHARE")]
    pub max_conn_per_share: usize,

    /// Connection pool size per protocol
    #[arg(long, default_value_t = 50, env = "BENNETT_RELAY_POOL_SIZE")]
    pub pool_size: usize,

    /// Enable P2P transport (stub for now)
    #[arg(long, default_value_t = false, env = "BENNETT_RELAY_ENABLE_P2P")]
    pub enable_p2p: bool,

    /// Log level
    #[arg(long, default_value = "info", env = "RUST_LOG")]
    pub log_level: String,
}

impl RelayConfig {
    /// Expand tilde in paths
    pub fn resolve_db_path(&self) -> PathBuf {
        let path = shellexpand::tilde(&self.db_path);
        PathBuf::from(path.as_str())
    }
}

// Minimal shellexpand replacement to avoid extra dependency
mod shellexpand {
    pub fn tilde(path: &str) -> String {
        if path.starts_with("~/") {
            if let Some(home) = std::env::var_os("HOME") {
                return format!("{}{}", home.to_string_lossy(), &path[1..]);
            }
        }
        path.to_string()
    }
}
