//! Prometheus-compatible metrics endpoint for relay
//! Exposes: active connections, total requests, errors, latency

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tracing::{error, info, warn};

/// Simple counter using AtomicU64
#[derive(Debug)]
pub struct Counter {
    value: AtomicU64,
}

impl Counter {
    pub fn new() -> Self {
        Self {
            value: AtomicU64::new(0),
        }
    }

    pub fn inc(&self) {
        self.value.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get(&self) -> u64 {
        self.value.load(Ordering::Relaxed)
    }
}

impl Default for Counter {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for Counter {
    fn clone(&self) -> Self {
        Self {
            value: AtomicU64::new(self.get()),
        }
    }
}

/// Global relay metrics
#[derive(Debug, Clone)]
pub struct RelayMetrics {
    pub connections_total: Counter,
    pub connections_active: Counter,
    pub http_requests_total: Counter,
    pub mysql_requests_total: Counter,
    pub errors_total: Counter,
    pub share_not_found_total: Counter,
    pub rate_limited_total: Counter,
}

impl RelayMetrics {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            connections_total: Counter::new(),
            connections_active: Counter::new(),
            http_requests_total: Counter::new(),
            mysql_requests_total: Counter::new(),
            errors_total: Counter::new(),
            share_not_found_total: Counter::new(),
            rate_limited_total: Counter::new(),
        })
    }

    pub fn record_connection(&self) {
        self.connections_total.inc();
        self.connections_active.inc();
    }

    pub fn record_disconnect(&self) {
        // connections_active is decremented elsewhere (ConnectionCounter)
        // This is just for tracking total lifecycle
    }

    pub fn record_http_request(&self) {
        self.http_requests_total.inc();
    }

    pub fn record_mysql_request(&self) {
        self.mysql_requests_total.inc();
    }

    pub fn record_error(&self) {
        self.errors_total.inc();
    }

    pub fn record_share_not_found(&self) {
        self.share_not_found_total.inc();
    }

    pub fn record_rate_limited(&self) {
        self.rate_limited_total.inc();
    }

    /// Export in Prometheus text format
    pub fn export(&self) -> String {
        format!(
            "# HELP bennett_relay_connections_total Total connections accepted\n\
             # TYPE bennett_relay_connections_total counter\n\
             bennett_relay_connections_total {}\n\
             \n\
             # HELP bennett_relay_connections_active Currently active connections\n\
             # TYPE bennett_relay_connections_active gauge\n\
             bennett_relay_connections_active {}\n\
             \n\
             # HELP bennett_relay_http_requests_total Total HTTP requests\n\
             # TYPE bennett_relay_http_requests_total counter\n\
             bennett_relay_http_requests_total {}\n\
             \n\
             # HELP bennett_relay_mysql_requests_total Total MySQL wire requests\n\
             # TYPE bennett_relay_mysql_requests_total counter\n\
             bennett_relay_mysql_requests_total {}\n\
             \n\
             # HELP bennett_relay_errors_total Total errors\n\
             # TYPE bennett_relay_errors_total counter\n\
             bennett_relay_errors_total {}\n\
             \n\
             # HELP bennett_relay_share_not_found_total Share not found errors\n\
             # TYPE bennett_relay_share_not_found_total counter\n\
             bennett_relay_share_not_found_total {}\n\
             \n\
             # HELP bennett_relay_rate_limited_total Rate limited connections\n\
             # TYPE bennett_relay_rate_limited_total counter\n\
             bennett_relay_rate_limited_total {}\n",
            self.connections_total.get(),
            self.connections_active.get(),
            self.http_requests_total.get(),
            self.mysql_requests_total.get(),
            self.errors_total.get(),
            self.share_not_found_total.get(),
            self.rate_limited_total.get(),
        )
    }
}

/// Global metrics instance
use std::sync::OnceLock;
static METRICS: OnceLock<Arc<RelayMetrics>> = OnceLock::new();

pub fn init_metrics() -> Arc<RelayMetrics> {
    METRICS.get_or_init(|| RelayMetrics::new()).clone()
}

pub fn get_metrics() -> Arc<RelayMetrics> {
    METRICS.get().cloned().unwrap_or_else(|| init_metrics())
}

/// Start a simple HTTP metrics server on the given port
pub async fn start_metrics_server(port: u16) -> tokio::task::JoinHandle<()> {
    let metrics = init_metrics();
    
    let addr = format!("127.0.0.1:{}", port);
    let listener = match TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(e) => {
            warn!("Failed to bind metrics server to {}: {}", addr, e);
            return tokio::spawn(async {});
        }
    };

    info!("Metrics server listening on http://{}", addr);

    tokio::spawn(async move {
        loop {
            let (mut stream, _) = match listener.accept().await {
                Ok(s) => s,
                Err(e) => {
                    error!("Metrics accept error: {}", e);
                    continue;
                }
            };

            let metrics_clone = metrics.clone();
            tokio::spawn(async move {
                let mut buf = [0u8; 1024];
                let _ = stream.read(&mut buf).await;

                let response = format!(
                    "HTTP/1.1 200 OK\r\n\
                     Content-Type: text/plain; version=0.0.4\r\n\
                     Content-Length: {}\r\n\
                     Connection: close\r\n\
                     \r\n\
                     {}",
                    metrics_clone.export().len(),
                    metrics_clone.export()
                );

                let _ = stream.write_all(response.as_bytes()).await;
                let _ = stream.flush().await;
            });
        }
    })
}
