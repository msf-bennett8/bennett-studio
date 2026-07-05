//! Pluggable transport layer
//! TCP (relay) is active. P2P (WebRTC/QUIC) is a stub for future fallback.
//!
//! Uses trait-variant for object-safe async traits (industry standard:
//! used by Quinn, rustls, and async Rust ecosystem).

use std::io;
use std::pin::Pin;
use std::future::Future;
use std::sync::Arc;
use tokio::net::TcpStream;
pub use crate::transport::ice::IceCandidates;

/// ALPN protocol identifiers (industry standard from IANA)
pub const ALPN_HTTP2: &[u8] = b"h2";
pub const ALPN_HTTP1: &[u8] = b"http/1.1";
pub const ALPN_MYSQL: &[u8] = b"mysql";

/// A bidirectional byte stream for proxying — can be TCP or P2P QUIC
pub enum ByteStream {
    Tcp(TcpStream),
    #[cfg(feature = "p2p")]
    Quic(quinn::Connection, quinn::SendStream, quinn::RecvStream),
}

impl ByteStream {
    /// Try to get TCP stream reference (for legacy splice() code)
    pub fn as_tcp(&self) -> Option<&TcpStream> {
        match self {
            ByteStream::Tcp(s) => Some(s),
            #[cfg(feature = "p2p")]
            _ => None,
        }
    }

    /// Check if this is a TCP stream
    pub fn is_tcp(&self) -> bool {
        matches!(self, ByteStream::Tcp(_))
    }
}

impl tokio::io::AsyncRead for ByteStream {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match self.get_mut() {
            ByteStream::Tcp(s) => std::pin::Pin::new(s).poll_read(cx, buf),
            #[cfg(feature = "p2p")]
            ByteStream::Quic(_, _, recv) => std::pin::Pin::new(recv).poll_read(cx, buf),
        }
    }
}

impl tokio::io::AsyncWrite for ByteStream {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        match self.get_mut() {
            ByteStream::Tcp(s) => std::pin::Pin::new(s).poll_write(cx, buf),
            #[cfg(feature = "p2p")]
            ByteStream::Quic(_, send, _) => {
                match std::pin::Pin::new(send).poll_write(cx, buf) {
                    std::task::Poll::Ready(Ok(n)) => std::task::Poll::Ready(Ok(n)),
                    std::task::Poll::Ready(Err(e)) => std::task::Poll::Ready(Err(std::io::Error::new(std::io::ErrorKind::Other, e))),
                    std::task::Poll::Pending => std::task::Poll::Pending,
                }
            }
        }
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match self.get_mut() {
            ByteStream::Tcp(s) => std::pin::Pin::new(s).poll_flush(_cx),
            #[cfg(feature = "p2p")]
            // QUIC SendStream has no explicit flush; data is sent immediately
            ByteStream::Quic(_, _, _) => std::task::Poll::Ready(Ok(())),
        }
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match self.get_mut() {
            ByteStream::Tcp(s) => std::pin::Pin::new(s).poll_shutdown(_cx),
            #[cfg(feature = "p2p")]
            // QUIC streams are closed via finish(), but that's a synchronous call
            // that returns ClosedStream. We can't call it in poll_shutdown without
            // risking a panic. Just return Ok for now.
            ByteStream::Quic(_, _, _) => std::task::Poll::Ready(Ok(())),
        }
    }
}

/// A pooled connection wrapper — generic over transport type
pub struct PooledConnection {
    pub stream: ByteStream,
    pub protocol: ProtocolType,
    pub created_at: std::time::Instant,
}

impl PooledConnection {
    pub fn is_stale(&self, max_age_secs: u64) -> bool {
        self.created_at.elapsed().as_secs() > max_age_secs
    }

    /// Get local address (for logging)
    pub fn local_addr(&self) -> std::io::Result<std::net::SocketAddr> {
        match &self.stream {
            ByteStream::Tcp(s) => s.local_addr(),
            #[cfg(feature = "p2p")]
            ByteStream::Quic(_, _, _) => Ok(std::net::SocketAddr::from(([0,0,0,0], 0))),
        }
    }

    /// Get peer address (for logging)
    pub fn peer_addr(&self) -> std::io::Result<std::net::SocketAddr> {
        match &self.stream {
            ByteStream::Tcp(s) => s.peer_addr(),
            #[cfg(feature = "p2p")]
            ByteStream::Quic(_, _, _) => Ok(std::net::SocketAddr::from(([0,0,0,0], 0))),
        }
    }
}

/// Object-safe transport trait with connection pooling
pub trait Transport: Send + Sync {
    /// Name of this transport (for logging)
    fn name(&self) -> &'static str;

    /// Acquire a connection from the pool (or create new)
    fn acquire(
        &self,
        protocol: ProtocolType,
    ) -> Pin<Box<dyn Future<Output = io::Result<PooledConnection>> + Send + '_>>;

    /// Return connection to pool
    fn release(&self, conn: PooledConnection);

    /// Check if this transport is healthy
    fn health_check(
        &self,
    ) -> Pin<Box<dyn Future<Output = bool> + Send + '_>>;
}

/// Protocol types that the engine supports
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolType {
    /// Connect-RPC / HTTP API (HTTP/1.1)
    ConnectRpc,
    /// gRPC / HTTP/2
    Grpc,
    /// MySQL wire protocol
    MySqlWire,
}

impl ProtocolType {
    /// Convert to ALPN protocol identifier
    pub fn as_alpn(&self) -> &'static [u8] {
        match self {
            ProtocolType::Grpc => ALPN_HTTP2,
            ProtocolType::ConnectRpc => ALPN_HTTP1,
            ProtocolType::MySqlWire => ALPN_MYSQL,
        }
    }

    /// Detect from ALPN protocol identifier
    pub fn from_alpn(alpn: &[u8]) -> Option<Self> {
        match alpn {
            b"h2" => Some(ProtocolType::Grpc),
            b"http/1.1" => Some(ProtocolType::ConnectRpc),
            b"mysql" => Some(ProtocolType::MySqlWire),
            _ => None,
        }
    }
}

impl ProtocolType {
    /// Detect protocol from first few bytes of a stream
    pub fn detect(peek: &[u8]) -> Option<Self> {
        if peek.is_empty() {
            return None;
        }

        if peek[0] == 0x0a {
            return Some(ProtocolType::MySqlWire);
        }

        let http_methods: &[&[u8]] = &[b"GET ", b"POST", b"PUT ", b"DELE", b"HEAD", b"OPTI", b"PATC"];
        for method in http_methods {
            if peek.starts_with(method) {
                return Some(ProtocolType::ConnectRpc);
            }
        }

        if peek.starts_with(b"PRI ") {
            return Some(ProtocolType::Grpc);
        }

        if peek[0] == 0x16 {
            return Some(ProtocolType::ConnectRpc);
        }

        None
    }

    pub fn default_port(&self) -> u16 {
        match self {
            ProtocolType::ConnectRpc => 3001,
            ProtocolType::MySqlWire => 13307,
            ProtocolType::Grpc => 50051,
        }
    }
}

/// Transport factory — creates the right transport based on config
pub struct TransportFactory;

impl TransportFactory {
    pub fn create_pooled_tcp(
        engine_http: std::net::SocketAddr,
        engine_mysql: std::net::SocketAddr,
        pool_size: usize,
    ) -> Arc<dyn Transport> {
        Arc::new(tcp::PooledTcpTransport::new(engine_http, engine_mysql, pool_size))
    }

    pub fn create_p2p_stub() -> Arc<dyn Transport> {
        Arc::new(p2p::P2pTransportStub)
    }

    pub async fn create_p2p_server(
        local_ice: IceCandidates,
        share_code: Option<String>,
    ) -> Result<Arc<dyn Transport>, p2p::P2pError> {
        let transport = p2p::P2pTransport::new_server(local_ice, share_code).await?;
        Ok(Arc::new(transport))
    }

    pub async fn create_p2p_client(
        remote_ice: IceCandidates,
        share_code: Option<String>,
    ) -> Result<Arc<dyn Transport>, p2p::P2pError> {
        let transport = p2p::P2pTransport::new_client(remote_ice, share_code).await?;
        Ok(Arc::new(transport))
    }
}

pub mod tcp;
pub mod p2p;
pub mod stun;
pub mod ice;
pub mod punch;
pub mod dtls;
pub mod quic;
