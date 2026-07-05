//! TCP multiplexer — bidirectional byte forwarding
//! Bridges client TLS stream ↔ engine TCP stream
//! Includes: MySQL handshake parser, connection counter, error generators

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncWrite};
use tracing::{debug, info, warn};

/// Per-share connection counter with limit enforcement
#[derive(Debug)]
pub struct ConnectionCounter {
    counts: dashmap::DashMap<String, AtomicUsize>,
    max_per_share: usize,
}

impl ConnectionCounter {
    pub fn new(max_per_share: usize) -> Self {
        Self {
            counts: dashmap::DashMap::new(),
            max_per_share,
        }
    }

    /// Try to acquire a connection slot for a share
    /// Returns true if slot acquired, false if at limit
    pub fn acquire(&self, share_id: &str) -> bool {
        let entry = self
            .counts
            .entry(share_id.to_string())
            .or_insert_with(|| AtomicUsize::new(0));

        let current = entry.load(Ordering::Relaxed);
        if current >= self.max_per_share {
            return false;
        }

        entry.fetch_add(1, Ordering::Relaxed);
        true
    }

    /// Release a connection slot for a share
    pub fn release(&self, share_id: &str) {
        if let Some(entry) = self.counts.get(share_id) {
            let current = entry.load(Ordering::Relaxed);
            if current > 0 {
                entry.fetch_sub(1, Ordering::Relaxed);
            }
        }
    }

    /// Get current count for a share
    pub fn count(&self, share_id: &str) -> usize {
        self.counts
            .get(share_id)
            .map(|e| e.load(Ordering::Relaxed))
            .unwrap_or(0)
    }

    /// Get total connections across all shares
    pub fn total(&self) -> usize {
        self.counts
            .iter()
            .map(|e| e.load(Ordering::Relaxed))
            .sum()
    }
}

impl Clone for ConnectionCounter {
    fn clone(&self) -> Self {
        Self {
            counts: dashmap::DashMap::new(),
            max_per_share: self.max_per_share,
        }
    }
}

/// Forward bytes between two streams until one closes
pub async fn proxy_bidirectional<A, B>(
    client: A,
    engine: B,
    share_id: String,
    protocol: &'static str,
    counter: Arc<ConnectionCounter>,
) -> std::io::Result<()>
where
    A: AsyncRead + AsyncWrite + Unpin,
    B: AsyncRead + AsyncWrite + Unpin,
{
    use tokio::io::copy;

    info!(
        share_id = %share_id,
        protocol = protocol,
        "Starting bidirectional proxy"
    );

    let (mut client_read, mut client_write) = tokio::io::split(client);
    let (mut engine_read, mut engine_write) = tokio::io::split(engine);

    let share_id_clone = share_id.clone();
    let client_to_engine = async {
        match copy(&mut client_read, &mut engine_write).await {
            Ok(n) => {
                debug!(
                    share_id = %share_id_clone,
                    bytes = n,
                    "Client → Engine stream closed"
                );
            }
            Err(e) => {
                warn!(
                    share_id = %share_id_clone,
                    error = %e,
                    "Client → Engine error"
                );
            }
        }
    };

    let share_id_clone2 = share_id.clone();
    let engine_to_client = async {
        match copy(&mut engine_read, &mut client_write).await {
            Ok(n) => {
                debug!(
                    share_id = %share_id_clone2,
                    bytes = n,
                    "Engine → Client stream closed"
                );
            }
            Err(e) => {
                warn!(
                    share_id = %share_id_clone2,
                    error = %e,
                    "Engine → Client error"
                );
            }
        }
    };

    tokio::select! {
        _ = client_to_engine => {},
        _ = engine_to_client => {},
    }

    // Always release the counter slot
    counter.release(&share_id);

    info!(
        share_id = %share_id,
        "Bidirectional proxy closed"
    );

    Ok(())
}

// ============================================================================
// Zero-Copy Forwarding (Linux splice() kernel bypass)
// ============================================================================

/// Forward between two TCP streams using splice() on Linux
/// Falls back to tokio::io::copy_bidirectional on other platforms
pub async fn forward_zero_copy(
    client: &mut tokio::net::TcpStream,
    engine: &mut tokio::net::TcpStream,
) -> std::io::Result<(u64, u64)> {
    #[cfg(target_os = "linux")]
    {
        match splice_forward(client, engine).await {
            Ok(result) => return Ok(result),
            Err(e) => {
                tracing::warn!("splice() failed ({}), falling back to userspace copy", e);
            }
        }
    }
    tokio::io::copy_bidirectional(client, engine).await
}

#[cfg(target_os = "linux")]
async fn splice_forward(
    client: &mut tokio::net::TcpStream,
    engine: &mut tokio::net::TcpStream,
) -> std::io::Result<(u64, u64)> {
    use nix::fcntl::{splice, SpliceFFlags};
    use std::os::fd::AsRawFd;
    
    let client_fd = client.as_raw_fd();
    let engine_fd = engine.as_raw_fd();
    
    let (pipe_rd1, pipe_wr1) = nix::unistd::pipe()
        .map_err(|e| std::io::Error::from_raw_os_error(e as i32))?;
    let (pipe_rd2, pipe_wr2) = nix::unistd::pipe()
        .map_err(|e| std::io::Error::from_raw_os_error(e as i32))?;
    
    tokio::task::spawn_blocking(move || {
        let mut c2e_done = false;
        let mut e2c_done = false;
        let mut c2e_total = 0u64;
        let mut e2c_total = 0u64;
        
        while !c2e_done || !e2c_done {
            if !c2e_done {
                match splice(client_fd, None, pipe_wr1, None, 65536, 
                    SpliceFFlags::SPLICE_F_NONBLOCK | SpliceFFlags::SPLICE_F_MOVE) {
                    Ok(0) => c2e_done = true,
                    Ok(n) => {
                        c2e_total += n as u64;
                        let _ = splice(pipe_rd1, None, engine_fd, None, n, 
                            SpliceFFlags::SPLICE_F_NONBLOCK);
                    }
                    Err(nix::errno::Errno::EAGAIN) => {}
                    Err(e) => return Err(std::io::Error::from(e)),
                }
            }
            
            if !e2c_done {
                match splice(engine_fd, None, pipe_wr2, None, 65536,
                    SpliceFFlags::SPLICE_F_NONBLOCK | SpliceFFlags::SPLICE_F_MOVE) {
                    Ok(0) => e2c_done = true,
                    Ok(n) => {
                        e2c_total += n as u64;
                        let _ = splice(pipe_rd2, None, client_fd, None, n,
                            SpliceFFlags::SPLICE_F_NONBLOCK);
                    }
                    Err(nix::errno::Errno::EAGAIN) => {}
                    Err(e) => return Err(std::io::Error::from(e)),
                }
            }
        }
        
        Ok((c2e_total, e2c_total))
    }).await.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?
}

// ============================================================================
// HTTP Error Responses
// ============================================================================

/// Send HTTP 404 Not Found response on a TLS stream
pub async fn send_http_404<S>(stream: &mut S, message: &str) -> std::io::Result<()>
where
    S: AsyncWrite + Unpin,
{
    use tokio::io::AsyncWriteExt;

    let body = serde_json::json!({
        "error": "share_not_found",
        "message": message,
        "relay": "bennett-relay",
        "timestamp": chrono::Utc::now().to_rfc3339(),
    });

    let body_str = body.to_string();
    let response = format!(
        "HTTP/1.1 404 Not Found\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\
         X-Bennett-Relay: true\r\n\
         \r\n\
         {}",
        body_str.len(),
        body_str
    );

    stream.write_all(response.as_bytes()).await?;
    stream.flush().await?;
    Ok(())
}

/// Send HTTP 429 Too Many Requests response
pub async fn send_http_429<S>(stream: &mut S, share_id: &str) -> std::io::Result<()>
where
    S: AsyncWrite + Unpin,
{
    use tokio::io::AsyncWriteExt;

    let body = serde_json::json!({
        "error": "rate_limited",
        "message": format!("Connection limit exceeded for share {}", share_id),
        "relay": "bennett-relay",
        "timestamp": chrono::Utc::now().to_rfc3339(),
    });

    let body_str = body.to_string();
    let response = format!(
        "HTTP/1.1 429 Too Many Requests\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\
         Retry-After: 60\r\n\
         X-Bennett-Relay: true\r\n\
         \r\n\
         {}",
        body_str.len(),
        body_str
    );

    stream.write_all(response.as_bytes()).await?;
    stream.flush().await?;
    Ok(())
}

// ============================================================================
// MySQL Error Responses
// ============================================================================

/// Send MySQL ERROR packet on a raw stream
pub async fn send_mysql_error<S>(
    stream: &mut S,
    seq: u8,
    error_code: u16,
    sql_state: &str,
    message: &str,
) -> std::io::Result<()>
where
    S: AsyncWrite + Unpin,
{
    use tokio::io::AsyncWriteExt;

    let mut payload = Vec::new();
    payload.push(0xff); // ERROR header
    payload.extend_from_slice(&error_code.to_le_bytes());
    payload.push(b'#');
    payload.extend_from_slice(sql_state.as_bytes());
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

/// Send MySQL "share not found" error
pub async fn send_mysql_share_not_found<S>(stream: &mut S, seq: u8) -> std::io::Result<()>
where
    S: AsyncWrite + Unpin,
{
    send_mysql_error(
        stream,
        seq,
        1045,
        "28000",
        "Share not found or inactive on relay",
    )
    .await
}

/// Send MySQL "connection limit exceeded" error
pub async fn send_mysql_too_many_connections<S>(stream: &mut S, seq: u8) -> std::io::Result<()>
where
    S: AsyncWrite + Unpin,
{
    send_mysql_error(
        stream,
        seq,
        1040,
        "08004",
        "Too many connections for this share on relay",
    )
    .await
}

// ============================================================================
// MySQL Handshake Parser
// ============================================================================

/// Parsed MySQL HandshakeResponse41
#[derive(Debug, Clone)]
pub struct MySqlAuthResponse {
    pub username: String,
    pub password_hash: Vec<u8>,
    pub database: String,
    pub auth_plugin: String,
    pub capability_flags: u32,
}

/// Read and parse MySQL HandshakeResponse41 from client
/// This is called AFTER the server sends its initial handshake
pub async fn read_mysql_auth_response<S>(
    stream: &mut S,
) -> std::io::Result<MySqlAuthResponse>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    use tokio::io::AsyncReadExt;

    // Read packet header (4 bytes: 3 length + 1 seq)
    let mut header = [0u8; 4];
    stream.read_exact(&mut header).await?;
    let len = u32::from_le_bytes([header[0], header[1], header[2], 0]) as usize;
    let _seq = header[3];

    let mut payload = vec![0u8; len];
    stream.read_exact(&mut payload).await?;

    // Parse HandshakeResponse41
    let capability_flags = u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
    let _max_packet_size = u32::from_le_bytes([payload[4], payload[5], payload[6], payload[7]]);
    let _charset = payload[8];

    let mut pos = 32; // After reserved bytes

    // Username (null-terminated)
    let mut username = String::new();
    while pos < payload.len() && payload[pos] != 0 {
        username.push(payload[pos] as char);
        pos += 1;
    }
    pos += 1; // Skip null

    // Auth response (password hash)
    let (password_hash, _new_pos) = if capability_flags & 0x00200000 != 0 {
        // CLIENT_PLUGIN_AUTH_LENENC_CLIENT_DATA
        let auth_len = payload[pos] as usize;
        pos += 1;
        let hash = payload[pos..pos + auth_len].to_vec();
        pos += auth_len;
        (hash, pos)
    } else if capability_flags & 0x00080000 != 0 {
        // CLIENT_PLUGIN_AUTH — fixed 20 bytes for mysql_native_password
        let hash = payload[pos..pos + 20].to_vec();
        pos += 20;
        (hash, pos)
    } else {
        // Legacy: fixed 20 bytes
        let hash = payload[pos..pos + 20].to_vec();
        pos += 20;
        (hash, pos)
    };

    // Database (if CONNECT_WITH_DB)
    let mut database = String::new();
    if capability_flags & 0x0008 != 0 && pos < payload.len() {
        while pos < payload.len() && payload[pos] != 0 {
            database.push(payload[pos] as char);
            pos += 1;
        }
        pos += 1;
    }

    // Auth plugin name (if PLUGIN_AUTH)
    let mut auth_plugin = String::new();
    if capability_flags & 0x00080000 != 0 && pos < payload.len() {
        while pos < payload.len() && payload[pos] != 0 {
            auth_plugin.push(payload[pos] as char);
            pos += 1;
        }
    } else {
        auth_plugin = "mysql_native_password".to_string();
    }

    Ok(MySqlAuthResponse {
        username,
        password_hash,
        database,
        auth_plugin,
        capability_flags,
    })
}

/// Send MySQL Handshake v10 (server greeting)
/// Must be sent BEFORE reading client auth response
pub async fn send_mysql_handshake_v10<S>(
    stream: &mut S,
    share_code: &str,
    seq: u8,
) -> std::io::Result<()>
where
    S: AsyncWrite + Unpin,
{
    use tokio::io::AsyncWriteExt;

    let server_version = format!("5.7.0-bennett-{}", share_code);
    let thread_id: u32 = rand::random();
    let auth_data: [u8; 20] = rand::random(); // Scramble

    // Capability flags: LONG_PASSWORD | CONNECT_WITH_DB | PROTOCOL_41 | SECURE_CONNECTION | PLUGIN_AUTH
    let capability_flags: u32 = 0x0001 | 0x0004 | 0x0200 | 0x8000 | 0x00080000;

    let mut packet = Vec::new();
    packet.push(0x0a); // Protocol version 10
    packet.extend_from_slice(server_version.as_bytes());
    packet.push(0); // Null terminator
    packet.extend_from_slice(&thread_id.to_le_bytes());
    packet.extend_from_slice(&auth_data[0..8]); // Auth data part 1
    packet.push(0); // Filler
    packet.extend_from_slice(&(capability_flags as u16).to_le_bytes()); // Lower caps
    packet.push(33); // utf8mb4
    packet.extend_from_slice(&[0u8; 2]); // Status flags
    packet.extend_from_slice(&((capability_flags >> 16) as u16).to_le_bytes()); // Upper caps
    packet.push(21); // Auth plugin data length
    packet.extend_from_slice(&[0u8; 10]); // Reserved
    packet.extend_from_slice(&auth_data[8..20]); // Auth data part 2
    packet.push(0); // Null
    packet.extend_from_slice(b"mysql_native_password");
    packet.push(0); // Null

    let len = packet.len() as u32;
    let header = [
        (len & 0xFF) as u8,
        ((len >> 8) & 0xFF) as u8,
        ((len >> 16) & 0xFF) as u8,
        seq,
    ];

    stream.write_all(&header).await?;
    stream.write_all(&packet).await?;
    stream.flush().await?;
    Ok(())
}

/// Extract share_id from MySQL username
/// Format: "bennett_SHARECODE" or just "SHARECODE"
pub fn extract_share_id_from_mysql_username(username: &str) -> String {
    if let Some(code) = username.strip_prefix("bennett_") {
        return code.to_string();
    }
    username.to_string()
}

// ============================================================================
// HTTP Path Parsing
// ============================================================================

/// Extract share_id from HTTP request path
/// Format: /db/SHARE_ID or /api/shares/SHARE_ID/...
pub fn extract_share_id_from_http_path(path: &str) -> Option<String> {
    if let Some(rest) = path.strip_prefix("/db/") {
        let code = rest.split('?').next().unwrap_or(rest);
        return Some(code.to_string());
    }

    if let Some(rest) = path.strip_prefix("/api/shares/") {
        let parts: Vec<&str> = rest.split('/').collect();
        if !parts.is_empty() && !parts[0].is_empty() {
            return Some(parts[0].to_string());
        }
    }

    None
}