use std::net::TcpListener;
use tracing::{info, warn};

#[derive(Debug, thiserror::Error)]
pub enum PortError {
    #[error("No free port found in range {0}-{1}")]
    NoFreePort(u16, u16),
    #[error("Port {0} is already allocated")]
    AlreadyAllocated(u16),
}

pub struct PortAllocator {
    postgres_range: (u16, u16),
    mysql_range: (u16, u16),
    redis_range: (u16, u16),
    mongo_range: (u16, u16),
    allocated: std::sync::Mutex<Vec<u16>>,
}

impl Default for PortAllocator {
    fn default() -> Self {
        Self {
            postgres_range: (5432, 5500),
            mysql_range: (3306, 3310),
            redis_range: (6379, 6385),
            mongo_range: (27017, 27025),
            allocated: std::sync::Mutex::new(Vec::new()),
        }
    }
}

impl PortAllocator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn allocate(&self, db_type: &str) -> Result<u16, PortError> {
        let range = match db_type {
            "postgres" | "postgresql" => self.postgres_range,
            "mysql" | "mariadb" => self.mysql_range,
            "redis" => self.redis_range,
            "mongo" | "mongodb" => self.mongo_range,
            _ => (3000, 3100),
        };

        let mut allocated = self.allocated.lock().unwrap();

        for port in range.0..=range.1 {
            if allocated.contains(&port) {
                continue;
            }
            if Self::is_port_free(port) {
                allocated.push(port);
                info!("Allocated port {} for {}", port, db_type);
                return Ok(port);
            }
        }

        warn!("No free port found in range {:?} for {}", range, db_type);
        Err(PortError::NoFreePort(range.0, range.1))
    }

    pub fn release(&self, port: u16) {
        let mut allocated = self.allocated.lock().unwrap();
        if let Some(pos) = allocated.iter().position(|&p| p == port) {
            allocated.remove(pos);
            info!("Released port {}", port);
        }
    }

    pub fn is_port_free(port: u16) -> bool {
        TcpListener::bind(("0.0.0.0", port)).is_ok()
    }

    pub fn list_allocated(&self) -> Vec<u16> {
        self.allocated.lock().unwrap().clone()
    }
}
