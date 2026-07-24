//! Public MySQL wire-protocol listener (Phase 3).
//! A real MySQL client (mysql CLI, Laravel/PDO, etc.) connects here with a
//! normal connection string:
//!   DB_HOST=bennett-relay.onrender.com  DB_PORT=<this port>
//!   DB_USERNAME=bennett_<name>          DB_PASSWORD=<wire password>
//!
//! Auth uses mysql_clear_password over mandatory TLS — the only way to
//! receive the real plaintext password (mysql_native_password sends an
//! irreversible SHA1 scramble instead). Connections that don't upgrade to
//! TLS are refused before any credential is read.
//!
//! Once authenticated, this becomes a byte-for-byte proxy: bytes are framed
//! (see wire_frame.rs) and sent over the engine's existing tunnel as raw
//! binary WebSocket frames — no JSON/base64 in the data path.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, oneshot};
use tokio_rustls::TlsAcceptor;
use tracing::{info, warn, debug};
use sha2::{Digest, Sha256};

use crate::wire_frame::encode_wire_frame;
use crate::wire_registry::WireCredentialRegistry;
use crate::tunnel_registry::{TunnelRegistry, TunnelMessageToEngine};

const CAP_LONG_PASSWORD: u32 = 0x0001;
const CAP_CONNECT_WITH_DB: u32 = 0x0008;
const CAP_PROTOCOL_41: u32 = 0x0200;
const CAP_SSL: u32 = 0x0800;
const CAP_SECURE_CONNECTION: u32 = 0x8000;
const CAP_PLUGIN_AUTH: u32 = 0x00080000;

pub struct WireListenerState {
    pub tunnel_registry: Arc<TunnelRegistry>,
    pub wire_credentials: Arc<WireCredentialRegistry>,
}

pub async fn start_wire_listener(
    bind_addr: SocketAddr,
    tls_acceptor: TlsAcceptor,
    state: Arc<WireListenerState>,
) -> anyhow::Result<()> {
    let listener = TcpListener::bind(bind_addr).await?;
    info!("MySQL wire-protocol listener on {}", bind_addr);

    loop {
        let (stream, peer_addr) = listener.accept().await?;
        let tls_acceptor = tls_acceptor.clone();
        let state = state.clone();

        tokio::spawn(async move {
            if let Err(e) = handle_client(stream, peer_addr, tls_acceptor, state).await {
                warn!("Wire client {} error: {}", peer_addr, e);
            }
        });
    }
}

fn hash_wire_password(password: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    let digest: [u8; 32] = hasher.finalize().into();
    digest.iter().map(|b| format!("{:02x}", b)).collect()
}

async fn handle_client(
    mut client: TcpStream,
    peer_addr: SocketAddr,
    tls_acceptor: TlsAcceptor,
    state: Arc<WireListenerState>,
) -> anyhow::Result<()> {
    // 1. Send server greeting advertising CLIENT_SSL + mysql_clear_password.
    //    Real clients that support SSL will respond with a short SSLRequest
    //    packet (capability flags only, no username) before the TLS handshake.
    send_handshake_v10(&mut client, 0).await?;

    let ssl_request = read_packet(&mut client).await?;
    let caps = if ssl_request.1.len() >= 4 {
        u32::from_le_bytes([ssl_request.1[0], ssl_request.1[1], ssl_request.1[2], ssl_request.1[3]])
    } else {
        0
    };

    if caps & CAP_SSL == 0 {
        // Client didn't request TLS — refuse. Clear-password auth over
        // plaintext would leak the credential.
        send_error(&mut client, 1, 1045, "28000", "SSL connection required for Bennett wire access").await?;
        return Ok(());
    }

    // 2. Upgrade to TLS. All subsequent bytes (the real HandshakeResponse41,
    //    then normal query/response traffic) travel encrypted.
    let mut tls_stream = tls_acceptor.accept(client).await?;

    // 3. Read the real HandshakeResponse41 over the now-encrypted stream.
    let (seq, auth) = read_packet(&mut tls_stream).await?;
    let parsed = match parse_handshake_response(&auth) {
        Some(p) => p,
        None => {
            send_error(&mut tls_stream, seq + 1, 1045, "28000", "Malformed handshake response").await?;
            return Ok(());
        }
    };

    // mysql_clear_password sends the plaintext password as a null-terminated
    // (or lenenc, per parse_handshake_response) byte string — not a hash.
    let plaintext_password = String::from_utf8_lossy(&parsed.auth_response).trim_end_matches('\0').to_string();
    let wire_password_hash = hash_wire_password(&plaintext_password);

    let host_id = match state.wire_credentials.resolve(&wire_password_hash) {
        Some(h) => h,
        None => {
            warn!("Wire auth failed for user '{}' from {}", parsed.username, peer_addr);
            send_error(&mut tls_stream, seq + 1, 1045, "28000", "Access denied").await?;
            return Ok(());
        }
    };

    // 4. Ask the engine to open a real DB connection for this stream.
    let stream_id = uuid::Uuid::new_v4().to_string();
    let (open_tx, open_rx) = oneshot::channel::<Result<(), String>>();
    // Registered before sending WireStreamOpen so the ack can never race ahead of us.
    register_pending_open(&state, &stream_id, open_tx).await;

    if let Err(e) = state.tunnel_registry.send_wire_control(&host_id, TunnelMessageToEngine::WireStreamOpen {
        stream_id: stream_id.clone(),
        wire_username: parsed.username.clone(),
        wire_password_hash: wire_password_hash.clone(),
    }).await {
        send_error(&mut tls_stream, seq + 1, 2003, "HY000", &format!("Engine unreachable: {}", e)).await?;
        return Ok(());
    }

    match tokio::time::timeout(Duration::from_secs(10), open_rx).await {
        Ok(Ok(Ok(()))) => {}
        Ok(Ok(Err(msg))) => {
            send_error(&mut tls_stream, seq + 1, 1045, "28000", &msg).await?;
            return Ok(());
        }
        _ => {
            send_error(&mut tls_stream, seq + 1, 2003, "HY000", "Timed out opening database connection").await?;
            return Ok(());
        }
    }

    send_ok(&mut tls_stream, seq + 2).await?;
    info!("Wire stream {} authenticated for '{}' from {}", stream_id, parsed.username, peer_addr);

    // 5. Bidirectional byte pump: client TLS stream <-> tunnel binary frames.
    let (client_out_tx, mut client_out_rx) = mpsc::unbounded_channel::<Vec<u8>>();
    register_client_sender(&state, &stream_id, client_out_tx);

    let (mut tls_read, mut tls_write) = tokio::io::split(tls_stream);

    let wire_out_tx = state.tunnel_registry.get_wire_tunnel(&host_id).await;
    let sid_for_read = stream_id.clone();
    let reader = tokio::spawn(async move {
        let mut buf = vec![0u8; 16384];
        loop {
            match tls_read.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => {
                    if let Some(ref tx) = wire_out_tx {
                        let framed = encode_wire_frame(&sid_for_read, &buf[..n]);
                        if tx.send(framed).is_err() {
                            break;
                        }
                    } else {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    let writer = tokio::spawn(async move {
        while let Some(chunk) = client_out_rx.recv().await {
            if tls_write.write_all(&chunk).await.is_err() {
                break;
            }
            if tls_write.flush().await.is_err() {
                break;
            }
        }
        let _ = tls_write.shutdown().await;
    });

    tokio::select! {
        _ = reader => {},
        _ = writer => {},
    }

    // Cleanup: notify the engine and unregister locally.
    unregister_client_sender(&state, &stream_id);
    let _ = state.tunnel_registry.send_wire_control(&host_id, TunnelMessageToEngine::WireStreamClose {
        stream_id: stream_id.clone(),
    }).await;
    debug!("Wire stream {} closed", stream_id);

    Ok(())
}

// These three helpers exist as a thin seam so WireCredentialRegistry-style
// state (pending opens, client senders) can live wherever ProxyApiState
// composes it — see main.rs wiring.
async fn register_pending_open(state: &Arc<WireListenerState>, stream_id: &str, tx: oneshot::Sender<Result<(), String>>) {
    // Delegates to the shared registry created in main.rs
    crate::WIRE_STREAM_REGISTRY.get().unwrap().register_pending_open(stream_id.to_string(), tx);
}

fn register_client_sender(state: &Arc<WireListenerState>, stream_id: &str, tx: mpsc::UnboundedSender<Vec<u8>>) {
    let _ = state; // state kept for future direct use; registry is global-static for now
    crate::WIRE_STREAM_REGISTRY.get().unwrap().register_client(stream_id.to_string(), tx);
}

fn unregister_client_sender(state: &Arc<WireListenerState>, stream_id: &str) {
    let _ = state;
    crate::WIRE_STREAM_REGISTRY.get().unwrap().unregister_client(stream_id);
}

// ============================================================================
// MySQL protocol helpers (handshake v10, HandshakeResponse41 parsing, OK/ERROR)
// ============================================================================

struct ParsedAuth {
    username: String,
    auth_response: Vec<u8>,
}

fn parse_handshake_response(payload: &[u8]) -> Option<ParsedAuth> {
    if payload.len() < 33 {
        return None;
    }
    let capability_flags = u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
    let mut pos = 32;

    let mut username = String::new();
    while pos < payload.len() && payload[pos] != 0 {
        username.push(payload[pos] as char);
        pos += 1;
    }
    pos += 1;

    let auth_response = if capability_flags & 0x00200000 != 0 {
        if pos >= payload.len() { return None; }
        let len = payload[pos] as usize;
        pos += 1;
        if pos + len > payload.len() { return None; }
        let data = payload[pos..pos + len].to_vec();
        data
    } else {
        // mysql_clear_password without lenenc flag: null-terminated plaintext
        let start = pos;
        while pos < payload.len() && payload[pos] != 0 {
            pos += 1;
        }
        payload[start..pos].to_vec()
    };

    Some(ParsedAuth { username, auth_response })
}

async fn read_packet(stream: &mut (impl tokio::io::AsyncRead + Unpin)) -> anyhow::Result<(u8, Vec<u8>)> {
    let mut header = [0u8; 4];
    stream.read_exact(&mut header).await?;
    let len = u32::from_le_bytes([header[0], header[1], header[2], 0]) as usize;
    let seq = header[3];
    let mut payload = vec![0u8; len];
    stream.read_exact(&mut payload).await?;
    Ok((seq, payload))
}

async fn write_packet(stream: &mut (impl tokio::io::AsyncWrite + Unpin), seq: u8, payload: &[u8]) -> anyhow::Result<()> {
    let len = payload.len() as u32;
    let header = [(len & 0xFF) as u8, ((len >> 8) & 0xFF) as u8, ((len >> 16) & 0xFF) as u8, seq];
    stream.write_all(&header).await?;
    stream.write_all(payload).await?;
    stream.flush().await?;
    Ok(())
}

async fn send_handshake_v10(stream: &mut (impl tokio::io::AsyncWrite + Unpin), seq: u8) -> anyhow::Result<()> {
    let server_version = "8.0.0-bennett-relay";
    let thread_id: u32 = rand::random();
    let auth_data: [u8; 20] = rand::random();

    let capability_flags: u32 = CAP_LONG_PASSWORD | CAP_CONNECT_WITH_DB | CAP_PROTOCOL_41
        | CAP_SSL | CAP_SECURE_CONNECTION | CAP_PLUGIN_AUTH;

    let mut packet = Vec::new();
    packet.push(0x0a);
    packet.extend_from_slice(server_version.as_bytes());
    packet.push(0);
    packet.extend_from_slice(&thread_id.to_le_bytes());
    packet.extend_from_slice(&auth_data[0..8]);
    packet.push(0);
    packet.extend_from_slice(&(capability_flags as u16).to_le_bytes());
    packet.push(33);
    packet.extend_from_slice(&[0u8; 2]);
    packet.extend_from_slice(&((capability_flags >> 16) as u16).to_le_bytes());
    packet.push(21);
    packet.extend_from_slice(&[0u8; 10]);
    packet.extend_from_slice(&auth_data[8..20]);
    packet.push(0);
    packet.extend_from_slice(b"mysql_clear_password");
    packet.push(0);

    write_packet(stream, seq, &packet).await
}

async fn send_ok(stream: &mut (impl tokio::io::AsyncWrite + Unpin), seq: u8) -> anyhow::Result<()> {
    let packet = vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    write_packet(stream, seq, &packet).await
}

async fn send_error(stream: &mut (impl tokio::io::AsyncWrite + Unpin), seq: u8, code: u16, sql_state: &str, message: &str) -> anyhow::Result<()> {
    let mut packet = Vec::new();
    packet.push(0xff);
    packet.extend_from_slice(&code.to_le_bytes());
    packet.push(b'#');
    packet.extend_from_slice(sql_state.as_bytes());
    packet.extend_from_slice(message.as_bytes());
    write_packet(stream, seq, &packet).await
}
