//! Query result cache with TTL
//! Phase 6: Cache repeated queries, invalidate on write

use crate::utils::ttl_cache::{TtlCache, CacheStats};
use crate::control_plane::connection::manager::QueryResult;
use std::time::Duration;
use std::sync::Arc;
use tracing::{debug, info};

/// Query cache key
#[derive(Clone, Hash, Eq, PartialEq, Debug)]
struct QueryCacheKey {
    db_id: String,
    sql: String,
    // Include permission context to prevent cache poisoning
    share_code: Option<String>,
}

/// Query result cache entry
#[derive(Clone, Debug)]
struct QueryCacheEntry {
    result: QueryResult,
    created_at: std::time::Instant,
    // Track tables referenced for invalidation
    tables: Vec<String>,
}

/// Query result cache
pub struct QueryCache {
    cache: Arc<TtlCache<QueryCacheKey, QueryCacheEntry>>,
    hit_count: std::sync::atomic::AtomicU64,
    miss_count: std::sync::atomic::AtomicU64,
}

impl QueryCache {
    pub fn new() -> Self {
        // 5 minute TTL, max 1000 entries
        let cache = TtlCache::new(Duration::from_secs(300), 1000);
        
        Self {
            cache,
            hit_count: std::sync::atomic::AtomicU64::new(0),
            miss_count: std::sync::atomic::AtomicU64::new(0),
        }
    }
    
    /// Get cached result if available
    pub async fn get(&self, db_id: &str, sql: &str, share_code: Option<&str>) -> Option<QueryResult> {
        let key = QueryCacheKey {
            db_id: db_id.to_string(),
            sql: sql.to_string(),
            share_code: share_code.map(|s| s.to_string()),
        };
        
        if let Some(entry) = self.cache.get(&key).await {
            self.hit_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            debug!("Query cache HIT for {}", sql);
            return Some(entry.result);
        }
        
        self.miss_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        debug!("Query cache MISS for {}", sql);
        None
    }
    
    /// Store result in cache
    pub async fn put(&self, db_id: &str, sql: &str, share_code: Option<&str>, result: QueryResult, tables: Vec<String>) {
        // Only cache SELECT queries
        let upper = sql.trim().to_uppercase();
        if !upper.starts_with("SELECT") && !upper.starts_with("WITH") {
            return;
        }
        
        // Don't cache if too large (> 10k rows)
        if result.row_count > 10000 {
            return;
        }
        
        let key = QueryCacheKey {
            db_id: db_id.to_string(),
            sql: sql.to_string(),
            share_code: share_code.map(|s| s.to_string()),
        };
        
        let entry = QueryCacheEntry {
            result,
            created_at: std::sync::time::Instant::now(),
            tables,
        };
        
        self.cache.insert(key, entry).await;
    }
    
    /// Invalidate cache entries for a database
    pub async fn invalidate_db(&self, db_id: &str) {
        let keys = self.cache.keys().await;
        let to_remove: Vec<_> = keys
            .into_iter()
            .filter(|k| k.db_id == db_id)
            .collect();
        
        for key in to_remove {
            self.cache.remove(&key).await;
        }
        
        if !to_remove.is_empty() {
            info!("Invalidated {} cache entries for db {}", to_remove.len(), db_id);
        }
    }
    
    /// Invalidate cache entries for specific tables
    pub async fn invalidate_tables(&self, db_id: &str, tables: &[String]) {
        let keys = self.cache.keys().await;
        let to_remove: Vec<_> = keys
            .into_iter()
            .filter(|k| {
                k.db_id == db_id && {
                    // Check if entry references any of the invalidated tables
                    // This requires storing table references in the key
                    // Simplified: invalidate all for this db
                    true
                }
            })
            .collect();
        
        for key in to_remove {
            self.cache.remove(&key).await;
        }
    }
    
    /// Get cache statistics
    pub fn stats(&self) -> QueryCacheStats {
        let hits = self.hit_count.load(std::sync::atomic::Ordering::Relaxed);
        let misses = self.miss_count.load(std::sync::atomic::Ordering::Relaxed);
        let total = hits + misses;
        
        QueryCacheStats {
            hits,
            misses,
            hit_rate: if total > 0 { hits as f64 / total as f64 } else { 0.0 },
        }
    }
    
    /// Get underlying cache stats
    pub async fn cache_stats(&self) -> CacheStats {
        self.cache.stats().await
    }
    
    /// Clear all cache
    pub async fn clear(&self) {
        self.cache.clear().await;
        self.hit_count.store(0, std::sync::atomic::Ordering::Relaxed);
        self.miss_count.store(0, std::sync::atomic::Ordering::Relaxed);
    }
}

#[derive(Debug, Clone)]
pub struct QueryCacheStats {
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f64,
}
