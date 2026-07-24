//! Relay-side registry for tunneled wire-protocol (MySQL/Postgres) byte
//! streams (Phase 2). Tracks:
//!  - stream_id -> sender feeding bytes to the real external client's TCP
//!    socket (populated by Phase 3's public listener when it accepts a client)
//!  - stream_id -> pending "stream opened" acknowledgement from the engine

use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};

/// Maps a wire-protocol (MySQL/Postgres) password hash to the host_id that
/// owns it — separate from ApiKeyRegistry, which maps the bnt_live_ key
/// hash used by /api/v1. Populated by ApiKeyRegistered tunnel messages
/// when a key has wire access enabled.
#[derive(Default)]
pub struct WireCredentialRegistry {
    entries: DashMap<String, String>, // wire_password_hash -> host_id
}

impl WireCredentialRegistry {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn register(&self, wire_password_hash: String, host_id: String) {
        self.entries.insert(wire_password_hash, host_id);
    }

    pub fn revoke(&self, wire_password_hash: &str) {
        self.entries.remove(wire_password_hash);
    }

    pub fn resolve(&self, wire_password_hash: &str) -> Option<String> {
        self.entries.get(wire_password_hash).map(|v| v.clone())
    }

    pub fn remove_all_host_credentials(&self, host_id: &str) {
        self.entries.retain(|_, v| v != host_id);
    }
}

pub type ClientFrameSender = mpsc::UnboundedSender<Vec<u8>>;

#[derive(Default)]
pub struct WireStreamRegistry {
    /// stream_id -> sender feeding bytes to the real client socket
    client_senders: DashMap<String, ClientFrameSender>,
    /// stream_id -> oneshot notified when the engine confirms open/fail
    pending_opens: DashMap<String, oneshot::Sender<Result<(), String>>>,
}

impl WireStreamRegistry {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Called by the public MySQL/Postgres listener (Phase 3) once it has
    /// accepted a client and knows where to write bytes back to it.
    pub fn register_client(&self, stream_id: String, tx: ClientFrameSender) {
        self.client_senders.insert(stream_id, tx);
    }

    pub fn unregister_client(&self, stream_id: &str) {
        self.client_senders.remove(stream_id);
    }

    /// Route a binary frame received from the engine to the matching
    /// real client socket. Returns false if no such stream is registered.
    pub fn route_to_client(&self, stream_id: &str, payload: Vec<u8>) -> bool {
        if let Some(tx) = self.client_senders.get(stream_id) {
            tx.send(payload).is_ok()
        } else {
            false
        }
    }

    /// Register a waiter for the engine's open acknowledgement.
    pub fn register_pending_open(&self, stream_id: String, tx: oneshot::Sender<Result<(), String>>) {
        self.pending_opens.insert(stream_id, tx);
    }

    /// Called when the engine confirms (or rejects) a stream open.
    pub fn complete_open(&self, stream_id: &str, result: Result<(), String>) {
        if let Some((_, tx)) = self.pending_opens.remove(stream_id) {
            let _ = tx.send(result);
        }
    }
}
