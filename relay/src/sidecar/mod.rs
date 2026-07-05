//! Sidecar Proxy for External Apps
//! Provides local HTTP (localhost:8080) and MySQL (localhost:3307) endpoints
//! that tunnel through P2P QUIC to the remote engine.
//!
//! Usage:
//!   ./bennett-p2p-proxy --share-url "https://share.bennett.studio/db/AG5BECGUT9?t=...&ice=..."
//!                       --http-bind 127.0.0.1:8080
//!                       --mysql-bind 127.0.0.1:3307

use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{error, info, warn};

use crate::transport::{ByteStream, IceCandidates, ProtocolType, Transport};
use crate::transport::p2p::P2pTransport;

/// HTTP proxy sidecar
pub struct HttpSidecar {
    bind_addr: SocketAddr,
    transport: std::sync::Arc<dyn Transport>,
}

impl HttpSidecar {
    pub fn new(bind_addr: SocketAddr, transport: std::sync::Arc<dyn Transport>) -> Self {
        Self { bind_addr, transport }
    }

    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        let listener = TcpListener::bind(self.bind_addr).await?;
        info!(addr = %self.bind_addr, "HTTP sidecar proxy listening");

        loop {
            let (client, addr) = listener.accept().await?;
            let transport = self.transport.clone();

            tokio::spawn(async move {
                if let Err(e) = handle_http_proxy(client, addr, transport).await {
                    warn!(addr = %addr, error = %e, "HTTP proxy error");
                }
            });
        }
    }
}

async fn handle_http_proxy(
    mut client: TcpStream,
    client_addr: std::net::SocketAddr,
    transport: std::sync::Arc<dyn Transport>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Read HTTP request from client
    let mut buf = [0u8; 8192];
    let n = client.read(&mut buf).await?;

    // Acquire P2P connection for HTTP
    let mut engine_conn = transport.acquire(ProtocolType::ConnectRpc).await
        .map_err(|e| format!("Failed to acquire P2P connection: {}", e))?;

    // Forward request to engine via P2P
    match &mut engine_conn.stream {
        ByteStream::Quic(_conn, send, recv) => {
            // Send the HTTP request
            send.write_all(&buf[..n]).await
                .map_err(|e| format!("Failed to send to engine: {}", e))?;

            // Stream response back to client
            let mut response_buf = [0u8; 8192];
            loop {
                match recv.read(&mut response_buf).await {
                    Ok(0) => break,
                    Ok(n) => {
                        client.write_all(&response_buf[..n]).await?;
                    }
                    Err(e) => {
                        warn!(error = %e, "P2P stream read error");
                        break;
                    }
                }
            }
        }
        ByteStream::Tcp(ref mut stream) => {
            // Fallback for TCP transport
            stream.write_all(&buf[..n]).await?;
            tokio::io::copy_bidirectional(&mut client, stream).await?;
        }
    }

    transport.release(engine_conn);
    Ok(())
}

/// MySQL proxy sidecar  
pub struct MySqlSidecar {
    bind_addr: SocketAddr,
    transport: std::sync::Arc<dyn Transport>,
}

impl MySqlSidecar {
    pub fn new(bind_addr: SocketAddr, transport: std::sync::Arc<dyn Transport>) -> Self {
        Self { bind_addr, transport }
    }

    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        let listener = TcpListener::bind(self.bind_addr).await?;
        info!(addr = %self.bind_addr, "MySQL sidecar proxy listening");

        loop {
            let (client, addr) = listener.accept().await?;
            let transport = self.transport.clone();

            tokio::spawn(async move {
                if let Err(e) = handle_mysql_proxy(client, addr, transport).await {
                    warn!(addr = %addr, error = %e, "MySQL proxy error");
                }
            });
        }
    }
}

async fn handle_mysql_proxy(
    mut client: TcpStream,
    _client_addr: std::net::SocketAddr,
    transport: std::sync::Arc<dyn Transport>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Send MySQL handshake to client
    send_mysql_handshake(&mut client).await?;

    // Read client auth
    let mut auth_buf = [0u8; 1024];
    let n = client.read(&mut auth_buf).await?;

    // Acquire P2P connection for MySQL
    let mut engine_conn = transport.acquire(ProtocolType::MySqlWire).await
        .map_err(|e| format!("Failed to acquire P2P connection: {}", e))?;

    // Forward auth to engine
    match &mut engine_conn.stream {
        ByteStream::Quic(_conn, send, recv) => {
            send.write_all(&auth_buf[..n]).await?;

            // Bidirectional proxy
            let (mut client_read, mut client_write) = client.into_split();
            let mut send_clone = send;
            let mut recv_clone = recv;

            let c2e = tokio::spawn(async move {
                let mut buf = [0u8; 4096];
                loop {
                    match client_read.read(&mut buf).await {
                        Ok(0) => break,
                        Ok(n) => {
                            if send_clone.write_all(&buf[..n]).await.is_err() {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
            });

            let e2c = tokio::spawn(async move {
                let mut buf = [0u8; 4096];
                loop {
                    match recv_clone.read(&mut buf).await {
                        Ok(0) => break,
                        Ok(n) => {
                            if client_write.write_all(&buf[..n]).await.is_err() {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
            });

            tokio::select! {
                _ = c2e => {},
                _ = e2c => {},
            }
        }
        ByteStream::Tcp(ref mut stream) => {
            stream.write_all(&auth_buf[..n]).await?;
            tokio::io::copy_bidirectional(&mut client, stream).await?;
        }
    }

    transport.release(engine_conn);
    Ok(())
}

async fn send_mysql_handshake(stream: &mut TcpStream) -> Result<(), Box<dyn std::error::Error>> {
    // Minimal MySQL handshake v10
    let mut packet = Vec::new();
    packet.push(0x0a); // Protocol version
    packet.extend_from_slice(b"5.7.0-bennett-p2p\0");
    packet.extend_from_slice(&1u32.to_le_bytes()); // Thread ID
    packet.extend_from_slice(&[0u8; 8]); // Auth plugin data part 1
    packet.push(0); // Filler
    packet.extend_from_slice(&0x0200u16.to_le_bytes()); // Capability flags
    packet.push(33); // Charset
    packet.extend_from_slice(&0u16.to_le_bytes()); // Status
    packet.extend_from_slice(&0x0000u16.to_le_bytes()); // Extended capabilities
    packet.push(21); // Auth length
    packet.extend_from_slice(&[0u8; 10]); // Reserved
    packet.extend_from_slice(&[0u8; 12]); // Auth plugin data part 2
    packet.push(0);
    packet.extend_from_slice(b"mysql_native_password\0");

    let len = packet.len() as u32;
    let header = [(len & 0xFF) as u8, ((len >> 8) & 0xFF) as u8, ((len >> 16) & 0xFF) as u8, 0];
    stream.write_all(&header).await?;
    stream.write_all(&packet).await?;
    stream.flush().await?;

    Ok(())
}
