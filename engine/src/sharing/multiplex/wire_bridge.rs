//! Wire Bridge — engine-side handler for tunneled MySQL/Postgres wire-protocol
//! byte streams (Phase 2). Bridges a virtual tunnel stream (identified by
//! stream_id) to a real local TCP connection to the target database.
//!
//! Bulk data travels as raw WebSocket Binary frames (see wire_frame.rs) —
//! not JSON/base64 — to keep overhead near zero for large result sets or
//! bulk imports. Only stream lifecycle (open/opened/error/close) travels
//! as JSON control messages via the existing TunnelMessage enum.

use std::sync::Arc;
use dashmap::DashMap;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tracing::{info, warn, debug};

use crate::models::database::DatabaseInstance;
use crate::sharing::share_store::ShareStore;
use crate::sharing::multiplex::wire_frame::encode_wire_frame;

pub type WireFrameSender = mpsc::UnboundedSender<Vec<u8>>;

/// Registry of active wire-bridge streams on the engine side.
/// stream_id -> sender feeding bytes INTO the local DB TCP connection
/// (bytes that arrived from the external client via the tunnel).
#[derive(Default)]
pub struct WireBridgeRegistry {
    inbound: DashMap<String, WireFrameSender>,
}

impl WireBridgeRegistry {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Route a binary frame received from the tunnel to the matching
    /// local DB connection. Returns false if no such stream is registered.
    pub fn route_inbound(&self, stream_id: &str, payload: Vec<u8>) -> bool {
        if let Some(tx) = self.inbound.get(stream_id) {
            tx.send(payload).is_ok()
        } else {
            false
        }
    }

    pub fn remove(&self, stream_id: &str) {
        self.inbound.remove(stream_id);
    }
}

/// Open a new wire-bridge stream: authenticate via wire_password_hash,
/// connect to the local database, and spawn the bidirectional byte pump.
///
/// `wire_out_tx` sends already-framed bytes out over the tunnel (engine -> relay -> client).
pub async fn open_wire_stream(
    share_store: Arc<ShareStore>,
    databases: Arc<std::sync::Mutex<Vec<DatabaseInstance>>>,
    registry: Arc<WireBridgeRegistry>,
    stream_id: String,
    wire_username: String,
    wire_password_hash: String,
    wire_out_tx: WireFrameSender,
) -> Result<(), String> {
    // Look up the API key by wire password hash and verify the username
    // matches. This authenticates the wire-protocol credential pair
    // specifically — distinct from the bnt_live_ key itself, which
    // authenticates /api/v1.
    let key = share_store.get_api_key_by_wire_password_hash(&wire_password_hash).await
        .map_err(|e| format!("Lookup failed: {}", e))?
        .ok_or_else(|| "Invalid wire credentials".to_string())?;

    if key.wire_username.as_deref() != Some(wire_username.as_str()) {
        warn!("Wire stream {}: username mismatch", stream_id);
        return Err("Invalid wire credentials".to_string());
    }

    // Find the target database instance
    let db_instance = {
        let dbs = databases.lock().unwrap();
        dbs.iter().find(|d| d.id == key.db_id).cloned()
    };
    let db_instance = db_instance.ok_or_else(|| "Database not available".to_string())?;

    // Connect to the real local database
    let db_stream = TcpStream::connect(format!("127.0.0.1:{}", db_instance.port)).await
        .map_err(|e| format!("Failed to connect to local database: {}", e))?;

    let (mut db_read, mut db_write) = db_stream.into_split();

    // Inbound: bytes from the tunnel (external client) -> written to local DB
    let (in_tx, mut in_rx) = mpsc::unbounded_channel::<Vec<u8>>();
    registry.inbound.insert(stream_id.clone(), in_tx);

    let sid_for_write = stream_id.clone();
    tokio::spawn(async move {
        while let Some(chunk) = in_rx.recv().await {
            if db_write.write_all(&chunk).await.is_err() {
                break;
            }
            if db_write.flush().await.is_err() {
                break;
            }
        }
        debug!("Wire stream {} inbound writer closed", sid_for_write);
    });

    // Outbound: bytes from local DB -> framed and sent back over the tunnel
    let sid_for_read = stream_id.clone();
    let registry_for_read = registry.clone();
    tokio::spawn(async move {
        let mut buf = vec![0u8; 16384];
        loop {
            match db_read.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => {
                    let framed = encode_wire_frame(&sid_for_read, &buf[..n]);
                    if wire_out_tx.send(framed).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
        registry_for_read.remove(&sid_for_read);
        debug!("Wire stream {} outbound reader closed", sid_for_read);
    });

    info!(
        "Wire stream {} opened: db '{}' via wire user '{}'",
        stream_id, db_instance.name, wire_username
    );
    let _ = share_store.touch_api_key(&key.key_hash).await;

    Ok(())
}
