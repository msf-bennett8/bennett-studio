//! TLS server — accepts public internet connections
//! Terminates TLS, detects protocol, routes to engine
//! Includes: buffered HTTP parsing, MySQL handshake, error responses, connection limits

use crate::config::RelayConfig;
use crate::multiplexer::{
    extract_share_id_from_http_path, extract_share_id_from_mysql_username,
    proxy_bidirectional, read_mysql_auth_response, send_http_404, send_http_429,
    send_mysql_error, send_mysql_handshake_v10, send_mysql_share_not_found,
    send_mysql_too_many_connections, ConnectionCounter,
};
use crate::router::ShareRouter;
use crate::transport::{ProtocolType, Transport};

use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::{rustls, TlsAcceptor};
use tracing::{info, warn};

/// Relay server state
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
            "Relay server initialized"
        );

        Ok(Self {
            config,
            router,
            transport,
            tls_acceptor,
            connection_counter: counter,
        })
    }

    /// Start accepting connections with graceful shutdown support
    pub async fn run(self, mut shutdown_rx: tokio::sync::watch::Receiver<bool>) -> anyhow::Result<()> {
        let listener = TcpListener::bind(self.config.bind).await?;
        info!(
            bind = %self.config.bind,
            "Relay server listening"
        );

        let server = Arc::new(self);

        loop {
            tokio::select! {
                accept_result = listener.accept() => {
                    let (client_stream, client_addr) = accept_result?;
                    let srv = server.clone();

                    tokio::spawn(async move {
                        if let Err(e) = srv.handle_client(client_stream, client_addr).await {
                            warn!(
                                addr = %client_addr,
                                error = %e,
                                "Client handler error"
                            );
                        }
                    });
                }
                _ = shutdown_rx.changed() => {
                    if *shutdown_rx.borrow() {
                        info!("Shutdown signal received, stopping listener");
                        break;
                    }
                }
            }
        }

        info!("Relay server stopped accepting new connections");
        Ok(())
    }

    /// Handle a single client connection
    async fn handle_client(
        self: &Arc<Self>,
        client_stream: TcpStream,
        client_addr: SocketAddr,
    ) -> anyhow::Result<()> {
        info!(
            addr = %client_addr,
            "New client connection"
        );

        // Accept TLS
        let mut tls_stream = match self.tls_acceptor.accept(client_stream).await {
            Ok(s) => s,
            Err(e) => {
                warn!(addr = %client_addr, error = %e, "TLS handshake failed");
                return Ok(());
            }
        };

        // Use BufReader for proper HTTP parsing without consuming TLS bytes
        let mut buf_stream = BufReader::new(&mut tls_stream);
        let mut peek_buf = [0u8; 32];

        // Peek at first bytes to detect protocol
        let n = match buf_stream.read(&mut peek_buf).await {
            Ok(0) => {
                warn!(addr = %client_addr, "Client closed connection immediately");
                return Ok(());
            }
            Ok(n) => n,
            Err(e) => {
                warn!(addr = %client_addr, error = %e, "Failed to peek at stream");
                return Ok(());
            }
        };

        // Put back the read bytes by using the underlying stream directly
        // Since we read from BufReader, we need to handle this carefully
        // For HTTP: we already have the request line in peek_buf
        // For MySQL: we need to forward the bytes to the engine

        let protocol = ProtocolType::detect(&peek_buf[..n])
            .unwrap_or(ProtocolType::ConnectRpc);

        info!(
            addr = %client_addr,
            protocol = ?protocol,
            "Protocol detected"
        );

        match protocol {
            ProtocolType::MySqlWire => {
                self.handle_mysql_client(tls_stream, client_addr, &peek_buf[..n]).await?;
            }
            ProtocolType::ConnectRpc | ProtocolType::Grpc => {
                self.handle_http_client(tls_stream, client_addr, &peek_buf[..n]).await?;
            }
        }

        Ok(())
    }

    /// Handle HTTP/Connect-RPC client
    async fn handle_http_client(
        &self,
        mut tls_stream: tokio_rustls::server::TlsStream<TcpStream>,
        client_addr: SocketAddr,
        initial_bytes: &[u8],
    ) -> anyhow::Result<()> {
        // Parse HTTP request line from initial bytes
        let request_line = parse_http_request_line(initial_bytes)?;

        info!(
            addr = %client_addr,
            method = %request_line.method,
            path = %request_line.path,
            "HTTP request received"
        );

        let share_id = extract_share_id_from_http_path(&request_line.path)
            .unwrap_or_else(|| "unknown".to_string());

        // Validate share exists and is active
        if !self.router.is_active(&share_id) {
            warn!(
                share_id = %share_id,
                "Share not found or inactive"
            );
            send_http_404(&mut tls_stream, &format!("Share '{}' not found or inactive", share_id)).await?;
            return Ok(());
        }

        // Check connection limit
        if !self.connection_counter.acquire(&share_id) {
            warn!(
                share_id = %share_id,
                "Connection limit exceeded"
            );
            send_http_429(&mut tls_stream, &share_id).await?;
            return Ok(());
        }

        info!(
            share_id = %share_id,
            conn_count = self.connection_counter.count(&share_id),
            "Routing HTTP to engine"
        );

        // Connect to engine via transport
        let mut engine_stream = self
            .transport
            .connect(&share_id, ProtocolType::ConnectRpc)
            .await
            .map_err(|e| {
                self.connection_counter.release(&share_id);
                anyhow::anyhow!("Engine connection failed: {}", e)
            })?;

        // Forward the initial bytes we already read
        if let Err(e) = engine_stream.write_all(initial_bytes).await {
            self.connection_counter.release(&share_id);
            return Err(anyhow::anyhow!("Failed to forward initial bytes to engine: {}", e));
        }

        // Now proxy bidirectionally using full streams
        let client_with_prefix = PrefixedStream {
            prefix: initial_bytes.to_vec(),
            stream: tls_stream,
            prefix_consumed: false,
        };

        proxy_bidirectional(
            client_with_prefix,
            engine_stream,
            share_id,
            "http",
            self.connection_counter.clone(),
        ).await?;

        Ok(())
    }

    /// Handle MySQL wire protocol client
    async fn handle_mysql_client(
        &self,
        mut tls_stream: tokio_rustls::server::TlsStream<TcpStream>,
        client_addr: SocketAddr,
        initial_bytes: &[u8],
    ) -> anyhow::Result<()> {
        // Send MySQL handshake v10 to client
        send_mysql_handshake_v10(&mut tls_stream, "unknown", 1).await?;

        // Read auth response
        let auth_response = match read_mysql_auth_response(&mut tls_stream).await {
            Ok(auth) => auth,
            Err(e) => {
                warn!(addr = %client_addr, error = %e, "Failed to read MySQL auth response");
                let _ = send_mysql_error(
                    &mut tls_stream, 1, 1045, "28000",
                    &format!("Auth response failed: {}", e)
                ).await;
                return Ok(());
            }
        };

        let share_id = extract_share_id_from_mysql_username(&auth_response.username);

        info!(
            addr = %client_addr,
            share_id = %share_id,
            username = %auth_response.username,
            "MySQL auth received"
        );

        // Validate share
        if !self.router.is_active(&share_id) {
            warn!(share_id = %share_id, "Share not found or inactive");
            send_mysql_share_not_found(&mut tls_stream, 1).await?;
            return Ok(());
        }

        // Check connection limit
        if !self.connection_counter.acquire(&share_id) {
            warn!(share_id = %share_id, "MySQL connection limit exceeded");
            send_mysql_too_many_connections(&mut tls_stream, 1).await?;
            return Ok(());
        }

        // Connect to engine MySQL proxy
        let mut engine_stream = self
            .transport
            .connect(&share_id, ProtocolType::MySqlWire)
            .await
            .map_err(|e| {
                self.connection_counter.release(&share_id);
                anyhow::anyhow!("Engine MySQL connection failed: {}", e)
            })?;

        info!(
            share_id = %share_id,
            "Routing MySQL to engine"
        );

        // Forward the initial bytes we already read, then proxy bidirectionally
        let engine_with_prefix = PrefixedStream {
            prefix: initial_bytes.to_vec(),
            stream: engine_stream,
            prefix_consumed: false,
        };

        // Client side: no prefix needed (handshake already consumed)
        let client_with_prefix = PrefixedStream {
            prefix: Vec::new(),
            stream: tls_stream,
            prefix_consumed: true,
        };

        proxy_bidirectional(
            client_with_prefix,
            engine_with_prefix,
            share_id,
            "mysql",
            self.connection_counter.clone(),
        ).await?;

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

/// Parsed HTTP request line
#[derive(Debug, Clone)]
pub struct HttpRequestLine {
    pub method: String,
    pub path: String,
    pub version: String,
}

/// Parse HTTP request line from raw bytes
/// Uses BufReader-like logic but works on a byte slice
fn parse_http_request_line(buf: &[u8]) -> anyhow::Result<HttpRequestLine> {
    // Find the first \r\n (end of request line)
    let line_end = buf.iter()
        .position(|&b| b == b'\r')
        .unwrap_or(buf.len());

    let line = std::str::from_utf8(&buf[..line_end])
        .map_err(|_| anyhow::anyhow!("Invalid UTF-8 in HTTP request line"))?;

    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 3 {
        return Err(anyhow::anyhow!("Invalid HTTP request line: expected METHOD PATH VERSION, got '{}'", line));
    }

    Ok(HttpRequestLine {
        method: parts[0].to_string(),
        path: parts[1].to_string(),
        version: parts[2].to_string(),
    })
}

// ============================================================================
// Prefixed Stream — prepends already-read bytes to a stream
// ============================================================================

use pin_project::pin_project;

/// A stream wrapper that first returns buffered bytes, then delegates to inner stream
/// Uses pin_project for proper pinning (industry standard in tokio ecosystem)
#[pin_project]
pub struct PrefixedStream<S> {
    prefix: Vec<u8>,
    #[pin]
    stream: S,
    prefix_consumed: bool,
}

impl<S: AsyncRead> AsyncRead for PrefixedStream<S> {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let this = self.project();

        if !*this.prefix_consumed && !this.prefix.is_empty() {
            let to_copy = std::cmp::min(buf.remaining(), this.prefix.len());
            buf.put_slice(&this.prefix[..to_copy]);
            this.prefix.drain(..to_copy);

            if this.prefix.is_empty() {
                *this.prefix_consumed = true;
            }

            return std::task::Poll::Ready(Ok(()));
        }

        this.stream.poll_read(cx, buf)
    }
}

impl<S: AsyncWrite> AsyncWrite for PrefixedStream<S> {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        self.project().stream.poll_write(cx, buf)
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        self.project().stream.poll_flush(cx)
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        self.project().stream.poll_shutdown(cx)
    }
}

// ============================================================================
// TLS Certificate Loading
// ============================================================================

/// Load TLS certificate from directory or generate self-signed
async fn load_tls_config(cert_dir: &std::path::Path) -> anyhow::Result<rustls::ServerConfig> {
    let cert_path = cert_dir.join("cert.pem");
    let key_path = cert_dir.join("key.pem");

    if cert_path.exists() && key_path.exists() {
        info!("Loading TLS certificate from {:?}", cert_dir);

        let cert_file = tokio::fs::read(&cert_path).await?;
        let key_file = tokio::fs::read(&key_path).await?;

        let certs = rustls_pemfile::certs(&mut cert_file.as_slice())
            .collect::<Result<Vec<_>, _>>()?;

        let key = rustls_pemfile::private_key(&mut key_file.as_slice())?
            .ok_or_else(|| anyhow::anyhow!("No private key found"))?;

        let config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)?;

        Ok(config)
    } else {
        info!("Generating self-signed TLS certificate");

        // Ensure certs directory exists
        tokio::fs::create_dir_all(cert_dir).await.map_err(|e| {
            anyhow::anyhow!("Failed to create certs directory {:?}: {}", cert_dir, e)
        })?;

        let cert = rcgen::generate_simple_self_signed(vec![
            "share.bennett.studio".to_string(),
            "localhost".to_string(),
            "127.0.0.1".to_string(),
        ])?;

        let cert_pem = cert.cert.pem();
        let key_pem = cert.key_pair.serialize_pem();

        // Save with explicit error handling
        tokio::fs::write(&cert_path, &cert_pem).await.map_err(|e| {
            anyhow::anyhow!("Failed to write cert.pem: {}", e)
        })?;
        tokio::fs::write(&key_path, &key_pem).await.map_err(|e| {
            anyhow::anyhow!("Failed to write key.pem: {}", e)
        })?;

        info!("Self-signed certificate saved to {:?}", cert_dir);

        let certs = rustls_pemfile::certs(&mut cert_pem.as_bytes())
            .collect::<Result<Vec<_>, _>>()?;

        let key = rustls_pemfile::private_key(&mut key_pem.as_bytes())?
            .ok_or_else(|| anyhow::anyhow!("No private key found in generated cert"))?;

        let config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)?;

        Ok(config)
    }
}
