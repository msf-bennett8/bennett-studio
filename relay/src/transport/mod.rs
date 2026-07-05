//! Pluggable transport layer
//! TCP (relay) is active. P2P (WebRTC/QUIC) is a stub for future fallback.

use async_trait::async_trait;
use std::io;
use tokio::net::TcpStream;

/// A bidirectional byte stream for proxying
pub type ByteStream = TcpStream;

/// Transport trait — abstracts how we reach the engine
#[async_trait]
pub trait Transport: Send + Sync {
    /// Name of this transport (for logging)
    fn name(&self) -> &'static str;

    /// Connect to the engine for a given share
    /// Returns a bidirectional stream
    async fn connect(
        &self,
        share_id: &str,
        protocol: ProtocolType,
    ) -> io::Result<ByteStream>;

    /// Check if this transport is healthy
    async fn health_check(&self) -> bool;
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

        // MySQL wire protocol: first byte is protocol version (0x0a = 10)
        if peek[0] == 0x0a {
            return Some(ProtocolType::MySqlWire);
        }

        // HTTP/1.x: starts with method names
        let http_methods = [b"GET ", b"POST", b"PUT ", b"DELE", b"HEAD", b"OPTI", b"PATC"];
        for method in &http_methods {
            if peek.starts_with(method) {
                return Some(ProtocolType::ConnectRpc);
            }
        }

        // HTTP/2: starts with PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n
        if peek.starts_with(b"PRI ") {
            return Some(ProtocolType::Grpc);
        }

        // TLS ClientHello: starts with 0x16 (handshake record)
        if peek[0] == 0x16 {
            // Could be HTTPS (gRPC or Connect-RPC)
            // We'll determine after TLS termination
            return Some(ProtocolType::ConnectRpc);
        }

        None
    }

    /// Default engine port for this protocol
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
    /// Create the primary transport (TCP relay to local engine)
    pub fn create_tcp(
        engine_http: std::net::SocketAddr,
        engine_mysql: std::net::SocketAddr,
    ) -> Box<dyn Transport> {
        Box::new(tcp::TcpTransport::new(engine_http, engine_mysql))
    }

    /// Create P2P transport stub (future implementation)
    pub fn create_p2p_stub() -> Box<dyn Transport> {
        Box::new(p2p::P2pTransportStub)
    }
}

pub mod tcp;
pub mod p2p;
