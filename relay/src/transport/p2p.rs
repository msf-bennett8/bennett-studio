//! P2P Transport — Full implementation using QUIC over UDP hole punching
//!
//! Replaces the previous stub. Supports:
//! - Server mode: waits for QUIC connections after hole punching
//! - Client mode: initiates hole punch and QUIC connection
//! - Multiplexed streams: HTTP API and MySQL wire over single QUIC connection
//!
//! ICE candidates are exchanged manually via share links (base64 encoded).

use super::{ByteStream, ProtocolType, PooledConnection, Transport};
use crate::transport::ice::IceCandidates;
use super::quic::{P2pQuicConnection, connect_quic_client, open_stream, start_quic_server, QuicError};
use std::collections::VecDeque;
use std::future::Future;
use std::io;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// P2P transport — can operate as server or client
pub struct P2pTransport {
    mode: P2pMode,
    local_ice: IceCandidates,
    remote_ice: Option<IceCandidates>,
    share_code: Option<String>,
    connection: Arc<RwLock<Option<P2pQuicConnection>>>,
    stream_pool: Arc<Mutex<VecDeque<PooledStream>>>,
    /// Server endpoint — only set in server mode, used to accept new connections
    server: Option<super::quic::P2pQuicServer>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum P2pMode {
    Server,
    Client,
}

/// A pooled stream over QUIC
struct PooledStream {
    send: quinn::SendStream,
    recv: quinn::RecvStream,
    protocol: ProtocolType,
    created_at: Instant,
}

impl P2pTransport {
    /// Create P2P transport in server mode
    pub async fn new_server(
        local_ice: IceCandidates,
        share_code: Option<String>,
    ) -> Result<Self, P2pError> {
        info!(mode = "server", "Creating P2P transport");

        // Start QUIC server endpoint (does NOT block on accept)
        let server = start_quic_server(&local_ice, share_code.clone())
            .await
            .map_err(|e| P2pError::QuicFailed(e))?;

        Ok(Self {
            mode: P2pMode::Server,
            local_ice,
            remote_ice: None,
            share_code,
            connection: Arc::new(RwLock::new(None)), // No connection yet — accepted in run_p2p()
            stream_pool: Arc::new(Mutex::new(VecDeque::new())),
            server: Some(server),
        })
    }

    /// Create P2P transport in client mode
    pub async fn new_client(
        remote_ice: IceCandidates,
        share_code: Option<String>,
    ) -> Result<Self, P2pError> {
        info!(mode = "client", "Creating P2P transport");

        // Gather our ICE for hole punching
        let local_ice = super::ice::gather_ice_candidates()
            .await
            .map_err(|e| P2pError::IceFailed(e))?;

        // Log candidate info for debugging
        if let Some(local_srflx) = local_ice.srflx_addr() {
            if let Some(remote_srflx) = remote_ice.srflx_addr() {
                if local_srflx.ip() == remote_srflx.ip() {
                    info!("Same-NAT detected — LAN fallback will be attempted");
                } else {
                    info!(local_srflx = %local_srflx, remote_srflx = %remote_srflx, "Different NATs — standard hole punching");
                }
            }
        }

        // Connect via QUIC (punch_hole handles LAN fallback internally)
        let quic_conn = connect_quic_client(&remote_ice, &local_ice)
            .await
            .map_err(|e| P2pError::QuicFailed(e))?;

        Ok(Self {
            mode: P2pMode::Client,
            local_ice,
            remote_ice: Some(remote_ice),
            share_code,
            connection: Arc::new(RwLock::new(Some(quic_conn))),
            stream_pool: Arc::new(Mutex::new(VecDeque::new())),
            server: None,
        })
    }

    /// Check if this transport is in server mode
    pub fn is_server(&self) -> bool {
        self.mode == P2pMode::Server
    }

    /// Accept an incoming QUIC connection (server mode only)
    /// Blocks until a client connects
    pub async fn accept_connection(&self) -> Result<(), P2pError> {
        if self.mode != P2pMode::Server {
            return Err(P2pError::NotConnected);
        }

        let server = self.server.as_ref()
            .ok_or(P2pError::NotConnected)?;

        let conn = server.accept().await
            .map_err(|e| P2pError::QuicFailed(e))?;

        let mut guard = self.connection.write().await;
        *guard = Some(conn);

        info!("P2P server accepted client connection");
        Ok(())
    }

    /// Accept an incoming bidirectional stream from the P2P connection
    pub async fn accept_stream(&self) -> Result<(super::ProtocolType, quinn::SendStream, quinn::RecvStream), P2pError> {
        let conn = self.get_connection().await?;
        super::quic::accept_stream(&conn).await
            .map_err(|e| P2pError::QuicFailed(e))
    }

    /// Get the connection (reconnect if lost — client mode only)
    async fn get_connection(&self) -> Result<P2pQuicConnection, P2pError> {
        let conn = self.connection.read().await;
        if let Some(ref c) = *conn {
            if c.connection.close_reason().is_none() {
              return Ok(P2pQuicConnection {
                  connection: c.connection.clone(),
                  remote_addr: c.remote_addr,
                  is_server: c.is_server,
                  local_addr: c.local_addr,
              });
            }
        }
        drop(conn);

        // Connection lost — try to reconnect (client mode only)
        if self.mode == P2pMode::Client {
            if let Some(ref remote_ice) = self.remote_ice {
                warn!("P2P connection lost, attempting reconnect");
                let new_conn = connect_quic_client(remote_ice, &self.local_ice)
                    .await
                    .map_err(|e| P2pError::QuicFailed(e))?;

                  let mut conn_guard = self.connection.write().await;
                  *conn_guard = Some(P2pQuicConnection {
                      connection: new_conn.connection.clone(),
                      remote_addr: new_conn.remote_addr,
                      is_server: new_conn.is_server,
                      local_addr: new_conn.local_addr,
                  });
                  return Ok(new_conn);
            }
        }

        // Server mode: connection should have been accepted by run_p2p()
        Err(P2pError::NotConnected)
    }
}

impl Transport for P2pTransport {
    fn name(&self) -> &'static str {
        "p2p-quic"
    }

    fn acquire(
        &self,
        protocol: ProtocolType,
    ) -> Pin<Box<dyn Future<Output = io::Result<PooledConnection>> + Send + '_>> {
          Box::pin(async move {
              // Try to get from pool first
              let pooled_stream = {
                  let mut pool = self.stream_pool.lock().unwrap();
                  pool.pop_front().filter(|s| s.created_at.elapsed().as_secs() < 300)
              };
              
              if let Some(stream) = pooled_stream {
                  debug!(protocol = ?protocol, "Reused pooled P2P stream");
                  let conn = self.get_connection().await.map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
                  return Ok(PooledConnection {
                      stream: ByteStream::Quic(
                          conn.connection,
                          stream.send,
                          stream.recv,
                      ),
                      protocol,
                      created_at: Instant::now(),
                  });
              }

            // Open new stream
            let conn = self.get_connection().await
                .map_err(|e| io::Error::new(io::ErrorKind::NotConnected, e))?;

            let (send, recv) = open_stream(&conn, protocol).await
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

            debug!(protocol = ?protocol, "Opened new P2P QUIC stream");

            Ok(PooledConnection {
                stream: ByteStream::Quic(conn.connection, send, recv),
                protocol,
                created_at: Instant::now(),
            })
        })
    }

    fn release(&self, conn: PooledConnection) {
        // For QUIC, we can't easily return streams to a generic pool
        // because ByteStream::Quic owns the send/recv. Instead, we just let
        // them drop — QUIC handles stream lifecycle efficiently.
        debug!(protocol = ?conn.protocol, "Released P2P connection (stream closed)");
    }

    fn health_check(&self) -> Pin<Box<dyn Future<Output = bool> + Send + '_>> {
        Box::pin(async move {
            // Server mode before first connection: check if endpoint is alive
            if self.mode == P2pMode::Server && self.server.is_some() {
                // Server endpoint is bound and ready
                return true;
            }
            // Otherwise check active connection
            match self.get_connection().await {
                Ok(conn) => conn.connection.close_reason().is_none(),
                Err(_) => false,
            }
        })
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// P2P transport errors
#[derive(Debug)]
pub enum P2pError {
    IceFailed(super::ice::IceError),
    QuicFailed(QuicError),
    NotConnected,
}

impl std::fmt::Display for P2pError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            P2pError::IceFailed(e) => write!(f, "ICE failed: {}", e),
            P2pError::QuicFailed(e) => write!(f, "QUIC failed: {}", e),
            P2pError::NotConnected => write!(f, "P2P not connected"),
        }
    }
}

impl std::error::Error for P2pError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            P2pError::IceFailed(e) => Some(e),
            P2pError::QuicFailed(e) => Some(e),
            _ => None,
        }
    }
}

// Keep the stub for backward compatibility during transition
#[derive(Clone)]
pub struct P2pTransportStub;

impl Transport for P2pTransportStub {
    fn name(&self) -> &'static str {
        "p2p-stub"
    }

    fn acquire(
        &self,
        _protocol: ProtocolType,
    ) -> Pin<Box<dyn Future<Output = io::Result<PooledConnection>> + Send + '_>> {
        Box::pin(async move {
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "P2P transport not yet implemented. Enable TCP transport instead.",
            ))
        })
    }

    fn release(&self, _conn: PooledConnection) {}

    fn health_check(&self) -> Pin<Box<dyn Future<Output = bool> + Send + '_>> {
        Box::pin(async move { false })
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
