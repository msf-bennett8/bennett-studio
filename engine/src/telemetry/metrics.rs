//! Prometheus-compatible metrics
//! Phase 6: /metrics endpoint for monitoring

use std::sync::atomic::{AtomicU64, Ordering};
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::info;

/// Counter metric
pub struct Counter {
    value: AtomicU64,
    name: String,
    help: String,
}

impl Counter {
    pub fn new(name: &str, help: &str) -> Self {
        Self {
            value: AtomicU64::new(0),
            name: name.to_string(),
            help: help.to_string(),
        }
    }
    
    pub fn inc(&self) {
        self.value.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn add(&self, n: u64) {
        self.value.fetch_add(n, Ordering::Relaxed);
    }
    
    pub fn get(&self) -> u64 {
        self.value.load(Ordering::Relaxed)
    }
    
    pub fn format_prometheus(&self) -> String {
        format!("# HELP {} {}\n# TYPE {} counter\n{} {}\n",
            self.name, self.help, self.name, self.name, self.get())
    }
}

/// Gauge metric
pub struct Gauge {
    value: AtomicU64,
    name: String,
    help: String,
}

impl Gauge {
    pub fn new(name: &str, help: &str) -> Self {
        Self {
            value: AtomicU64::new(0),
            name: name.to_string(),
            help: help.to_string(),
        }
    }
    
    pub fn set(&self, n: u64) {
        self.value.store(n, Ordering::Relaxed);
    }
    
    pub fn get(&self) -> u64 {
        self.value.load(Ordering::Relaxed)
    }
    
    pub fn format_prometheus(&self) -> String {
        format!("# HELP {} {}\n# TYPE {} gauge\n{} {}\n",
            self.name, self.help, self.name, self.name, self.get())
    }
}

/// Histogram metric (simplified)
pub struct Histogram {
    buckets: Vec<(f64, AtomicU64)>,
    sum: AtomicU64,
    count: AtomicU64,
    name: String,
    help: String,
}

impl Histogram {
    pub fn new(name: &str, help: &str, buckets: &[f64]) -> Self {
        Self {
            buckets: buckets.iter().map(|&b| (b, AtomicU64::new(0))).collect(),
            sum: AtomicU64::new(0),
            count: AtomicU64::new(0),
            name: name.to_string(),
            help: help.to_string(),
        }
    }
    
    pub fn observe(&self, value: f64) {
        self.count.fetch_add(1, Ordering::Relaxed);
        self.sum.fetch_add(value as u64, Ordering::Relaxed);
        
        for (bucket, counter) in &self.buckets {
            if value <= *bucket {
                counter.fetch_add(1, Ordering::Relaxed);
            }
        }
    }
    
    pub fn format_prometheus(&self) -> String {
        let mut output = format!("# HELP {} {}\n# TYPE {} histogram\n",
            self.name, self.help, self.name);
        
        for (bucket, counter) in &self.buckets {
            output.push_str(&format!("{}_bucket{{le=\"{}\"}} {}\n",
                self.name, bucket, counter.load(Ordering::Relaxed)));
        }
        
        output.push_str(&format!("{}_sum {}\n", self.name, self.sum.load(Ordering::Relaxed)));
        output.push_str(&format!("{}_count {}\n", self.name, self.count.load(Ordering::Relaxed)));
        
        output
    }
}

/// Metrics registry
pub struct MetricsRegistry {
    counters: RwLock<HashMap<String, Counter>>,
    gauges: RwLock<HashMap<String, Gauge>>,
    histograms: RwLock<HashMap<String, Histogram>>,
}

impl MetricsRegistry {
    pub fn new() -> Self {
        Self {
            counters: RwLock::new(HashMap::new()),
            gauges: RwLock::new(HashMap::new()),
            histograms: RwLock::new(HashMap::new()),
        }
    }
    
    pub async fn register_counter(&self, name: &str, help: &str) -> Counter {
        let counter = Counter::new(name, help);
        let mut counters = self.counters.write().await;
        counters.insert(name.to_string(), counter.clone());
        counter
    }
    
    pub async fn register_gauge(&self, name: &str, help: &str) -> Gauge {
        let gauge = Gauge::new(name, help);
        let mut gauges = self.gauges.write().await;
        gauges.insert(name.to_string(), gauge.clone());
        gauge
    }
    
    pub async fn register_histogram(&self, name: &str, help: &str, buckets: &[f64]) -> Histogram {
        let hist = Histogram::new(name, help, buckets);
        let mut histograms = self.histograms.write().await;
        histograms.insert(name.to_string(), hist.clone());
        hist
    }
    
    /// Export all metrics in Prometheus format
    pub async fn export_prometheus(&self) -> String {
        let mut output = String::new();
        
        // Counters
        let counters = self.counters.read().await;
        for counter in counters.values() {
            output.push_str(&counter.format_prometheus());
        }
        
        // Gauges
        let gauges = self.gauges.read().await;
        for gauge in gauges.values() {
            output.push_str(&gauge.format_prometheus());
        }
        
        // Histograms
        let histograms = self.histograms.read().await;
        for hist in histograms.values() {
            output.push_str(&hist.format_prometheus());
        }
        
        output
    }
}

// Clone implementations for Counter/Gauge/Histogram
impl Clone for Counter {
    fn clone(&self) -> Self {
        Self {
            value: AtomicU64::new(self.get()),
            name: self.name.clone(),
            help: self.help.clone(),
        }
    }
}

impl Clone for Gauge {
    fn clone(&self) -> Self {
        Self {
            value: AtomicU64::new(self.get()),
            name: self.name.clone(),
            help: self.help.clone(),
        }
    }
}

impl Clone for Histogram {
    fn clone(&self) -> Self {
        Self {
            buckets: self.buckets.iter().map(|(b, c)| (*b, AtomicU64::new(c.load(Ordering::Relaxed)))).collect(),
            sum: AtomicU64::new(self.sum.load(Ordering::Relaxed)),
            count: AtomicU64::new(self.count.load(Ordering::Relaxed)),
            name: self.name.clone(),
            help: self.help.clone(),
        }
    }
}

/// Global metrics instance
use std::sync::OnceLock;
static METRICS: OnceLock<MetricsRegistry> = OnceLock::new();

pub fn init_metrics() -> &'static MetricsRegistry {
    METRICS.get_or_init(|| {
        info!("Metrics registry initialized");
        MetricsRegistry::new()
    })
}

/// Pre-defined metrics
pub mod predefined {
    use super::*;
    
    pub fn queries_total() -> Counter {
        Counter::new("bennett_queries_total", "Total queries executed")
    }
    
    pub fn query_duration_seconds() -> Histogram {
        Histogram::new("bennett_query_duration_seconds", "Query execution time", 
            &[0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0])
    }
    
    pub fn active_connections() -> Gauge {
        Gauge::new("bennett_active_connections", "Current active database connections")
    }
    
    pub fn active_shares() -> Gauge {
        Gauge::new("bennett_active_shares", "Current active share sessions")
    }
    
    pub fn cache_hit_rate() -> Gauge {
        Gauge::new("bennett_cache_hit_rate", "Query cache hit rate")
    }
}
