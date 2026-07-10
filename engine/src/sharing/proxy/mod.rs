//! Wire Protocol Proxy
//! Phase 5: TCP tunnel for MySQL/PostgreSQL wire protocols
//! Allows standard DB drivers (psql, mysql CLI) to connect via share URL
//!
//! Architecture:
//! Guest (psql) -> TCP :3307 -> Proxy -> Validate JWT -> Forward to local :3306
//!
//! TLS: Self-signed cert auto-generated per share, rotated every 24h

pub mod mysql;
pub mod postgres;
pub mod tls;
pub mod router;

use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::AsyncWriteExt;
use tracing::{info, warn};
use std::sync::Arc;

use crate::AppState;
use crate::sharing::proxy::tls::CertManager;
use crate::sharing::proxy::router::ProxyRouter;

/// Wire protocol proxy server
pub struct WireProxyServer {
    state: AppState,
    bind_addr: SocketAddr,
    cert_manager: Arc<tls::CertManager>,
    router: Arc<router::ProxyRouter>,
}

impl WireProxyServer {
    pub fn new(state: AppState, port: u16) -> Self {
        let bind_addr = SocketAddr::from(([0, 0, 0, 0], port));
        let cert_manager = Arc::new(tls::CertManager::new());
        let router = Arc::new(router::ProxyRouter::new());
        
        Self {
            state,
            bind_addr,
            cert_manager,
            router,
        }
    }
    
    /// Start the wire protocol proxy server
    pub async fn start(self) -> Result<(), Box<dyn std::error::Error>> {
        let listener = TcpListener::bind(self.bind_addr).await?;
        info!("Wire protocol proxy listening on {}", self.bind_addr);
        
        // Auto-register all active shares from share store into router
        // Query all non-revoked, non-expired shares and register them for wire protocol access
        let proxy_port = self.bind_addr.port();
        let all_dbs = {
            let dbs = self.state.databases.lock().unwrap();
            dbs.iter().map(|d| d.id.clone()).collect::<Vec<_>>()
        };
        
        for db_id in all_dbs {
            match self.state.share_store.list_shares_by_db(&db_id).await {
                Ok(shares) => {
                    for share in shares {
                        let db_type = if share.db_id.starts_with("postgres") {
                            "postgres"
                        } else {
                            "mysql"
                        };
                        let local_port = share.port.unwrap_or(3306);
                        let external_port = local_port + 1000;
                        
                        if let Err(e) = self.router.register_share(&share.code, db_type, local_port).await {
                            warn!("Failed to register share {} in wire proxy router: {}", share.code, e);
                        } else {
                            info!("Auto-registered wire proxy: share {} -> port {} (type: {}, db: {})", 
                                share.code, external_port, db_type, db_id);
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to list shares for db {}: {}", db_id, e);
                }
            }
        }
        
        // Also register the proxy port itself as a fallback
        if let Err(e) = self.router.register_share("wire-proxy-fallback", "mysql", proxy_port.saturating_sub(1000)).await {
            warn!("Failed to register fallback wire proxy port: {}", e);
        }
        
        loop {
            let (stream, peer_addr) = listener.accept().await?;
            let state = self.state.clone();
            let cert_manager = self.cert_manager.clone();
            let router = self.router.clone();
            let port = self.bind_addr.port();
            
            tokio::spawn(async move {
                if let Err(e) = handle_connection(stream, peer_addr, state, cert_manager, router, port).await {
                    warn!("Wire proxy connection from {} failed: {}", peer_addr, e);
                }
            });
        }
    }
}

/// Handle incoming TCP connection
/// Protocol detection: MySQL (0x0a handshake) vs PostgreSQL (SSLRequest/StartupMessage)
async fn handle_connection(
    mut client_stream: TcpStream,
    peer_addr: SocketAddr,
    state: AppState,
    cert_manager: Arc<CertManager>,
    router: Arc<ProxyRouter>,
    port: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    // Protocol detection by port:
    // MySQL clients wait for server handshake first — they send NOTHING on connect.
    // PostgreSQL clients send SSLRequest/StartupMessage immediately.
    // We use port-based detection instead of peek() to avoid blocking on MySQL.

    // Check connection limit
    if let Err(e) = router.try_connect(port).await {
        tracing::warn!("Connection limit exceeded for port {}: {}", port, e);
        let _ = client_stream.write_all(format!("Connection limit exceeded: {}\n", e).as_bytes()).await;
        return Ok(());
    }

    // Port-based protocol detection (avoids blocking peek on MySQL)
    let protocol = if port >= 13307 && port < 54300 {
        WireProtocol::MySQL
    } else if port >= 54300 {
        WireProtocol::PostgreSQL
    } else {
        // Fallback: try non-blocking peek for PostgreSQL pattern
        match tokio::time::timeout(tokio::time::Duration::from_millis(100), client_stream.peek(&mut [0u8; 1])).await {
            Ok(Ok(1)) => {
                // Client sent data — likely PostgreSQL
                WireProtocol::PostgreSQL
            }
            _ => {
                // No data from client — assume MySQL (server speaks first)
                WireProtocol::MySQL
            }
        }
    };
    
    info!("Wire proxy: {} connection from {}", protocol, peer_addr);
    
    match protocol {
        WireProtocol::MySQL => {
            let mysql_result = match mysql::handle_mysql_client(client_stream, peer_addr, state, cert_manager).await {
                Ok(()) => Ok(()),
                Err(e) => Err(format!("MySQL proxy error: {}", e)),
            };
            let _ = router.disconnect(port).await;
            if let Err(e) = mysql_result {
                return Err(e.into());
            }
        }
        WireProtocol::PostgreSQL => {
            let pg_result = match postgres::handle_postgres_client(client_stream, peer_addr, state, cert_manager).await {
                Ok(()) => Ok(()),
                Err(e) => Err(format!("PostgreSQL proxy error: {}", e)),
            };
            let _ = router.disconnect(port).await;
            if let Err(e) = pg_result {
                return Err(e.into());
            }
        }
        WireProtocol::Unknown => {
            warn!("Unknown wire protocol from {}, disconnecting", peer_addr);
            // Send error and close
            let _ = client_stream.write_all(b"Unknown protocol. Use MySQL or PostgreSQL wire protocol.\n").await;
        }
    }
    
    Ok(())
}

#[derive(Debug, Clone, Copy)]
enum WireProtocol {
    MySQL,
    PostgreSQL,
    Unknown,
}

impl std::fmt::Display for WireProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WireProtocol::MySQL => write!(f, "MySQL"),
            WireProtocol::PostgreSQL => write!(f, "PostgreSQL"),
            WireProtocol::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Validate share token from wire protocol connection
/// MySQL: username = share_code, password = JWT token
/// PostgreSQL: username = share_code, password = JWT token
pub async fn validate_wire_auth(
    state: &AppState,
    share_code: &str,
    token: &str,
    peer_addr: SocketAddr,
) -> Result<WireAuthResult, String> {
    // Log connection attempt
    if let Some(audit) = &state.audit_service {
        let entry = crate::audit::create_entry(
            share_code,
            "unknown", // Will be updated after validation
            &peer_addr.to_string(),
            "-- wire protocol connection attempt --",
            0,
            0,
            true,
            "ro",
        );
        let _ = audit.log_query(entry).await;
    }
    // Get share record
    let record = state.share_store.get_share(share_code).await
        .map_err(|e| format!("Database error: {}", e))?
        .ok_or_else(|| "Share not found".to_string())?;
    
    if record.revoked {
        return Err("Share has been revoked".to_string());
    }
    
    if record.expires_at < chrono::Utc::now() {
        return Err("Share has expired".to_string());
    }
    
    // Validate JWT
    let token_manager = state.token_manager.read().await;
    let validated = token_manager.validate_token(token)
        .map_err(|e| format!("Invalid token: {}", e))?;
    
    if validated.code != share_code {
        return Err("Token does not match share code".to_string());
    }
    
    // Check revocation
    if state.share_store.is_revoked(&validated.jti).await {
        return Err("Token has been revoked".to_string());
    }
    
    // Rate limit check
    let _rate_key = format!("{}:{}", share_code, peer_addr.ip());
    if let Err(e) = state.rate_limiter.check(share_code, &peer_addr.ip()).await {
        // Log rate limit violation
        if let Some(audit) = &state.audit_service {
            let entry = crate::audit::create_entry(
                share_code,
                &record.db_id,
                &peer_addr.to_string(),
                "-- wire protocol rate limit exceeded --",
                0,
                0,
                false,
                &record.permission,
            );
            let _ = audit.log_query(entry).await;
        }
        return Err(e);
    }
    
    // Find database
    let db_instance = {
        let dbs = state.databases.lock().unwrap();
        dbs.iter().find(|d| d.id == record.db_id).cloned()
    };
    
    let db_instance = db_instance.ok_or_else(|| "Database not available".to_string())?;
    
    Ok(WireAuthResult {
        validated,
        db_instance,
        peer_addr,
    })
}

/// Authentication result for wire protocol connections
pub struct WireAuthResult {
    pub validated: crate::auth::share_token::ValidatedShare,
    pub db_instance: crate::models::database::DatabaseInstance,
    pub peer_addr: SocketAddr,
}

// Wire protocol proxy implementation complete
// Features: connection limits per share, RLS injection, audit logging, MySQL + PostgreSQL support
