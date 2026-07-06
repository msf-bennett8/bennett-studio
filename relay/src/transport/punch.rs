//! UDP Hole Punching
//! Establishes bidirectional UDP through CGNAT by having both peers send
//! packets to each other's server-reflexive (srflx) candidates.
//!
//! Algorithm:
//! 1. Both peers gather ICE candidates (host + srflx)
//! 2. Exchange srflx addresses via share link / signaling
//! 3. Both peers send UDP STUN-like probes to each other's srflx
//! 4. CGNAT creates outbound mappings allowing return traffic
//! 5. First successful bidirectional packet = hole punched

use std::net::SocketAddr;
use tokio::net::UdpSocket;
use tokio::time::{timeout, Duration};
use tracing::{info, trace};

use super::ice::IceCandidates;

/// Hole punching result
#[derive(Debug)]
pub struct PunchedSocket {
    /// The local UDP socket (now hole-punched)
    pub socket: UdpSocket,
    /// The remote peer's public address
    pub remote_addr: SocketAddr,
    /// Whether we were the initiator (sent first packet)
    pub is_initiator: bool,
}

/// Perform UDP hole punching with LAN fallback
///
/// Industry best practice (WebRTC/iroh): When both peers share the same
/// public IP (same NAT/CGNAT), try host-to-host connection first before
/// attempting UDP hole punching through the NAT. This avoids hairpinning
/// failures which are not widely supported per RFC 5128 §3.3.2.
///
/// # Arguments
/// * `local_socket` — Pre-bound UDP socket (from ICE gathering)
/// * `local_ice` — Our ICE candidates
/// * `remote_ice` — Remote peer's ICE candidates (from share link)
/// * `timeout_secs` — Max time to attempt punching (default 10s)
///
/// # Returns
/// * `Ok(PunchedSocket)` — Bidirectional UDP path established
/// * `Err(...)` — Hole punching failed
pub async fn punch_hole(
    local_socket: &UdpSocket,
    local_ice: &IceCandidates,
    remote_ice: &IceCandidates,
    timeout_secs: u64,
) -> Result<PunchedSocket, PunchError> {
    let local_addr = local_socket
        .local_addr()
        .map_err(|e| PunchError::SocketError(e))?;

    let local_srflx = local_ice
        .srflx_addr()
        .ok_or(PunchError::NoSrflxCandidate)?;
    let remote_srflx = remote_ice
        .srflx_addr()
        .ok_or(PunchError::NoRemoteSrflx)?;

    info!(
        local = %local_addr,
        local_srflx = %local_srflx,
        remote_srflx = %remote_srflx,
        "Starting UDP hole punch"
    );

    // Note: Same-NAT/LAN detection is handled at the QUIC layer (connect_quic_client).
    // This punch_hole function is only called for actual different-NAT hole punching.
    // If both peers share the same public IP, connect_quic_client bypasses this
    // entirely and uses connect_quic_localhost() instead.

    // Send probe packets to remote srflx while listening for responses
    let probe_interval = Duration::from_millis(200);
    let max_duration = Duration::from_secs(timeout_secs);
    let start = std::time::Instant::now();

    let mut is_initiator = false;
    let mut remote_addr: Option<SocketAddr> = None;

    // Buffer for receiving
    let mut buf = [0u8; 64];

    while start.elapsed() < max_duration {
        // Try to receive first (non-blocking check)
        match timeout(Duration::from_millis(50), local_socket.recv_from(&mut buf)).await {
            Ok(Ok((len, addr))) => {
                trace!(from = %addr, bytes = len, "Received packet during hole punch");

                // Verify it's from the remote peer's expected range
                // (NAT may map to a different port than expected)
                if is_from_remote(&addr, remote_ice) {
                    info!(from = %addr, "Hole punch successful — received from remote peer");
                    remote_addr = Some(addr);
                    break;
                }
            }
            _ => {}
        }

        // Send probe to remote srflx
        let probe = build_probe_packet(&local_ice);
        match local_socket.send_to(&probe, remote_srflx).await {
            Ok(_) => {
                trace!(to = %remote_srflx, "Sent probe packet");
                if !is_initiator {
                    is_initiator = true;
                }
            }
            Err(e) => {
                trace!(error = %e, "Probe send failed");
            }
        }

        tokio::time::sleep(probe_interval).await;
    }

    let remote_addr = remote_addr.ok_or(PunchError::Timeout)?;

    // Send a few confirmation packets to solidify the NAT mapping
    for _ in 0..3 {
        let confirm = build_confirm_packet();
        let _ = local_socket.send_to(&confirm, remote_addr).await;
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    info!(
        local = %local_addr,
        remote = %remote_addr,
        initiator = is_initiator,
        "UDP hole punch complete"
    );

    Ok(PunchedSocket {
        // Note: We return the socket reference; caller owns it
        socket: UdpSocket::bind(local_addr)
            .await
            .map_err(|e| PunchError::SocketError(e))?,
        remote_addr,
        is_initiator,
    })
}

/// Check if an incoming address matches any remote candidate
fn is_from_remote(addr: &SocketAddr, remote_ice: &IceCandidates) -> bool {
    // Check exact match on srflx
    if let Some(srflx) = remote_ice.srflx_addr() {
        // IP must match; port may differ due to NAT hairpinning
        if addr.ip() == srflx.ip() {
            return true;
        }
    }

    // Check host candidate (for LAN/direct connections)
    if let Some(host) = remote_ice.host_addr() {
        if addr.ip() == host.ip() && addr.port() == host.port() {
            return true;
        }
    }

    false
}

/// Build a probe packet containing our ICE info
fn build_probe_packet(local_ice: &IceCandidates) -> Vec<u8> {
    // Simple protocol: magic + version + ICE base64
    let ice_json = serde_json::to_string(local_ice).unwrap_or_default();
    let mut packet = Vec::with_capacity(4 + 1 + ice_json.len());

    // Magic: "BENN" in ASCII
    packet.extend_from_slice(b"BENN");
    // Version
    packet.push(1);
    // ICE data
    packet.extend_from_slice(ice_json.as_bytes());

    packet
}

/// Build a confirmation packet
fn build_confirm_packet() -> Vec<u8> {
    vec![b'B', b'E', b'N', b'N', 1, b'O', b'K']
}

/// Parse a probe packet
pub fn parse_probe_packet(data: &[u8]) -> Option<IceCandidates> {
    if data.len() < 6 {
        return None;
    }
    if &data[0..4] != b"BENN" {
        return None;
    }
    if data[4] != 1 {
        return None;
    }

    let ice_json = std::str::from_utf8(&data[5..]).ok()?;
    serde_json::from_str(ice_json).ok()
}

// Note: try_lan_connect removed — LAN/same-machine connections are handled
// at the QUIC layer via connect_quic_localhost() in quic.rs.
// This avoids the UDP probe approach which conflicts with QUIC's socket ownership.

/// Hole punching errors
#[derive(Debug)]
pub enum PunchError {
    SocketError(std::io::Error),
    NoSrflxCandidate,
    NoRemoteSrflx,
    Timeout,
}

impl std::fmt::Display for PunchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PunchError::SocketError(e) => write!(f, "Socket error: {}", e),
            PunchError::NoSrflxCandidate => write!(f, "No server-reflexive candidate available"),
            PunchError::NoRemoteSrflx => write!(f, "Remote peer has no server-reflexive candidate"),
            PunchError::Timeout => write!(f, "Hole punching timed out"),
        }
    }
}

impl std::error::Error for PunchError {}
