//! TLS server — accepts public internet connections
//! Terminates TLS, detects protocol, routes to engine

use crate::config::RelayConfig;
use crate::multiplexer::{extract_share_id_from_http_path, proxy_bidirectional};
use crate::router::ShareRouter;
use crate::transport::{ProtocolType, Transport};

use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::{rustls, TlsAcceptor};
use tracing::{error, info, warn};

/// Relay server state
pub struct RelayServer {
    config: RelayConfig,
    router: Arc<ShareRouter>,
    transport: Arc<dyn Transport>,
    tls_acceptor: TlsAcceptor,
}

impl RelayServer {
    pub async fn new(
        config: RelayConfig,
        router: Arc<ShareRouter>,
        transport: Arc<dyn Transport>,
    ) -> anyhow::Result<Self> {
        // Load or generate TLS certificate
        let tls_config = load_tls_config(&config.cert_dir).await?;
        let tls_acceptor = TlsAcceptor::from(Arc::new(tls_config));

        info!(
            bind = %config.bind,
            transport = transport.name(),
            "Relay server initialized"
        );

        Ok(Self {
            config,
            router,
            transport,
            tls_acceptor,
        })
    }

    /// Start accepting connections
    pub async fn run(self) -> anyhow::Result<()> {
        let listener = TcpListener::bind(self.config.bind).await?;
        info!(
            bind = %self.config.bind,
            "Relay server listening"
        );

        loop {
            let (client_stream, client_addr) = listener.accept().await?;
            let server = Arc::new(self.clone());

            tokio::spawn(async move {
                if let Err(e) = server.handle_client(client_stream, client_addr).await {
                    warn!(
                        addr = %client_addr,
                        error = %e,
                        "Client handler error"
                    );
                }
            });
        }
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
        let mut tls_stream = self.tls_acceptor.accept(client_stream).await?;

        // Peek at first bytes to detect protocol
        let mut peek_buf = [0u8; 16];
        let n = tokio::io::AsyncReadExt::peek(&mut tls_stream, &mut peek_buf).await?;

        let protocol = ProtocolType::detect(&peek_buf[..n])
            .unwrap_or(ProtocolType::ConnectRpc);

        info!(
            addr = %client_addr,
            protocol = ?protocol,
            "Protocol detected"
        );

        // Extract share_id based on protocol
        let share_id = match protocol {
            ProtocolType::MySqlWire => {
                // For MySQL, we need to read the handshake to get username
                // For now, accept and let engine validate
                // TODO: Parse MySQL handshake to extract username/share_id
                "unknown".to_string()
            }
            ProtocolType::ConnectRpc | ProtocolType::Grpc => {
                // Read HTTP request line to extract path
                let request = read_http_request_line(&mut tls_stream).await?;
                extract_share_id_from_http_path(&request.path)
                    .unwrap_or_else(|| "unknown".to_string())
            }
        };

        // Validate share exists and is active
        if !self.router.is_active(&share_id) {
            warn!(
                share_id = %share_id,
                "Share not found or inactive"
            );
            // TODO: Send appropriate error response
            return Ok(());
        }

        info!(
            share_id = %share_id,
            "Routing to engine"
        );

        // Connect to engine via transport
        let engine_stream = self
            .transport
            .connect(&share_id, protocol)
            .await
            .map_err(|e| anyhow::anyhow!("Engine connection failed: {}", e))?;

        // Proxy bidirectionally
        proxy_bidirectional(tls_stream, engine_stream, share_id, "relay").await?;

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
        }
    }
}

/// Load TLS certificate from directory or generate self-signed
async fn load_tls_config(cert_dir: &std::path::Path) -> anyhow::Result<rustls::ServerConfig> {
    let cert_path = cert_dir.join("cert.pem");
    let key_path = cert_dir.join("key.pem");

    if cert_path.exists() && key_path.exists() {
        // Load existing certificate
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
        // Generate self-signed certificate for testing
        info!("Generating self-signed TLS certificate");

        let cert = rcgen::generate_simple_self_signed(vec![
            "share.bennett.studio".to_string(),
            "localhost".to_string(),
            "127.0.0.1".to_string(),
        ])?;

        let cert_pem = cert.cert.pem();
        let key_pem = cert.key_pair.serialize_pem();

        // Save for future use
        tokio::fs::create_dir_all(cert_dir).await.ok();
        tokio::fs::write(&cert_path, &cert_pem).await.ok();
        tokio::fs::write(&key_path, &key_pem).await.ok();

        let certs = rustls_pemfile::certs(&mut cert_pem.as_bytes())
            .collect::<Result<Vec<_>, _>>()?;

        let key = rustls_pemfile::private_key(&mut key_pem.as_bytes())?
            .ok_or_else(|| anyhow::anyhow!("No private key found"))?;

        let config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)?;

        Ok(config)
    }
}

/// Minimal HTTP request line parser
struct HttpRequestLine {
    method: String,
    path: String,
    version: String,
}

async fn read_http_request_line<S>(stream: &mut S) -> anyhow::Result<HttpRequestLine>
where
    S: tokio::io::AsyncRead + Unpin,
{
    use tokio::io::AsyncReadExt;

    let mut buf = [0u8; 4096];
    let n = stream.read(&mut buf).await?;

    let request = String::from_utf8_lossy(&buf[..n]);
    let line = request.lines().next().unwrap_or("");

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
