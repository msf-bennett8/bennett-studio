# Bennett Studio PaaS Improvement Roadmap

> **Target:** Layer application deployment on top of the DBaaS foundation, enabling users to deploy full-stack apps with `git push`.

---

## Prerequisites

Before starting PaaS, DBaaS Phase 1 (Tunnel Infrastructure) must be complete and stable:
- [x] Relay server deployed and accepting tunnels
- [x] Engine connects to relay reliably
- [x] Subdomain-based URL generation works
- [x] API key authentication is operational
- [x] Connection pooling (ProxySQL/PgBouncer) is in place

---

## Phase 1: Build Pipeline (Weeks 1-4)

### 1.1 Git Receiver

**Goal:** Accept code pushes and trigger builds.

**Architecture:**
```
User: git push bennett main
    ↓
SSH server or HTTP webhook
    ↓
Git receiver extracts repo, branch, commit
    ↓
Queues build job
```

**Files to create:**
- `paas/src/git/receiver.rs` — SSH/HTTP git receiver
- `paas/src/git/webhook.rs` — GitHub/GitLab webhook handler
- `paas/src/git/queue.rs` — Build job queue (Redis-backed)

**Implementation:**
```rust
// paas/src/git/receiver.rs
pub struct GitReceiver {
    ssh_port: u16,
    webhook_secret: String,
    queue: RedisQueue,
}

impl GitReceiver {
    pub async fn handle_push(&self, event: PushEvent) -> Result<BuildJob, Error> {
        let job = BuildJob {
            id: Uuid::new_v4(),
            repo_url: event.repo_url,
            branch: event.branch,
            commit: event.commit,
            user_id: event.user_id,
            created_at: Utc::now(),
        };
        self.queue.enqueue(job.clone()).await?;
        Ok(job)
    }
}
```

**Git URL format:**
```
git@git.bennett-studio.dev:user/app.git
https://git.bennett-studio.dev/user/app.git
```

### 1.2 Build System (Nixpacks Integration)

**Goal:** Auto-detect language/framework and build Docker image without Dockerfile.

**Supported languages (Phase 1):**
| Language | Detection File | Build Command | Runtime |
|----------|---------------|-------------|---------|
| Node.js | `package.json` | `npm install && npm run build` | `node` |
| PHP/Laravel | `composer.json` | `composer install` | `php-fpm` + `nginx` |
| Python | `requirements.txt` | `pip install` | `python` |
| Rust | `Cargo.toml` | `cargo build --release` | Binary |
| Go | `go.mod` | `go build` | Binary |

**Files to create:**
- `paas/src/build/detect.rs` — Language detection
- `paas/src/build/nixpacks.rs` — Nixpacks wrapper
- `paas/src/build/docker.rs` — Docker image build
- `paas/src/build/cache.rs` — Layer caching (S3/MinIO)

**Implementation:**
```rust
// paas/src/build/detect.rs
pub fn detect_language(path: &Path) -> Result<Language, Error> {
    if path.join("package.json").exists() {
        return Ok(Language::NodeJs);
    }
    if path.join("composer.json").exists() {
        return Ok(Language::Php);
    }
    if path.join("Cargo.toml").exists() {
        return Ok(Language::Rust);
    }
    if path.join("go.mod").exists() {
        return Ok(Language::Go);
    }
    if path.join("requirements.txt").exists() {
        return Ok(Language::Python);
    }
    Err(Error::UnknownLanguage)
}

pub fn generate_nixpacks_plan(lang: Language) -> NixpacksPlan {
    match lang {
        Language::NodeJs => NixpacksPlan {
            providers: vec!["node"],
            build_cmd: Some("npm run build".to_string()),
            start_cmd: Some("npm start".to_string()),
            ..Default::default()
        },
        Language::Php => NixpacksPlan {
            providers: vec!["php", "nginx"],
            build_cmd: None,
            start_cmd: Some("php-fpm".to_string()),
            ..Default::default()
        },
        // ...
    }
}
```

**Build flow:**
```bash
# 1. Clone repo to /tmp/build-{job-id}
# 2. Detect language → generate Nixpacks plan
# 3. nixpacks build . --name app-{job-id}
# 4. Tag and push to registry: registry.bennett-studio.dev/app-{job-id}:latest
# 5. Update job status → trigger deploy
```

### 1.3 Container Registry

**Goal:** Store built images for deployment.

**Options:**
| Registry | Cost | Integration |
|----------|------|-------------|
| Self-hosted (Docker Distribution) | Free (storage cost) | Full control |
| GitHub Container Registry | Free for public | OAuth integration |
| AWS ECR | $0.10/GB/mo | If on AWS |
| GCP Artifact Registry | $0.10/GB/mo | If on GCP |

**Recommendation:** Self-hosted registry using Docker Distribution on the same VPS cluster.

**Files to create:**
- `infra/registry/distribution.yml` — Registry config
- `paas/src/registry/client.rs` — Push/pull images

---

## Phase 2: Container Orchestration (Weeks 5-8)

### 2.1 Scheduler

**Goal:** Place containers on worker nodes based on resources.

**Architecture:**
```
┌─────────────────────────────────────────┐
│  Control Plane (API Server)              │
│  • Receives deploy requests              │
│  • Queries node capacity               │
│  • Assigns to least-loaded node        │
├─────────────────────────────────────────┤
│  Worker Node 1 (Hetzner CX21)          │
│  • Containerd runtime                  │
│  • 2 vCPU, 4GB RAM                     │
│  • Running: 5 app containers           │
├─────────────────────────────────────────┤
│  Worker Node 2                           │
│  • 2 vCPU, 4GB RAM                     │
│  • Running: 3 app containers             │
└─────────────────────────────────────────┘
```

**Files to create:**
- `paas/src/scheduler/mod.rs` — Scheduler core
- `paas/src/scheduler/nodes.rs` — Node registration and health
- `paas/src/scheduler/placement.rs` — Bin-packing algorithm

**Placement logic:**
```rust
// paas/src/scheduler/placement.rs
pub fn select_node(nodes: &[Node], requirements: &ResourceReq) -> Option<Node> {
    nodes.iter()
        .filter(|n| n.status == NodeStatus::Healthy)
        .filter(|n| n.available_cpu >= requirements.cpu)
        .filter(|n| n.available_memory >= requirements.memory)
        .min_by_key(|n| n.active_containers.len())
        .cloned()
}
```

### 2.2 Runtime Integration

**Goal:** Reuse existing Docker runtime from DBaaS for app containers.

**Files to modify:**
- `engine/src/runtime/container/docker.rs` — Add app container support
- `engine/src/runtime/port/` — Already exists, reuse

**Container spec for apps:**
```rust
pub struct AppContainer {
    pub image: String,
    pub env_vars: HashMap<String, String>,
    pub port: u16,           // Internal port (3000, 8000, etc.)
    pub host_port: u16,      // Allocated by port allocator
    pub memory_limit: u64,   // e.g., 512MB
    pub cpu_limit: f64,      // e.g., 0.5 cores
    pub volumes: Vec<Volume>,
}
```

### 2.3 Health Checks & Self-Healing

**Goal:** Restart failed containers automatically.

**Implementation:**
```rust
// paas/src/runtime/health.rs
pub async fn health_check_loop(containers: &ContainerManager) {
    loop {
        for container in containers.list().await {
            match check_health(&container).await {
                HealthStatus::Healthy => {},
                HealthStatus::Unhealthy => {
                    log::warn!("Container {} unhealthy, restarting", container.id);
                    containers.restart(&container.id).await;
                }
                HealthStatus::Dead => {
                    log::error!("Container {} dead, rescheduling", container.id);
                    containers.reschedule(&container.id).await;
                }
            }
        }
        tokio::time::sleep(Duration::from_secs(30)).await;
    }
}
```

---

## Phase 3: HTTP Routing & Ingress (Weeks 9-12)

### 3.1 Reverse Proxy (Traefik)

**Goal:** Route `app-xyz.bennett-studio.dev` to the correct container.

**Architecture:**
```
Internet User → https://app-xyz.bennett-studio.dev
    ↓
Cloudflare (DNS + DDoS protection)
    ↓
Traefik (reverse proxy)
    ↓
Discovers container label: traefik.http.routers.app-xyz
    ↓
Forwards to 10.0.1.5:32768 (container internal IP:port)
```

**Files to create:**
- `infra/traefik/traefik.yml` — Static config
- `infra/traefik/dynamic.yml` — Dynamic routing rules
- `paas/src/ingress/traefik.rs` — Traefik API client

**Container labels for routing:**
```yaml
labels:
  - "traefik.enable=true"
  - "traefik.http.routers.app-xyz.rule=Host(`app-xyz.bennett-studio.dev`)"
  - "traefik.http.routers.app-xyz.tls=true"
  - "traefik.http.routers.app-xyz.tls.certresolver=letsencrypt"
  - "traefik.http.services.app-xyz.loadbalancer.server.port=3000"
```

### 3.2 Custom Domains

**Goal:** Users point their own domain to their app.

**Flow:**
```bash
# User adds custom domain in dashboard
bennett domains add myapp.com --app app-xyz

# User adds DNS record:
# CNAME myapp.com → app-xyz.bennett-studio.dev

# Bennett verifies domain ownership (HTTP-01 challenge)
# Traefik generates TLS certificate automatically
```

**Files to create:**
- `paas/src/domains/mod.rs` — Domain management
- `paas/src/domains/verify.rs` — DNS verification

### 3.3 SSL/TLS Automation

**Goal:** Automatic HTTPS for all apps.

**Using Let's Encrypt + Traefik:**
```yaml
# traefik.yml
certificatesResolvers:
  letsencrypt:
    acme:
      email: ssl@bennett-studio.dev
      storage: /letsencrypt/acme.json
      tlsChallenge: {}
      httpChallenge:
        entryPoint: web
```

---

## Phase 4: Database Attachment (Weeks 13-16)

### 4.1 Auto-Link DB to App

**Goal:** When user deploys app, automatically attach a DBaaS database.

**Flow:**
```bash
# User deploys app
bennett deploy --app myapi --db mydb

# Bennett does:
# 1. Builds and runs app container
# 2. Injects DATABASE_URL env var pointing to mydb
# 3. App connects automatically
```

**Implementation:**
```rust
// paas/src/apps/attach_db.rs
pub async fn attach_database(
    app_id: &str,
    db_id: &str,
    permissions: DbPermission,
) -> Result<Attachment, Error> {
    let db = db_service.get(db_id).await?;
    let connection_string = db.get_connection_string(permissions).await?;

    app_service.set_env_var(app_id, "DATABASE_URL", &connection_string).await?;
    app_service.set_env_var(app_id, "DB_HOST", &db.host).await?;
    app_service.set_env_var(app_id, "DB_PORT", &db.port.to_string()).await?;
    app_service.set_env_var(app_id, "DB_NAME", &db.name).await?;
    app_service.set_env_var(app_id, "DB_USER", &db.user).await?;
    app_service.set_env_var(app_id, "DB_PASSWORD", &db.password).await?;

    // Restart app to pick up new env vars
    app_service.restart(app_id).await?;

    Ok(Attachment { app_id: app_id.to_string(), db_id: db_id.to_string() })
}
```

### 4.2 Environment Variables & Secrets

**Goal:** Securely inject config without hardcoding.

**Files to create:**
- `paas/src/apps/env.rs` — Env var management
- `paas/src/apps/secrets.rs` — Encrypted secret storage (Vault)

**Secret types:**
```rust
pub enum SecretType {
    PlainText,      // Non-sensitive (APP_NAME, PORT)
    Encrypted,      // Sensitive (API_KEY, PASSWORD)
    DatabaseUrl,    // Auto-generated from attached DB
    File,           // Mounted as file in container (TLS certs)
}
```

---

## Phase 5: Scaling & Advanced Features (Weeks 17-24)

### 5.1 Horizontal Scaling

**Goal:** Run multiple instances of an app behind a load balancer.

```bash
bennett scale app-xyz --replicas 3
```

**Implementation:**
```rust
// paas/src/apps/scale.rs
pub async fn scale_app(app_id: &str, replicas: u32) -> Result<(), Error> {
    let app = app_service.get(app_id).await?;
    let current = app.containers.len() as u32;

    if replicas > current {
        // Spin up new containers
        for _ in 0..(replicas - current) {
            scheduler.spawn_container(&app.spec).await?;
        }
    } else if replicas < current {
        // Gracefully drain and remove
        for container in &app.containers[replicas as usize..] {
            container.drain().await?;
            container.stop().await?;
        }
    }

    // Update Traefik to load balance across all containers
    ingress.update_backend(app_id, &app.containers).await?;
    Ok(())
}
```

### 5.2 Rolling Deployments

**Goal:** Deploy new version without downtime.

**Strategy:**
```
Current: [v1] [v1] [v1]
    ↓
Start: [v1] [v1] [v1] [v2]
    ↓
Health check v2 → pass
    ↓
Route traffic to v2
    ↓
Stop v1: [v2] [v2] [v2]
```

### 5.3 Log Aggregation

**Goal:** Centralized logs for all apps.

**Stack:**
- Vector (log shipper) on each node
- ClickHouse or Loki (log storage)
- Grafana (visualization)

**Files to create:**
- `infra/logging/vector.yml` — Vector config
- `paas/src/logs/mod.rs` — Log query API

---

## Integration with DBaaS

### Shared Components

| Component | DBaaS Use | PaaS Use |
|-----------|-----------|----------|
| Relay server | Tunnel local DBs | Tunnel local apps (dev mode) |
| Port allocator | DB container ports | App container ports |
| Docker runtime | DB containers | App containers |
| Auth (JWT/API keys) | DB access control | App deployment control |
| Web UI | DB management | App management |

### Unified Dashboard

```
Bennett Studio Dashboard
├── Databases
│   ├── mydb (PostgreSQL)
│   └── mydb-shared (MariaDB, shared URL active)
├── Apps
│   ├── myapi (Node.js, 2 replicas, healthy)
│   └── myfrontend (React, 1 replica, healthy)
└── Integrations
    ├── Stripe (billing)
    └── GitHub (OAuth)
```

---

## Pricing Model

| Tier | Price | Apps | DBs | Compute | Bandwidth |
|------|-------|------|-----|---------|-----------|
| **Free** | $0 | 1 | 1 | 512MB RAM, 0.5 vCPU | 10GB/mo |
| **Hobby** | $9/mo | 3 | 3 | 1GB RAM, 1 vCPU | 100GB/mo |
| **Pro** | $29/mo | 10 | 10 | 2GB RAM, 2 vCPU | 500GB/mo |
| **Team** | $99/mo | Unlimited | Unlimited | 8GB RAM, 4 vCPU | 2TB/mo |
| **Enterprise** | Custom | Unlimited | Unlimited | Dedicated nodes | Unlimited |

---

## Risk Register (PaaS Specific)

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Build system abuse (crypto mining) | High | High | Resource limits, sandboxing, rate limits |
| Container escape | Low | Critical | Rootless containers, seccomp, gVisor |
| Supply chain (malicious dependencies) | Medium | High | Scan images with Trivy before deploy |
| Resource exhaustion | Medium | High | Per-user quotas, auto-scaling limits |
| Cold start latency | Medium | Medium | Keep warm containers, pre-warm pools |

---

## Success Metrics

| Metric | Target (6 months) | Target (12 months) |
|--------|-------------------|-------------------|
| Deployed apps | 500 | 10,000 |
| Successful builds | 95% | 98% |
| Deploy time (git push → live) | <5 min | <2 min |
| App uptime | 99.9% | 99.95% |
| PaaS MRR | $2,000 | $50,000 |

---

## Next Steps (After DBaaS Phase 1)

1. [ ] Install Nixpacks binary on build workers
2. [ ] Create `paas` crate in workspace
3. [ ] Implement git webhook receiver
4. [ ] Build first successful app deployment (Node.js hello world)
5. [ ] Configure Traefik with Let's Encrypt
6. [ ] Test end-to-end: git push → build → deploy → browse URL

---

*Depends on: DBaaS Improvement Roadmap (Phase 1 complete)*
*Last updated: 2026-06-22*
*Owner: Bennett Studio Engineering*
