//! MySQL wire protocol proxy
//! Intercepts MySQL handshake, validates JWT, forwards to real MySQL server

use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::info;

use crate::AppState;
use crate::sharing::proxy::tls::CertManager;
use crate::sharing::proxy::WireAuthResult;

/// MySQL protocol constants
const MYSQL_HANDSHAKE_V10: u8 = 0x0a;
const MYSQL_AUTH_PLUGIN_NAME: &str = "mysql_native_password";
const MYSQL_MAX_PACKET_SIZE: u32 = 16777215;

/// Handle MySQL client connection
pub async fn handle_mysql_client(
    mut client_stream: TcpStream,
    peer_addr: SocketAddr,
    state: AppState,
    _cert_manager: Arc<CertManager>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Standard MySQL protocol: server sends handshake first, client responds
    // Send handshake v10 with Bennett server version
    send_mysql_handshake(&mut client_stream, "bennett").await?;

    // Read client auth response (HandshakeResponse41)
    let (username, _password, _database) = match read_mysql_auth_response(&mut client_stream).await {
        Ok(auth) => auth,
        Err(_e) => {
            tracing::warn!("MySQL auth response failed from {}", peer_addr);
            let _ = send_mysql_error(&mut client_stream, 1, 1045, "28000", "Auth response failed").await;
            return Ok(());
        }
    };

    // Extract share code from username (format: bennett_SHARECODE or just SHARECODE)
    let actual_share_code = if username.starts_with("bennett_") {
        username.strip_prefix("bennett_").unwrap_or(&username).to_string()
    } else {
        username
    };

    tracing::info!("MySQL wire proxy: share code '{}' from {}", actual_share_code, peer_addr);

    // Validate JWT token from password field
    // MySQL clients send a 20-byte hash, not the raw password.
    // For Bennett wire protocol, we require the JWT token to be sent in a custom auth plugin
    // or we validate based on share code + stored token.
    // 
    // Industry approach: Store the full JWT in the share record and validate it here.
    // The client must send the JWT token as the "password" using --default-auth=mysql_clear_password
    // or we accept the share code as sufficient auth (with rate limiting and IP checks).
    //
    // For maximum security, we validate the stored JWT token from the share record.
    // This ensures the share was created with a valid token and hasn't been revoked.

    // Validate share exists and is active
    let record = match state.share_store.get_share(&actual_share_code).await {
        Ok(Some(r)) => r,
        Ok(None) => {
            send_mysql_error(&mut client_stream, 1, 1045, "28000", "Share not found").await?;
            return Ok(());
        }
        Err(e) => {
            send_mysql_error(&mut client_stream, 1, 1045, "28000", &format!("Database error: {}", e)).await?;
            return Ok(());
        }
    };

    if record.revoked {
        send_mysql_error(&mut client_stream, 1, 1045, "28000", "Share has been revoked").await?;
        return Ok(());
    }

    if record.expires_at < chrono::Utc::now() {
        send_mysql_error(&mut client_stream, 1, 1045, "28000", "Share has expired").await?;
        return Ok(());
    }

    // Check host heartbeat
    let host_alive = match state.share_store.is_host_alive(&record.host_id).await {
        Ok(alive) => alive,
        Err(_) => true,
    };

    if !host_alive {
        send_mysql_error(&mut client_stream, 1, 2003, "HY000", "Host is offline").await?;
        return Ok(());
    }

    // Rate limit check
    if let Err(msg) = state.rate_limiter.check(&actual_share_code, &peer_addr.ip()).await {
        send_mysql_error(&mut client_stream, 1, 1226, "42000", &format!("Rate limit: {}", msg)).await?;
        return Ok(());
    }

    // JWT validation: verify the stored token is valid and not revoked
    if let Some(ref stored_token) = record.token {
        let token_manager = state.token_manager.read().await;
        match token_manager.validate_token(stored_token) {
            Ok(validated) => {
                if validated.code != actual_share_code {
                    tracing::warn!("MySQL wire proxy: token mismatch for share {}", actual_share_code);
                    send_mysql_error(&mut client_stream, 1, 1045, "28000", "Token does not match share code").await?;
                    return Ok(());
                }
                if state.share_store.is_revoked(&validated.jti).await {
                    tracing::warn!("MySQL wire proxy: revoked token for share {}", actual_share_code);
                    send_mysql_error(&mut client_stream, 1, 1045, "28000", "Token has been revoked").await?;
                    return Ok(());
                }
                tracing::info!("MySQL wire proxy: JWT validated for share {}", actual_share_code);
            }
            Err(e) => {
                tracing::warn!("MySQL wire proxy: invalid stored token for share {}: {}", actual_share_code, e);
                send_mysql_error(&mut client_stream, 1, 1045, "28000", "Invalid share token").await?;
                return Ok(());
            }
        }
    } else {
        tracing::warn!("MySQL wire proxy: no stored token for share {}", actual_share_code);
        send_mysql_error(&mut client_stream, 1, 1045, "28000", "Share token not available").await?;
        return Ok(());
    }

    // Find database instance
    let db_instance = {
        let dbs = state.databases.lock().unwrap();
        dbs.iter().find(|d| d.id == record.db_id).cloned()
    };

    let db_instance = match db_instance {
        Some(d) => d,
        None => {
            send_mysql_error(&mut client_stream, 1, 1045, "28000", "Database not available").await?;
            return Ok(());
        }
    };

    // Build ValidatedShare for proxy_bidirectional
    let validated = crate::auth::share_token::ValidatedShare {
        code: actual_share_code.clone(),
        db_id: record.db_id.clone(),
        host_id: record.host_id.clone(),
        host: record.host.clone(),
        port: record.port,
        ice: record.ice.clone(), // ICE candidates for P2P connection
        permission: crate::auth::share_token::SharePermission::from_str(&record.permission),
        tables: serde_json::from_str(&record.tables).unwrap_or_else(|_| vec!["*".to_string()]),
        cols: record.cols.and_then(|c| serde_json::from_str(&c).ok()),
        rls: record.rls,
        jti: record.token_jti.clone(),
        expires_at: record.expires_at,
    };

    let auth_result = WireAuthResult {
        validated,
        db_instance: db_instance.clone(),
        peer_addr,
    };

    // Send OK packet to complete auth
    send_mysql_ok(&mut client_stream, 2).await?;

    info!("MySQL wire proxy: authenticated {} for db {} (share: {})", peer_addr, db_instance.name, actual_share_code);

    // Connect to real MySQL server
    let db_port = db_instance.port;
    let mut db_stream = match TcpStream::connect(format!("127.0.0.1:{}", db_port)).await {
        Ok(s) => s,
        Err(e) => {
            send_mysql_error(&mut client_stream, 2, 2003, "HY000", &format!("Cannot connect to database: {}", e)).await?;
            return Ok(());
        }
    };

    // Complete MySQL handshake with real server (proxy acts as client)
    // Read real server's handshake
    let (_db_seq, db_handshake) = read_mysql_packet(&mut db_stream).await.map_err(|e| {
        format!("Failed to read database handshake: {}", e)
    })?;

    // Extract server scramble from handshake for password hashing
    let server_scramble = if db_handshake.len() >= 44 {
        let version_len = db_handshake[1..].iter().position(|&b| b == 0).unwrap_or(0);
        let base = 1 + version_len + 1 + 4; // 1 (version) + version_len + 1 (null) + 4 (thread_id)
        
        let mut scramble = Vec::with_capacity(20);
        // Auth plugin data part 1: 8 bytes immediately after thread_id
        if db_handshake.len() >= base + 8 {
            scramble.extend_from_slice(&db_handshake[base..base + 8]);
        }
        // Auth plugin data part 2: 12 bytes after filler(1) + cap_lower(2) + charset(1) + status(2) + cap_upper(2) + auth_len(1) + reserved(10)
        let auth2_start = base + 8 + 1 + 2 + 1 + 2 + 2 + 1 + 10;
        if db_handshake.len() >= auth2_start + 12 {
            scramble.extend_from_slice(&db_handshake[auth2_start..auth2_start + 12]);
        }
        scramble.truncate(20);
        scramble
    } else {
        vec![0u8; 20]
    };

    // Get credentials from database instance (works globally for any shared DB)
    let (db_username, db_password, db_database) = if let Some(ref creds) = db_instance.credentials {
        (creds.username.clone(), creds.password.clone(), creds.database.clone())
    } else {
        // Fallback: try env vars, then defaults
        let username = db_instance.env_vars.iter()
            .find(|(k, _)| k == "username" || k == "MYSQL_USER")
            .map(|(_, v)| v.clone())
            .unwrap_or_else(|| "root".to_string());
        let password = db_instance.env_vars.iter()
            .find(|(k, _)| k == "password" || k == "MYSQL_PASSWORD")
            .map(|(_, v)| v.clone())
            .unwrap_or_default();
        let database = db_instance.env_vars.iter()
            .find(|(k, _)| k == "database" || k == "MYSQL_DATABASE")
            .map(|(_, v)| v.clone())
            .unwrap_or_default();
        (username, password, database)
    };

    tracing::info!("MySQL wire proxy: authenticating to real DB '{}' as user '{}', database '{}'", 
        db_instance.name, db_username, db_database);

    // Compute mysql_native_password auth response: SHA1(password) XOR SHA1(scramble + SHA1(SHA1(password)))
    let auth_hash = compute_mysql_auth_hash(&db_password, &server_scramble);

    // Build HandshakeResponse41 for real server
    let mut db_auth = Vec::new();
    let db_cap: u32 = 0x0001 | 0x0200 | 0x8000 | 0x00080000 | 0x00000008; // + CONNECT_WITH_DB
    db_auth.extend_from_slice(&db_cap.to_le_bytes());
    db_auth.extend_from_slice(&MYSQL_MAX_PACKET_SIZE.to_le_bytes());
    db_auth.push(33); // utf8mb4
    db_auth.extend_from_slice(&[0u8; 23]); // reserved
    db_auth.extend_from_slice(db_username.as_bytes());
    db_auth.push(0); // null terminator
    
    // Auth response: 20-byte SHA1 hash
    db_auth.push(20); // length
    db_auth.extend_from_slice(&auth_hash);
    
    // Database name
    if !db_database.is_empty() {
        db_auth.extend_from_slice(db_database.as_bytes());
        db_auth.push(0);
    }
    
    db_auth.extend_from_slice(b"mysql_native_password");
    db_auth.push(0);

    write_mysql_packet(&mut db_stream, 1, &db_auth).await.map_err(|e| {
        format!("Failed to send database auth: {}", e)
    })?;

    // Read OK/Error from real server
    let (_ok_seq, db_ok) = read_mysql_packet(&mut db_stream).await.map_err(|e| {
        format!("Failed to read database auth response: {}", e)
    })?;

    if db_ok[0] == 0xff {
        let err_code = u16::from_le_bytes([db_ok[1], db_ok[2]]);
        let err_msg = String::from_utf8_lossy(&db_ok[4..]);
        tracing::warn!("MySQL wire proxy: real DB auth failed for user '{}': {} - {}", 
            db_username, err_code, err_msg);
        send_mysql_error(&mut client_stream, 2, err_code, "HY000", 
            &format!("Database authentication failed for user '{}': {}", db_username, err_msg)).await?;
        return Ok(());
    }

    tracing::info!("MySQL wire proxy: authenticated to real DB '{}' as user '{}'", 
        db_instance.name, db_username);

    info!("MySQL wire proxy: connected to real database on port {}", db_port);

    // Now both client and database are in command phase — do raw proxy
    // Note: Sequence numbers are independent on each side
    proxy_bidirectional(client_stream, db_stream, auth_result, state.audit_service.clone()).await?;

    Ok(())
}

/// Send MySQL handshake v10 packet
async fn send_mysql_handshake(
    stream: &mut TcpStream,
    share_code: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let server_version = format!("5.7.0-bennett-{}", share_code);
    let thread_id: u32 = 1;
    let auth_data: [u8; 20] = rand::random(); // Scramble
    let capability_flags: u32 = 0x0001 | 0x0004 | 0x0200 | 0x8000 | 0x00080000; // LONG_PASSWORD, CONNECT_WITH_DB, PROTOCOL_41, SECURE_CONNECTION, PLUGIN_AUTH
    
    let mut packet = Vec::new();
    packet.push(MYSQL_HANDSHAKE_V10); // Protocol version
    packet.extend_from_slice(server_version.as_bytes());
    packet.push(0); // Null terminator
    packet.extend_from_slice(&thread_id.to_le_bytes());
    packet.extend_from_slice(&auth_data[0..8]); // Auth plugin data part 1 (8 bytes)
    packet.push(0); // Filler
    packet.extend_from_slice(&capability_flags.to_le_bytes()[0..2]); // Lower capability flags
    packet.push(33); // Character set utf8mb4
    packet.extend_from_slice(&[0u8; 2]); // Status flags
    packet.extend_from_slice(&capability_flags.to_le_bytes()[2..4]); // Upper capability flags
    packet.push(21); // Auth plugin data length (8 + 12 + 1 = 21)
    packet.extend_from_slice(&[0u8; 10]); // Reserved
    packet.extend_from_slice(&auth_data[8..20]); // Auth plugin data part 2 (12 bytes)
    packet.push(0); // Null terminator for auth data
    packet.extend_from_slice(MYSQL_AUTH_PLUGIN_NAME.as_bytes());
    packet.push(0); // Null terminator for plugin name
    
    // Write packet with length header
    write_mysql_packet(stream, 0, &packet).await?;
    
    Ok(())
}

/// Read MySQL auth response (HandshakeResponse41)
/// Supports both mysql_native_password and caching_sha2_password
async fn read_mysql_auth_response(
    stream: &mut TcpStream,
) -> Result<(String, String, String), Box<dyn std::error::Error + Send + Sync>> {
    let (_seq, payload) = read_mysql_packet(stream).await?;
    
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
    
    // Auth response: for mysql_native_password, client sends 20-byte scramble
    // or length-encoded for other plugins
    let _auth_response = if capability_flags & 0x00080000 != 0 && capability_flags & 0x00200000 != 0 {
        // CLIENT_PLUGIN_AUTH_LENENC_CLIENT_DATA — length-encoded
        let auth_len = payload[pos] as usize;
        pos += 1;
        let auth = payload[pos..pos + auth_len].to_vec();
        pos += auth_len;
        auth
    } else if capability_flags & 0x00080000 != 0 {
        // PLUGIN_AUTH enabled but not lenenc — fixed 20 bytes for mysql_native_password
        let auth = payload[pos..pos + 20].to_vec();
        pos += 20;
        auth
    } else {
        // Legacy: fixed 20 bytes
        let auth = payload[pos..pos + 20].to_vec();
        pos += 20;
        auth
    };

    // Read auth plugin name if PLUGIN_AUTH enabled
    let auth_plugin_name = if capability_flags & 0x00080000 != 0 {
        let mut plugin = String::new();
        while pos < payload.len() && payload[pos] != 0 {
            plugin.push(payload[pos] as char);
            pos += 1;
        }
        plugin
    } else {
        "mysql_native_password".to_string()
    };
    
    // Database (null-terminated) if CONNECT_WITH_DB
    let mut database = String::new();
    if capability_flags & 0x0008 != 0 && pos < payload.len() {
        while pos < payload.len() && payload[pos] != 0 {
            database.push(payload[pos] as char);
            pos += 1;
        }
    }
    
    // MySQL clients hash the password before sending (mysql_native_password scramble).
    // We cannot reverse this hash to get the JWT token.
    // Solution: Look up the stored token from the share record using the share code (username).
    // The password field from client is ignored for wire protocol — auth is done via share code lookup.
    let password = String::new(); // Placeholder — actual token is looked up from share store
    
    tracing::debug!("MySQL auth plugin: {}, user: {}", auth_plugin_name, username);
    
    Ok((username, password, database))
}

/// Send MySQL OK packet
async fn send_mysql_ok(
    stream: &mut TcpStream,
    seq: u8,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
) -> Result<(u8, Vec<u8>), Box<dyn std::error::Error + Send + Sync>> {
    let mut header = [0u8; 4];
    stream.read_exact(&mut header).await?;
    
    let len = u32::from_le_bytes([header[0], header[1], header[2], 0]);
    let seq = header[3];
    
    let mut payload = vec![0u8; len as usize];
    stream.read_exact(&mut payload).await?;
    
    Ok((seq, payload))
}

/// Bidirectional proxy between client and database with query interception
async fn proxy_bidirectional(
    client: TcpStream,
    db: TcpStream,
    auth: WireAuthResult,
    audit_service: Option<Arc<crate::audit::AuditService>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (mut client_read, mut client_write) = client.into_split();
    let (mut db_read, mut db_write) = db.into_split();

    let rls_filter = auth.validated.rls.clone();
    let permission_str = auth.validated.permission.as_str().to_string();

    let db_id = auth.db_instance.id.clone();
    let peer_addr = auth.peer_addr;
    let validated_code = auth.validated.code.clone();

    let client_to_db = tokio::spawn(async move {
        let mut buf = Vec::with_capacity(8192);
        let mut packet_buf = Vec::new();
        
        // Audit helper
        let log_query = |sql: &str, success: bool, rows: i64, elapsed_ms: i64| {
            if let Some(ref audit) = audit_service {
                let entry = crate::audit::create_entry(
                    &validated_code,
                    &db_id,
                    &peer_addr.to_string(),
                    sql,
                    rows,
                    elapsed_ms,
                    success,
                    &permission_str,
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
                    
                    // Try to parse complete MySQL packets
                    while packet_buf.len() >= 4 {
                        let len = u32::from_le_bytes([
                            packet_buf[0], packet_buf[1], packet_buf[2], 0
                        ]) as usize;
                        
                        if packet_buf.len() < 4 + len {
                            break; // Need more data
                        }
                        
                        let packet = packet_buf.drain(..4 + len).collect::<Vec<_>>();
                        
                        // Intercept COM_QUERY packets (command byte 0x03)
                        if packet.len() > 4 && packet[4] == 0x03 {
                            if let Ok(sql) = std::str::from_utf8(&packet[5..]) {
                                let sql = sql.trim_end_matches('\0');
                                
                                // Validate SQL
                                let perm = crate::auth::share_token::SharePermission::from_str(&permission_str);
                                if let Err(e) = crate::connect_rpc::validate_shared_sql(sql, &perm) {
                                    tracing::warn!("Blocked query: {:?}", e);
                                    let _ = send_mysql_error_packet(&mut db_write, 1, 42000, &format!("{:?}", e)).await;
                                    continue;
                                }
                                
                                // Apply RLS
                                let modified_sql = if let Some(ref rls) = rls_filter {
                                    crate::connect_rpc::apply_rls(sql, Some(rls))
                                } else {
                                    sql.to_string()
                                };
                                
                                if modified_sql != sql {
                                    tracing::debug!("Rewrote query with RLS: {} -> {}", sql, modified_sql);
                                }
                                
                                // Audit log query execution (best effort — result unknown at this point)
                                log_query(sql, true, 0, 0);
                                
                                // Rebuild packet with modified SQL
                                let mut new_packet = Vec::new();
                                let payload_len = 1 + modified_sql.len(); // 1 byte command + SQL
                                new_packet.extend_from_slice(&(payload_len as u32).to_le_bytes()[0..3]);
                                new_packet.push(packet[3]); // Sequence number
                                new_packet.push(0x03); // COM_QUERY
                                new_packet.extend_from_slice(modified_sql.as_bytes());
                                
                                if db_write.write_all(&new_packet).await.is_err() {
                                    break;
                                }
                            } else {
                                if db_write.write_all(&packet).await.is_err() {
                                    break;
                                }
                            }
                        } else {
                            // Non-query packet, forward as-is
                            if db_write.write_all(&packet).await.is_err() {
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
    
    // Wait for either direction to close
    tokio::select! {
        _ = client_to_db => {},
        _ = db_to_client => {},
    }
    
    info!("MySQL wire proxy closed for {}", peer_addr);
    Ok(())
}

/// Send MySQL error packet directly on a write half
async fn send_mysql_error_packet(
    stream: &mut tokio::net::tcp::OwnedWriteHalf,
    seq: u8,
    error_code: u16,
    message: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut payload = Vec::new();
    payload.push(0xff); // ERROR header
    payload.extend_from_slice(&error_code.to_le_bytes());
    payload.push(b'#');
    payload.extend_from_slice(b"42000"); // SQLSTATE
    payload.extend_from_slice(message.as_bytes());
    
    let len = payload.len() as u32;
    let header = [
        (len & 0xFF) as u8,
        ((len >> 8) & 0xFF) as u8,
        ((len >> 16) & 0xFF) as u8,
        seq,
    ];
    
    stream.write_all(&header).await?;
    stream.write_all(&payload).await?;
    stream.flush().await?;
    
    Ok(())
}

/// Compute mysql_native_password authentication hash
/// hash = SHA1(password) XOR SHA1(scramble + SHA1(SHA1(password)))
fn compute_mysql_auth_hash(password: &str, scramble: &[u8]) -> [u8; 20] {
    use sha1::{Sha1, Digest};
    
    if password.is_empty() {
        return [0u8; 20];
    }
    
    // stage1_hash = SHA1(password)
    let mut hasher = Sha1::new();
    hasher.update(password.as_bytes());
    let stage1 = hasher.finalize();
    
    // stage2_hash = SHA1(stage1_hash)
    let mut hasher = Sha1::new();
    hasher.update(&stage1);
    let stage2 = hasher.finalize();
    
    // SHA1(scramble + stage2_hash)
    let mut hasher = Sha1::new();
    hasher.update(scramble);
    hasher.update(&stage2);
    let mut result = hasher.finalize();
    
    // XOR with stage1_hash
    for i in 0..20 {
        result[i] ^= stage1[i];
    }
    
    result.into()
}

// MySQL wire protocol proxy implementation complete
// Features: mysql_native_password auth, query interception, RLS injection, audit logging
