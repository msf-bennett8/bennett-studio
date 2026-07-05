//! STUN Client — RFC 8489 (Session Traversal Utilities for NAT)
//! Discovers public IP/port through CGNAT using free public STUN servers.
//!
//! Free STUN servers:
//!   - stun.l.google.com:19302
//!   - stun1.l.google.com:19302
//!   - stun.cloudflare.com:3478
//!   - global.stun.twilio.com:3478

use std::net::SocketAddr;
use tokio::net::UdpSocket;
use tracing::{debug, trace, warn};

/// STUN message types
const STUN_BINDING_REQUEST: u16 = 0x0001;
const STUN_BINDING_SUCCESS: u16 = 0x0101;

/// STUN attribute types
const ATTR_XOR_MAPPED_ADDRESS: u16 = 0x0020;
const ATTR_MAPPED_ADDRESS: u16 = 0x0001;

/// Magic cookie for XOR-MAPPED-ADDRESS (RFC 5389)
const STUN_MAGIC_COOKIE: u32 = 0x2112A442;

/// Free public STUN servers
pub const DEFAULT_STUN_SERVERS: &[&str] = &[
    "stun.l.google.com:19302",
    "stun1.l.google.com:19302",
    "stun.cloudflare.com:3478",
    "global.stun.twilio.com:3478",
];

/// Result of a STUN binding request
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StunResult {
    pub server: String,
    pub public_addr: SocketAddr,
}

/// Send STUN Binding Request to discover public endpoint
///
/// # Arguments
/// * `local_socket` — Bound UDP socket to send from
/// * `stun_server` — STUN server address (e.g. "stun.l.google.com:19302")
///
/// # Returns
/// * `Ok(StunResult)` — Public IP/port discovered
/// * `Err(...)` — STUN request failed
pub async fn query_stun(
    local_socket: &UdpSocket,
    stun_server: &str,
) -> Result<StunResult, StunError> {
    let server_addr: SocketAddr = tokio::net::lookup_host(stun_server)
        .await
        .map_err(|e| StunError::ResolveFailed(stun_server.to_string(), e))?
        .next()
        .ok_or_else(|| StunError::NoAddresses(stun_server.to_string()))?;

    // Build STUN Binding Request (RFC 5389/8489)
    let transaction_id = rand::random::<[u8; 12]>();
    let request = build_binding_request(&transaction_id);

    trace!(server = %stun_server, "Sending STUN Binding Request");

    // Send request
    local_socket
        .send_to(&request, server_addr)
        .await
        .map_err(|e| StunError::SendFailed(e))?;

    // Receive response with timeout
    let mut buf = [0u8; 256];
    let (len, from) = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        local_socket.recv_from(&mut buf),
    )
    .await
    .map_err(|_| StunError::Timeout)?
    .map_err(|e| StunError::RecvFailed(e))?;

    trace!(server = %stun_server, bytes = len, from = %from, "STUN response received");

    // Parse response
    let result = parse_binding_response(&buf[..len], &transaction_id)
        .map_err(|e| StunError::ParseFailed(e))?;

    debug!(
        server = %stun_server,
        public_addr = %result.public_addr,
        "STUN public endpoint discovered"
    );

    Ok(result)
}

/// Query multiple STUN servers and return the first successful result
pub async fn query_stun_servers(
    stun_servers: &[&str],
) -> Result<StunResult, StunError> {
    // Bind a local UDP socket
    let local_socket = UdpSocket::bind("0.0.0.0:0")
        .await
        .map_err(|e| StunError::BindFailed(e))?;

    for server in stun_servers {
        match query_stun(&local_socket, server).await {
            Ok(result) => return Ok(result),
            Err(e) => {
                warn!(server = %server, error = %e, "STUN server failed, trying next");
            }
        }
    }

    Err(StunError::AllServersFailed)
}

/// Build STUN Binding Request message
fn build_binding_request(transaction_id: &[u8; 12]) -> Vec<u8> {
    let mut msg = Vec::with_capacity(20);

    // Message Type: Binding Request
    msg.extend_from_slice(&STUN_BINDING_REQUEST.to_be_bytes());

    // Message Length: 0 (no attributes)
    msg.extend_from_slice(&0u16.to_be_bytes());

    // Magic Cookie
    msg.extend_from_slice(&STUN_MAGIC_COOKIE.to_be_bytes());

    // Transaction ID (12 bytes)
    msg.extend_from_slice(transaction_id);

    msg
}

/// Parse STUN Binding Success Response
fn parse_binding_response(
    data: &[u8],
    expected_tx: &[u8; 12],
) -> Result<StunResult, String> {
    if data.len() < 20 {
        return Err("Response too short".to_string());
    }

    // Message Type
    let msg_type = u16::from_be_bytes([data[0], data[1]]);
    if msg_type != STUN_BINDING_SUCCESS {
        return Err(format!("Unexpected message type: 0x{:04x}", msg_type));
    }

    // Message Length
    let msg_len = u16::from_be_bytes([data[2], data[3]]) as usize;

    // Magic Cookie
    let magic = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
    if magic != STUN_MAGIC_COOKIE {
        return Err("Invalid magic cookie".to_string());
    }

    // Transaction ID
    let tx = &data[8..20];
    if tx != expected_tx {
        return Err("Transaction ID mismatch".to_string());
    }

    // Parse attributes
    let mut pos = 20;
    let end = 20 + msg_len;

    while pos + 4 <= data.len().min(end) {
        let attr_type = u16::from_be_bytes([data[pos], data[pos + 1]]);
        let attr_len = u16::from_be_bytes([data[pos + 2], data[pos + 3]]) as usize;
        pos += 4;

        if pos + attr_len > data.len() {
            break;
        }

        match attr_type {
            ATTR_XOR_MAPPED_ADDRESS => {
                if let Some(addr) = parse_xor_mapped_address(&data[pos..pos + attr_len], magic) {
                    return Ok(StunResult {
                        server: String::new(), // filled by caller
                        public_addr: addr,
                    });
                }
            }
            ATTR_MAPPED_ADDRESS => {
                if let Some(addr) = parse_mapped_address(&data[pos..pos + attr_len]) {
                    return Ok(StunResult {
                        server: String::new(),
                        public_addr: addr,
                    });
                }
            }
            _ => {}
        }

        // Attribute padding to 4-byte boundary
        pos += attr_len + ((4 - (attr_len % 4)) % 4);
    }

    Err("No MAPPED-ADDRESS or XOR-MAPPED-ADDRESS attribute found".to_string())
}

/// Parse XOR-MAPPED-ADDRESS attribute (RFC 5389)
fn parse_xor_mapped_address(data: &[u8], magic: u32) -> Option<SocketAddr> {
    if data.len() < 4 {
        return None;
    }

    let family = data[1];
    let x_port = u16::from_be_bytes([data[2], data[3]]);
    let port = x_port ^ ((magic >> 16) as u16);

    match family {
        0x01 => {
            // IPv4
            if data.len() < 8 {
                return None;
            }
            let x_ip = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
            let ip = std::net::Ipv4Addr::from(x_ip ^ magic);
            Some(SocketAddr::from((ip, port)))
        }
        0x02 => {
            // IPv6 — not handling for now
            None
        }
        _ => None,
    }
}

/// Parse MAPPED-ADDRESS attribute (legacy, for compatibility)
fn parse_mapped_address(data: &[u8]) -> Option<SocketAddr> {
    if data.len() < 4 {
        return None;
    }

    let family = data[1];
    let port = u16::from_be_bytes([data[2], data[3]]);

    match family {
        0x01 => {
            if data.len() < 8 {
                return None;
            }
            let ip = std::net::Ipv4Addr::from([data[4], data[5], data[6], data[7]]);
            Some(SocketAddr::from((ip, port)))
        }
        _ => None,
    }
}

/// STUN client errors
#[derive(Debug)]
pub enum StunError {
    BindFailed(std::io::Error),
    ResolveFailed(String, std::io::Error),
    NoAddresses(String),
    SendFailed(std::io::Error),
    RecvFailed(std::io::Error),
    Timeout,
    ParseFailed(String),
    AllServersFailed,
}

impl std::fmt::Display for StunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StunError::BindFailed(e) => write!(f, "Failed to bind UDP socket: {}", e),
            StunError::ResolveFailed(s, e) => write!(f, "Failed to resolve {}: {}", s, e),
            StunError::NoAddresses(s) => write!(f, "No addresses found for {}", s),
            StunError::SendFailed(e) => write!(f, "Failed to send STUN request: {}", e),
            StunError::RecvFailed(e) => write!(f, "Failed to receive STUN response: {}", e),
            StunError::Timeout => write!(f, "STUN request timed out"),
            StunError::ParseFailed(s) => write!(f, "Failed to parse STUN response: {}", s),
            StunError::AllServersFailed => write!(f, "All STUN servers failed"),
        }
    }
}

impl std::error::Error for StunError {}
