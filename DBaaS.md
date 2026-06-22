# Bennett Studio DBaaS Improvement Roadmap

> **Target:** Transform Bennett Studio from a local database manager into a production-grade Database-as-a-Service platform.
> **Status:** Foundation built. Relay and sharing infrastructure exist. Ready for hardening and cloud scaling.
> **Last updated:** 2026-06-22

---

## Executive Summary

Bennett Studio currently manages local Docker containers for PostgreSQL, MySQL, MariaDB, SQLite, Redis, and MongoDB through a Rust engine (Axum API), Tauri desktop app, and React web UI. The platform has basic LAN sharing (mDNS), relay client infrastructure, session management, multiplex tunnels, JWT/API key auth, port allocation, and Docker runtime — but lacks public internet exposure, connection pooling, cloud hosting, automated backups, and enterprise-grade security.

This roadmap moves from **local dev tool → shareable DB URLs → cloud-hosted DBaaS → enterprise platform** in 5 phases.

---

## Current State Analysis

### What Works Today

| Component | Status | Location |
|-----------|--------|----------|
| Local Docker container management | ✅ Stable | `engine/src/runtime/container/docker.rs` |
| Rust engine with Axum API | ✅ Stable | `engine/src/` |
| Desktop app (Tauri) | ✅ Stable | `desktop/` |
| Web UI (React) | ✅ Stable | `web/` on port 5173 |
| LAN sharing via mDNS | ✅ Working | `engine/src/sharing/lan/` |
| Relay client infrastructure | ✅ Exists | `engine/src/sharing/relay/` |
| Session management | ✅ Exists | `engine/src/sharing/session/` |
| Multiplex tunnel | ✅ Exists | `engine/src/sharing/multiplex/` |
| JWT authentication | ✅ Exists | `engine/src/auth/jwt.rs` |
| API key system | ✅ Exists | `engine/src/auth/api_keys.rs` |
| Port allocator | ✅ Exists | `engine/src/runtime/port/` |
| CLI commands | ✅ Partial | `cli/src/commands/` |

### What's Missing for Production DBaaS

| Gap | Impact | Phase |
|-----|--------|-------|
| Public relay server deployed | Cannot share DBs over internet | 1 |
| Subdomain-based URL generation | No clean public URLs | 1 |
| Connection pooling (ProxySQL/PgBouncer) | Multiple users crash local DB | 2 |
| Cloud-hosted database instances | Users must install locally | 3 |
| Automated backups | Data loss risk | 3 |
| Usage-based billing | No revenue model | 5 |
| Team/organization support | No multi-user | 4 |
| SOC 2 compliance foundation | Enterprise blocker | 4 |
| Query audit logging | Security/compliance gap | 4 |
| Rate limiting & DDoS protection | Abuse vulnerability | 2 |

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                     CLIENT LAYER                           │
│  Web UI (React) │ Desktop (Tauri) │ CLI (Rust)            │
├─────────────────────────────────────────────────────────────┤
│                     API LAYER                                │
│  Axum REST API │ GraphQL (future) │ WebSocket (realtime)  │
├─────────────────────────────────────────────────────────────┤
│                   SERVICE LAYER                            │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────────────┐ │
│  │   Auth      │ │   Sharing   │ │   Database Ops      │ │
│  │  JWT/API    │ │  Relay/LAN  │ │  Create/Start/Stop  │ │
│  │  Keys/OAuth │ │  Tunnels    │ │  Backup/Restore     │ │
│  └─────────────┘ └─────────────┘ └─────────────────────┘ │
├─────────────────────────────────────────────────────────────┤
│                   RUNTIME LAYER                            │
│  Docker Engine │ Port Allocator │ Volume Manager          │
├─────────────────────────────────────────────────────────────┤
│                   DATA LAYER                                 │
│  SQLite (metadata) │ PostgreSQL (cloud) │ S3 (backups)    │
└─────────────────────────────────────────────────────────────┘
```

---

## Phase 1: Tunnel Infrastructure (Weeks 1–4)

### Goal
A public relay server accepts outbound tunnels from local engines and routes database traffic securely.

### 1.1 Deploy Relay Server

**Architecture:**
```
User's Laptop (Local Engine)
    │
    │ WebSocket/TCP tunnel (TLS 1.3, outbound)
    ▼
┌─────────────────────────────────────────┐
│  Cloud Relay Server (VPS)               │
│  • Public IP + DNS                      │
│  • Accepts tunnel registrations         │
│  • Routes TCP connections to tunnels    │
│  • Subdomain → tunnel mapping           │
└─────────────────────────────────────────┘
    ▲
    │ MySQL/PostgreSQL wire protocol
Remote User (Developer in Nairobi)
```

**Deployment target:**
| Provider | Spec | Cost | Why |
|----------|------|------|-----|
| Hetzner CX11 | 1 vCPU, 2GB RAM | €4.51/mo | Best price/performance |
| DigitalOcean Droplet | 1 vCPU, 512MB RAM | $6/mo | Simple, good docs |
| Oracle Cloud Free | 2 VMs, 1GB RAM each | $0 | Free forever tier |
| Fly.io | Shared CPU | $0 (sleeps) | Good for testing |

**Recommended:** Start with Oracle Cloud Free Tier (no cost) or Hetzner (reliable, cheap).

**Server setup:**
```bash
# Ubuntu 22.04 LTS
sudo apt update && sudo apt upgrade -y
sudo apt install -y docker.io docker-compose nginx certbot

# Create bennett user
sudo useradd -m -s /bin/bash bennett
sudo usermod -aG docker bennett

# Firewall
sudo ufw allow 22/tcp
sudo ufw allow 80/tcp
sudo ufw allow 443/tcp
sudo ufw allow 3306/tcp
sudo ufw allow 5432/tcp
sudo ufw allow 8080/tcp
sudo ufw enable
```

**DNS setup:**
```
*.bennett-studio.dev    A    RELAY_IP
relay.bennett-studio.dev A    RELAY_IP
```

**Process manager (systemd):**
```ini
# /etc/systemd/system/bennett-relay.service
[Unit]
Description=Bennett Studio Relay Server
After=network.target

[Service]
Type=simple
User=bennett
WorkingDirectory=/opt/bennett
ExecStart=/opt/bennett/bennett-relay --host 0.0.0.0 --port 8080
Restart=always
RestartSec=5
Environment=RUST_LOG=info

[Install]
WantedBy=multi-user.target
```

### 1.2 Wire Engine to Relay

**Files to modify:**
- `engine/src/sharing/relay/client.rs` — Add connection + registration logic
- `engine/src/sharing/relay/protocol.rs` — Define wire protocol messages
- `engine/src/sharing/relay/reconnect.rs` — Handle disconnections with backoff

**Configuration (add to `engine/.env`):**
```bash
# Relay connection
BENNETT_RELAY_HOST=relay.bennett-studio.dev
BENNETT_RELAY_PORT=443
BENNETT_RELAY_TOKEN=bsk_your_api_key_here
BENNETT_RELAY_TLS=true

# Local database to expose
BENNETT_SHARE_DB_HOST=127.0.0.1
BENNETT_SHARE_DB_PORT=3307  # Your Docker MariaDB port
```

**Registration flow:**
```rust
// engine/src/sharing/relay/client.rs
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use serde::{Serialize, Deserialize};

#[derive(Serialize)]
pub struct RegisterRequest {
    pub db_id: String,
    pub local_port: u16,
    pub db_type: DatabaseType,  // MySQL, PostgreSQL, etc.
    pub auth_token: String,
}

#[derive(Deserialize)]
pub struct RegisterResponse {
    pub public_url: String,     // db-abc123.bennett-studio.dev
    pub public_port: u16,
    pub share_token: String,
    pub expires_at: Option<DateTime<Utc>>,
}

pub struct RelayClient {
    ws: WebSocketStream,
    config: RelayConfig,
}

impl RelayClient {
    pub async fn connect(config: RelayConfig) -> Result<Self, RelayError> {
        let url = format!("wss://{}:{}/v1/tunnel", config.host, config.port);
        let (ws, _) = connect_async(url).await?;
        Ok(Self { ws, config })
    }

    pub async fn register_database(
        &mut self,
        db_id: &str,
        local_port: u16,
    ) -> Result<RegisterResponse, RelayError> {
        let request = RegisterRequest {
            db_id: db_id.to_string(),
            local_port,
            db_type: DatabaseType::MariaDB,
            auth_token: self.config.token.clone(),
        };

        self.ws.send(Message::Text(serde_json::to_string(&request)?)).await?;

        let response = self.ws.recv().await.ok_or(RelayError::ConnectionClosed)??;
        let response: RegisterResponse = serde_json::from_str(&response.to_string())?;

        Ok(response)
    }
}
```

### 1.3 Subdomain Generation

**Algorithm — cryptographically secure, collision-resistant:**
```rust
// relay/src/server/session.rs
use nanoid::nanoid;

const SAFE_ALPHABET: [char; 36] = [
    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm',
    'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9',
];

fn generate_subdomain() -> String {
    // 10 chars = 36^10 ≈ 3.6 quadrillion combinations
    format!("db-{}", nanoid!(10, &SAFE_ALPHABET))
}

// Result: db-abc123def456.bennett-studio.dev
```

**URL format by database type:**
| Type | URL Format | Default Port |
|------|-----------|-------------|
| MySQL/MariaDB | `mysql://db-xxx.bennett-studio.dev:3306` | 3306 |
| PostgreSQL | `postgresql://db-xxx.bennett-studio.dev:5432` | 5432 |
| MongoDB | `mongodb://db-xxx.bennett-studio.dev:27017` | 27017 |
| Redis | `redis://db-xxx.bennett-studio.dev:6379` | 6379 |

### 1.4 TCP Proxy Layer

**Critical challenge:** MySQL/PostgreSQL speak binary wire protocols, not HTTP. You cannot `nginx proxy_pass` them. You need raw TCP forwarding.

**Approach comparison:**

| Approach | Pros | Cons | Recommendation |
|----------|------|------|---------------|
| **Raw TCP tunnel** (start here) | Any protocol, simple, fast | No pooling, no query inspection | Phase 1 |
| **ProxySQL** (MySQL) | Connection pooling, query rules, caching | Extra dependency, config complexity | Phase 2 |
| **PgBouncer** (PostgreSQL) | Pooling, TLS, auth modes | PostgreSQL only | Phase 2 |
| **Custom protocol proxy** | Full control, query logging, RLS | Significant engineering effort | Phase 4 |

**Phase 1 implementation — raw TCP tunnel:**
```rust
// relay/src/server/proxy.rs
use tokio::io::{copy, AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

pub struct TcpProxy {
    tunnel_manager: Arc<TunnelManager>,
}

impl TcpProxy {
    pub async fn run(&self, bind_addr: &str) -> Result<(), ProxyError> {
        let listener = TcpListener::bind(bind_addr).await?;

        loop {
            let (client_stream, client_addr) = listener.accept().await?;
            let tunnel_mgr = self.tunnel_manager.clone();

            tokio::spawn(async move {
                if let Err(e) = Self::handle_connection(client_stream, client_addr, tunnel_mgr).await {
                    log::error!("Proxy error for {}: {}", client_addr, e);
                }
            });
        }
    }

    async fn handle_connection(
        mut client: TcpStream,
        addr: SocketAddr,
        tunnel_mgr: Arc<TunnelManager>,
    ) -> Result<(), ProxyError> {
        // Extract subdomain from SNI (TLS) or port mapping
        let subdomain = extract_subdomain(&client).await?;

        // Look up tunnel
        let tunnel = tunnel_mgr.get(&subdomain).await
            .ok_or(ProxyError::TunnelNotFound)?;

        // Split streams for bidirectional copy
        let (mut client_read, mut client_write) = client.split();
        let (mut tunnel_read, mut tunnel_write) = tunnel.split();

        // Bidirectional forward
        tokio::select! {
            res = copy(&mut client_read, &mut tunnel_write) => {
                log::debug!("Client → Tunnel: {} bytes", res.unwrap_or(0));
            }
            res = copy(&mut tunnel_read, &mut client_write) => {
                log::debug!("Tunnel → Client: {} bytes", res.unwrap_or(0));
            }
        }

        Ok(())
    }
}
```

**TLS termination:**
```rust
// relay/src/server/tls.rs
use tokio_rustls::{TlsAcceptor, server::TlsStream};
use rustls_pemfile;

pub fn load_tls_config(cert_path: &str, key_path: &str) -> Result<ServerConfig, TlsError> {
    let cert_file = File::open(cert_path)?;
    let key_file = File::open(key_path)?;

    let certs: Vec<Certificate> = rustls_pemfile::certs(&mut BufReader::new(cert_file))
        .filter_map(|r| r.ok())
        .map(Certificate)
        .collect();

    let key = rustls_pemfile::private_key(&mut BufReader::new(key_file))?
        .map(PrivateKey)
        .ok_or(TlsError::NoPrivateKey)?;

    let config = ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(certs, key)?;

    Ok(config)
}
```

### 1.5 Health & Monitoring

**Relay health endpoints:**
```rust
// relay/src/server/health.rs
pub async fn health_check() -> impl IntoResponse {
    Json(json!({
        "status": "healthy",
        "version": env!("CARGO_PKG_VERSION"),
        "active_tunnels": TUNNEL_MANAGER.active_count(),
        "total_connections": METRICS.total_connections(),
        "uptime_seconds": START_TIME.elapsed().as_secs(),
    }))
}

pub async fn metrics() -> impl IntoResponse {
    // Prometheus format
    format!(
        "# HELP bennett_relay_active_tunnels Number of active tunnels
         # TYPE bennett_relay_active_tunnels gauge
         bennett_relay_active_tunnels {}
",
        TUNNEL_MANAGER.active_count()
    )
}
```

---

## Phase 2: Authentication & Authorization (Weeks 5–8)

### 2.1 API Key System

**Goal:** Machine-to-machine authentication for tunnels, CLI, and SDKs.

**Schema (SQLite for local, PostgreSQL for cloud):**
```sql
CREATE TABLE api_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    key_hash VARCHAR(255) NOT NULL,  -- bcrypt hash of bsk_xxx
    name VARCHAR(100) NOT NULL,
    scopes JSONB DEFAULT '["read"]',
    rate_limit_per_minute INTEGER DEFAULT 60,
    created_at TIMESTAMP DEFAULT NOW(),
    expires_at TIMESTAMP,
    last_used_at TIMESTAMP,
    revoked_at TIMESTAMP,
    ip_allowlist JSONB  -- ["192.168.1.0/24", "10.0.0.0/8"]
);

CREATE INDEX idx_api_keys_user_id ON api_keys(user_id);
CREATE INDEX idx_api_keys_hash ON api_keys(key_hash);
```

**Key format:**
```
bsk_live_51H8m...32chars    # Production
bsk_test_51H8m...32chars    # Development/testing
```

**Validation flow:**
```rust
// engine/src/auth/api_keys.rs
pub async fn validate_api_key(
    key: &str,
    required_scope: &str,
    conn: &mut DbConnection,
) -> Result<ApiKeyContext, AuthError> {
    // 1. Extract prefix
    let (prefix, token) = key.split_at(8); // "bsk_live_"

    // 2. Hash lookup (constant-time comparison)
    let hash = bcrypt::hash(token, 12)?;
    let api_key = sqlx::query_as::<_, ApiKey>(
        "SELECT * FROM api_keys WHERE key_hash = $1 AND revoked_at IS NULL"
    )
    .bind(&hash)
    .fetch_optional(conn)
    .await?;

    let api_key = api_key.ok_or(AuthError::InvalidKey)?;

    // 3. Check expiration
    if let Some(expires) = api_key.expires_at {
        if Utc::now() > expires {
            return Err(AuthError::KeyExpired);
        }
    }

    // 4. Check scopes
    if !api_key.scopes.contains(&required_scope.to_string()) {
        return Err(AuthError::InsufficientScope);
    }

    // 5. Update last_used
    sqlx::query("UPDATE api_keys SET last_used_at = NOW() WHERE id = $1")
        .bind(api_key.id)
        .execute(conn)
        .await?;

    Ok(ApiKeyContext {
        user_id: api_key.user_id,
        scopes: api_key.scopes,
    })
}
```

### 2.2 Share-Level Permissions

**Permission model:**
```rust
// engine/src/sharing/policy/model.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SharePermission {
    ReadOnly,      // SELECT only, no DDL
    ReadWrite,     // SELECT, INSERT, UPDATE, DELETE
    Schema,        // + CREATE, ALTER, DROP tables
    Admin,         // Full access, can revoke share
}

#[derive(Debug, Clone)]
pub struct Share {
    pub id: String,
    pub db_id: String,
    pub owner_id: String,
    pub public_url: String,
    pub permission: SharePermission,
    pub allowed_ips: Option<Vec<IpNet>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub max_connections: Option<u32>,
    pub created_at: DateTime<Utc>,
}
```

**Database-level grants (stored in metadata DB):**
```sql
CREATE TABLE shares (
    id UUID PRIMARY KEY,
    db_id UUID NOT NULL REFERENCES databases(id),
    owner_id UUID NOT NULL REFERENCES users(id),
    subdomain VARCHAR(50) UNIQUE NOT NULL,
    permission VARCHAR(20) NOT NULL,
    allowed_ips JSONB,
    expires_at TIMESTAMP,
    max_connections INTEGER DEFAULT 10,
    connection_count INTEGER DEFAULT 0,
    created_at TIMESTAMP DEFAULT NOW(),
    revoked_at TIMESTAMP
);

CREATE TABLE share_connections (
    id UUID PRIMARY KEY,
    share_id UUID NOT NULL REFERENCES shares(id),
    client_ip INET NOT NULL,
    connected_at TIMESTAMP DEFAULT NOW(),
    disconnected_at TIMESTAMP,
    queries_executed INTEGER DEFAULT 0
);
```

### 2.3 Rate Limiting & Abuse Protection

**Implementation:**
```rust
// relay/src/server/rate_limit.rs
use governor::{Quota, RateLimiter};
use std::num::NonZeroU32;

pub struct RateLimitConfig {
    pub per_key: Quota,      // Per API key
    pub per_ip: Quota,       // Per IP address
    pub per_tunnel: Quota,   // Per shared database
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            per_key: Quota::per_minute(NonZeroU32::new(60).unwrap()),
            per_ip: Quota::per_minute(NonZeroU32::new(30).unwrap()),
            per_tunnel: Quota::per_minute(NonZeroU32::new(100).unwrap()),
        }
    }
}
```

**DDoS mitigation:**
- Cloudflare proxy in front of relay (free tier)
- Connection rate limiting per IP
- Automatic IP ban after 5 failed auth attempts
- Challenge-response for suspicious traffic

---

## Phase 3: Cloud-Hosted Databases (Weeks 9–16)

### 3.1 Engine as a Service

**Goal:** Users create databases in the cloud without installing Bennett Studio locally.

**Architecture:**
```
User signs up on bennett-studio.dev
    │
    ▼
┌─────────────────────────────────────────┐
│  Cloud Control Plane                    │
│  • User management                      │
│  • Database provisioning API            │
│  • Billing metering                     │
└─────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────┐
│  Kubernetes Cluster                     │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐ │
│  │ DB Pod  │ │ DB Pod  │ │ DB Pod  │ │
│  │MariaDB  │ │Postgres │ │Redis    │ │
│  │+ PVC    │ │+ PVC    │ │+ PVC    │ │
│  └─────────┘ └─────────┘ └─────────┘ │
└─────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────┐
│  Persistent Storage                     │
│  • Ceph RBD / Longhorn / cloud SSD    │
│  • Automated snapshots                │
└─────────────────────────────────────────┘
```

**Kubernetes resources:**
```yaml
# infra/k8s/db-statefulset.yaml
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: bennett-db-mariadb
spec:
  serviceName: bennett-db-mariadb
  replicas: 1
  selector:
    matchLabels:
      app: bennett-db-mariadb
  template:
    metadata:
      labels:
        app: bennett-db-mariadb
    spec:
      containers:
      - name: mariadb
        image: mariadb:11.2
        ports:
        - containerPort: 3306
        env:
        - name: MYSQL_ROOT_PASSWORD
          valueFrom:
            secretKeyRef:
              name: bennett-db-secrets
              key: root-password
        volumeMounts:
        - name: data
          mountPath: /var/lib/mysql
        resources:
          requests:
            memory: "512Mi"
            cpu: "250m"
          limits:
            memory: "1Gi"
            cpu: "500m"
  volumeClaimTemplates:
  - metadata:
      name: data
    spec:
      accessModes: ["ReadWriteOnce"]
      storageClassName: bennett-fast-ssd
      resources:
        requests:
          storage: 10Gi
```

**Provisioning API:**
```rust
// cloud/src/provision/mod.rs
pub async fn create_database(
    request: CreateDatabaseRequest,
    user: AuthenticatedUser,
) -> Result<Database, ProvisionError> {
    // 1. Validate user limits
    let current_dbs = db.count_user_databases(user.id).await?;
    if current_dbs >= user.plan.max_databases {
        return Err(ProvisionError::QuotaExceeded);
    }

    // 2. Select region (closest to user)
    let region = Region::closest_to(user.location);

    // 3. Generate credentials
    let password = generate_secure_password(32);
    let db_name = format!("bennett_{}", nanoid!(8));
    let db_user = format!("bennett_{}", nanoid!(8));

    // 4. Create K8s resources
    let k8s_client = kube_client.for_region(region);
    k8s_client.create_statefulset(&CreateStatefulSetRequest {
        name: &db_name,
        db_type: request.db_type,
        storage_gb: request.storage_gb,
        memory_mb: request.memory_mb,
        cpu_millicores: request.cpu_millicores,
        env_vars: hashmap! {
            "MYSQL_ROOT_PASSWORD".to_string() => password.clone(),
            "MYSQL_DATABASE".to_string() => db_name.clone(),
            "MYSQL_USER".to_string() => db_user.clone(),
            "MYSQL_PASSWORD".to_string() => generate_secure_password(32),
        },
    }).await?;

    // 5. Wait for readiness
    k8s_client.wait_for_ready(&db_name, Duration::from_secs(120)).await?;

    // 6. Store in metadata DB
    let database = Database {
        id: Uuid::new_v4(),
        name: request.name,
        db_type: request.db_type,
        region,
        host: format!("{}.bennett-db.svc.cluster.local", db_name),
        port: 3306,
        username: db_user,
        password: password.clone(), // encrypted at rest
        storage_gb: request.storage_gb,
        status: DatabaseStatus::Running,
        created_at: Utc::now(),
    };

    db.insert_database(&database).await?;

    Ok(database)
}
```

### 3.2 Multi-Region Support

**Regions:**
| Region Code | Location | Latency from Nairobi | Cloud Provider |
|-------------|----------|---------------------|---------------|
| eu-west1 | Frankfurt, Germany | ~120ms | Hetzner/GCP |
| eu-west2 | London, UK | ~130ms | AWS/GCP |
| us-east1 | Virginia, USA | ~220ms | AWS/GCP |
| me-west1 | Doha, Qatar | ~80ms | GCP |
| af-south1 | Johannesburg, SA | ~45ms | AWS |

**Region selection logic:**
```rust
// cloud/src/region/mod.rs
pub fn closest_region(lat: f64, lon: f64) -> Region {
    let regions = vec![
        Region::new("eu-west1", 50.1109, 8.6821),
        Region::new("eu-west2", 51.5074, -0.1278),
        Region::new("us-east1", 38.9072, -77.0369),
        Region::new("me-west1", 25.276987, 51.520008),
        Region::new("af-south1", -26.2041, 28.0473),
    ];

    regions.into_iter()
        .min_by_key(|r| haversine_distance(lat, lon, r.lat, r.lon))
        .unwrap_or(Region::EuWest1)
}
```

### 3.3 Automated Backups

**Strategy:**
| Backup Type | Frequency | Retention | Method |
|-------------|-----------|-----------|--------|
| Full dump | Daily at 2 AM UTC | 7 days | `pg_dump` / `mysqldump` |
| Incremental | Continuous | 24 hours | WAL archiving (PostgreSQL) / Binlog (MySQL) |
| Point-in-time | On-demand | 30 days | WAL + base backup restore |
| Cross-region | Weekly | 30 days | Replicate to S3 in different region |

**Implementation:**
```rust
// cloud/src/backup/engine.rs
pub struct BackupEngine {
    s3: S3Client,
    k8s: KubeClient,
}

impl BackupEngine {
    pub async fn run_daily_backup(&self, db: &Database) -> Result<Backup, BackupError> {
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let backup_key = format!("backups/{}/{}/{}.sql.gz", db.id, db.db_type, timestamp);

        // Execute dump inside pod
        let dump_output = self.k8s.exec(
            &db.pod_name,
            &db.namespace,
            &format!("mysqldump -u{} -p{} {} | gzip", db.username, db.password, db.name),
        ).await?;

        // Upload to S3
        self.s3.put_object(
            &backup_key,
            &dump_output,
            S3Metadata {
                db_id: db.id.to_string(),
                db_type: db.db_type.to_string(),
                timestamp: Utc::now(),
                checksum: sha256(&dump_output),
            },
        ).await?;

        Ok(Backup {
            id: Uuid::new_v4(),
            db_id: db.id,
            s3_key: backup_key,
            size_bytes: dump_output.len() as u64,
            checksum: sha256(&dump_output),
            created_at: Utc::now(),
        })
    }

    pub async fn restore_from_backup(
        &self,
        db: &Database,
        backup: &Backup,
    ) -> Result<(), BackupError> {
        // 1. Download backup
        let dump_data = self.s3.get_object(&backup.s3_key).await?;

        // 2. Verify checksum
        if sha256(&dump_data) != backup.checksum {
            return Err(BackupError::ChecksumMismatch);
        }

        // 3. Restore inside pod
        self.k8s.exec(
            &db.pod_name,
            &db.namespace,
            &format!("gunzip | mysql -u{} -p{} {}", db.username, db.password, db.name),
        )
        .stdin(dump_data)
        .await?;

        Ok(())
    }
}
```

---

## Phase 4: Enterprise Features (Weeks 17–24)

### 4.1 Team Workspaces

**Schema:**
```sql
CREATE TABLE organizations (
    id UUID PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    slug VARCHAR(100) UNIQUE NOT NULL,
    plan VARCHAR(50) DEFAULT 'free',
    billing_email VARCHAR(255),
    stripe_customer_id VARCHAR(100),
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW()
);

CREATE TABLE organization_members (
    org_id UUID REFERENCES organizations(id),
    user_id UUID REFERENCES users(id),
    role VARCHAR(50) CHECK (role IN ('owner', 'admin', 'developer', 'viewer')),
    invited_by UUID REFERENCES users(id),
    joined_at TIMESTAMP DEFAULT NOW(),
    PRIMARY KEY (org_id, user_id)
);

CREATE TABLE organization_databases (
    org_id UUID REFERENCES organizations(id),
    db_id UUID REFERENCES databases(id),
    added_by UUID REFERENCES users(id),
    added_at TIMESTAMP DEFAULT NOW(),
    PRIMARY KEY (org_id, db_id)
);
```

### 4.2 Audit Logging

**Schema (ClickHouse for production, SQLite for local):**
```sql
CREATE TABLE audit_logs (
    timestamp DateTime64(3),
    event_type LowCardinality(String),  -- 'query', 'connect', 'share_created', 'schema_change', 'backup'
    severity LowCardinality(String),     -- 'info', 'warning', 'error', 'critical'
    user_id UUID,
    org_id UUID,
    database_id UUID,
    share_id UUID,
    query String,
    query_hash String,  -- For deduplication
    rows_affected UInt32,
    rows_returned UInt32,
    duration_ms UInt32,
    client_ip IPv4,
    user_agent String,
    error_message String,
    INDEX idx_user (user_id) TYPE minmax GRANULARITY 4,
    INDEX idx_db (database_id) TYPE minmax GRANULARITY 4,
    INDEX idx_timestamp (timestamp) TYPE minmax GRANULARITY 4
) ENGINE = MergeTree()
PARTITION BY toYYYYMMDD(timestamp)
ORDER BY (timestamp, event_type, org_id)
TTL timestamp + INTERVAL 1 YEAR;
```

### 4.3 SOC 2 Foundation

**Trust Services Criteria mapping:**

| Criteria | Control | Implementation | Evidence |
|----------|---------|---------------|----------|
| **Security** | Access control | RBAC + MFA | IAM logs |
| | Encryption at rest | LUKS for volumes, AES-256 | Disk encryption config |
| | Encryption in transit | TLS 1.3 mandatory | SSL certificate inventory |
| | Penetration testing | Annual third-party pentest | Pentest report |
| **Availability** | 99.9% SLA | Multi-AZ deployment | Uptime monitoring |
| | Disaster recovery | Cross-region backups | DR runbook + test results |
| | Incident response | PagerDuty + runbooks | Incident logs |
| **Processing Integrity** | Input validation | Schema validation on all APIs | API test suite |
| | Audit trails | ClickHouse audit logs | Log retention policy |
| | Error handling | Circuit breakers, retries | Error rate metrics |
| **Confidentiality** | Data classification | Public/Internal/Confidential/Restricted | Classification policy |
| | NDAs | All employees + contractors | Signed NDAs on file |
| | Access reviews | Quarterly access recertification | Review documentation |
| **Privacy** | GDPR compliance | Data processing agreements | DPA signed |
| | Right to deletion | `DELETE /v1/users/me` endpoint | Deletion audit trail |
| | Data retention | Auto-purge after policy period | Retention config |

---

## Phase 5: Billing & Monetization (Weeks 25–32)

### 5.1 Usage Metrics

**Dimensions to track:**
| Metric | Unit | Source |
|--------|------|--------|
| Compute hours | Hours | Container runtime |
| Storage GB | GB | Volume usage |
| Bandwidth ingress | GB | Traefik/nginx logs |
| Bandwidth egress | GB | Traefik/nginx logs |
| DB queries | Count | ProxySQL/PgBouncer |
| DB connections | Count | Connection pooler |
| Shared connection minutes | Minutes | Tunnel manager |
| Backup storage | GB | S3 usage |

### 5.2 Pricing Tiers

| Tier | Price | DBs | Storage | Connections | Shares | Support |
|------|-------|-----|---------|-------------|--------|---------|
| **Free** | $0/mo | 1 | 1GB | 5 | 1 | Community |
| **Hobby** | $9/mo | 3 | 5GB | 20 | 5 | Email |
| **Pro** | $29/mo | 10 | 25GB | 100 | Unlimited | Priority email |
| **Team** | $99/mo/user | 25 | 100GB | 500 | Unlimited | Slack + phone |
| **Enterprise** | Custom | Unlimited | Unlimited | Unlimited | Unlimited | Dedicated TAM |

**Overage pricing:**
| Resource | Price |
|----------|-------|
| Additional storage | $0.10/GB/mo |
| Additional connections | $0.01/connection/hour |
| Bandwidth egress | $0.05/GB |
| Backup storage | $0.05/GB/mo |

### 5.3 Stripe Integration

```rust
// cloud/src/billing/stripe.rs
use stripe::{Client, CreateSubscription, CreateSubscriptionItems, PriceId};

pub struct BillingEngine {
    stripe: Client,
}

impl BillingEngine {
    pub async fn create_subscription(
        &self,
        org: &Organization,
        plan: &Plan,
    ) -> Result<Subscription, BillingError> {
        // 1. Create Stripe customer if not exists
        let customer_id = org.stripe_customer_id.clone()
            .unwrap_or_else(|| self.create_customer(org).await?.id);

        // 2. Create subscription
        let subscription = stripe::Subscription::create(
            &self.stripe,
            CreateSubscription {
                customer: Some(customer_id),
                items: Some(vec![
                    CreateSubscriptionItems {
                        price: Some(plan.stripe_price_id.clone()),
                        ..Default::default()
                    }
                ]),
                ..Default::default()
            }
        ).await?;

        // 3. Update org record
        db.update_org_stripe_subscription(org.id, &subscription.id).await?;

        Ok(subscription)
    }

    pub async fn report_usage(
        &self,
        org: &Organization,
        usage: &UsageReport,
    ) -> Result<(), BillingError> {
        for item in &usage.metered_items {
            stripe::UsageRecord::create(
                &self.stripe,
                &item.stripe_subscription_item_id,
                stripe::CreateUsageRecord {
                    quantity: item.quantity as u64,
                    timestamp: Some(stripe::Timestamp::from(Utc::now())),
                    action: Some(stripe::UsageRecordAction::Set),
                }
            ).await?;
        }
        Ok(())
    }
}
```

---

## Technical Debt & Hardening

### Immediate (Before Phase 1)
- [ ] Fix `mariadb-control` false-positive (apply oshocks fix to engine)
- [ ] Add health check endpoint to relay: `GET /health`
- [ ] Standardize error responses across all APIs (RFC 7807 Problem Details)
- [ ] Add request ID tracing (OpenTelemetry)
- [ ] Add structured logging (JSON format) with correlation IDs

### Short-term (Phase 1–2)
- [ ] Replace SQLite metadata store with PostgreSQL for cloud deployments
- [ ] Add connection pooling metrics (active, idle, waiting)
- [ ] Implement circuit breaker for relay reconnections (exponential backoff)
- [ ] Add rate limiting per API key and per IP
- [ ] Add request/response payload size limits
- [ ] Implement graceful shutdown (drain connections before exit)

### Long-term (Phase 3–5)
- [ ] Migrate from Docker to containerd for Kubernetes compatibility
- [ ] Implement WAL streaming for real-time replication (PostgreSQL)
- [ ] Add read replicas for read-heavy workloads
- [ ] Build custom query planner for distributed queries
- [ ] Implement column-level encryption for sensitive data
- [ ] Add query result caching layer (Redis)

---

## Risk Register

| Risk | Likelihood | Impact | Mitigation | Owner |
|------|-----------|--------|------------|-------|
| Relay server DDoS | Medium | High | Cloudflare proxy, rate limiting, connection limits | Phase 1 |
| Database container escape | Low | Critical | Rootless Docker, seccomp profiles, gVisor runtime | Phase 3 |
| Credential leak | Medium | High | Vault auto-rotation, memory encryption, short-lived tokens | Phase 2 |
| Data loss (no backup) | Low | Critical | Automated backups, cross-region replication, restore testing | Phase 3 |
| Compliance failure (SOC 2) | Medium | High | Document controls from day one, quarterly audits | Phase 4 |
| Cloud provider lock-in | Medium | Medium | Multi-cloud abstraction layer, portable K8s manifests | Phase 3 |
| Relay latency (Nairobi → EU) | High | Medium | Add af-south1 region, connection pooling, edge caching | Phase 3 |
| Dependency vulnerability | Medium | High | Dependabot, `cargo audit`, SBOM generation | Ongoing |

---

## Success Metrics

| Metric | Phase 1 Target | Phase 3 Target | Phase 5 Target |
|--------|---------------|---------------|---------------|
| Active shared databases | 50 | 1,000 | 10,000 |
| Daily API requests | 1,000 | 50,000 | 1,000,000 |
| Paid customers | 0 | 50 | 500 |
| MRR | $0 | $2,000 | $25,000 |
| Uptime | 99.0% | 99.9% | 99.99% |
| P95 query latency | <500ms | <200ms | <100ms |
| Backup success rate | — | 99.5% | 99.99% |
| Mean time to restore | — | <30 min | <10 min |

---

## This Week's Action Items

1. [ ] **Build relay binary:** `cargo build -p bennett-relay --release`
2. [ ] **Get VPS:** Sign up for Oracle Cloud Free Tier or Hetzner CX11
3. [ ] **Deploy relay:** Copy binary to VPS, configure systemd, start service
4. [ ] **Configure DNS:** Point `*.bennett-studio.dev` to relay IP
5. [ ] **Wire engine:** Add `BENNETT_RELAY_HOST` to `engine/.env`
6. [ ] **Test tunnel:** Start local MariaDB, register with relay, connect from phone/another machine
7. [ ] **Document:** Write `docs/relay-setup.md` with exact commands

**If relay build fails:** Share the error output and we'll fix it together.
**If you don't have a domain:** Use `nip.io` for testing (`34.205.42.171.nip.io`)

---

## Dependencies

| Phase | External Dependencies | Internal Dependencies |
|-------|----------------------|----------------------|
| 1 | VPS, domain, TLS certs | Relay crate, engine relay client |
| 2 | Redis (rate limiting), Cloudflare | Auth system, metadata DB |
| 3 | Kubernetes, S3/MinIO, Ceph/Longhorn | Cloud control plane, provisioner |
| 4 | ClickHouse, PagerDuty | Audit system, org management |
| 5 | Stripe, accounting software | Billing engine, usage metrics |

---

## Appendix A: Wire Protocol Deep Dive

### MySQL Wire Protocol (for tunnel implementation reference)

```
Client → Server: HandshakeRequest
  [4] packet_length
  [1] sequence_id
  [1] protocol_version (0x0a)
  [string] server_version
  [4] connection_id
  [8] auth_plugin_data_part_1
  [1] filler
  [2] capability_flags_1
  [1] character_set
  [2] status_flags
  [2] capability_flags_2
  [1] auth_plugin_data_len
  [10] reserved
  [string] auth_plugin_data_part_2
  [string] auth_plugin_name

Server → Client: HandshakeResponse
  [4] client_capabilities
  [4] max_packet_size
  [1] character_set
  [23] reserved
  [string] username
  [length-encoded] auth_response
  [string] database
  [string] auth_plugin_name

Then: COM_QUERY packets for SQL, ResultSet packets for responses
```

### PostgreSQL Wire Protocol

```
Client → Server: StartupMessage
  [4] length
  [4] protocol_version (0x0003_0000 for 3.0)
  [string, string] key-value pairs (user, database, etc.)

Server → Client: AuthenticationRequest
  [1] 'R' (message type)
  [4] length
  [4] auth_type (0=OK, 3=cleartext, 10=SASL, etc.)

Then: Query (Q), RowDescription (T), DataRow (D), CommandComplete (C)
```

---

## Appendix B: Database Engine Comparison

| Engine | Use Case | Container Image | Default Port | Pros | Cons |
|--------|----------|----------------|-------------|------|------|
| **PostgreSQL** | General purpose, complex queries | `postgres:16-alpine` | 5432 | ACID, JSONB, extensions | Heavier than MySQL |
| **MariaDB** | MySQL-compatible, web apps | `mariadb:11.2` | 3306 | Drop-in MySQL replacement | Fewer enterprise features |
| **MySQL** | Enterprise, Oracle ecosystem | `mysql:8.0` | 3306 | Widely supported | Oracle licensing |
| **SQLite** | Embedded, testing, edge | `n/a` (file-based) | — | Zero config, serverless | No concurrent writes |
| **Redis** | Caching, sessions, queues | `redis:7-alpine` | 6379 | In-memory speed | Data loss risk (no persistence) |
| **MongoDB** | Document store, flexible schema | `mongo:7.0` | 27017 | JSON documents, horizontal scaling | Memory hungry |

---

## Appendix C: Performance Benchmarks

### Local Development (Your Laptop)
| Operation | Latency |
|-----------|---------|
| DB container start | 2-5s |
| Query execution | 0.1-1ms |
| Backup (1GB DB) | 30-60s |
| Restore (1GB DB) | 45-90s |

### Cloud-Hosted (Frankfurt)
| Operation | Latency |
|-----------|---------|
| DB provision | 30-60s |
| Query execution (same region) | 5-20ms |
| Query execution (Nairobi → Frankfurt) | 150-300ms |
| Backup (1GB DB) | 60-120s |
| Cross-region replication lag | <1s |

---

*Document owner: Bennett Studio Engineering*
*Review cycle: Weekly during Phase 1–2, Bi-weekly during Phase 3–5*
*Next review: 2026-06-29*




## ALias
| Question                   | Your Stance                                                | My Recommendation                                                                                                                                                                                                       | Decision                                                                                                                                                                          |
| -------------------------- | ---------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Relay Infrastructure**   | Host everything locally, computer-to-computer via internet | **Direct P2P with STUN/TURN fallback** — no cloud relay needed initially. Use WebRTC data channels or QUIC for NAT traversal, with optional self-hosted TURN server later.                                              | **Self-hosted relay** on the sharer's machine + STUN for NAT hole punching. If both behind symmetric NAT, use a lightweight signaling server (can be self-hosted on either peer). |
| **Authentication**         | "Like OneDesk" — simple, minimal friction                  | **Signed JWT tokens embedded in the share URL** (e.g., `https://share.bennett.studio/db/abc-123?token=eyJhbG...`). No account required for guests. Host generates token with expiry + permissions.                      | **Signed URL with JWT** — guest clicks link, token validates, session established. Host can revoke by invalidating the token signature or session ID.                             |
| **Schema Autocomplete**    | Industry best                                              | **Host pushes schema metadata via gRPC streaming** on connection + incremental updates. Guest caches locally with TTL (e.g., 30s). This reduces latency and ensures consistency.                                        | **Push-based schema sync** with local guest cache.                                                                                                                                |
| **Connect-RPC Transport**  | Best and scalable                                          | **Connect-RPC over HTTP/2 primary, HTTP/1.1 fallback for `curl`, gRPC-Web for browser**. This gives maximum compatibility without sacrificing performance.                                                              | **Triple transport**: HTTP/2 (gRPC-native), HTTP/1.1 (Connect + `curl`), gRPC-Web (browser).                                                                                      |
| **Permission Granularity** | Best from day one, ready for scaling                       | **Row-Level Security (RLS) + Table-Level + Column-Level** from day one. Use a policy engine that injects `WHERE` clauses and column projections at query parse time.                                                    | **Full policy engine**: table allowlist/blocklist, column projection, RLS WHERE injection, query type restrictions (SELECT only, no DDL, no DML, etc.).                           |
| **Session Storage**        | Industry best, future-proof                                | **SQLite for local session state** (you already have it) + **Redis protocol-compatible embedded store** (like `mini-redis` or `sled`) for distributed/session state. For now, keep it in SQLite with TTL cleanup tasks. | **SQLite with TTL + background janitor** for now. Migration path to Redis/Valkey when you go multi-node.                                                                          |


## The Core Insight: What the "Relay Server" Actually Does
You asked what the server was supposed to do. In your original DBaaS doc, the relay was a cloud-hosted TCP proxy that solved NAT traversal (both computers behind routers). Since you want local/P2P, we replace that with:

┌─────────────────┐                      ┌─────────────────┐
│   Host Machine  │                      │  Guest Machine  │
│  (has database) │                      │ (has Bennett  │
│                 │                      │    Studio app)  │
│  ┌───────────┐  │    QUIC/WebRTC     │  ┌───────────┐  │
│  │  Engine   │  │◄───────────────────►│  │  Engine   │  │
│  │  (sharer) │  │   or direct TCP    │  │  (guest)  │  │
│  └─────┬─────┘  │   with STUN        │  └─────┬─────┘  │
│        │        │                      │        │        │
│  ┌─────▼─────┐  │                      │  ┌─────▼─────┐  │
│  │  MariaDB  │  │                      │  │  SQL UI   │  │
│  │  :3306    │  │                      │  │  Console  │  │
│  └───────────┘  │                      │  └───────────┘  │
└─────────────────┘                      └─────────────────┘

Guest Machine                    Host Machine
┌─────────────┐                ┌─────────────┐
│ Bennett App │ ──HTTP/2/gRPC──►│   Engine    │
│  (React)    │   or Connect    │  (Axum +    │
│             │   over TCP      │   tonic)    │
└─────────────┘                └──────┬──────┘
                                      │
                                ┌─────▼─────┐
                                │  MariaDB  │
                                │  :3306    │
                                └───────────┘

                                {
  "sub": "ACQPFDAQ7P",           // Bennett code
  "db_id": "uuid-of-database",     // Internal database ID
  "host_id": "fingerprint-of-host", // Host machine identity
  "perm": "ro",                   // ro = read-only, rw = read-write, adm = admin
  "tables": ["*"],                // ["*"] = all, or ["users", "orders"]
  "cols": null,                   // null = all columns, or { "users": ["id", "name"] }
  "rls": null,                    // null = no RLS, or "tenant_id = 5"
  "iat": 1719072000,              // Issued at
  "exp": 1719158400,              // Expires: 24h later
  "jti": "unique-session-id"      // For revocation
}



Your e-commerce app stores:
  BENNETT_SHARE_CODE=ACQPFDAQ7P
  BENNETT_API_ENDPOINT=https://share.bennett.studio

At runtime:
  GET https://share.bennett.studio/v1/resolve/ACQPFDAQ7P
  → Returns: { "host": "192.168.1.100", "port": 3001, "token": "eyJ..." }

Or even better — use a **stable subdomain**:
  https://ACQPFDAQ7P.share.bennett.studio → auto-resolves to host
  (like how ngrok does it: abc123.ngrok.io)

For e-commerce: Store only the CODE, resolve at app startup via API.
For SDKs: Built-in resolution — `new BennettClient({ code: "ACQPFDAQ7P" })` handles lookup.

┌─────────────────────────────────────────────────────────────────┐
│                     E-COMMERCE APP (Vercel/Render)              │
│                                                                 │
│  Stores: BENNETT_SHARE_CODE=ACQPFDAQ7P                          │
│          BENNETT_RESOLVER=https://resolve.bennett.studio        │
│                                                                 │
│  At startup:                                                    │
│    GET /v1/resolve/ACQPFDAQ7P                                  │
│    → Returns: {                                                 │
│         "direct_url": "https://192.168.1.100:3001",  ← LAN    │
│         "relay_url": "https://relay.bennett.studio/tunnel/xyz", │
│         "status": "host_online",                               │
│         "expires_at": "2026-06-23T16:00:00Z"                   │
│      }                                                          │
│                                                                 │
│  If direct_url fails (timeout 3s):                              │
│    Fallback to relay_url (WebSocket tunnel through any NAT)    │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  RESOLVER SERVICE (can be self-hosted on same box as e-com)   │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────────┐    │
│  │ Code→Host   │    │ Host Health │    │ Relay Coordination│   │
│  │ Registry    │◄──►│ Checker     │◄──►│ (STUN/TURN lite) │    │
│  │ (SQLite)    │    │ (ping every │    │                  │    │
│  │             │    │  30s)       │    │                  │    │
│  └─────────────┘    └─────────────┘    └─────────────────┘    │
└─────────────────────────────────────────────────────────────────┘
         ▲
         │
    ┌────┴────┐
    │  Host   │ ← Bennett Studio on your laptop
    │  pings  │   "I'm here, my IP is X, my tunnel port is Y"
    │resolver │
    │every 30s│
    └─────────┘

    // Bennett Studio host generates:
let share = Share {
    code: "ACQPFDAQ7P",           // Bennett code
    db_id: "uuid-of-database",
    token: jwt,                    // 24h expiry
    host_fingerprint: "abc123...",  // Unique host ID
};

// Host registers with resolver (if configured):
POST https://resolve.bennett.studio/v1/register
{
  "code": "ACQPFDAQ7P",
  "host_ip": "192.168.1.100",
  "host_port": 3001,
  "fingerprint": "abc123...",
  "tunnel_port": 3478  // For NAT traversal
}

// In your Next.js / Node.js app:
import { BennettClient } from '@bennett-studio/sdk';

const client = new BennettClient({
  code: process.env.BENNETT_SHARE_CODE,      // "ACQPFDAQ7P"
  resolver: process.env.BENNETT_RESOLVER,     // "https://resolve.bennett.studio"
});

// This happens automatically:
// 1. Resolve code to host
// 2. Try direct connection (LAN speed)
// 3. If direct fails, use WebSocket tunnel (works through NAT)
// 4. Cache the working URL for 5 minutes

// SDK handles this:
try {
  const result = await client.query("SELECT * FROM products");
} catch (err) {
  if (err.code === "HOST_OFFLINE") {
    // Show "Database host is offline" in your UI
    // Or queue for retry
  }
}