//! PostgreSQL wire protocol proxy
//! Intercepts PostgreSQL startup, validates JWT, forwards to real PostgreSQL server

use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::{info, warn, error};

use crate::AppState;
use crate::sharing::proxy::tls::CertManager;
use crate::sharing::proxy::{validate_wire_auth, WireAuthResult};

/// PostgreSQL protocol constants
const PG_SSL_REQUEST: i32 = 80877103; // 1234, 5679 in network byte order
const PG_STARTUP_VERSION: i32 = 196608; // 3.0

/// Handle PostgreSQL client connection
pub async fn handle_postgres_client(
    mut client_stream: TcpStream,
    peer_addr: SocketAddr,
    state: AppState,
    _cert_manager: Arc<CertManager>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Read startup message
    let startup = read_pg_startup_message(&mut client_stream).await?;
    
    // Check for SSL request
    if startup.is_ssl_request {
        // Deny SSL for now (simplified), or negotiate
        client_stream.write_all(b"N").await?; // 'N' = SSL not supported
        // Re-read startup
        let startup = read_pg_startup_message(&mut client_stream).await?;
        return handle_startup(startup, client_stream, peer_addr, state).await;
    }
    
    handle_startup(startup, client_stream, peer_addr, state).await
}

async fn handle_startup(
    startup: PgStartupMessage,
    mut client_stream: TcpStream,
    peer_addr: SocketAddr,
    state: AppState,
) -> Result<(), Box<dyn std::error::Error>> {
    // Extract credentials from startup parameters
    let user = startup.params.get("user").cloned().unwrap_or_default();
    let database = startup.params.get("database").cloned().unwrap_or_default();
    
    // Extract share code from user (format: bennett_SHARECODE)
    let share_code = if user.starts_with("bennett_") {
        user.strip_prefix("bennett_").unwrap_or(&user).to_string()
    } else {
        user.clone()
    };
    
    // Password is sent in AuthenticationCleartextPassword or AuthenticationMD5Password
    // Send AuthenticationCleartextPassword request
    send_pg_auth_request(&mut client_stream, 3).await?; // 3 = cleartext
    
    // Read password message
    let password = read_pg_password_message(&mut client_stream).await?;
    
    // Validate
    let auth_result = match validate_wire_auth(&state, &share_code, &password, peer_addr).await {
        Ok(r) => r,
        Err(e) => {
            send_pg_error(&mut client_stream, "28P01", &format!("authentication failed: {}", e)).await?;
            return Ok(());
        }
    };
    
    // Send AuthenticationOK
    send_pg_auth_ok(&mut client_stream).await?;
    
    // Send parameter status messages
    send_pg_parameter_status(&mut client_stream, "server_version", "14.0 (Bennett Proxy)").await?;
    send_pg_parameter_status(&mut client_stream, "server_encoding", "UTF8").await?;
    send_pg_parameter_status(&mut client_stream, "client_encoding", "UTF8").await?;
    send_pg_parameter_status(&mut client_stream, "DateStyle", "ISO, MDY").await?;
    
    // Send ReadyForQuery
    send_pg_ready_for_query(&mut client_stream, 'I').await?; // 'I' = Idle
    
    info!("PostgreSQL wire proxy: authenticated {} for db {}", peer_addr, auth_result.db_instance.name);
    
    // Connect to real PostgreSQL server
    let db_port = auth_result.db_instance.port;
    let mut db_stream = match TcpStream::connect(format!("127.0.0.1:{}", db_port)).await {
        Ok(s) => s,
        Err(e) => {
            send_pg_error(&mut client_stream, "08001", &format!("could not connect to database: {}", e)).await?;
            return Ok(());
        }
    };
    
    // Forward startup to real server
    // TODO: Implement proper PostgreSQL proxy with query interception
    
    // For now, send error indicating proxy mode
    send_pg_error(&mut client_stream, "0A000", "Wire protocol proxy is in development. Use Connect-RPC or gRPC for full functionality.").await?;
    
    Ok(())
}

/// PostgreSQL startup message
struct PgStartupMessage {
    version: i32,
    is_ssl_request: bool,
    params: std::collections::HashMap<String, String>,
}

/// Read PostgreSQL startup message
async fn read_pg_startup_message(
    stream: &mut TcpStream,
) -> Result<PgStartupMessage, Box<dyn std::error::Error>> {
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).await?;
    let len = i32::from_be_bytes(len_buf);
    
    if len == 8 {
        // Could be SSL request or cancel request
        let mut code_buf = [0u8; 4];
        stream.read_exact(&mut code_buf).await?;
        let code = i32::from_be_bytes(code_buf);
        
        if code == PG_SSL_REQUEST {
            return Ok(PgStartupMessage {
                version: code,
                is_ssl_request: true,
                params: std::collections::HashMap::new(),
            });
        }
    }
    
    // Regular startup message
    let mut version_buf = [0u8; 4];
    version_buf.copy_from_slice(&len_buf); // First 4 bytes were version
    let version = i32::from_be_bytes(version_buf);
    
    let payload_len = (len - 4) as usize;
    let mut payload = vec![0u8; payload_len];
    stream.read_exact(&mut payload).await?;
    
    // Parse null-terminated key-value pairs
    let mut params = std::collections::HashMap::new();
    let mut pos = 0;
    while pos < payload.len() {
        // Read key
        let mut key = String::new();
        while pos < payload.len() && payload[pos] != 0 {
            key.push(payload[pos] as char);
            pos += 1;
        }
        pos += 1; // Skip null
        
        if key.is_empty() {
            break; // Double null terminator
        }
        
        // Read value
        let mut value = String::new();
        while pos < payload.len() && payload[pos] != 0 {
            value.push(payload[pos] as char);
            pos += 1;
        }
        pos += 1; // Skip null
        
        params.insert(key, value);
    }
    
    Ok(PgStartupMessage {
        version,
        is_ssl_request: false,
        params,
    })
}

/// Read PostgreSQL password message
async fn read_pg_password_message(
    stream: &mut TcpStream,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut type_buf = [0u8; 1];
    stream.read_exact(&mut type_buf).await?;
    
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).await?;
    let len = i32::from_be_bytes(len_buf);
    
    let payload_len = (len - 4) as usize;
    let mut payload = vec![0u8; payload_len];
    stream.read_exact(&mut payload).await?;
    
    // Remove trailing null
    if payload.last() == Some(&0) {
        payload.pop();
    }
    
    Ok(String::from_utf8_lossy(&payload).to_string())
}

/// Send PostgreSQL authentication request
async fn send_pg_auth_request(
    stream: &mut TcpStream,
    auth_type: i32,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut msg = Vec::new();
    msg.push(b'R');
    msg.extend_from_slice(&(8i32.to_be_bytes())); // Length
    msg.extend_from_slice(&auth_type.to_be_bytes());
    
    stream.write_all(&msg).await?;
    stream.flush().await?;
    Ok(())
}

/// Send PostgreSQL authentication OK
async fn send_pg_auth_ok(
    stream: &mut TcpStream,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut msg = Vec::new();
    msg.push(b'R');
    msg.extend_from_slice(&(8i32.to_be_bytes()));
    msg.extend_from_slice(&0i32.to_be_bytes()); // Auth OK
    
    stream.write_all(&msg).await?;
    stream.flush().await?;
    Ok(())
}

/// Send PostgreSQL parameter status
async fn send_pg_parameter_status(
    stream: &mut TcpStream,
    name: &str,
    value: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut payload = Vec::new();
    payload.extend_from_slice(name.as_bytes());
    payload.push(0);
    payload.extend_from_slice(value.as_bytes());
    payload.push(0);
    
    let mut msg = Vec::new();
    msg.push(b'S');
    msg.extend_from_slice(&((4 + payload.len()) as i32).to_be_bytes());
    msg.extend_from_slice(&payload);
    
    stream.write_all(&msg).await?;
    Ok(())
}

/// Send PostgreSQL ready for query
async fn send_pg_ready_for_query(
    stream: &mut TcpStream,
    status: u8,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut msg = Vec::new();
    msg.push(b'Z');
    msg.extend_from_slice(&(5i32.to_be_bytes()));
    msg.push(status);
    
    stream.write_all(&msg).await?;
    stream.flush().await?;
    Ok(())
}

/// Send PostgreSQL error response
async fn send_pg_error(
    stream: &mut TcpStream,
    sqlstate: &str,
    message: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut payload = Vec::new();
    payload.push(b'S'); // Severity
    payload.extend_from_slice(b"ERROR");
    payload.push(0);
    payload.push(b'C'); // Code
    payload.extend_from_slice(sqlstate.as_bytes());
    payload.push(0);
    payload.push(b'M'); // Message
    payload.extend_from_slice(message.as_bytes());
    payload.push(0);
    payload.push(0); // Terminator
    
    let mut msg = Vec::new();
    msg.push(b'E');
    msg.extend_from_slice(&((4 + payload.len()) as i32).to_be_bytes());
    msg.extend_from_slice(&payload);
    
    stream.write_all(&msg).await?;
    stream.flush().await?;
    Ok(())
}

/// TODO: Phase 5 - Implement full PostgreSQL proxy with query parsing
/// TODO: Phase 5 - Implement query audit logging for PostgreSQL
/// TODO: Phase 5 - Implement RLS injection for PostgreSQL queries
