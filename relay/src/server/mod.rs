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

        // HTTP mode: serve proxy API directly (Render/cloud deployment)
        if self.config.http_mode {
            return self.run_http_mode(shutdown_rx).await;
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

    /// HTTP mode: plain HTTP server for Render/cloud deployments
    /// Serves proxy API routes directly without TLS termination
    async fn run_http_mode(
        self,
        mut shutdown_rx: tokio::sync::watch::Receiver<bool>,
    ) -> anyhow::Result<()> {
        let tunnel_registry = crate::tunnel_registry::TunnelRegistry::new();
        let router = self.router.with_tunnel_registry(tunnel_registry.clone());

        let app = build_proxy_api_router(router, tunnel_registry);

        let addr = self.config.bind;
        info!("HTTP relay starting on http://{}", addr);

        let listener = TcpListener::bind(addr).await?;
        axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                let _ = shutdown_rx.changed().await;
                info!("HTTP mode shutdown signal received");
            })
            .await?;

        info!("HTTP relay stopped");
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

        if !self.router.is_active(&share_id).await {
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

        // Downcast to P2pTransport to access P2P-specific methods
        let p2p_transport = self.transport.as_any()
            .downcast_ref::<crate::transport::p2p::P2pTransport>()
            .ok_or_else(|| anyhow::anyhow!("P2P mode requires P2pTransport"))?;

        // In server mode: accept the initial QUIC connection first
        if p2p_transport.is_server() {
            info!("P2P server mode — waiting for client connection...");
            tokio::select! {
                _ = shutdown_rx.changed() => {
                    info!("P2P shutdown before client connected");
                    return Ok(());
                }
                result = p2p_transport.accept_connection() => {
                    result.map_err(|e| anyhow::anyhow!("Failed to accept P2P connection: {}", e))?;
                }
            }
        }

        info!("P2P connection established — accepting streams");

        loop {
            tokio::select! {
                biased;

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

                // Periodic health check
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

fn _parse_http_request_line(buf: &[u8]) -> anyhow::Result<HttpRequestLine> {
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

// ============================================================================
// HTTP Proxy API Module — Used by run_http_mode() and start_http_proxy_api()
// ============================================================================

use axum::{
    extract::{Path, State, Json, ws::{Message, WebSocket, WebSocketUpgrade}},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post, options},
    Router,
};
use futures_util::{sink::SinkExt, stream::StreamExt};
use tower_http::cors::CorsLayer;
use axum::http::{HeaderValue, Method};

#[derive(Clone)]
pub struct ProxyApiState {
    pub router: Arc<crate::router::ShareRouter>,
    pub tunnel_registry: Arc<crate::tunnel_registry::TunnelRegistry>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct WebRtcOfferRequest {
    pub sdp: String,
    pub ice_candidates: Vec<serde_json::Value>,
    pub token: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct WebRtcOfferResponse {
    pub success: bool,
    pub answer_sdp: Option<String>,
    pub answer_ice: Vec<serde_json::Value>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct WebRtcIceRequest {
    pub candidate: serde_json::Value,
    pub token: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct SchemaQueryParams {
    pub token: String,
}

/// CORS preflight response
pub async fn cors_preflight() -> impl IntoResponse {
    let mut headers = axum::http::HeaderMap::new();
    for (k, v) in crate::router::cors_headers() {
        headers.insert(k, v.parse().unwrap());
    }
    (StatusCode::NO_CONTENT, headers)
}

/// Health check for proxy API
pub async fn proxy_health() -> impl IntoResponse {
    let body = serde_json::json!({ "status": "ok", "service": "bennett-proxy" });
    (StatusCode::OK, axum::Json(body))
}

/// Handle WebRTC offer from browser
pub async fn webrtc_offer_handler(
    Path(code): Path<String>,
    State(state): State<ProxyApiState>,
    Json(_req): Json<WebRtcOfferRequest>,
) -> impl IntoResponse {
    let mut headers = axum::http::HeaderMap::new();
    for (k, v) in crate::router::cors_headers() {
        headers.insert(k, v.parse().unwrap());
    }

    if !state.router.is_active(&code).await {
        return (
            StatusCode::NOT_FOUND,
            headers,
            axum::Json(WebRtcOfferResponse {
                success: false,
                answer_sdp: None,
                answer_ice: vec![],
                error: Some("Share not found or expired".to_string()),
            }),
        );
    }

    info!(share = %code, "WebRTC offer received from browser");

    (
        StatusCode::OK,
        headers,
        axum::Json(WebRtcOfferResponse {
            success: true,
            answer_sdp: None,
            answer_ice: vec![],
            error: Some("WebRTC P2P not yet active — use WebSocket relay instead".to_string()),
        }),
    )
}

/// Handle ICE candidate trickle from browser
pub async fn webrtc_ice_handler(
    Path(_code): Path<String>,
    State(_state): State<ProxyApiState>,
    Json(_req): Json<WebRtcIceRequest>,
) -> impl IntoResponse {
    let mut headers = axum::http::HeaderMap::new();
    for (k, v) in crate::router::cors_headers() {
        headers.insert(k, v.parse().unwrap());
    }
    (StatusCode::OK, headers, axum::Json(serde_json::json!({ "received": true })))
}

/// Execute a query through the proxy
pub async fn proxy_query(
    Path(code): Path<String>,
    State(state): State<ProxyApiState>,
    Json(req): Json<crate::router::ProxyQueryRequest>,
) -> impl IntoResponse {
    let mut headers = axum::http::HeaderMap::new();
    for (k, v) in crate::router::cors_headers() {
        headers.insert(k, v.parse().unwrap());
    }

    if !state.router.is_active(&code).await {
        return (
            StatusCode::NOT_FOUND,
            headers,
            axum::Json(serde_json::json!({
                "success": false,
                "error": "Share not found or expired"
            })),
        );
    }

    info!(share = %code, sql_len = %req.sql.len(), "Proxy query received — forwarding to engine");

    let engine_body = serde_json::json!({
        "sql": req.sql,
        "limit": req.limit,
        "offset": req.offset,
    });

    match state.router.forward_to_engine(
        &code,
        "POST",
        &format!("/api/shares/{}/query", code),
        Some(engine_body.to_string().into_bytes()),
        &req.token,
    ).await {
        Ok(response_bytes) => {
            let response_str = String::from_utf8_lossy(&response_bytes);
            if let Some(body_start) = response_str.find("\r\n\r\n") {
                let body = &response_str[body_start + 4..];
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(body) {
                    return (StatusCode::OK, headers, axum::Json(json));
                }
            }
            (
                StatusCode::OK,
                headers,
                axum::Json(serde_json::json!({
                    "success": true,
                    "raw_response": response_str.to_string(),
                })),
            )
        }
        Err(e) => {
            warn!(share = %code, error = %e, "Engine forwarding failed");
            (
                StatusCode::BAD_GATEWAY,
                headers,
                axum::Json(serde_json::json!({
                    "success": false,
                    "error": format!("Engine forwarding failed: {}", e)
                })),
            )
        }
    }
}

/// Get schema through the proxy
pub async fn proxy_schema(
    Path(code): Path<String>,
    State(state): State<ProxyApiState>,
    axum::extract::Query(params): axum::extract::Query<SchemaQueryParams>,
) -> impl IntoResponse {
    let token = params.token;
    let mut headers = axum::http::HeaderMap::new();
    for (k, v) in crate::router::cors_headers() {
        headers.insert(k, v.parse().unwrap());
    }

    if !state.router.is_active(&code).await {
        return (
            StatusCode::NOT_FOUND,
            headers,
            axum::Json(serde_json::json!({
                "success": false,
                "error": "Share not found or expired"
            })),
        );
    }

    info!(share = %code, "Proxy schema request received — forwarding to engine");

    match state.router.forward_to_engine(
        &code,
        "GET",
        &format!("/api/shares/{}/schema", code),
        None,
        &token,
    ).await {
        Ok(response_bytes) => {
            let response_str = String::from_utf8_lossy(&response_bytes);
            if let Some(body_start) = response_str.find("\r\n\r\n") {
                let body = &response_str[body_start + 4..];
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(body) {
                    return (StatusCode::OK, headers, axum::Json(json));
                }
            }
            (
                StatusCode::OK,
                headers,
                axum::Json(serde_json::json!({
                    "success": true,
                    "raw_response": response_str.to_string(),
                })),
            )
        }
        Err(e) => {
            warn!(share = %code, error = %e, "Engine schema forwarding failed");
            (
                StatusCode::BAD_GATEWAY,
                headers,
                axum::Json(serde_json::json!({
                    "success": false,
                    "error": format!("Engine forwarding failed: {}", e)
                })),
            )
        }
    }
}

/// Validate a share through the proxy
pub async fn proxy_validate_share(
    Path(code): Path<String>,
    State(state): State<ProxyApiState>,
    Json(req): Json<serde_json::Value>,
) -> impl IntoResponse {
    let mut headers = axum::http::HeaderMap::new();
    for (k, v) in crate::router::cors_headers() {
        headers.insert(k, v.parse().unwrap());
    }

    if !state.router.is_active(&code).await {
        return (
            StatusCode::NOT_FOUND,
            headers,
            axum::Json(serde_json::json!({
                "success": false,
                "error": "Share not found or expired"
            })),
        );
    }

    info!(share = %code, "Proxy validate request received — forwarding to engine");

    match state.router.forward_to_engine(
        &code,
        "POST",
        &format!("/api/shares/{}/validate", code),
        Some(req.to_string().into_bytes()),
        "",
    ).await {
        Ok(response_bytes) => {
            let response_str = String::from_utf8_lossy(&response_bytes);
            if let Some(body_start) = response_str.find("\r\n\r\n") {
                let body = &response_str[body_start + 4..];
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(body) {
                    return (StatusCode::OK, headers, axum::Json(json));
                }
            }
            (
                StatusCode::OK,
                headers,
                axum::Json(serde_json::json!({
                    "success": true,
                    "raw_response": response_str.to_string(),
                })),
            )
        }
        Err(e) => {
            warn!(share = %code, error = %e, "Engine validate forwarding failed");
            (
                StatusCode::BAD_GATEWAY,
                headers,
                axum::Json(serde_json::json!({
                    "success": false,
                    "error": format!("Engine forwarding failed: {}", e)
                })),
            )
        }
    }
}

/// Get public share info
pub async fn proxy_share_info(
    Path(code): Path<String>,
    State(state): State<ProxyApiState>,
) -> impl IntoResponse {
    let mut headers = axum::http::HeaderMap::new();
    for (k, v) in crate::router::cors_headers() {
        headers.insert(k, v.parse().unwrap());
    }

    if !state.router.is_active(&code).await {
        return (
            StatusCode::NOT_FOUND,
            headers,
            axum::Json(serde_json::json!({
                "success": false,
                "error": "Share not found or expired"
            })),
        );
    }

    (
        StatusCode::OK,
        headers,
        axum::Json(serde_json::json!({
            "success": true,
            "code": code,
            "status": "active",
            "message": "Share is active — connect with BennettClient"
        })),
    )
}

// ============================================================================
// WebSocket Streaming Proxy
// ============================================================================

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum WsProxyRequest {
    Query { sql: String, limit: Option<i64>, request_id: Option<String> },
    SubscribeSchema,
    Ping,
    Ack { message_id: u64 },
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum WsProxyResponse {
    QueryResult {
        columns: Vec<String>,
        rows: Vec<Vec<serde_json::Value>>,
        row_count: usize,
        execution_time_ms: u64,
        request_id: Option<String>,
        message_id: u64,
    },
    QueryError {
        error: String,
        request_id: Option<String>,
        message_id: u64,
    },
    SchemaUpdate {
        tables: Vec<serde_json::Value>,
        database_name: String,
        database_type: String,
        message_id: u64,
    },
    Status {
        connected: bool,
        share_code: String,
        message: String,
        message_id: u64,
    },
    Pong,
    Error { message: String, message_id: u64 },
}

/// WebSocket upgrade handler for browser share connections
pub async fn ws_proxy_handler(
    ws: WebSocketUpgrade,
    Path(code): Path<String>,
    State(state): State<ProxyApiState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws_proxy(socket, code, state))
}

async fn handle_ws_proxy(
    socket: WebSocket,
    code: String,
    state: ProxyApiState,
) {
    let (mut sender, mut receiver) = socket.split();
    let mut message_id: u64 = 0;

    info!(share = %code, "WebSocket proxy connection established");

    message_id += 1;
    let status_msg = WsProxyResponse::Status {
        connected: state.router.is_active(&code).await,
        share_code: code.clone(),
        message: "Connected to Bennett relay proxy".to_string(),
        message_id,
    };
    let _ = sender.send(Message::Text(
        serde_json::to_string(&status_msg).unwrap()
    )).await;

    if !state.router.is_active(&code).await {
        message_id += 1;
        let _ = sender.send(Message::Text(
            serde_json::to_string(&WsProxyResponse::Error {
                message: "Share not found or expired".to_string(),
                message_id,
            }).unwrap()
        )).await;
        let _ = sender.close().await;
        return;
    }

    loop {
        tokio::select! {
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        match serde_json::from_str::<WsProxyRequest>(&text) {
                            Ok(WsProxyRequest::Ping) => {
                                let _ = sender.send(Message::Text(
                                    serde_json::to_string(&WsProxyResponse::Pong).unwrap()
                                )).await;
                            }
                            Ok(WsProxyRequest::Query { sql, limit, request_id }) => {
                                info!(share = %code, sql_len = %sql.len(), "WebSocket query received");

                                if sql.trim().is_empty() {
                                    message_id += 1;
                                    let _ = sender.send(Message::Text(
                                        serde_json::to_string(&WsProxyResponse::QueryError {
                                            error: "Empty SQL query".to_string(),
                                            request_id: request_id.clone(),
                                            message_id,
                                        }).unwrap()
                                    )).await;
                                    continue;
                                }

                                message_id += 1;
                                let start = std::time::Instant::now();

                                let engine_body = serde_json::json!({
                                    "sql": sql,
                                    "limit": limit.unwrap_or(1000),
                                });

                                match state.router.forward_to_engine(
                                    &code,
                                    "POST",
                                    &format!("/api/shares/{}/query", code),
                                    Some(engine_body.to_string().into_bytes()),
                                    "",
                                ).await {
                                    Ok(response_bytes) => {
                                        let response_str = String::from_utf8_lossy(&response_bytes);
                                        if let Some(body_start) = response_str.find("\r\n\r\n") {
                                            let body = &response_str[body_start + 4..];
                                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(body) {
                                                let data = json.get("data").unwrap_or(&json);
                                                let columns = data.get("columns")
                                                    .and_then(|c| c.as_array())
                                                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                                                    .unwrap_or_else(|| vec!["result".to_string()]);
                                                let rows = data.get("rows")
                                                    .and_then(|r| r.as_array())
                                                    .map(|arr| arr.iter().map(|v| vec![v.clone()]).collect())
                                                    .unwrap_or_default();

                                                let response = WsProxyResponse::QueryResult {
                                                    columns,
                                                    rows,
                                                    row_count: data.get("row_count").and_then(|v| v.as_u64()).unwrap_or(0) as usize,
                                                    execution_time_ms: start.elapsed().as_millis() as u64,
                                                    request_id,
                                                    message_id,
                                                };
                                                let _ = sender.send(Message::Text(
                                                    serde_json::to_string(&response).unwrap()
                                                )).await;
                                            } else {
                                                let response = WsProxyResponse::QueryResult {
                                                    columns: vec!["raw".to_string()],
                                                    rows: vec![vec![serde_json::json!(body)]],
                                                    row_count: 1,
                                                    execution_time_ms: start.elapsed().as_millis() as u64,
                                                    request_id,
                                                    message_id,
                                                };
                                                let _ = sender.send(Message::Text(
                                                    serde_json::to_string(&response).unwrap()
                                                )).await;
                                            }
                                        } else {
                                            let _ = sender.send(Message::Text(
                                                serde_json::to_string(&WsProxyResponse::QueryError {
                                                    error: "Invalid engine response".to_string(),
                                                    request_id,
                                                    message_id,
                                                }).unwrap()
                                            )).await;
                                        }
                                    }
                                    Err(e) => {
                                        let _ = sender.send(Message::Text(
                                            serde_json::to_string(&WsProxyResponse::QueryError {
                                                error: format!("Engine forwarding failed: {}", e),
                                                request_id,
                                                message_id,
                                            }).unwrap()
                                        )).await;
                                    }
                                }
                            }
                            Ok(WsProxyRequest::SubscribeSchema) => {
                                message_id += 1;
                                let response = WsProxyResponse::SchemaUpdate {
                                    tables: vec![],
                                    database_name: "Remote Database".to_string(),
                                    database_type: "unknown".to_string(),
                                    message_id,
                                };
                                let _ = sender.send(Message::Text(
                                    serde_json::to_string(&response).unwrap()
                                )).await;
                            }
                            Ok(WsProxyRequest::Ack { message_id: ack_id }) => {
                                debug!("Client acked message {}", ack_id);
                            }
                            Err(e) => {
                                message_id += 1;
                                let _ = sender.send(Message::Text(
                                    serde_json::to_string(&WsProxyResponse::Error {
                                        message: format!("Invalid message: {}", e),
                                        message_id,
                                    }).unwrap()
                                )).await;
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        info!(share = %code, "WebSocket client disconnected");
                        break;
                    }
                    Some(Err(e)) => {
                        warn!(share = %code, error = %e, "WebSocket error");
                        break;
                    }
                    _ => {}
                }
            }
            _ = tokio::time::sleep(tokio::time::Duration::from_secs(30)) => {
                message_id += 1;
                let _ = sender.send(Message::Text(
                    serde_json::to_string(&WsProxyResponse::Status {
                        connected: state.router.is_active(&code).await,
                        share_code: code.clone(),
                        message: "Keepalive".to_string(),
                        message_id,
                    }).unwrap()
                )).await;
            }
        }
    }

    info!(share = %code, "WebSocket proxy connection closed");
}

// ============================================================================
// Engine Tunnel WebSocket Handler
// ============================================================================

#[derive(Clone)]
struct TunnelState {
    router: Arc<crate::router::ShareRouter>,
    tunnels: Arc<tokio::sync::RwLock<std::collections::HashMap<String, tokio::sync::mpsc::UnboundedSender<TunnelEngineMessage>>>>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum TunnelEngineMessage {
    QueryRequest {
        request_id: String,
        share_code: String,
        token: String,
        sql: String,
        limit: Option<i32>,
        offset: Option<i32>,
    },
    SchemaRequest {
        request_id: String,
        share_code: String,
        token: String,
    },
    Ping,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum TunnelEngineResponse {
    QueryResponse {
        request_id: String,
        result: serde_json::Value,
    },
    SchemaResponse {
        request_id: String,
        schema: serde_json::Value,
    },
    Register {
        host_id: String,
        version: String,
        capabilities: Vec<String>,
    },
    ShareCreated {
        code: String,
        db_id: String,
        permission: String,
        expires_at: i64,
    },
    ShareRevoked {
        code: String,
    },
    Pong,
}

/// WebSocket handler for engine tunnels
pub async fn tunnel_ws_handler(
    ws: WebSocketUpgrade,
    Path(host_id): Path<String>,
    State(state): State<ProxyApiState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_tunnel_ws(socket, host_id, state))
}

async fn handle_tunnel_ws(
    socket: WebSocket,
    host_id: String,
    state: ProxyApiState,
) {
    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<TunnelEngineMessage>();

    info!("Engine tunnel connected: host_id={}", host_id);
    state.router.mark_host_online(&host_id).await;

    let (wrap_tx, mut wrap_rx) = tokio::sync::mpsc::unbounded_channel::<crate::tunnel_registry::TunnelMessageToEngine>();

    let tx_clone = tx.clone();
    tokio::spawn(async move {
        while let Some(msg) = wrap_rx.recv().await {
            let engine_msg = match msg {
                crate::tunnel_registry::TunnelMessageToEngine::QueryRequest { request_id, share_code, token, sql, limit, offset } => {
                    TunnelEngineMessage::QueryRequest { request_id, share_code, token, sql, limit, offset }
                }
                crate::tunnel_registry::TunnelMessageToEngine::SchemaRequest { request_id, share_code, token } => {
                    TunnelEngineMessage::SchemaRequest { request_id, share_code, token }
                }
                crate::tunnel_registry::TunnelMessageToEngine::Ping => TunnelEngineMessage::Ping,
            };
            let _ = tx_clone.send(engine_msg);
        }
    });

    state.tunnel_registry.register_tunnel(host_id.clone(), wrap_tx).await;
    info!("Registered tunnel in registry for host: {}", host_id);

    let _ = sender.send(Message::Text(
        serde_json::to_string(&TunnelEngineMessage::Ping).unwrap()
    )).await;

    loop {
        tokio::select! {
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(resp) = serde_json::from_str::<TunnelEngineResponse>(&text) {
                            match resp {
                                TunnelEngineResponse::Register { host_id, version, capabilities } => {
                                    info!("Engine registered: {} (v{}, caps: {:?})", host_id, version, capabilities);
                                }
                                TunnelEngineResponse::ShareCreated { code, db_id, permission: _, expires_at } => {
                                    info!("Engine reported share created: {}", code);
                                    let route = crate::router::ShareRoute {
                                        share_id: code.clone(),
                                        db_id,
                                        protocol: crate::transport::ProtocolType::ConnectRpc,
                                        engine_port: 0,
                                        expires_at: chrono::DateTime::from_timestamp(expires_at, 0)
                                            .unwrap_or_else(|| chrono::Utc::now() + chrono::Duration::hours(24)),
                                        revoked: false,
                                        host_id: Some(host_id.clone()),
                                    };
                                    state.router.add_remote_route(route).await;
                                }
                                TunnelEngineResponse::ShareRevoked { code } => {
                                    info!("Engine reported share revoked: {}", code);
                                    state.router.remove_remote_route(&code).await;
                                }
                                TunnelEngineResponse::QueryResponse { request_id, result } => {
                                    debug!("Engine query response: {}", request_id);
                                    state.tunnel_registry.complete_request(&request_id, result).await;
                                }
                                TunnelEngineResponse::SchemaResponse { request_id, schema } => {
                                    debug!("Engine schema response: {}", request_id);
                                    state.tunnel_registry.complete_request(&request_id, schema).await;
                                }
                                TunnelEngineResponse::Pong => {
                                    debug!("Engine tunnel pong received");
                                }
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        info!("Engine tunnel disconnected: host_id={}", host_id);
                        break;
                    }
                    Some(Err(e)) => {
                        warn!("Engine tunnel error: {}", e);
                        break;
                    }
                    _ => {}
                }
            }
            Some(msg) = rx.recv() => {
                let text = serde_json::to_string(&msg).unwrap();
                if let Err(e) = sender.send(Message::Text(text)).await {
                    warn!("Failed to send to engine tunnel: {}", e);
                    break;
                }
            }
            _ = tokio::time::sleep(tokio::time::Duration::from_secs(30)) => {
                let _ = sender.send(Message::Text(
                    serde_json::to_string(&TunnelEngineMessage::Ping).unwrap()
                )).await;
            }
        }
    }

    info!("Engine tunnel closed: host_id={}", host_id);
    state.tunnel_registry.unregister_tunnel(&host_id).await;
    state.router.remove_all_host_routes(&host_id).await;
}

/// Build the proxy API router — used by run_http_mode() and start_http_proxy_api()
pub fn build_proxy_api_router(
    router: Arc<crate::router::ShareRouter>,
    tunnel_registry: Arc<crate::tunnel_registry::TunnelRegistry>,
) -> Router {
    let app_state = ProxyApiState {
        router,
        tunnel_registry,
    };

    let cors = CorsLayer::new()
        .allow_origin([
            "https://share-bennett-studio.vercel.app".parse::<HeaderValue>().unwrap(),
            "http://localhost:5173".parse::<HeaderValue>().unwrap(),
            "http://localhost:5174".parse::<HeaderValue>().unwrap(),
            "http://localhost:3000".parse::<HeaderValue>().unwrap(),
            "http://localhost:3001".parse::<HeaderValue>().unwrap(),
            "tauri://localhost".parse::<HeaderValue>().unwrap(),
        ])
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::AUTHORIZATION,
            axum::http::header::HeaderName::from_static("x-share-code"),
            axum::http::header::HeaderName::from_static("x-share-token"),
            axum::http::header::HeaderName::from_static("x-requested-with"),
        ])
        .allow_credentials(true);

    Router::new()
        .route("/health", get(proxy_health))
        .route("/api/health", get(proxy_health))
        .route("/api/share/:code/query", options(cors_preflight))
        .route("/api/share/:code/schema", options(cors_preflight))
        .route("/api/share/:code/validate", options(cors_preflight))
        .route("/ws/share/:code", options(cors_preflight))
        .route("/api/share/:code/webrtc/offer", post(webrtc_offer_handler))
        .route("/api/share/:code/webrtc/ice", post(webrtc_ice_handler))
        .route("/api/share/:code/query", post(proxy_query))
        .route("/api/share/:code/schema", get(proxy_schema))
        .route("/api/share/:code", get(proxy_share_info))
        .route("/api/share/:code/validate", post(proxy_validate_share))
        .route("/ws/share/:code", get(ws_proxy_handler))
        .route("/ws/tunnel/:host_id", get(tunnel_ws_handler))
        .fallback(options(cors_preflight))
        .layer(cors)
        .with_state(app_state)
}