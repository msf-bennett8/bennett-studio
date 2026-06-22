//! MySQL wire protocol proxy
//! Intercepts MySQL handshake, validates JWT, forwards to real MySQL server

use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::{info, warn, error};

use crate::AppState;
use crate::sharing::proxy::tls::CertManager;
use crate::sharing::proxy::{validate_wire_auth, WireAuthResult};

/// MySQL protocol constants
const MYSQL_HANDSHAKE_V10: u8 = 0x0a;
const MYSQL_AUTH_PLUGIN_NAME: &str = "mysql_native_password";
const MYSQL_MAX_PACKET_SIZE: u32 = 16777215;

/// Handle MySQL client connection
pub async fn handle_mysql_client(
    mut client_stream: TcpStream,
    peer_addr: SocketAddr,
    state: AppState,
    cert_manager: Arc<CertManager>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Read handshake response (client sends auth info first in some modes)
    // Standard MySQL: server sends handshake first
    // We need to send our own handshake with the share code as server version
    
    // Send handshake v10
    let share_code = "UNKNOWN"; // Will be extracted from username
    send_mysql_handshake(&mut client_stream, share_code).await?;
    
    // Read client auth response
    let (username, password, database) = read_mysql_auth_response(&mut client_stream).await?;
    
    // Extract share code from username (format: bennett_SHARECODE)
    let actual_share_code = if username.starts_with("bennett_") {
        username.strip_prefix("bennett_").unwrap_or(&username)
    } else {
        &username
    };
    
    // Validate
    let auth_result = match validate_wire_auth(&state, actual_share_code, &password, peer_addr).await {
        Ok(r) => r,
        Err(e) => {
            send_mysql_error(&mut client_stream, 1045, "28000", &format!("Access denied: {}", e)).await?;
            return Ok(());
        }
    };
    
    // Send OK packet
    send_mysql_ok(&mut client_stream, 1).await?;
    
    info!("MySQL wire proxy: authenticated {} for db {}", peer_addr, auth_result.db_instance.name);
    
    // Connect to real MySQL server
    let db_port = auth_result.db_instance.port;
    let mut db_stream = match TcpStream::connect(format!("127.0.0.1:{}", db_port)).await {
        Ok(s) => s,
        Err(e) => {
            send_mysql_error(&mut client_stream, 2003, "HY000", &format!("Cannot connect to database: {}", e)).await?;
            return Ok(());
        }
    };
    
    // Bidirectional proxy
    proxy_bidirectional(client_stream, db_stream, &auth_result).await?;
    
    Ok(())
}

/// Send MySQL handshake v10 packet
async fn send_mysql_handshake(
    stream: &mut TcpStream,
    share_code: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let server_version = format!("5.7.0-bennett-{}", share_code);
    let thread_id: u32 = 1;
    let auth_data: [u8; 20] = rand::random(); // Scramble
    let capability_flags: u32 = 0x0001 | 0x0004 | 0x0200 | 0x8000; // LONG_PASSWORD, CONNECT_WITH_DB, PROTOCOL_41, SECURE_CONNECTION
    
    let mut packet = Vec::new();
    packet.push(MYSQL_HANDSHAKE_V10); // Protocol version
    packet.extend_from_slice(server_version.as_bytes());
    packet.push(0); // Null terminator
    packet.extend_from_slice(&thread_id.to_le_bytes());
    packet.extend_from_slice(&auth_data[0..8]); // Auth plugin data part 1
    packet.push(0); // Filler
    packet.extend_from_slice(&capability_flags.to_le_bytes()[0..2]); // Lower capability flags
    packet.push(33); // Character set utf8mb4
    packet.extend_from_slice(&[0u8; 2]); // Status flags
    packet.extend_from_slice(&capability_flags.to_le_bytes()[2..4]); // Upper capability flags
    packet.push(21); // Auth plugin data length
    packet.extend_from_slice(&[0u8; 10]); // Reserved
    packet.extend_from_slice(&auth_data[8..20]); // Auth plugin data part 2
    packet.push(0);
    packet.extend_from_slice(MYSQL_AUTH_PLUGIN_NAME.as_bytes());
    packet.push(0);
    
    // Write packet with length header
    write_mysql_packet(stream, 0, &packet).await?;
    
    Ok(())
}

/// Read MySQL auth response (HandshakeResponse41)
async fn read_mysql_auth_response(
    stream: &mut TcpStream,
) -> Result<(String, String, String), Box<dyn std::error::Error>> {
    let (seq, payload) = read_mysql_packet(stream).await?;
    
    // Parse HandshakeResponse41
    let capability_flags = u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
    let _max_packet_size = u32::from_le_bytes([payload[4], payload[5], payload[6], payload[7]]);
    let _charset = payload[8];
    
    let mut pos = 32; // After reserved
    
    // Username (null-terminated)
    let mut username = String::new();
    while pos < payload.len() && payload[pos] != 0 {
        username.push(payload[pos] as char);
        pos += 1;
    }
    pos += 1; // Skip null
    
    // Auth response length-encoded
    let auth_len = payload[pos] as usize;
    pos += 1;
    let auth_response = &payload[pos..pos + auth_len];
    pos += auth_len;
    
    // Database (null-terminated) if CONNECT_WITH_DB
    let mut database = String::new();
    if capability_flags & 0x0008 != 0 && pos < payload.len() {
        while pos < payload.len() && payload[pos] != 0 {
            database.push(payload[pos] as char);
            pos += 1;
        }
    }
    
    // Decode password from auth response (simplified - in production use proper auth plugin)
    let password = String::from_utf8_lossy(auth_response).to_string();
    
    Ok((username, password, database))
}

/// Send MySQL OK packet
async fn send_mysql_ok(
    stream: &mut TcpStream,
    seq: u8,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut packet = Vec::new();
    packet.push(0x00); // OK header
    packet.push(0x00); // Affected rows (length encoded)
    packet.push(0x00); // Last insert ID
    packet.extend_from_slice(&[0x00, 0x00]); // Status flags
    packet.extend_from_slice(&[0x00, 0x00]); // Warnings
    
    write_mysql_packet(stream, seq, &packet).await?;
    Ok(())
}

/// Send MySQL ERROR packet
async fn send_mysql_error(
    stream: &mut TcpStream,
    seq: u8,
    error_code: u16,
    sql_state: &str,
    message: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut packet = Vec::new();
    packet.push(0xff); // ERROR header
    packet.extend_from_slice(&error_code.to_le_bytes());
    packet.push(b'#');
    packet.extend_from_slice(sql_state.as_bytes());
    packet.extend_from_slice(message.as_bytes());
    
    write_mysql_packet(stream, seq, &packet).await?;
    Ok(())
}

/// Write MySQL packet with 4-byte header
async fn write_mysql_packet(
    stream: &mut TcpStream,
    seq: u8,
    payload: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    let len = payload.len() as u32;
    let header = [
        (len & 0xFF) as u8,
        ((len >> 8) & 0xFF) as u8,
        ((len >> 16) & 0xFF) as u8,
        seq,
    ];
    
    stream.write_all(&header).await?;
    stream.write_all(payload).await?;
    stream.flush().await?;
    
    Ok(())
}

/// Read MySQL packet
async fn read_mysql_packet(
    stream: &mut TcpStream,
) -> Result<(u8, Vec<u8>), Box<dyn std::error::Error>> {
    let mut header = [0u8; 4];
    stream.read_exact(&mut header).await?;
    
    let len = u32::from_le_bytes([header[0], header[1], header[2], 0]);
    let seq = header[3];
    
    let mut payload = vec![0u8; len as usize];
    stream.read_exact(&mut payload).await?;
    
    Ok((seq, payload))
}

/// Bidirectional proxy between client and database
async fn proxy_bidirectional(
    client: TcpStream,
    db: TcpStream,
    auth: &WireAuthResult,
) -> Result<(), Box<dyn std::error::Error>> {
    let (mut client_read, mut client_write) = client.into_split();
    let (mut db_read, mut db_write) = db.into_split();
    
    let client_to_db = tokio::spawn(async move {
        let mut buf = [0u8; 8192];
        loop {
            match client_read.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => {
                    if db_write.write_all(&buf[..n]).await.is_err() {
                        break;
                    }
                    if db_write.flush().await.is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });
    
    let db_to_client = tokio::spawn(async move {
        let mut buf = [0u8; 8192];
        loop {
            match db_read.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => {
                    if client_write.write_all(&buf[..n]).await.is_err() {
                        break;
                    }
                    if client_write.flush().await.is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });
    
    // Wait for either direction to close
    tokio::select! {
        _ = client_to_db => {},
        _ = db_to_client => {},
    }
    
    info!("MySQL wire proxy closed for {}", auth.peer_addr);
    Ok(())
}

/// TODO: Phase 5 - Implement proper MySQL auth plugin (caching_sha2_password)
/// TODO: Phase 5 - Implement query interception for audit logging
/// TODO: Phase 5 - Implement RLS injection for MySQL queries
