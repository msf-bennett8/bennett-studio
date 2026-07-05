//! Pluggable transport layer
//! TCP (relay) is active. P2P (WebRTC/QUIC) is a stub for future fallback.
//!
//! Uses trait-variant for object-safe async traits (industry standard:
//! used by Quinn, rustls, and async Rust ecosystem).

use std::io;
use std::sync::Arc;
use tokio::net::TcpStream;

/// A bidirectional byte stream for proxying
pub type ByteStream = TcpStream;

/// Object-safe transport trait using trait-variant
/// This generates both async methods (for implementors) and
/// poll-based methods (for dyn compatibility)
pub trait Transport: Send + Sync {
    /// Name of this transport (for logging)
    fn name(&self) -> &'static str;

    /// Connect to the engine for a given share
    fn connect(
        &self,
        share_id: &str,
        protocol: ProtocolType,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = io::Result<ByteStream>> + Send + '_>>;

    /// Check if this transport is healthy
    fn health_check(
        &self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = bool> + Send + '_>>;
}

/// Protocol types that the engine supports
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolType {
    /// Connect-RPC / HTTP API
    ConnectRpc,
    /// MySQL wire protocol
    MySqlWire,
    /// gRPC HTTP/2
    Grpc,
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
    pub fn create_tcp(
        engine_http: std::net::SocketAddr,
        engine_mysql: std::net::SocketAddr,
    ) -> Arc<dyn Transport> {
        Arc::new(tcp::TcpTransport::new(engine_http, engine_mysql))
    }

    pub fn create_p2p_stub() -> Arc<dyn Transport> {
        Arc::new(p2p::P2pTransportStub)
    }
}

pub mod tcp;
pub mod p2p;
