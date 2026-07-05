//! P2P Transport — STUB for future WebRTC/QUIC implementation
//!
//! This module is intentionally minimal. When you move to a CGNAT network
//! or need direct peer-to-peer connections, implement:
//! - ICE candidate gathering (STUN/TURN)
//! - DTLS handshake
//! - SCTP or QUIC data channels
//! - Signaling via WebSocket

use super::{ProtocolType, Transport};
use std::io;

/// P2P transport stub — returns "not implemented" for all operations
#[derive(Clone)]
pub struct P2pTransportStub;

impl Transport for P2pTransportStub {
    fn name(&self) -> &'static str {
        "p2p-stub"
    }

    fn connect(
        &self,
        _share_id: &str,
        _protocol: ProtocolType,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = io::Result<tokio::net::TcpStream>> + Send + '_>> {
        Box::pin(async move {
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "P2P transport not yet implemented. Enable TCP transport instead.",
            ))
        })
    }

    fn health_check(
        &self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = bool> + Send + '_>> {
        Box::pin(async move { false })
    }
}

// Future P2P transport implementation notes:
//
// 1. Signaling: WebSocket to relay.bennett.studio/signaling
// 2. ICE: Gather host, srflx, relay candidates via STUN/TURN
// 3. Connection: DTLS + SCTP (WebRTC data channel) or QUIC
// 4. Multiplexing: One P2P connection, many share streams
// 5. Fallback: If P2P fails after 5s, fall back to TCP relay
//
// When implemented, swap P2pTransportStub for P2pTransport:
//
// pub struct P2pTransport {
//     signaling_url: String,
//     ice_servers: Vec<String>,
//     connection_pool: DashMap<String, P2pConnection>,
// }
