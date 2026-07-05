//! ICE (Interactive Connectivity Establishment) Candidate Gathering
//! Gathers host and server-reflexive (srflx) candidates for P2P connections.
//!
//! Candidates are base64-encoded and embedded in share links for manual exchange.

use std::net::SocketAddr;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};


use super::stun::{query_stun_servers, DEFAULT_STUN_SERVERS};
/// ICE candidate types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CandidateType {
    /// Local interface address
    Host,
    /// Public address discovered via STUN (server-reflexive)
    ServerReflexive,
    /// TURN relay address (future)
    Relay,
}

/// A single ICE candidate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IceCandidate {
    pub candidate_type: CandidateType,
    pub address: SocketAddr,
    pub protocol: String, // "udp"
    pub priority: u32,
}

/// Collection of ICE candidates for a peer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IceCandidates {
    pub candidates: Vec<IceCandidate>,
    pub generated_at: chrono::DateTime<chrono::Utc>,
}

impl IceCandidates {
    /// Encode to base64 JSON for embedding in URLs
    pub fn to_base64(&self) -> String {
        let json = serde_json::to_string(self).unwrap_or_default();
        use base64::Engine;
        base64::engine::general_purpose::STANDARD.encode(&json)
    }

    /// Decode from base64 JSON
    pub fn from_base64(encoded: &str) -> Result<Self, String> {
        use base64::Engine;
        let json = base64::engine::general_purpose::STANDARD.decode(encoded).map_err(|e| format!("Base64 decode failed: {}", e))?;
        serde_json::from_slice(&json).map_err(|e| format!("JSON parse failed: {}", e))
    }

    /// Get the best candidate for hole punching (srflx preferred, then host)
    pub fn best_for_punching(&self) -> Option<&IceCandidate> {
        // Prefer server-reflexive (has public IP through NAT)
        self.candidates
            .iter()
            .find(|c| c.candidate_type == CandidateType::ServerReflexive)
            .or_else(|| {
                // Fallback to host candidate
                self.candidates
                    .iter()
                    .find(|c| c.candidate_type == CandidateType::Host)
            })
    }

    /// Get just the srflx candidate address
    pub fn srflx_addr(&self) -> Option<SocketAddr> {
        self.candidates
            .iter()
            .find(|c| c.candidate_type == CandidateType::ServerReflexive)
            .map(|c| c.address)
    }

    /// Get just the host candidate address
    pub fn host_addr(&self) -> Option<SocketAddr> {
        self.candidates
            .iter()
            .find(|c| c.candidate_type == CandidateType::Host)
            .map(|c| c.address)
    }
}

/// Gather ICE candidates for this host
///
/// 1. Binds a UDP socket on a random local port
/// 2. Queries STUN servers to discover public IP/port
/// 3. Returns host + srflx candidates
pub async fn gather_ice_candidates() -> Result<IceCandidates, IceError> {
    info!("Gathering ICE candidates...");

    // Bind local UDP socket
    let local_socket = tokio::net::UdpSocket::bind("0.0.0.0:0")
        .await
        .map_err(|e| IceError::BindFailed(e))?;

    let local_addr = local_socket
        .local_addr()
        .map_err(|e| IceError::BindFailed(e))?;

    debug!(local_addr = %local_addr, "Local UDP socket bound");

    // Host candidate
    let host_candidate = IceCandidate {
        candidate_type: CandidateType::Host,
        address: local_addr,
        protocol: "udp".to_string(),
        priority: candidate_priority(CandidateType::Host, false),
    };

    // Query STUN servers for srflx candidate
    let stun_result = query_stun_servers(DEFAULT_STUN_SERVERS)
        .await
        .map_err(|e| IceError::StunFailed(e))?;

    let srflx_candidate = IceCandidate {
        candidate_type: CandidateType::ServerReflexive,
        address: stun_result.public_addr,
        protocol: "udp".to_string(),
        priority: candidate_priority(CandidateType::ServerReflexive, false),
    };

    info!(
        host = %local_addr,
        srflx = %stun_result.public_addr,
        "ICE candidates gathered"
    );

    Ok(IceCandidates {
        candidates: vec![host_candidate, srflx_candidate],
        generated_at: chrono::Utc::now(),
    })
}

/// Calculate candidate priority (RFC 8445 formula simplified)
///
/// type preference: host=126, srflx=100, relay=0
/// local preference: 65535
fn candidate_priority(candidate_type: CandidateType, _is_relay: bool) -> u32 {
    let type_pref = match candidate_type {
        CandidateType::Host => 126,
        CandidateType::ServerReflexive => 100,
        CandidateType::Relay => 0,
    };

    let local_pref: u32 = 65535;

    (1u32 << 24) * type_pref as u32 + (1u32 << 8) * local_pref + (256u32 - 0)
}

/// ICE gathering errors
#[derive(Debug)]
pub enum IceError {
    BindFailed(std::io::Error),
    StunFailed(super::stun::StunError),
}

impl std::fmt::Display for IceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IceError::BindFailed(e) => write!(f, "Failed to bind socket: {}", e),
            IceError::StunFailed(e) => write!(f, "STUN query failed: {}", e),
        }
    }
}

impl std::error::Error for IceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            IceError::BindFailed(e) => Some(e),
            IceError::StunFailed(e) => Some(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base64_roundtrip() {
        let candidates = IceCandidates {
            candidates: vec![IceCandidate {
                candidate_type: CandidateType::Host,
                address: "127.0.0.1:12345".parse().unwrap(),
                protocol: "udp".to_string(),
                priority: 2130706431,
            }],
            generated_at: chrono::Utc::now(),
        };

        let encoded = candidates.to_base64();
        let decoded = IceCandidates::from_base64(&encoded).unwrap();
        assert_eq!(decoded.candidates.len(), 1);
        assert_eq!(decoded.candidates[0].address, "127.0.0.1:12345".parse().unwrap());
    }

    #[test]
    fn test_best_for_punching() {
        let candidates = IceCandidates {
            candidates: vec![
                IceCandidate {
                    candidate_type: CandidateType::Host,
                    address: "192.168.1.1:12345".parse().unwrap(),
                    protocol: "udp".to_string(),
                    priority: 2130706431,
                },
                IceCandidate {
                    candidate_type: CandidateType::ServerReflexive,
                    address: "203.0.113.1:54321".parse().unwrap(),
                    protocol: "udp".to_string(),
                    priority: 1694498815,
                },
            ],
            generated_at: chrono::Utc::now(),
        };

        let best = candidates.best_for_punching().unwrap();
        assert_eq!(best.candidate_type, CandidateType::ServerReflexive);
    }
}
