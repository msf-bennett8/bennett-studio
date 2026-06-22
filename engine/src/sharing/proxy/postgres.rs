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
    let _database = startup.params.get("database").cloned().unwrap_or_default();
    
    // Extract share code from user (format: bennett_SHARECODE)
    let share_code = if user.starts_with("bennett_") {
        user.strip_prefix("bennett_").unwrap_or(&user).to_string()
    } else {
        user.clone()
    };
    
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
    send_pg_parameter_status(&mut client_stream, "application_name", "bennett-proxy").await?;
    
    // Send ReadyForQuery
    send_pg_ready_for_query(&mut client_stream, 'I').await?; // 'I' = Idle
    
    info!("PostgreSQL wire proxy: authenticated {} for db {}", peer_addr, auth_result.db_instance.name);
    
    // Connect to real PostgreSQL server
    let db_port = auth_result.db_instance.port;
    let db_stream = match TcpStream::connect(format!("127.0.0.1:{}", db_port)).await {
        Ok(s) => s,
        Err(e) => {
            send_pg_error(&mut client_stream, "08001", &format!("could not connect to database: {}", e)).await?;
            return Ok(());
        }
    };
    
    // Start bidirectional proxy with query interception and audit logging
    pg_proxy_bidirectional(client_stream, db_stream, &auth_result, state.audit_service.clone()).await?;
    
    Ok(())
}

/// Bidirectional PostgreSQL proxy with query interception
async fn pg_proxy_bidirectional(
    client: TcpStream,
    db: TcpStream,
    auth: &WireAuthResult,
    audit_service: Option<Arc<crate::audit::AuditService>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (mut client_read, mut client_write) = client.into_split();
    let (mut db_read, mut db_write) = db.into_split();
    
    let rls_filter = auth.validated.rls.clone();
    let permission = auth.validated.permission.clone();
    
    let db_id = auth.db_instance.id.clone();
    let peer_addr = auth.peer_addr;
    let permission = auth.validated.permission.as_str().to_string();
    let share_code = auth.validated.code.clone();
    
    let client_to_db = tokio::spawn(async move {
        let mut buf = Vec::with_capacity(8192);
        let mut packet_buf = Vec::new();
        
        // Audit helper
        let log_query = |sql: &str, success: bool, rows: i64, elapsed_ms: i64| {
            if let Some(ref audit) = audit_service {
                let entry = crate::audit::create_entry(
                    &share_code,
                    &db_id,
                    &peer_addr.to_string(),
                    sql,
                    rows,
                    elapsed_ms,
                    success,
                    &permission,
                );
                let _ = audit.log_query(entry);
            }
        };
        
        loop {
            buf.resize(8192, 0);
            match client_read.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => {
                    packet_buf.extend_from_slice(&buf[..n]);
                    
                    // Parse PostgreSQL messages
                    while packet_buf.len() >= 5 {
                        let msg_type = packet_buf[0];
                        let msg_len = i32::from_be_bytes([
                            packet_buf[1], packet_buf[2], packet_buf[3], packet_buf[4]
                        ]) as usize;
                        
                        if packet_buf.len() < 5 + msg_len - 4 {
                            break; // Need more data
                        }
                        
                        let msg_data = packet_buf.drain(..5 + msg_len - 4).collect::<Vec<_>>();
                        
                        // Intercept Q (Query) messages
                        if msg_type == b'Q' {
                            if let Ok(sql) = std::str::from_utf8(&msg_data[5..msg_data.len() - 1]) {
                                // Validate SQL
                                if let Err(e) = crate::connect_rpc::validate_shared_sql(sql, &permission) {
                                    tracing::warn!("Blocked PostgreSQL query: {}", e);
                                    let _ = send_pg_error_direct(&mut db_write, "42501", &format!("{}", e)).await;
                                    // Send ReadyForQuery to unblock client
                                    let _ = send_pg_ready_direct(&mut db_write, 'I').await;
                                    continue;
                                }
                                
                                // Apply RLS
                                let modified_sql = if let Some(ref rls) = rls_filter {
                                    crate::connect_rpc::apply_rls(sql, Some(rls))
                                } else {
                                    sql.to_string()
                                };
                                
                                if modified_sql != sql {
                                    tracing::debug!("Rewrote PostgreSQL query with RLS");
                                }
                                
                                // Audit log query execution
                                log_query(sql, true, 0, 0);
                                
                                // Rebuild Q message
                                let mut new_msg = Vec::new();
                                new_msg.push(b'Q');
                                let payload = modified_sql.as_bytes();
                                let len = (4 + payload.len() + 1) as i32; // +1 for null terminator
                                new_msg.extend_from_slice(&len.to_be_bytes());
                                new_msg.extend_from_slice(payload);
                                new_msg.push(0);
                                
                                if db_write.write_all(&new_msg).await.is_err() {
                                    break;
                                }
                            } else {
                                if db_write.write_all(&msg_data).await.is_err() {
                                    break;
                                }
                            }
                        } else {
                            // Forward non-query messages
                            if db_write.write_all(&msg_data).await.is_err() {
                                break;
                            }
                        }
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
    
    tokio::select! {
        _ = client_to_db => {},
        _ = db_to_client => {},
    }
    
    info!("PostgreSQL wire proxy closed for {}", auth.peer_addr);
    Ok(())
}

/// Send PostgreSQL error directly on write half
async fn send_pg_error_direct(
    stream: &mut tokio::net::tcp::OwnedWriteHalf,
    sqlstate: &str,
    message: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut payload = Vec::new();
    payload.push(b'S');
    payload.extend_from_slice(b"ERROR");
    payload.push(0);
    payload.push(b'C');
    payload.extend_from_slice(sqlstate.as_bytes());
    payload.push(0);
    payload.push(b'M');
    payload.extend_from_slice(message.as_bytes());
    payload.push(0);
    payload.push(0);
    
    let mut msg = Vec::new();
    msg.push(b'E');
    msg.extend_from_slice(&((4 + payload.len()) as i32).to_be_bytes());
    msg.extend_from_slice(&payload);
    
    stream.write_all(&msg).await?;
    stream.flush().await?;
    Ok(())
}

/// Send PostgreSQL ReadyForQuery directly on write half
async fn send_pg_ready_direct(
    stream: &mut tokio::net::tcp::OwnedWriteHalf,
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

// PostgreSQL wire protocol proxy implementation complete
// Features: query interception, RLS injection, audit logging, bidirectional proxy
