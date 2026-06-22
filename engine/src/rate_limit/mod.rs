//! Rate limiting service
//! Phase 5: Token bucket per share, per IP
//! Prevents abuse and ensures fair resource sharing

use governor::{Quota, RateLimiter, clock::DefaultClock, state::keyed::DefaultKeyedStateStore};
use nonzero_ext::nonzero;
use std::net::IpAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use tracing::{info, warn, debug};

/// Rate limiter keyed by (share_code, ip)
pub struct RateLimitService {
    limiters: Arc<RwLock<HashMap<String, Arc<RateLimiter<String, DefaultKeyedStateStore<String>, DefaultClock>>>>>,
    default_quota: Quota,
}

impl RateLimitService {
    pub fn new() -> Self {
        // Default: 100 requests per second, burst of 50
        let default_quota = Quota::per_second(nonzero!(100u32))
            .allow_burst(nonzero!(50u32));
        
        Self {
            limiters: Arc::new(RwLock::new(HashMap::new())),
            default_quota,
        }
    }
    
    /// Check if request is allowed
    pub async fn check(&self, share_code: &str, ip: &IpAddr) -> Result<(), String> {
        let key = format!("{}:{}", share_code, ip);
        let limiter_key = format!("{}", share_code); // Per-share limiter
        
        // Get or create limiter for this share
        let limiter = {
            let limiters = self.limiters.read().await;
            limiters.get(&limiter_key).cloned()
        };
        
        let limiter = match limiter {
            Some(l) => l,
            None => {
                let new_limiter = Arc::new(RateLimiter::keyed(self.default_quota));
                let mut limiters = self.limiters.write().await;
                limiters.entry(limiter_key.clone()).or_insert_with(|| new_limiter.clone());
                new_limiter
            }
        };
        
        // Check rate
        match limiter.check_key(&key) {
            Ok(_) => {
                debug!("Rate limit OK for {}", key);
                Ok(())
            }
            Err(_) => {
                warn!("Rate limit exceeded for {}", key);
                Err("Rate limit exceeded. Please slow down.".to_string())
            }
        }
    }
    
    /// Configure custom quota for a share
    pub async fn set_quota(&self, share_code: &str, requests_per_second: u32, burst: u32) {
        let rps = nonzero!(requests_per_second);
        let burst = nonzero!(burst);
        let quota = Quota::per_second(rps).allow_burst(burst);
        
        let new_limiter = Arc::new(RateLimiter::keyed(quota));
        let mut limiters = self.limiters.write().await;
        limiters.insert(share_code.to_string(), new_limiter);
        
        info!("Set custom rate limit for {}: {} rps, burst {}", share_code, requests_per_second, burst);
    }
    
    /// Remove limiter (when share is revoked)
    pub async fn remove(&self, share_code: &str) {
        let mut limiters = self.limiters.write().await;
        limiters.remove(share_code);
    }
}

/// Global rate limiter for anonymous/unauthenticated requests
pub struct GlobalRateLimiter {
    limiter: Arc<RateLimiter<&'static str, DefaultKeyedStateStore<&'static str>, DefaultClock>>,
}

impl GlobalRateLimiter {
    pub fn new() -> Self {
        // Very restrictive: 10 req/s, burst of 5
        let quota = Quota::per_second(nonzero!(10u32))
            .allow_burst(nonzero!(5u32));
        
        Self {
            limiter: Arc::new(RateLimiter::keyed(quota)),
        }
    }
    
    pub fn check(&self, key: &'static str) -> Result<(), String> {
        match self.limiter.check_key(&key) {
            Ok(_) => Ok(()),
            Err(_) => Err("Global rate limit exceeded".to_string()),
        }
    }
}
