//! API Key Registry — relay-side routing table mapping API key hash to
//! the engine host_id that owns it. Populated via ApiKeyRegistered
//! tunnel messages. Authorization/permission checks happen engine-side
//! (relay is a dumb router — same trust boundary as share JWTs today).

use dashmap::DashMap;
use std::sync::Arc;

#[derive(Default)]
pub struct ApiKeyRegistry {
    keys: DashMap<String, String>, // key_hash -> host_id
}

impl ApiKeyRegistry {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn register(&self, key_hash: String, host_id: String) {
        self.keys.insert(key_hash, host_id);
    }

    pub fn revoke(&self, key_hash: &str) {
        self.keys.remove(key_hash);
    }

    pub fn resolve(&self, key_hash: &str) -> Option<String> {
        self.keys.get(key_hash).map(|v| v.clone())
    }

    /// Remove all keys belonging to a disconnected host
    pub fn remove_all_host_keys(&self, host_id: &str) {
        self.keys.retain(|_, v| v != host_id);
    }
}
