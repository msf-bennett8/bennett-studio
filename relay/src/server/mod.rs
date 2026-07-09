//! Bennett Relay Server — Industry-best architecture
//!
//! Single-port (443) with ALPN protocol negotiation:
//!   - h2 → HTTP/2 (gRPC-Web)
//!   - http/1.1 → Connect-RPC / REST
//!   - mysql → MySQL wire over TLS
//!
//! Features:
//!   - TLS termination with ALPN
//!   - Connection pooling to engine
//!   - splice() zero-copy on Linux
//!   - Per-share rate limiting
//!   - Structured JSON error responses

use crate::config::RelayConfig;
use crate::multiplexer::{
    extract_share_id_from_mysql_username,
    read_mysql_auth_response, send_mysql_error, send_mysql_handshake_v10, send_mysql_share_not_found,
    send_mysql_too_many_connections, ConnectionCounter,
};
use crate::router::ShareRouter;
use crate::transport::{ProtocolType, Transport, ALPN_HTTP1, ALPN_HTTP2, ALPN_MYSQL};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
// Note: splice() zero-copy only works on plain TcpStream pairs.
// TLS-terminated traffic uses tokio::io::copy_bidirectional.

use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::{rustls, TlsAcceptor};
use tracing::{debug, info, warn};

/// Relay server with ALPN-based single-port routing
pub struct RelayServer {
    config: RelayConfig,
    router: Arc<ShareRouter>,
    transport: Arc<dyn Transport>,
    tls_acceptor: TlsAcceptor,
    connection_counter: Arc<ConnectionCounter>,
}

impl RelayServer {
    pub async fn new(
        config: RelayConfig,
        router: Arc<ShareRouter>,
        transport: Arc<dyn Transport>,
    ) -> anyhow::Result<Self> {
        let tls_config = load_tls_config(&config.cert_dir).await?;
        let tls_acceptor = TlsAcceptor::from(Arc::new(tls_config));

        let counter = Arc::new(ConnectionCounter::new(config.max_conn_per_share));

        info!(
            bind = %config.bind,
            transport = transport.name(),
            max_conn_per_share = config.max_conn_per_share,
            "Relay server initialized (ALPN single-port)"
        );

        Ok(Self {
            config,
            router,
            transport,
            tls_acceptor,
            connection_counter: counter,
        })
    }

    pub async fn run(self, mut shutdown_rx: tokio::sync::watch::Receiver<bool>) -> anyhow::Result<()> {
        // P2P mode: don't bind TLS listener, handle QUIC streams instead
        if self.config.enable_p2p {
            return Arc::new(self).run_p2p(shutdown_rx).await;
        }

        let listener = TcpListener::bind(self.config.bind).await?;
        info!(bind = %self.config.bind, "Relay listening (ALPN: h2, http/1.1, mysql)");

        let server = Arc::new(self);

        loop {
            tokio::select! {
                accept_result = listener.accept() => {
                    let (client_stream, client_addr) = accept_result?;
                    let srv = server.clone();

                    tokio::spawn(async move {
                        if let Err(e) = srv.handle_client(client_stream, client_addr).await {
                            warn!(addr = %client_addr, error = %e, "Client handler error");
                        }
                    });
                }
                _ = shutdown_rx.changed() => {
                    if *shutdown_rx.borrow() {
                        info!("Shutdown signal received");
                        break;
                    }
                }
            }
        }

        info!("Relay server stopped");
        Ok(())
    }

    /// Handle single client — ALPN determines protocol
    async fn handle_client(
        self: &Arc<Self>,
        client_stream: TcpStream,
        client_addr: SocketAddr,
    ) -> anyhow::Result<()> {
        // Accept TLS — ALPN is negotiated during handshake
        let tls_stream = match self.tls_acceptor.accept(client_stream).await {
            Ok(s) => s,
            Err(e) => {
                warn!(addr = %client_addr, error = %e, "TLS handshake failed");
                return Ok(());
            }
        };

        // Get ALPN protocol from TLS session
        let alpn = tls_stream.get_ref().1.alpn_protocol();
        
        match alpn {
            Some(ALPN_HTTP2) => {
                debug!(addr = %client_addr, "ALPN: h2");
                self.handle_http2(tls_stream, client_addr).await?;
            }
            Some(ALPN_HTTP1) | None => {
                debug!(addr = %client_addr, "ALPN: http/1.1");
                self.handle_http1(tls_stream, client_addr).await?;
            }
            Some(ALPN_MYSQL) => {
                debug!(addr = %client_addr, "ALPN: mysql");
                self.handle_mysql_tls(tls_stream, client_addr).await?;
            }
            Some(other) => {
                warn!(addr = %client_addr, alpn = ?String::from_utf8_lossy(other), "Unknown ALPN");
            }
        }
        Ok(())
    }

    // ========================================================================
    // HTTP/1.1 Handler — Transparent Proxy
    // ========================================================================

    async fn handle_http1(
        &self,
        mut tls_stream: tokio_rustls::server::TlsStream<TcpStream>,
        client_addr: SocketAddr,
    ) -> anyhow::Result<()> {
        info!(addr = %client_addr, "HTTP/1.1 connection");

        // Acquire pooled connection to engine HTTP port
        let mut engine_conn = match self.transport.acquire(ProtocolType::ConnectRpc).await {
            Ok(conn) => conn,
            Err(e) => {
                warn!(addr = %client_addr, error = %e, "Engine HTTP connection failed");
                return Ok(());
            }
        };

        // Transparent bidirectional proxy — engine validates JWT, not relay
        let result = tokio::io::copy_bidirectional(&mut tls_stream, &mut engine_conn.stream).await;

        self.transport.release(engine_conn);

        match result {
            Ok((up, down)) => debug!(addr = %client_addr, bytes_up = up, bytes_down = down, "HTTP/1.1 done"),
            Err(e) => debug!(addr = %client_addr, error = %e, "HTTP/1.1 forward error"),
        }

        Ok(())
    }

    // ========================================================================
    // HTTP/2 Handler — Transparent Proxy
    // ========================================================================

    async fn handle_http2(
        &self,
        mut tls_stream: tokio_rustls::server::TlsStream<TcpStream>,
        client_addr: SocketAddr,
    ) -> anyhow::Result<()> {
        info!(addr = %client_addr, "HTTP/2 connection");

        // Acquire pooled connection to engine HTTP port
        let mut engine_conn = match self.transport.acquire(ProtocolType::ConnectRpc).await {
            Ok(conn) => conn,
            Err(e) => {
                warn!(addr = %client_addr, error = %e, "Engine HTTP/2 connection failed");
                return Ok(());
            }
        };

        // Transparent bidirectional proxy — engine handles h2 natively
        let result = tokio::io::copy_bidirectional(&mut tls_stream, &mut engine_conn.stream).await;

        self.transport.release(engine_conn);

        match result {
            Ok((up, down)) => debug!(addr = %client_addr, bytes_up = up, bytes_down = down, "HTTP/2 done"),
            Err(e) => debug!(addr = %client_addr, error = %e, "HTTP/2 forward error"),
        }

        Ok(())
    }

    // ========================================================================
    // MySQL over TLS Handler
    // ========================================================================
    
    async fn handle_mysql_tls(
        &self,
        mut tls_stream: tokio_rustls::server::TlsStream<TcpStream>,
        client_addr: SocketAddr,
    ) -> anyhow::Result<()> {
        // MySQL client expects server handshake over TLS
        send_mysql_handshake_v10(&mut tls_stream, "bennett-relay", 1).await?;

        let auth = match read_mysql_auth_response(&mut tls_stream).await {
            Ok(auth) => auth,
            Err(e) => {
                warn!(addr = %client_addr, error = %e, "MySQL auth failed");
                let _ = send_mysql_error(&mut tls_stream, 1, 1045, "28000", "Auth failed").await;
                return Ok(());
            }
        };

        let share_id = extract_share_id_from_mysql_username(&auth.username);

        info!(addr = %client_addr, share_id = %share_id, "MySQL/TLS auth");

        if !self.router.is_active(&share_id) {
            send_mysql_share_not_found(&mut tls_stream, 1).await?;
            return Ok(());
        }

        if !self.connection_counter.acquire(&share_id) {
            send_mysql_too_many_connections(&mut tls_stream, 1).await?;
            return Ok(());
        }

        let mut engine_conn = self.transport.acquire(ProtocolType::MySqlWire).await
            .map_err(|e| {
                self.connection_counter.release(&share_id);
                anyhow::anyhow!("Engine MySQL connection failed: {}", e)
            })?;

        let result = tokio::io::copy_bidirectional(&mut tls_stream, &mut engine_conn.stream).await;
        
        self.connection_counter.release(&share_id);
        self.transport.release(engine_conn);

        match result {
            Ok((up, down)) => debug!(share_id = %share_id, bytes_up = up, bytes_down = down, "MySQL/TLS done"),
            Err(e) => warn!(share_id = %share_id, error = %e, "MySQL/TLS forward error"),
        }

        Ok(())
    }

    // ========================================================================
    // P2P Mode — QUIC stream handling
    // ========================================================================

    async fn run_p2p(
        self: Arc<Self>,
        mut shutdown_rx: tokio::sync::watch::Receiver<bool>,
    ) -> anyhow::Result<()> {
        info!("Relay running in P2P mode");

        // Downcast to P2pTransport to access accept_stream()
        let p2p_transport = self.transport.as_any()
            .downcast_ref::<crate::transport::p2p::P2pTransport>()
            .ok_or_else(|| anyhow::anyhow!("P2P mode requires P2pTransport"))?;

        loop {
            tokio::select! {
                biased; // Check shutdown first to avoid starving it

                _ = shutdown_rx.changed() => {
                    if *shutdown_rx.borrow() {
                        info!("P2P shutdown signal received");
                        break;
                    }
                }

                // Accept incoming QUIC bidirectional streams from remote peer
                stream_result = p2p_transport.accept_stream() => {
                    match stream_result {
                        Ok((protocol, send, recv)) => {
                            let srv = self.clone();
                            tokio::spawn(async move {
                                if let Err(e) = srv.handle_p2p_stream(protocol, send, recv).await {
                                    warn!("P2P stream handler error: {}", e);
                                }
                            });
                        }
                        Err(e) => {
                            warn!("P2P stream accept failed: {}", e);
                            // Brief backoff to avoid tight spin on persistent errors
                            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                        }
                    }
                }

                // Periodic health check (less frequent since we have real work now)
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(5)) => {
                    if !self.transport.health_check().await {
                        warn!("P2P transport unhealthy");
                    }
                }
            }
        }

        info!("P2P relay stopped");
        Ok(())
    }

    /// Handle a single P2P QUIC stream — proxy between remote peer and local engine
    async fn handle_p2p_stream(
        self: &Arc<Self>,
        protocol: ProtocolType,
        mut client_send: quinn::SendStream,
        mut client_recv: quinn::RecvStream,
    ) -> anyhow::Result<()> {
        info!(protocol = ?protocol, "Handling P2P stream");

        // Acquire connection to local engine via TCP
        let mut engine_conn = match self.transport.acquire(protocol).await {
            Ok(conn) => conn,
            Err(e) => {
                warn!(protocol = ?protocol, error = %e, "Engine connection failed");
                return Ok(());
            }
        };

        // Bridge the P2P QUIC stream to the local engine TCP connection
        match &mut engine_conn.stream {
            crate::transport::ByteStream::Tcp(tcp_stream) => {
                let (mut engine_read, mut engine_write) = tokio::io::split(tcp_stream);

                // Client (P2P remote) → Engine (local TCP)
                let client_to_engine = async {
                    let mut buf = [0u8; 8192];
                    loop {
                        match client_recv.read(&mut buf).await {
                            Ok(Some(n)) => {
                                if n == 0 {
                                    break;
                                }
                                if let Err(e) = engine_write.write_all(&buf[..n]).await {
                                    debug!("P2P→Engine write error: {}", e);
                                    break;
                                }
                            }
                            Ok(None) => break,
                            Err(e) => {
                                debug!("P2P recv error: {}", e);
                                break;
                            }
                        }
                    }
                };

                // Engine (local TCP) → Client (P2P remote)
                let engine_to_client = async {
                    let mut buf = [0u8; 8192];
                    loop {
                        match engine_read.read(&mut buf).await {
                            Ok(n) => {
                                if n == 0 {
                                    break;
                                }
                                if let Err(e) = client_send.write_all(&buf[..n]).await {
                                    debug!("Engine→P2P write error: {}", e);
                                    break;
                                }
                            }
                            Err(e) => {
                                debug!("Engine read error: {}", e);
                                break;
                            }
                        }
                    }
                };

                tokio::select! {
                    _ = client_to_engine => {},
                    _ = engine_to_client => {},
                }

                // Gracefully close the QUIC send stream
                let _ = client_send.finish();
            }
            #[cfg(feature = "p2p")]
            crate::transport::ByteStream::Quic(_, _, _) => {
                warn!("P2P-to-P2P stream bridging not yet implemented");
            }
        }

        self.transport.release(engine_conn);
        debug!(protocol = ?protocol, "P2P stream handler complete");
        Ok(())
    }
}

impl Clone for RelayServer {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            router: self.router.clone(),
            transport: self.transport.clone(),
            tls_acceptor: self.tls_acceptor.clone(),
            connection_counter: self.connection_counter.clone(),
        }
    }
}

// ============================================================================
// HTTP Request Line Parser
// ============================================================================

#[derive(Debug, Clone)]
pub struct HttpRequestLine {
    pub method: String,
    pub path: String,
    pub version: String,
}

fn parse_http_request_line(buf: &[u8]) -> anyhow::Result<HttpRequestLine> {
    let line_end = buf.iter().position(|&b| b == b'\r').unwrap_or(buf.len());
    let line = std::str::from_utf8(&buf[..line_end])
        .map_err(|_| anyhow::anyhow!("Invalid UTF-8"))?;

    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 3 {
        return Err(anyhow::anyhow!("Invalid HTTP request line"));
    }

    Ok(HttpRequestLine {
        method: parts[0].to_string(),
        path: parts[1].to_string(),
        version: parts[2].to_string(),
    })
}

// ============================================================================
// TLS Certificate Loading with ALPN
// ============================================================================

async fn load_tls_config(cert_dir: &std::path::Path) -> anyhow::Result<rustls::ServerConfig> {
    let cert_path = cert_dir.join("cert.pem");
    let key_path = cert_dir.join("key.pem");

    let (certs, key) = if cert_path.exists() && key_path.exists() {
        info!("Loading TLS certificate from {:?}", cert_dir);
        let cert_file = tokio::fs::read(&cert_path).await?;
        let key_file = tokio::fs::read(&key_path).await?;

        let certs = rustls_pemfile::certs(&mut cert_file.as_slice())
            .collect::<Result<Vec<_>, _>>()?;

        let key = rustls_pemfile::private_key(&mut key_file.as_slice())?
            .ok_or_else(|| anyhow::anyhow!("No private key found"))?;

        (certs, key)
    } else {
        info!("Generating self-signed TLS certificate");
        tokio::fs::create_dir_all(cert_dir).await?;

        let cert = rcgen::generate_simple_self_signed(vec![
            "share.bennett.studio".to_string(),
            "localhost".to_string(),
            "127.0.0.1".to_string(),
        ])?;

        let cert_pem = cert.cert.pem();
        let key_pem = cert.key_pair.serialize_pem();

        tokio::fs::write(&cert_path, &cert_pem).await?;
        tokio::fs::write(&key_path, &key_pem).await?;

        let certs = rustls_pemfile::certs(&mut cert_pem.as_bytes())
            .collect::<Result<Vec<_>, _>>()?;

        let key = rustls_pemfile::private_key(&mut key_pem.as_bytes())?
            .ok_or_else(|| anyhow::anyhow!("No private key found"))?;

        (certs, key)
    };

    // Configure ALPN protocols (order = preference)
    let mut config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)?;

    config.alpn_protocols = vec![
        ALPN_HTTP1.to_vec(),  // Prefer HTTP/1.1 (Connect-RPC)
        ALPN_HTTP2.to_vec(),  // HTTP/2 fallback
        ALPN_MYSQL.to_vec(),  // MySQL wire
    ];

    Ok(config)
}
