//! Reconnection logic with exponential backoff
//! Used by RelayTunnelClient and P2PConnection

use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn};

/// Reconnection policy with exponential backoff
pub struct ReconnectPolicy {
    initial_delay: Duration,
    max_delay: Duration,
    current_delay: Duration,
    attempt: u32,
    max_attempts: Option<u32>,
}

impl ReconnectPolicy {
    pub fn new(initial_delay_secs: u64, max_delay_secs: u64) -> Self {
        let initial = Duration::from_secs(initial_delay_secs);
        Self {
            initial_delay: initial,
            max_delay: Duration::from_secs(max_delay_secs),
            current_delay: initial,
            attempt: 0,
            max_attempts: None,
        }
    }

    pub fn with_max_attempts(mut self, max: u32) -> Self {
        self.max_attempts = Some(max);
        self
    }

    /// Get next delay and increment attempt
    pub fn next_delay(&mut self) -> Option<Duration> {
        if let Some(max) = self.max_attempts {
            if self.attempt >= max {
                return None;
            }
        }

        let delay = self.current_delay;
        self.attempt += 1;
        
        // Exponential backoff with jitter
        self.current_delay = std::cmp::min(
            self.current_delay * 2 + Duration::from_millis(fastrand::u64(0..500)),
            self.max_delay,
        );

        info!("Reconnection attempt {} in {}ms", self.attempt, delay.as_millis());
        Some(delay)
    }

    /// Reset on successful connection
    pub fn reset(&mut self) {
        self.attempt = 0;
        self.current_delay = self.initial_delay;
        info!("Reconnection policy reset");
    }

    pub fn attempts(&self) -> u32 {
        self.attempt
    }
}

/// Retry an async operation with backoff
pub async fn retry_with_backoff<F, Fut, T, E>(
    mut operation: F,
    policy: &mut ReconnectPolicy,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    loop {
        match operation().await {
            Ok(result) => {
                policy.reset();
                return Ok(result);
            }
            Err(e) => {
                warn!("Operation failed: {}", e);
                if let Some(delay) = policy.next_delay() {
                    sleep(delay).await;
                } else {
                    return Err(e);
                }
            }
        }
    }
}
