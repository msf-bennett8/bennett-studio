use std::net::{SocketAddr, TcpStream};
use std::time::Duration;
use tracing::{info, debug};

use crate::models::database::{DatabaseInstance, DatabaseStatus, DatabaseSource};

#[derive(Debug, Clone)]
pub struct DiscoveredDatabase {
    pub host: String,
    pub port: u16,
    pub db_type: String,
    pub version_hint: Option<String>,
}

pub struct LocalScanner;

impl LocalScanner {
    pub fn new() -> Self {
        Self
    }

    pub async fn scan(&self) -> Vec<DiscoveredDatabase> {
        let targets = vec![
            (5432, "postgres"),
            (3306, "mysql"),
            (3307, "mariadb"),
            (6379, "redis"),
            (27017, "mongo"),
            (5433, "postgres"),
            (3308, "mysql"),
        ];

        let mut found = Vec::new();

        for (port, db_type) in targets {
            let addr = SocketAddr::from(([127, 0, 0, 1], port));
            debug!("Probing {} on port {}", db_type, port);

            match TcpStream::connect_timeout(&addr, Duration::from_millis(500)) {
                Ok(mut stream) => {
                    // Basic protocol fingerprinting
                    let version_hint = Self::fingerprint(&mut stream, db_type);
                    info!("Discovered local {} on port {} (version hint: {:?})", db_type, port, version_hint);
                    found.push(DiscoveredDatabase {
                        host: "127.0.0.1".to_string(),
                        port,
                        db_type: db_type.to_string(),
                        version_hint,
                    });
                }
                Err(_) => {
                    debug!("No response on port {}", port);
                }
            }
        }

        found
    }

    fn fingerprint(stream: &mut TcpStream, db_type: &str) -> Option<String> {
        use std::io::{Read, Write};

        match db_type {
            "postgres" => {
                // Send SSLRequest to trigger a response
                let ssl_req = vec![
                    0x00, 0x00, 0x00, 0x08, // length
                    0x04, 0xD2, 0x16, 0x2F, // SSL request code
                ];
                let _ = stream.write_all(&ssl_req);
                let mut buf = [0u8; 1];
                match stream.read_exact(&mut buf) {
                    Ok(_) if buf[0] == b'S' || buf[0] == b'N' => Some("unknown".to_string()),
                    _ => None,
                }
            }
            "mysql" | "mariadb" => {
                let mut buf = [0u8; 1024];
                match stream.read(&mut buf) {
                    Ok(n) if n > 5 => {
                        // MySQL handshake starts with protocol version (0x0a = 10)
                        if buf[0] == 0x0a {
                            Some("unknown".to_string())
                        } else {
                            None
                        }
                    }
                    _ => None,
                }
            }
            "redis" => {
                let _ = stream.write_all(b"PING\r\n");
                let mut buf = [0u8; 7];
                match stream.read_exact(&mut buf) {
                    Ok(_) if &buf == b"+PONG\r\n" => Some("unknown".to_string()),
                    _ => None,
                }
            }
            "mongo" => {
                // MongoDB responds to a minimal OP_QUERY
                let _ = stream.write_all(&[0u8; 16]); // minimal header
                Some("unknown".to_string()) // optimistic
            }
            _ => None,
        }
    }

    pub fn to_instance(&self, disc: &DiscoveredDatabase) -> DatabaseInstance {
        DatabaseInstance {
            id: format!("local-{}-{}", disc.db_type, disc.port),
            name: format!("local-{}-{}", disc.db_type, disc.port),
            db_type: disc.db_type.clone(),
            version: disc.version_hint.clone().unwrap_or_else(|| "unknown".to_string()),
            status: DatabaseStatus::Running,
            port: disc.port,
            size: "Unknown".to_string(),
            created_at: chrono::Local::now().format("%Y-%m-%d").to_string(),
            container_id: None,
            volume_name: None,
            env_vars: Vec::new(),
            source: DatabaseSource::Local,
        }
    }
}
