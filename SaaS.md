# Bennett Studio SaaS Improvement Roadmap

> **Target:** Transform Bennett Studio from infrastructure platform into a complete software ecosystem with managed services, marketplace, and business tools.

---

## Prerequisites

Before starting SaaS, both DBaaS and PaaS must be operational:
- [x] DBaaS: Public URLs, auth, backups, cloud-hosted DBs
- [x] PaaS: Git push deploy, auto-build, custom domains, scaling
- [x] Billing infrastructure (Stripe integration)
- [x] Team/organization support

---

## Phase 1: Managed Services Marketplace (Weeks 1-6)

### 1.1 One-Click Add-ons

**Goal:** Users add third-party services without leaving Bennett Studio.

**Initial services:**
| Service | Category | Integration Type | Provider |
|---------|----------|-----------------|----------|
| Email (SMTP) | Communication | SMTP relay | SendGrid, Mailgun, AWS SES |
| Object Storage | Storage | S3-compatible API | MinIO, AWS S3, Cloudflare R2 |
| Search | Database | HTTP API | Meilisearch, Algolia |
| Queue | Background jobs | Redis/AMQP | Redis, RabbitMQ, AWS SQS |
| Monitoring | Observability | Agent + API | Datadog, Grafana Cloud |
| CDN | Performance | DNS + proxy | Cloudflare, Fastly |
| Auth | Identity | OAuth/SAML | Clerk, Auth0, Firebase Auth |
| Payments | Commerce | SDK | Stripe, PayPal |

**Architecture:**
```
User clicks "Add Email" in dashboard
    ↓
Bennett provisions SendGrid subaccount via API
    ↓
Injects SMTP credentials as env vars into user's app
    ↓
User configures: EMAIL_FROM=noreply@myapp.com
```

**Files to create:**
- `saas/src/marketplace/mod.rs` — Marketplace core
- `saas/src/marketplace/catalog.rs` — Service definitions
- `saas/src/marketplace/provision.rs` — Auto-provisioning
- `saas/src/marketplace/billing.rs` — Usage tracking & billing

**Service definition:**
```rust
// saas/src/marketplace/catalog.rs
pub struct Service {
    pub id: String,              // "sendgrid-email"
    pub name: String,            // "SendGrid Email"
    pub category: Category,      // Communication
    pub icon: String,            // URL to icon
    pub description: String,
    pub plans: Vec<ServicePlan>,
    pub provisioner: Box<dyn Provisioner>,
}

pub struct ServicePlan {
    pub id: String,              // "free", "starter", "pro"
    pub name: String,
    pub price_monthly: u64,    // cents
    pub features: Vec<String>,
    pub limits: Limits,
}

pub trait Provisioner: Send + Sync {
    async fn provision(&self, org_id: &str, plan: &str) -> Result<ProvisionedService, Error>;
    async fn deprovision(&self, service_id: &str) -> Result<(), Error>;
    async fn rotate_credentials(&self, service_id: &str) -> Result<Credentials, Error>;
}
```

### 1.2 Self-Hosted Add-ons

**Goal:** Run add-ons as containers on Bennett infrastructure instead of third-party APIs.

**Self-hosted services:**
| Service | Container | Resource Usage |
|---------|-----------|---------------|
| Meilisearch | `getmeili/meilisearch:latest` | 512MB RAM |
| Redis | `redis:7-alpine` | 256MB RAM |
| MinIO | `minio/minio:latest` | 512MB RAM |
| Grafana | `grafana/grafana:latest` | 512MB RAM |
| Prometheus | `prom/prometheus:latest` | 1GB RAM |

**Provisioning flow:**
```rust
// saas/src/marketplace/provisioners/self_hosted.rs
pub struct SelfHostedProvisioner {
    scheduler: Arc<dyn Scheduler>,
    db_service: Arc<dyn DbService>,
}

impl Provisioner for SelfHostedProvisioner {
    async fn provision(&self, org_id: &str, plan: &str) -> Result<ProvisionedService, Error> {
        // 1. Allocate port
        let port = self.scheduler.allocate_port().await?;

        // 2. Spawn container
        let container = self.scheduler.spawn(ContainerSpec {
            image: "getmeili/meilisearch:latest".to_string(),
            env_vars: hashmap! {
                "MEILI_MASTER_KEY".to_string() => generate_key(),
            },
            port: 7700,
            host_port: port,
            memory_limit: 512 * 1024 * 1024, // 512MB
            ..Default::default()
        }).await?;

        // 3. Generate URL
        let url = format!("https://search-{}.bennett-studio.dev", container.id);

        // 4. Store in metadata DB
        self.db_service.create_addon(AddOn {
            org_id: org_id.to_string(),
            service_type: "meilisearch".to_string(),
            container_id: container.id,
            url: url.clone(),
            credentials: container.env_vars.clone(),
        }).await?;

        Ok(ProvisionedService {
            id: container.id,
            url,
            credentials: container.env_vars,
        })
    }
}
```

---

## Phase 2: Business Tools (Weeks 7-12)

### 2.1 Team Collaboration

**Goal:** Organizations manage users, roles, and permissions.

**Role hierarchy:**
```
Organization
├── Owner (1)
│   ├── Full access
│   ├── Billing management
│   └── Can delete org
├── Admin (1-5)
│   ├── Manage apps and DBs
│   ├── Invite/remove members
│   └── View billing
├── Developer (unlimited)
│   ├── Deploy apps
│   ├── Manage databases
│   └── View logs
└── Viewer (unlimited)
    ├── Read-only access
    └── View metrics
```

**Files to create:**
- `saas/src/teams/mod.rs` — Team management
- `saas/src/teams/roles.rs` — Role definitions
- `saas/src/teams/invites.rs` — Email invitations
- `saas/src/teams/audit.rs` — Activity logs

### 2.2 Usage Analytics Dashboard

**Goal:** Users see resource consumption, costs, and trends.

**Metrics to track:**
| Metric | Source | Granularity |
|--------|--------|-------------|
| Compute hours | Container runtime | Hourly |
| Storage GB | Volume usage | Daily |
| Bandwidth | Traefik logs | Hourly |
| DB queries | ProxySQL/PgBouncer | Per-query |
| Build minutes | Build pipeline | Per-build |
| Add-on usage | Service APIs | Varies |

**Dashboard UI:**
```
┌─────────────────────────────────────────┐
│  Usage This Month: $47.23               │
├─────────────────────────────────────────┤
│  Compute: 340 hrs ($34.00) ████████░░  │
│  Storage: 12 GB ($12.00)   ███░░░░░░░  │
│  Bandwidth: 45 GB ($1.23)  █░░░░░░░░░  │
│  Add-ons: $0.00            ░░░░░░░░░░  │
├─────────────────────────────────────────┤
│  Projected: $51.40 (based on trend)    │
└─────────────────────────────────────────┘
```

**Files to create:**
- `saas/src/analytics/mod.rs` — Metrics aggregation
- `saas/src/analytics/billing.rs` — Cost calculation
- `saas/src/analytics/forecast.rs` — Usage prediction

### 2.3 Advanced Billing

**Goal:** Flexible billing for complex usage patterns.

**Billing models:**
| Model | Use Case | Implementation |
|-------|----------|---------------|
| Flat monthly | Fixed resources | Stripe subscription |
| Usage-based | Variable compute | Metered billing |
| Prepaid credits | Enterprise | Credit system |
| Hybrid | Most common | Base fee + overage |

**Implementation:**
```rust
// saas/src/billing/engine.rs
pub struct BillingEngine {
    stripe: StripeClient,
    metrics: Arc<dyn MetricsStore>,
}

impl BillingEngine {
    pub async fn generate_invoice(&self, org_id: &str, period: BillingPeriod) -> Result<Invoice, Error> {
        let usage = self.metrics.get_usage(org_id, period).await?;

        let line_items = vec![
            LineItem {
                description: "Compute".to_string(),
                quantity: usage.compute_hours as f64,
                unit_price: 0.10, // $0.10/hour
                amount: usage.compute_hours as f64 * 0.10,
            },
            LineItem {
                description: "Storage".to_string(),
                quantity: usage.storage_gb as f64,
                unit_price: 1.00, // $1/GB/month
                amount: usage.storage_gb as f64 * 1.00,
            },
            // ...
        ];

        let total: f64 = line_items.iter().map(|l| l.amount).sum();

        // Create Stripe invoice
        let stripe_invoice = self.stripe.create_invoice(
            org_id,
            &line_items,
            total,
        ).await?;

        Ok(Invoice {
            id: stripe_invoice.id,
            line_items,
            total,
            status: InvoiceStatus::Open,
            due_date: period.end + Duration::days(7),
        })
    }
}
```

---

## Phase 3: Developer Experience (Weeks 13-20)

### 3.1 CLI v2

**Goal:** Powerful command-line interface for power users.

**Commands:**
```bash
# Database management
bennett db create mydb --type postgres --region eu-west1
bennett db list
bennett db connect mydb  # Opens psql/mysql CLI
bennett db share mydb --readonly --expires 24h
bennett db backup mydb --name "before-migration"
bennett db restore mydb --from backup-2026-06-22

# App management
bennett app create myapi --git https://github.com/user/repo.git
bennett app deploy myapi --branch main
bennett app logs myapi --follow
bennett app scale myapi --replicas 3
bennett app env set myapi API_KEY=secret
bennett app domain add myapi myapp.com

# Marketplace
bennett addon create search --type meilisearch
bennett addon list
bennett addon connect search --app myapi

# Teams
bennett team invite developer@example.com --role developer
bennett team list

# Billing
bennett billing usage
bennett billing invoices
```

**Files to create:**
- `cli/src/commands/db.rs` — DB commands
- `cli/src/commands/app.rs` — App commands
- `cli/src/commands/addon.rs` — Add-on commands
- `cli/src/commands/team.rs` — Team commands
- `cli/src/commands/billing.rs` — Billing commands

### 3.2 API & SDKs

**Goal:** Programmatic access to all Bennett features.

**REST API:**
```
GET  /v1/databases
POST /v1/databases
GET  /v1/databases/{id}
DELETE /v1/databases/{id}
POST /v1/databases/{id}/share

GET  /v1/apps
POST /v1/apps
GET  /v1/apps/{id}/logs
POST /v1/apps/{id}/deploy
POST /v1/apps/{id}/scale

GET  /v1/addons
POST /v1/addons
GET  /v1/teams
POST /v1/teams/invites

GET  /v1/billing/usage
GET  /v1/billing/invoices
```

**SDKs:**
| Language | Package | Features |
|----------|---------|----------|
| JavaScript/TypeScript | `@bennett-studio/sdk` | Full API wrapper |
| Python | `bennett-studio` | Full API wrapper |
| Go | `github.com/bennett/studio-go` | Full API wrapper |
| Rust | `bennett-studio` | Native, async |

**SDK example (TypeScript):**
```typescript
import { BennettStudio } from '@bennett-studio/sdk';

const bs = new BennettStudio({ apiKey: 'bsk_xxxxxxxx' });

// Create database
const db = await bs.databases.create({
  name: 'mydb',
  type: 'postgres',
  region: 'eu-west1',
});

// Deploy app
const app = await bs.apps.create({
  name: 'myapi',
  gitUrl: 'https://github.com/user/api.git',
  env: {
    DATABASE_URL: db.connectionString,
  },
});

// Scale
await bs.apps.scale(app.id, { replicas: 3 });
```

### 3.3 GitHub/GitLab Integration

**Goal:** Deep integration with Git providers.

**Features:**
- OAuth login
- Auto-deploy on push
- PR preview environments
- Status checks
- Secrets sync from GitHub Actions

**PR Preview Flow:**
```
User opens PR #42 on GitHub
    ↓
Webhook → Bennett creates preview app
    ↓
Deploys branch `feature/new-ui`
    ↓
URL: myapp-pr-42.bennett-studio.dev
    ↓
GitHub status check: "Preview ready"
    ↓
PR merged → auto-deploy to production
    ↓
Preview app destroyed
```

---

## Phase 4: Enterprise & Compliance (Weeks 21-30)

### 4.1 Single Sign-On (SSO)

**Goal:** Enterprise customers use their identity provider.

**Supported providers:**
| Provider | Protocol | Implementation |
|----------|----------|---------------|
| Okta | SAML 2.0 | `saml2` crate |
| Google Workspace | OAuth 2.0 / OIDC | `openidconnect` crate |
| Microsoft Entra | SAML 2.0 / OIDC | `saml2` crate |
| JumpCloud | SAML 2.0 | `saml2` crate |

**Files to create:**
- `saas/src/sso/mod.rs` — SSO core
- `saas/src/sso/saml.rs` — SAML implementation
- `saas/src/sso/oidc.rs` — OIDC implementation

### 4.2 SOC 2 Type II

**Goal:** Enterprise sales require SOC 2 compliance.

**Trust Services Criteria:**
| Criteria | Implementation |
|----------|---------------|
| Security | Encryption, access controls, penetration testing |
| Availability | 99.99% SLA, redundant infrastructure |
| Processing Integrity | Input validation, audit trails, error handling |
| Confidentiality | Data classification, access logs, NDAs |
| Privacy | GDPR compliance, data retention policies |

**Required documentation:**
- Information Security Policy
- Access Control Policy
- Change Management Policy
- Incident Response Plan
- Business Continuity Plan
- Risk Assessment
- Vendor Management Policy

### 4.3 Data Residency & Compliance

**Goal:** Meet regional data requirements.

| Region | Requirement | Implementation |
|--------|-------------|---------------|
| EU | GDPR | Data in EU regions, DPA signed |
| US | CCPA | California data handling |
| UK | UK GDPR | UK-specific data handling |
| Healthcare | HIPAA | BAA, encryption, access logs |
| Finance | PCI DSS | Isolated infrastructure, audits |

**Files to create:**
- `saas/src/compliance/mod.rs` — Compliance framework
- `saas/src/compliance/gdpr.rs` — GDPR-specific features
- `saas/src/compliance/audit.rs` — Audit trail generation

---

## Phase 5: Ecosystem & Platform (Weeks 31-40)

### 5.1 Bennett Studio API Platform

**Goal:** Third parties build on Bennett Studio.

**Partner API:**
```rust
// Partners can:
// 1. Register their service in marketplace
// 2. Receive webhooks for provisioning events
// 3. Bill through Bennett's Stripe account (revenue share)
// 4. Access aggregated usage metrics

pub struct PartnerApi {
    pub register_service: Endpoint,
    pub provision_webhook: WebhookConfig,
    pub billing_integration: BillingIntegration,
    pub analytics: PartnerAnalytics,
}
```

### 5.2 Bennett Studio Functions (Serverless)

**Goal:** Deploy functions without managing containers.

**Architecture:**
```
User writes: functions/hello.ts
    ↓
Bennett bundles with esbuild
    ↓
Deploys to Firecracker microVM
    ↓
URL: https://fn-abc123.bennett-studio.dev/hello
    ↓
Cold start: <50ms
    ↓
Scales to zero when idle
```

**Supported runtimes:**
| Runtime | Version | Cold Start |
|---------|---------|-----------|
| Node.js | 20.x | 30ms |
| Python | 3.12 | 40ms |
| Rust | 1.78 | 10ms |
| Go | 1.22 | 15ms |

### 5.3 Bennett Studio AI (Future)

**Goal:** AI-assisted development and operations.

**Features:**
- **SQL Copilot:** Natural language to SQL queries
- **Schema Optimizer:** Suggests indexes, normalization
- **Anomaly Detection:** Alerts on unusual query patterns
- **Auto-Scaling:** ML-based predictive scaling
- **Code Review:** AI review of deployed code

---

## Unified Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      CLIENT LAYER                            │
│  Web UI (React) │ CLI (Rust) │ SDKs (JS, Python, Go, Rust) │
├─────────────────────────────────────────────────────────────┤
│                     API GATEWAY                              │
│  Rate limiting │ Auth (JWT/API keys) │ Request routing      │
├─────────────────────────────────────────────────────────────┤
│                     SERVICE LAYER                            │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────────────┐   │
│  │   DBaaS     │ │    PaaS     │ │       SaaS          │   │
│  │  Databases  │ │    Apps     │ │  Marketplace        │   │
│  │  Sharing    │ │   Builds    │ │  Teams              │   │
│  │  Backups    │ │   Scaling   │ │  Billing            │   │
│  │  Migration  │ │   Routing   │ │  Analytics          │   │
│  └─────────────┘ └─────────────┘ └─────────────────────┘   │
├─────────────────────────────────────────────────────────────┤
│                   INFRASTRUCTURE LAYER                       │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────────────┐   │
│  │   Docker    │ │ Kubernetes  │ │   Cloud Providers   │   │
│  │  Runtime    │ │  Scheduler  │ │  (AWS, GCP, Hetzner)│   │
│  └─────────────┘ └─────────────┘ └─────────────────────┘   │
├─────────────────────────────────────────────────────────────┤
│                     DATA LAYER                               │
│  PostgreSQL (metadata) │ ClickHouse (analytics) │ S3 (files)│
└─────────────────────────────────────────────────────────────┘
```

---

## Revenue Model

| Stream | Description | Margin |
|--------|-------------|--------|
| DBaaS subscriptions | Monthly DB fees | 30-40% |
| PaaS subscriptions | Monthly compute fees | 20-30% |
| Marketplace commissions | 20% of add-on revenue | 20% |
| Enterprise support | Dedicated support, SLA | 70%+ |
| Professional services | Migration, consulting | 80%+ |

**Projected Revenue (Year 3):**
| Segment | Monthly Revenue |
|---------|----------------|
| DBaaS | $150,000 |
| PaaS | $300,000 |
| Marketplace | $50,000 |
| Enterprise | $200,000 |
| **Total MRR** | **$700,000** |

---

## Risk Register (SaaS Specific)

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Marketplace partner churn | Medium | Medium | Multi-provider for each category |
| Enterprise sales cycle | High | Medium | Self-serve → sales assist model |
| Feature creep | High | Medium | Strict OKRs, quarterly planning |
| Multi-tenancy isolation failure | Low | Critical | Strong container isolation, gVisor |
| Vendor lock-in perception | Medium | Medium | Export tools, open-source core |

---

## Success Metrics

| Metric | Target (Year 1) | Target (Year 2) | Target (Year 3) |
|--------|----------------|----------------|----------------|
| Total users | 10,000 | 100,000 | 500,000 |
| Paying customers | 500 | 5,000 | 25,000 |
| MRR | $25,000 | $250,000 | $700,000 |
| Marketplace services | 5 | 20 | 50 |
| Enterprise customers | 5 | 25 | 100 |
| NPS score | 40 | 50 | 60 |

---

## Next Steps (After PaaS Phase 1)

1. [ ] Design marketplace UI mockups
2. [ ] Implement SendGrid email add-on as proof-of-concept
3. [ ] Create partner API specification
4. [ ] Build usage analytics aggregation pipeline
5. [ ] Design team/organization data model
6. [ ] Start SOC 2 documentation

---

*Depends on: DBaaS Improvement Roadmap (complete) + PaaS Improvement Roadmap (Phase 1 complete)*
*Last updated: 2026-06-22*
*Owner: Bennett Studio Engineering*
