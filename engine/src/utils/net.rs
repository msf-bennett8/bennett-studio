// Network utilities

use std::net::{IpAddr, Ipv4Addr, TcpStream};

/// Detect the primary LAN IP address of this machine
/// Strategy: Connect to a known public IP (Google DNS 8.8.8.8:53)
/// to reveal the local IP used for outbound connections
pub fn detect_lan_ip() -> Option<String> {
    if let Ok(socket) = TcpStream::connect("8.8.8.8:53") {
        if let Ok(local_addr) = socket.local_addr() {
            if let IpAddr::V4(ip) = local_addr.ip() {
                if !ip.is_loopback() {
                    return Some(ip.to_string());
                }
            }
        }
    }
    None
}

/// Detect the engine's configured port
/// Checks env var BENNETT_ENGINE_PORT, defaults to 3001
pub fn detect_engine_port() -> u16 {
    std::env::var("BENNETT_ENGINE_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3001)
}
