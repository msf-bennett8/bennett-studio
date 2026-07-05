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

/// ALPN protocol identifiers (industry standard from IANA)
pub const ALPN_HTTP2: &[u8] = b"h2";
pub const ALPN_HTTP1: &[u8] = b"http/1.1";
pub const ALPN_MYSQL: &[u8] = b"mysql";

/// A bidirectional byte stream for proxying
pub type ByteStream = TcpStream;

/// A pooled TCP connection wrapper
pub struct PooledConnection {
    pub stream: TcpStream,
    pub protocol: ProtocolType,
    pub created_at: std::time::Instant,
}

impl PooledConnection {
    pub fn is_stale(&self, max_age_secs: u64) -> bool {
        self.created_at.elapsed().as_secs() > max_age_secs
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
}

pub mod tcp;
pub mod p2p;
