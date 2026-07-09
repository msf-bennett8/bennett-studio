//! QUIC transport over punched UDP
//! Uses the quinn crate for QUIC implementation.
//!
//! Architecture:
//! - One QUIC connection per P2P session
//! - Multiple bidirectional streams per connection
//! - Stream 0: HTTP/Connect-RPC API
//! - Stream 1: MySQL wire protocol

use std::net::SocketAddr;
use tracing::{debug, info};

use super::dtls::{build_quinn_client_config, build_quinn_server_config, DtlsError};
use super::ice::IceCandidates;
use super::punch::{punch_hole, PunchError};

/// P2P QUIC connection manager
#[derive(Clone)]
pub struct P2pQuicConnection {
    pub connection: quinn::Connection,
    pub remote_addr: SocketAddr,
    pub local_addr: SocketAddr,
    pub is_server: bool,
}

/// Start QUIC server on a punched UDP socket
///
/// # Arguments
/// * `local_ice` — Our ICE candidates
/// * `share_code` — Expected share code for validation
pub async fn start_quic_server(
    local_ice: &IceCandidates,
    _share_code: Option<String>,
) -> Result<P2pQuicConnection, QuicError> {
    let server_config = build_quinn_server_config()
        .map_err(|e| QuicError::TlsFailed(e))?;

    // Industry best practice: Reuse the ICE host candidate socket for QUIC.
    // This ensures the QUIC server listens on the exact port advertised
    // in the host candidate, enabling same-machine localhost connections.
    let bind_addr = if let Some(host) = local_ice.host_addr() {
        info!(host_port = host.port(), "Binding QUIC server on ICE host candidate port");
        SocketAddr::from(([0, 0, 0, 0], host.port()))
    } else {
        // Fallback: random port (should never happen if ICE gathered correctly)
        "0.0.0.0:0".parse().unwrap()
    };

    let endpoint = quinn::Endpoint::server(server_config, bind_addr)
        .map_err(|e| QuicError::EndpointFailed(e))?;

    let local_addr = endpoint.local_addr()
        .map_err(|e| QuicError::EndpointFailed(e))?;

    info!(local_addr = %local_addr, "QUIC server endpoint bound");

    // Wait for incoming connection
    // In P2P, the client will connect after hole punching
    let incoming = endpoint.accept()
        .await
        .ok_or(QuicError::NoIncomingConnection)?;

    let connection = incoming.await
        .map_err(|e| QuicError::ConnectionFailed(e))?;

    let remote_addr = connection.remote_address();
    let local_ip = connection.local_ip().unwrap_or_else(|| std::net::IpAddr::from([0,0,0,0]));

    info!(
        remote = %remote_addr,
        stable_id = connection.stable_id(),
        "QUIC P2P client connected"
    );

    Ok(P2pQuicConnection {
        connection,
        remote_addr,
        local_addr: SocketAddr::new(local_ip, 0),
        is_server: true,
    })
}

/// Connect QUIC client to a punched UDP path
///
/// # Arguments
/// * `remote_ice` — Remote peer's ICE candidates
/// * `local_ice` — Our ICE candidates (for hole punching)
pub async fn connect_quic_client(
    remote_ice: &IceCandidates,
    local_ice: &IceCandidates,
) -> Result<P2pQuicConnection, QuicError> {
    // Detect same-machine/same-NAT scenario
    let same_nat = match (local_ice.srflx_addr(), remote_ice.srflx_addr()) {
        (Some(local_srflx), Some(remote_srflx)) => local_srflx.ip() == remote_srflx.ip(),
        _ => false,
    };

    if same_nat {
        return connect_quic_localhost(remote_ice, local_ice).await;
    }

    // Normal path: bind UDP socket and perform hole punching
    let local_socket = tokio::net::UdpSocket::bind("0.0.0.0:0")
        .await
        .map_err(|e| QuicError::IoError(e))?;

    // Perform hole punching
    let punched = punch_hole(&local_socket, local_ice, remote_ice, 10)
        .await
        .map_err(|e| QuicError::PunchFailed(e))?;

    info!(
        local = %punched.socket.local_addr().unwrap(),
        remote = %punched.remote_addr,
        "UDP hole punched, starting QUIC"
    );

    // Create QUIC endpoint on the punched socket
    let client_config = build_quinn_client_config()
        .map_err(|e| QuicError::TlsFailed(e))?;

    let mut endpoint = quinn::Endpoint::client(punched.socket.local_addr().unwrap())
        .map_err(|e| QuicError::EndpointFailed(e))?;

    endpoint.set_default_client_config(client_config);

    // Connect to remote
    let connection = endpoint
        .connect(punched.remote_addr, "bennett-p2p.local")
        .map_err(|e| QuicError::ConnectFailed(e))?
        .await
        .map_err(|e| QuicError::ConnectionFailed(e))?;

    let remote_addr = connection.remote_address();
    let local_ip = connection.local_ip().unwrap_or_else(|| std::net::IpAddr::from([0,0,0,0]));

    info!(
        remote = %remote_addr,
        stable_id = connection.stable_id(),
        "QUIC P2P connection established"
    );

    Ok(P2pQuicConnection {
        connection,
        remote_addr,
        local_addr: SocketAddr::new(local_ip, 0),
        is_server: false,
    })
}

/// Connect QUIC client directly via localhost (same-machine testing)
/// Bypasses hole punching entirely — used when both peers are on the same host
pub async fn connect_quic_localhost(
    remote_ice: &IceCandidates,
    _local_ice: &IceCandidates,
) -> Result<P2pQuicConnection, QuicError> {
    info!("Same-machine detected — using localhost QUIC connection (bypassing hole punch)");

    // Get the engine's QUIC server port from the remote ICE data
    // The engine stores its QUIC endpoint port in the ICE candidates metadata
    // For now, we use a well-known localhost approach:
    // The engine's QUIC server binds to 0.0.0.0:0 (random), but we need to know the port.
    // 
    // SOLUTION: The engine should include its QUIC endpoint port in the ICE candidates.
    // For now, we extract it from the host candidate if it was set, or use a fallback.

    let client_config = build_quinn_client_config()
        .map_err(|e| QuicError::TlsFailed(e))?;

    let mut endpoint = quinn::Endpoint::client("127.0.0.1:0".parse().unwrap())
        .map_err(|e| QuicError::EndpointFailed(e))?;

    endpoint.set_default_client_config(client_config);

    // Use the remote's host candidate port as the QUIC server port.
    // The engine binds QUIC directly on its ICE host candidate port,
    // so we connect to localhost:that_port for same-machine testing.
    let remote_quic_addr = if let Some(host) = remote_ice.host_addr() {
        // Engine binds QUIC on 0.0.0.0:host_port → accessible via 127.0.0.1:host_port
        let addr = SocketAddr::from(([127, 0, 0, 1], host.port()));
        info!(engine_host_port = host.port(), localhost_addr = %addr, "Resolved engine QUIC endpoint");
        addr
    } else {
        return Err(QuicError::EndpointFailed(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "No host candidate for localhost connection"
        )));
    };

    info!(remote = %remote_quic_addr, "Connecting QUIC to localhost");

    let connection = endpoint
        .connect(remote_quic_addr, "bennett-p2p.local")
        .map_err(|e| QuicError::ConnectFailed(e))?
        .await
        .map_err(|e| QuicError::ConnectionFailed(e))?;

    let remote_addr = connection.remote_address();
    let local_ip = connection.local_ip().unwrap_or_else(|| std::net::IpAddr::from([127, 0, 0, 1]));

    info!(
        remote = %remote_addr,
        stable_id = connection.stable_id(),
        "QUIC localhost connection established"
    );

    Ok(P2pQuicConnection {
        connection,
        remote_addr,
        local_addr: SocketAddr::new(local_ip, 0),
        is_server: false,
    })
}

/// Open a bidirectional stream for a specific protocol
pub async fn open_stream(
    conn: &P2pQuicConnection,
    protocol: super::ProtocolType,
) -> Result<(quinn::SendStream, quinn::RecvStream), QuicError> {
    let (send, recv) = conn.connection
        .open_bi()
        .await
        .map_err(|e| QuicError::StreamFailed(e))?;

    // Send protocol identifier as first byte
    let proto_byte = match protocol {
        super::ProtocolType::ConnectRpc => 0x01,
        super::ProtocolType::Grpc => 0x02,
        super::ProtocolType::MySqlWire => 0x03,
    };

      let mut send = send;
      send.write_all(&[proto_byte])
          .await
          .map_err(|e| QuicError::StreamWriteFailed(e))?;

    debug!(protocol = ?protocol, "Opened QUIC stream");

    Ok((send, recv))
}

/// Accept a bidirectional stream and detect protocol
pub async fn accept_stream(
    conn: &P2pQuicConnection,
) -> Result<(super::ProtocolType, quinn::SendStream, quinn::RecvStream), QuicError> {
    let (send, mut recv) = conn.connection
        .accept_bi()
        .await
        .map_err(|e| QuicError::StreamFailed(e))?;

    // Read protocol identifier
    let mut proto_buf = [0u8; 1];
      recv.read_exact(&mut proto_buf)
          .await
          .map_err(|e| QuicError::StreamReadFailed(e))?;

    let protocol = match proto_buf[0] {
        0x01 => super::ProtocolType::ConnectRpc,
        0x02 => super::ProtocolType::Grpc,
        0x03 => super::ProtocolType::MySqlWire,
        _ => super::ProtocolType::ConnectRpc, // Default fallback
    };

    debug!(protocol = ?protocol, "Accepted QUIC stream");

    Ok((protocol, send, recv))
}

/// QUIC errors
#[derive(Debug)]
pub enum QuicError {
    TlsFailed(DtlsError),
    EndpointFailed(std::io::Error),
    IoError(std::io::Error),
    PunchFailed(PunchError),
    NoIncomingConnection,
    ConnectFailed(quinn::ConnectError),
    ConnectionFailed(quinn::ConnectionError),
    StreamFailed(quinn::ConnectionError),
    StreamWriteFailed(quinn::WriteError),
    StreamReadFailed(quinn::ReadExactError),
}

impl std::fmt::Display for QuicError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QuicError::TlsFailed(e) => write!(f, "TLS setup failed: {}", e),
            QuicError::EndpointFailed(e) => write!(f, "Endpoint creation failed: {}", e),
            QuicError::IoError(e) => write!(f, "I/O error: {}", e),
            QuicError::PunchFailed(e) => write!(f, "Hole punching failed: {}", e),
            QuicError::NoIncomingConnection => write!(f, "No incoming QUIC connection"),
            QuicError::ConnectFailed(e) => write!(f, "QUIC connect failed: {}", e),
            QuicError::ConnectionFailed(e) => write!(f, "QUIC connection failed: {}", e),
            QuicError::StreamFailed(e) => write!(f, "QUIC stream failed: {}", e),
            QuicError::StreamWriteFailed(e) => write!(f, "QUIC stream write failed: {}", e),
            QuicError::StreamReadFailed(e) => write!(f, "QUIC stream read failed: {}", e),
        }
    }
}

impl std::error::Error for QuicError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            QuicError::TlsFailed(e) => Some(e),
            QuicError::PunchFailed(e) => Some(e),
            _ => None,
        }
    }
}
