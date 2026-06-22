//! Time-to-live cache with automatic expiration
//! Phase 6: Memory-bounded structures, no unbounded HashMaps

use std::collections::HashMap;
use std::hash::Hash;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Entry with TTL
struct TtlEntry<V> {
    value: V,
    expires_at: Instant,
    last_accessed: Instant,
}

/// TTL cache with automatic cleanup
pub struct TtlCache<K, V> {
    store: RwLock<HashMap<K, TtlEntry<V>>>,
    default_ttl: Duration,
    max_size: usize,
    cleanup_interval: Duration,
}

impl<K, V> TtlCache<K, V>
where
    K: Eq + Hash + Clone + Send + Sync + 'static,
    V: Send + Sync + 'static,
{
    pub fn new(default_ttl: Duration, max_size: usize) -> Self {
        Self {
            store: RwLock::new(HashMap::with_capacity(max_size.min(1024))),
            default_ttl,
            max_size,
            cleanup_interval: Duration::from_secs(300), // 5 min
        }
    }

    /// Start background janitor. Call after wrapping in Arc.
    pub fn start_janitor(self_arc: &Arc<Self>) {
        let cache_clone = self_arc.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(300));
            loop {
                interval.tick().await;
                cache_clone.cleanup().await;
            }
        });
    }
    
    /// Get value if not expired
    pub async fn get(&self, key: &K) -> Option<V>
    where
        V: Clone,
    {
        let mut store = self.store.write().await;
        
        if let Some(entry) = store.get_mut(key) {
            if entry.expires_at > Instant::now() {
                entry.last_accessed = Instant::now();
                return Some(entry.value.clone());
            }
            // Expired, remove
            store.remove(key);
        }
        
        None
    }
    
    /// Insert with default TTL
    pub async fn insert(&self, key: K, value: V) {
        self.insert_with_ttl(key, value, self.default_ttl).await;
    }
    
    /// Insert with custom TTL
    pub async fn insert_with_ttl(&self, key: K, value: V, ttl: Duration) {
        let mut store = self.store.write().await;
        
        // Check max size, evict oldest if needed
        if store.len() >= self.max_size && !store.contains_key(&key) {
            // Find oldest entry
            if let Some(oldest_key) = store
                .iter()
                .min_by_key(|(_, v)| v.last_accessed)
                .map(|(k, _)| k.clone())
            {
                store.remove(&oldest_key);
                debug!("Evicted oldest entry from TTL cache");
            }
        }
        
        store.insert(key, TtlEntry {
            value,
            expires_at: Instant::now() + ttl,
            last_accessed: Instant::now(),
        });
    }
    
    /// Remove entry
    pub async fn remove(&self, key: &K) -> Option<V> {
        let mut store = self.store.write().await;
        store.remove(key).map(|e| e.value)
    }
    
    /// Check if key exists and not expired
    pub async fn contains_key(&self, key: &K) -> bool {
        self.get(key).await.is_some()
    }
    
    /// Get all non-expired keys
    pub async fn keys(&self) -> Vec<K>
    where
        V: Clone,
    {
        let store = self.store.read().await;
        let now = Instant::now();
        
        store
            .iter()
            .filter(|(_, e)| e.expires_at > now)
            .map(|(k, _)| k.clone())
            .collect()
    }
    
    /// Cleanup expired entries
    pub async fn cleanup(&self) {
        let mut store = self.store.write().await;
        let now = Instant::now();
        let before = store.len();
        
        store.retain(|_, entry| entry.expires_at > now);
        
        let after = store.len();
        let removed = before - after;
        
        if removed > 0 {
            info!("TTL cache cleanup: removed {} expired entries, {} remaining", removed, after);
        }
    }
    
    /// Clear all entries
    pub async fn clear(&self) {
        let mut store = self.store.write().await;
        store.clear();
        info!("TTL cache cleared");
    }
    
    /// Get cache stats
    pub async fn stats(&self) -> CacheStats {
        let store = self.store.read().await;
        let now = Instant::now();
        
        let total = store.len();
        let expired = store.values().filter(|e| e.expires_at <= now).count();
        let active = total - expired;
        
        CacheStats {
            total_entries: total,
            active_entries: active,
            expired_entries: expired,
            max_size: self.max_size,
            default_ttl_secs: self.default_ttl.as_secs(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_entries: usize,
    pub active_entries: usize,
    pub expired_entries: usize,
    pub max_size: usize,
    pub default_ttl_secs: u64,
}

use std::sync::Arc;
