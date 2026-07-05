//! Health check service
//! Polls engine endpoints and reports status

use crate::router::ShareRouter;
use crate::transport::Transport;
use std::sync::Arc;
use tokio::time::{interval, Duration};
use tracing::{error, info, warn};

/// Health monitor — periodically checks engine and updates route cache
pub struct HealthMonitor;

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

                // Refresh routes (also validates DB connectivity)
                if let Err(e) = router.refresh_routes().await {
                    error!("Route refresh failed during health check: {}", e);
                }

                if transport_ok {
                    info!("Health check: OK");
                }
            }
        })
    }
}
