//! Health check service
//! Polls engine heartbeats and marks routes online/offline

use crate::router::ShareRouter;
use std::sync::Arc;
use tokio::time::{interval, Duration};
use tracing::{error, info, warn, debug};
use chrono::Utc;

/// Health monitor — periodically checks engine heartbeats and updates route availability
pub struct HealthMonitor;

/// Heartbeat status for a host
#[derive(Debug, Clone)]
pub struct HostStatus {
    pub host_id: String,
    pub last_beat: chrono::DateTime<Utc>,
    pub ip_address: Option<String>,
    pub port: Option<u16>,
    pub version: Option<String>,
    pub online: bool,
}

impl HealthMonitor {
    pub fn start(
        router: Arc<ShareRouter>,
        transport: Arc<dyn crate::transport::Transport>,
        interval_secs: u64,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(interval_secs));

            loop {
                ticker.tick().await;

                // Check transport health
                let transport_ok = transport.health_check().await;
                if !transport_ok {
                    warn!(
                        transport = transport.name(),
                        "Transport health check failed"
                    );
                }

                // PHASE F: Check engine heartbeats and mark stale routes offline
                if let Err(e) = router.check_host_heartbeats().await {
                    error!("Host heartbeat check failed: {}", e);
                }

                // Refresh local routes from SQLite (if available)
                if let Err(e) = router.refresh_routes().await {
                    error!("Route refresh failed during health check: {}", e);
                }

                if transport_ok {
                    debug!("Health check: OK");
                }
            }
        })
    }
}
