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

    pub async fn scan(&self, existing: &[DatabaseInstance]) -> Vec<DiscoveredDatabase> {
        let mut found = Vec::new();

        // 1. TCP port scan for running services
        found.extend(self.scan_ports().await);

        // 2. Filesystem scan for stopped/native databases
        found.extend(self.scan_filesystem(existing));

        found
    }

    async fn scan_ports(&self) -> Vec<DiscoveredDatabase> {
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

    fn scan_filesystem(&self, existing: &[DatabaseInstance]) -> Vec<DiscoveredDatabase> {
        let mut found = Vec::new();

        // MySQL/MariaDB native data directories
        let mysql_paths = ["/var/lib/mysql", "/var/lib/mariadb"];
        for path in &mysql_paths {
            if let Ok(entries) = std::fs::read_dir(path) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    // Skip system databases
                    if name == "mysql" || name == "performance_schema" || name == "sys" || name == "information_schema" {
                        continue;
                    }
                    // Skip if already known as a Bennett container
                    if existing.iter().any(|db| db.name == name && db.source == DatabaseSource::Bennett) {
                        continue;
                    }
                    if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                        info!("Discovered native MySQL/MariaDB database '{}' in {}", name, path);
                        found.push(DiscoveredDatabase {
                            host: "127.0.0.1".to_string(),
                            port: 0, // 0 = not running, needs port assignment
                            db_type: "mysql".to_string(),
                            version_hint: Some(format!("native:{}", name)),
                        });
                    }
                }
            }
        }

        // PostgreSQL native data directories
        let pg_paths = ["/var/lib/postgres", "/var/lib/postgresql"];
        for path in &pg_paths {
            if let Ok(entries) = std::fs::read_dir(path) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if existing.iter().any(|db| db.name == name && db.source == DatabaseSource::Bennett) {
                        continue;
                    }
                    if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                        info!("Discovered native PostgreSQL cluster '{}' in {}", name, path);
                        found.push(DiscoveredDatabase {
                            host: "127.0.0.1".to_string(),
                            port: 0,
                            db_type: "postgres".to_string(),
                            version_hint: Some(format!("native:{}", name)),
                        });
                    }
                }
            }
        }

        // SQLite files
        if let Ok(entries) = std::fs::read_dir(".") {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.ends_with(".db") || name.ends_with(".sqlite") || name.ends_with(".sqlite3") {
                    if existing.iter().any(|db| db.name == name && db.source == DatabaseSource::Bennett) {
                        continue;
                    }
                    info!("Discovered SQLite database '{}'", name);
                    found.push(DiscoveredDatabase {
                        host: "127.0.0.1".to_string(),
                        port: 0,
                        db_type: "sqlite".to_string(),
                        version_hint: Some(name.clone()),
                    });
                }
            }
        }

        found
    }

    fn fingerprint(stream: &mut TcpStream, db_type: &str) -> Option<String> {
        use std::io::{Read, Write};

        match db_type {
            "postgres" => {
                let ssl_req = vec![
                    0x00, 0x00, 0x00, 0x08,
                    0x04, 0xD2, 0x16, 0x2F,
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
                let _ = stream.write_all(&[0u8; 16]);
                Some("unknown".to_string())
            }
            _ => None,
        }
    }

    pub fn to_instance(&self, disc: &DiscoveredDatabase) -> DatabaseInstance {
        DatabaseInstance {
            id: format!("local-{}-{}", disc.db_type, disc.port),
            name: disc.version_hint.clone().unwrap_or_else(|| format!("local-{}-{}", disc.db_type, disc.port)),
            db_type: disc.db_type.clone(),
            version: "unknown".to_string(),
            status: if disc.port == 0 { DatabaseStatus::Stopped } else { DatabaseStatus::Running },
            port: disc.port,
            size: "Unknown".to_string(),
            created_at: chrono::Local::now().format("%Y-%m-%d").to_string(),
            container_id: None,
            volume_name: None,
            env_vars: Vec::new(),
            source: DatabaseSource::Local,
            is_discovered: true,
            credentials: None,
            is_unlocked: false,
        }
    }
}
