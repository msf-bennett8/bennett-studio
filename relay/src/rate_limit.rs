//! Rate limiting for the public /api/v1/* gateway.
//! Two tiers, mirroring engine/src/rate_limit/mod.rs:
//!  - per-API-key limiter (keyed by key_hash) — the primary defense since
//!    durable keys have no expiry to naturally bound abuse
//!  - per-IP limiter for requests that haven't resolved to a valid key yet
//!    (protects against brute-force key-hash scanning)

use governor::{Quota, RateLimiter, clock::DefaultClock, state::keyed::DefaultKeyedStateStore};
use std::net::IpAddr;
use std::num::NonZeroU32;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use tracing::warn;

pub struct ApiV1RateLimiter {
    per_key: Arc<RwLock<HashMap<String, Arc<RateLimiter<String, DefaultKeyedStateStore<String>, DefaultClock>>>>>,
    default_quota: Quota,
    per_ip: Arc<RateLimiter<IpAddr, DefaultKeyedStateStore<IpAddr>, DefaultClock>>,
}

impl ApiV1RateLimiter {
    pub fn new(default_rps: u32, default_burst: u32) -> Self {
        let default_quota = Quota::per_second(NonZeroU32::new(default_rps.max(1)).unwrap())
            .allow_burst(NonZeroU32::new(default_burst.max(1)).unwrap());

        // Pre-auth IP limiter: deliberately looser than per-key (these
        // requests haven't proven ownership of a valid key yet, but we
        // still don't want to block legitimate retries during a cold key)
        let ip_quota = Quota::per_second(NonZeroU32::new(20).unwrap())
            .allow_burst(NonZeroU32::new(10).unwrap());

        Self {
            per_key: Arc::new(RwLock::new(HashMap::new())),
            default_quota,
            per_ip: Arc::new(RateLimiter::keyed(ip_quota)),
        }
    }

    /// Check before the key has been resolved (guards key-lookup itself)
    pub fn check_ip(&self, ip: IpAddr) -> Result<(), &'static str> {
        match self.per_ip.check_key(&ip) {
            Ok(_) => Ok(()),
            Err(_) => {
                warn!("Pre-auth rate limit exceeded for IP {}", ip);
                Err("Too many requests. Please slow down.")
            }
        }
    }

    /// Check after the key has resolved to a valid host
    pub async fn check_key(&self, key_hash: &str) -> Result<(), &'static str> {
        let limiter = {
            let map = self.per_key.read().await;
            map.get(key_hash).cloned()
        };
        let limiter = match limiter {
            Some(l) => l,
            None => {
                let new_limiter = Arc::new(RateLimiter::keyed(self.default_quota));
                let mut map = self.per_key.write().await;
                map.entry(key_hash.to_string()).or_insert_with(|| new_limiter.clone());
                new_limiter
            }
        };

        match limiter.check_key(&key_hash.to_string()) {
            Ok(_) => Ok(()),
            Err(_) => {
                warn!("Rate limit exceeded for API key {}...", &key_hash[..8.min(key_hash.len())]);
                Err("Rate limit exceeded for this API key. Please slow down.")
            }
        }
    }

    /// Remove a key's limiter state (called when a key is revoked)
    pub async fn remove_key(&self, key_hash: &str) {
        self.per_key.write().await.remove(key_hash);
    }
}
