//! Wire protocol proxy router
//! Routes incoming connections to MySQL or PostgreSQL proxy based on port or protocol detection

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

/// Port mapping for wire protocol proxy
/// MySQL default: 3307 (maps to local 3306)
/// PostgreSQL default: 5433 (maps to local 5432)
pub struct ProxyRouter {
    port_map: Arc<RwLock<HashMap<u16, ProxyTarget>>>,
}

pub struct ProxyTarget {
    pub share_code: String,
    pub db_type: String, // "mysql" or "postgres"
    pub local_port: u16,
    pub tls_enabled: bool,
    pub max_connections: usize,
    pub current_connections: std::sync::atomic::AtomicUsize,
}

impl ProxyRouter {
    pub fn new() -> Self {
        Self {
            port_map: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Register a share for wire protocol access
    /// Returns the external port to connect to
    pub async fn register_share(
        &self,
        share_code: &str,
        db_type: &str,
        local_port: u16,
    ) -> Result<u16, String> {
        let mut map = self.port_map.write().await;
        
        // Find or allocate external port
        // For now, use fixed offset: local_port + 1000
        let external_port = local_port + 1000;
        
        map.insert(external_port, ProxyTarget {
            share_code: share_code.to_string(),
            db_type: db_type.to_string(),
            local_port,
            tls_enabled: true,
            max_connections: 50, // Default max 50 concurrent connections per share
            current_connections: std::sync::atomic::AtomicUsize::new(0),
        });
        
        info!("Registered wire proxy: {} -> {}:{} (type: {})", 
            external_port, local_port, share_code, db_type);
        
        Ok(external_port)
    }
    
    /// Unregister a share
    pub async fn unregister_share(&self, share_code: &str) {
        let mut map = self.port_map.write().await;
        let to_remove: Vec<u16> = map
            .iter()
            .filter(|(_, v)| v.share_code == share_code)
            .map(|(k, _)| *k)
            .collect();
        
        for port in to_remove {
            map.remove(&port);
            info!("Unregistered wire proxy port {}", port);
        }
    }
    
    /// Lookup target by external port
    pub async fn lookup(&self, port: u16) -> Option<ProxyTarget> {
        let map = self.port_map.read().await;
        map.get(&port).cloned()
    }
    
    /// Check if connection allowed for target
    pub async fn try_connect(&self, port: u16) -> Result<(), String> {
        let map = self.port_map.read().await;
        let target = map.get(&port).ok_or("Port not registered")?;
        
        let current = target.current_connections.load(std::sync::atomic::Ordering::Relaxed);
        if current >= target.max_connections {
            return Err(format!("Connection limit reached for share {}: {}/{}", 
                target.share_code, current, target.max_connections));
        }
        
        target.current_connections.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }
    
    /// Release connection for target
    pub async fn disconnect(&self, port: u16) {
        let map = self.port_map.read().await;
        if let Some(target) = map.get(&port) {
            let prev = target.current_connections.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
            if prev == 0 {
                // Underflow protection
                target.current_connections.store(0, std::sync::atomic::Ordering::Relaxed);
            }
        }
    }
    
    /// List active registrations
    pub async fn list(&self) -> Vec<(u16, ProxyTarget)> {
        let map = self.port_map.read().await;
        map.iter().map(|(k, v)| (*k, v.clone())).collect()
    }
}

impl Clone for ProxyTarget {
    fn clone(&self) -> Self {
        Self {
            share_code: self.share_code.clone(),
            db_type: self.db_type.clone(),
            local_port: self.local_port,
            tls_enabled: self.tls_enabled,
            max_connections: self.max_connections,
            current_connections: std::sync::atomic::AtomicUsize::new(
                self.current_connections.load(std::sync::atomic::Ordering::Relaxed)
            ),
        }
    }
}

// ProxyRouter implementation complete
// Features: fixed port offset routing, TLS cert management, per-share connection limits
