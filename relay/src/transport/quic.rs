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
    _local_ice: &IceCandidates,
    _share_code: Option<String>,
) -> Result<P2pQuicConnection, QuicError> {
    let server_config = build_quinn_server_config()
        .map_err(|e| QuicError::TlsFailed(e))?;

    let endpoint = quinn::Endpoint::server(server_config, "0.0.0.0:0".parse().unwrap())
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
    let local_addr = connection.local_ip().unwrap_or_else(|| std::net::IpAddr::from([0,0,0,0]));

    info!(
        remote = %remote_addr,
        stable_id = connection.stable_id(),
        "QUIC P2P client connected"
    );

    Ok(P2pQuicConnection {
        connection,
        remote_addr,
        local_addr: SocketAddr::new(local_addr, 0),
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
    _local_ice: &IceCandidates,
) -> Result<P2pQuicConnection, QuicError> {
    // Bind local UDP socket
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
    let local_addr = connection.local_ip().unwrap_or_else(|| std::net::IpAddr::from([0,0,0,0]));

    info!(
        remote = %remote_addr,
        stable_id = connection.stable_id(),
        "QUIC P2P connection established"
    );

    Ok(P2pQuicConnection {
        connection,
        remote_addr,
        local_addr: SocketAddr::new(local_addr, 0),
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
          .map_err(|e| QuicError::IoError(e))?;

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
          .map_err(|e| QuicError::IoError(e))?;

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
