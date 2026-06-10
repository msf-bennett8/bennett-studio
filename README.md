# Bennett Studio

> **The Database Workspace for Modern Developers**
>
> Install, manage, query, and share local databases — all from one unified interface.

---

## Table of Contents

1. [Overview](#overview)
2. [Architecture](#architecture)
3. [Core Features](#core-features)
4. [Sharing & Collaboration](#sharing--collaboration)
5. [Security](#security)
6. [Technology Stack](#technology-stack)
7. [Data Flows](#data-flows)
8. [Roadmap](#roadmap)
9. [Installation](#installation)
10. [Contributing](#contributing)
11. [License](#license)

---

## Overview

Bennett Studio is an open-source, enterprise-grade database management platform designed for developers who work with local databases. It eliminates the friction of installing, configuring, and managing PostgreSQL, MySQL, MariaDB, SQLite, Redis, and MongoDB — while enabling secure, real-time sharing of database instances across LAN and remote networks.

Unlike traditional database GUIs that require manual installation and configuration, Bennett Studio provides a **headless engine** that manages database lifecycles through Docker containers, with multiple client interfaces (Desktop, Web, CLI, VS Code Extension) connecting via gRPC and WebSocket protocols.

### Key Differentiators

- **Zero-Config Database Provisioning**: One-click installation of any supported database version via Docker
- **Headless Architecture**: Engine runs independently; UI is just one of many possible clients
- **Secure Tunnel Sharing**: Share local databases with teammates without firewall configuration or VPNs
- **Schema-Aware Permissions**: Granular access control at table and row level for shared connections
- **Multi-Client Ecosystem**: Desktop app, web interface, CLI, and IDE extensions — all speaking the same protocol
- **Enterprise-Ready**: Audit logging, credential vaulting, and RBAC built-in from day one

---

## Architecture

Bennett Studio follows a **layered, headless architecture** that separates the database runtime engine from presentation layers, enabling maximum flexibility and future extensibility.

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      CLIENT LAYER                            │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐  │
│  │ Web App  │  │ Desktop  │  │ CLI      │  │ VS Code  │  │
│  │ (React)  │  │ (Tauri)  │  │ (Rust)   │  │ Extension│  │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘  │
│       └──────────────┴──────────────┴──────────────┘        │
│                      WebSocket / gRPC                        │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                     CONTROL PLANE                            │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │ API Gateway  │  │ Auth & RBAC  │  │ Telemetry    │     │
│  │ (Axum)       │  │ (JWT + API   │  │ (OpenTelemetry│    │
│  │              │  │  Keys)       │  │  + Prometheus)│     │
│  └──────────────┘  └──────────────┘  └──────────────┘     │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │ Connection   │  │ Query Engine │  │ Export       │     │
│  │ Pool Manager │  │ (SQL parser, │  │ Orchestrator │     │
│  │ (deadpool)   │  │  query plan) │  │              │     │
│  └──────────────┘  └──────────────┘  └──────────────┘     │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │ Credential   │  │ Schema Policy│  │ Migration    │     │
│  │ Vault        │  │ Engine       │  │ Runner       │     │
│  │ (per-share)  │  │ (RLS, views) │  │ (migra,     │     │
│  │              │  │              │  │  skeema)     │     │
│  └──────────────┘  └──────────────┘  └──────────────┘     │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                     RUNTIME ENGINE                           │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │ Container    │  │ Process      │  │ Native       │     │
│  │ Runtime      │  │ Supervisor   │  │ Binary       │     │
│  │ (Docker/     │  │ (tokio +     │  │ Manager      │     │
│  │  Podman)     │  │  health chk) │  │ (download +  │     │
│  │              │  │              │  │  verify)     │     │
│  └──────────────┘  └──────────────┘  └──────────────┘     │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │ Volume       │  │ Network      │  │ Port         │     │
│  │ Manager      │  │ Isolation    │  │ Allocator    │     │
│  │ (bind mounts,│  │ (localhost   │  │ (ephemeral   │     │
│  │  named vols) │  │  only, no    │  │  ports)      │     │
│  │              │  │  remote)     │  │              │     │
│  └──────────────┘  └──────────────┘  └──────────────┘     │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │ WebAssembly  │  │ Plugin       │  │ AI Assistant │     │
│  │ Runtime      │  │ Loader       │  │ Hook         │     │
│  │ (wasmtime)   │  │ (OCI compat) │  │ (schema API) │     │
│  └──────────────┘  └──────────────┘  └──────────────┘     │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                     DATABASE INSTANCES                       │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐    │
│  │ Postgres │  │ MySQL    │  │ MariaDB  │  │ SQLite   │    │
│  │ 15/16/17 │  │ 8.0/8.4  │  │ 11.x     │  │ (file)   │    │
│  │ (Docker) │  │ (Docker) │  │ (Docker) │  │ (native) │    │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘    │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐                   │
│  │ Redis    │  │ MongoDB  │  │ Future...│                   │
│  │ (Docker) │  │ (Docker) │  │ (plugin) │                   │
│  └──────────┘  └──────────┘  └──────────┘                   │
└─────────────────────────────────────────────────────────────┘
```

### Sharing Layer Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     SHARING LAYER                            │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │ Session      │  │ Credential   │  │ Schema Policy│     │
│  │ Manager      │  │ Vault        │  │ Engine       │     │
│  │ (UUID maps)  │  │ (per-share   │  │ (table/row   │     │
│  │              │  │  tokens)     │  │  level RLS)  │     │
│  └──────────────┘  └──────────────┘  └──────────────┘     │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │ Multiplex    │  │ Offline      │  │ Audit        │     │
│  │ Tunnel (1:N) │  │ Buffer       │  │ Log          │     │
│  │              │  │ (reconnect)  │  │ (SQLite/     │     │
│  │              │  │              │  │  ClickHouse) │     │
│  └──────────────┘  └──────────────┘  └──────────────┘     │
│  ┌──────────────┐  ┌──────────────┐                         │
│  │ LAN Discovery│  │ Relay Client │                         │
│  │ (mDNS/Bonjour│  │ (WebSocket   │                         │
│  │              │  │  outbound)   │                         │
│  └──────────────┘  └──────────────┘                         │
└─────────────────────────────────────────────────────────────┘
```

---

## Core Features

### 1. Database Lifecycle Management

| Feature | Description |
|---------|-------------|
| **One-Click Install** | Install PostgreSQL, MySQL, MariaDB, Redis, or MongoDB via Docker with automatic version detection and dependency resolution |
| **Version Management** | Run multiple versions side-by-side (e.g., PostgreSQL 15, 16, 17) with isolated data volumes |
| **Auto-Configuration** | Automatic port allocation, credential generation, and health checks |
| **Native SQLite** | Zero-container overhead for SQLite — direct file access with full GUI support |
| **Process Supervision** | Automatic restart on crash, resource limits enforcement, graceful shutdown |

### 2. Query & Data Interface

| Feature | Description |
|---------|-------------|
| **Monaco SQL Editor** | Syntax highlighting, autocomplete, error squiggles, and schema-aware IntelliSense |
| **Virtualized Data Grid** | Handle millions of rows with TanStack Table virtual scrolling |
| **Query Plan Visualization** | EXPLAIN output rendered as interactive trees |
| **Multi-Result Tabs** | Run multiple queries simultaneously, compare results side-by-side |
| **Export Formats** | SQL dumps, CSV, JSON, Parquet, and Excel with streaming for large datasets |
| **Import Wizard** | CSV/JSON to table mapping with type inference and batch insertion |

### 3. Schema Management

| Feature | Description |
|---------|-------------|
| **Visual Schema Designer** | ERD-style table relationship mapping with drag-and-drop foreign keys |
| **Migration Runner** | Built-in support for migra, skeema, and custom migration scripts |
| **Schema Diff** | Compare two databases or versions and generate migration scripts |
| **Index Advisor** | Query performance suggestions based on slow query log analysis |

---

## Sharing & Collaboration

Bennett Studio provides **two sharing modes** optimized for different use cases:

### Mode 1: LAN Sharing (Same Network)

- **Auto-discovery** via mDNS/Bonjour — no manual IP entry
- **Direct connection** — traffic stays local, zero latency
- **No relay required** — peer-to-peer over local network

### Mode 2: Remote Sharing (Different Locations)

- **Reverse tunnel** — host initiates outbound WebSocket to relay server; no firewall holes needed
- **UUID-based URLs** — `https://share.bennett.studio/db/abc-123-def`
- **Connection multiplexing** — single tunnel supports multiple authenticated peers (1:N)
- **Ephemeral sessions** — share dies when host disconnects (default, screen-sharing model)
- **Persistent sessions** — query buffer/queue with reconnect resume (async team model)

### Sharing Security Model

| Layer | Implementation |
|-------|----------------|
| **Authentication** | JWT tokens for both host and guests; API keys for service accounts |
| **Credential Vault** | Per-share, auto-rotated database credentials with TTL |
| **Schema Policy** | Row-level security injection via query rewriting; table allowlists/blocklists |
| **Encryption** | TLS 1.3 over WebSocket; WireGuard option for high-throughput scenarios |
| **Audit Logging** | Every query, connection, and schema change logged with user attribution |
| **Rate Limiting** | Per-share bandwidth and query rate limits |

---

## Security

### Threat Model

| Threat | Mitigation |
|--------|------------|
| Container escape | Rootless Docker, seccomp profiles, cgroup v2 resource limits |
| Credential leak | Vault auto-rotation, no creds in URLs, memory-zeroing on share end |
| SQL injection | Query parsing with sqlparser-rs; parameterized query enforcement for shared sessions |
| Man-in-the-middle | Certificate pinning for relay; mTLS for enterprise deployments |
| Data exfiltration | Schema policy engine blocks `COPY TO`, `\copy`, and `INTO OUTFILE` for read-only shares |
| Privilege escalation | RBAC with principle of least privilege; no superuser access for shared connections |

### Compliance Roadmap

- **SOC 2 Type II** — Audit logging, access controls, change management
- **GDPR** — Data residency controls, right-to-erasure for audit logs
- **HIPAA** — BAA-ready encryption and audit trails (enterprise tier)

---

## Technology Stack

| Component | Technology | Rationale |
|-----------|-----------|-----------|
| **Control Plane** | Rust (Axum) | Memory-safe, async-native, single binary deployment |
| **Protocol** | gRPC (tonic) + WebSocket | gRPC for commands, WebSocket for streaming results/dumps |
| **Container Runtime** | Docker API (bollard) | Battle-tested, versioned, isolated dependencies |
| **Process Supervision** | tokio + custom health checks | Async-first, cancellation-safe, resource-efficient |
| **SQL Parsing** | sqlparser-rs | Schema-aware autocomplete, query validation, policy injection |
| **Connection Pooling** | deadpool | Async connection pools per database instance |
| **Client Desktop** | Tauri v2 | Lightweight, native feel, small binary, Rust core |
| **Client Web** | React + TanStack Query | Fast data fetching, optimistic updates, offline support |
| **Query Editor** | Monaco Editor | Industry-standard SQL editing with LSP-like services |
| **Data Grid** | TanStack Table | Virtualized, sortable, filterable, millions of rows |
| **State Sync** | SQLite (meta DB) | Zero-config, embedded, later upgradable to PostgreSQL |
| **Credential Vault** | Rust-native encryption (ring) + SQLite | AES-256-GCM, argon2 key derivation |
| **WebAssembly** | wasmtime | User-defined functions, SQLite extensions in browser |
| **Telemetry** | OpenTelemetry + Prometheus | Distributed tracing, metrics, alerting hooks |
| **AI Integration** | OpenAI/Claude API (optional) | Natural language to SQL, schema documentation |

---

## Data Flows

### Query Execution Flow

```
User writes SELECT * FROM users
        │
        ▼
┌───────────────┐
│ Monaco Editor │──Parse, validate, autocomplete
│ (with schema  │   via LSP-like service
│  metadata)    │
└───────────────┘
        │
        ▼
┌───────────────┐
│ gRPC to       │
│ Control Plane │
│ (Query Engine)│
└───────────────┘
        │
        ▼
┌───────────────┐
│ Schema Policy │──Inject RLS WHERE clauses
│ Engine        │   for shared sessions
└───────────────┘
        │
        ▼
┌───────────────┐
│ Deadpool grabs│
│ connection to │
│ local Postgres│
│ on port 5433  │
└───────────────┘
        │
        ▼
┌───────────────┐
│ Stream rows   │
│ back via      │
│ WebSocket     │
│ (cursor-based)│
└───────────────┘
        │
        ▼
┌───────────────┐
│ TanStack Table│
│ (virtualized, │
│  10k+ rows)   │
└───────────────┘
```

### Database Installation Flow

```
User clicks "Add PostgreSQL 16"
        │
        ▼
┌───────────────┐
│ Check Docker  │──No?──▶ Offer native binary download
│ daemon        │         or Docker Desktop install link
└──────────────┘
        │ Yes
        ▼
┌───────────────┐
│ Pull postgres:│
│ 16-alpine     │
│ Verify digest │
│ (SHA-256)     │
└───────────────┘
        │
        ▼
┌───────────────┐
│ Allocate port │──Check 5432-5500 range
│ (ephemeral)   │   Skip occupied, mark in SQLite meta
└───────────────┘
        │
        ▼
┌───────────────┐
│ Create named  │
│ volume:       │
│ pg_16_myproject│
└───────────────┘
        │
        ▼
┌───────────────┐
│ docker run -d │
│ --name pg_16  │
│ -p 5433:5432  │
│ -v pg_16_data │
│ postgres:16   │
└───────────────┘
        │
        ▼
┌───────────────┐
│ Health check  │
│ (5 retries,   │
│  exponential  │
│  backoff)     │
└───────────────┘
        │
        ▼
┌───────────────┐
│ Generate      │
│ credentials   │
│ (auto-secure  │
│  password)    │
└───────────────┘
        │
        ▼
┌───────────────┐
│ Store metadata│
│ in SQLite:    │
│ port, version,│
│ volume, creds,│
│ status        │
└───────────────┘
        │
        ▼
┌───────────────┐
│ Emit event    │
│ to UI: ready  │
│ (WebSocket    │
│  broadcast)   │
└───────────────┘
```

### Export Dump Flow

```
User clicks "Export" → format: SQL / CSV / JSON / Parquet
        │
        ▼
┌───────────────┐
│ Spawn:        │
│ docker exec   │
│ pg_container  │
│ pg_dump -Fc   │
│ (or custom    │
│  export SQL)  │
│ Stream stdout │
│ to temp file  │
└───────────────┘
        │
        ▼
┌───────────────┐
│ Progress via  │
│ WebSocket     │
│ (bytes read / │
│  total est.)  │
└───────────────┘
        │
        ▼
┌───────────────┐
│ Transform:    │
│ CSV/JSON:     │
│   stream parse│
│ Parquet:      │
│   arrow batch │
│ Compress:     │
│   zstd level 3│
└───────────────┘
        │
        ▼
┌───────────────┐
│ Encrypt       │
│ (optional,    │
│  user-provided│
│  passphrase)  │
│ AES-256-GCM   │
└───────────────┘
        │
        ▼
┌───────────────┐
│ Tauri writes  │
│ to ~/Downloads│
│ or user picks │
│ path via      │
│ native dialog │
└───────────────┘
```

### Remote Sharing Flow

```
User A (Host)            Relay Server              User B (Guest)
     │                        │                        │
     │  1. Authenticate       │                        │
     │───────────────────────▶│                        │
     │  (JWT + API key)       │                        │
     │                        │                        │
     │  2. Open tunnel        │                        │
     │───────────────────────▶│                        │
     │  (WebSocket outbound)  │                        │
     │                        │                        │
     │                        │  3. Generate UUID      │
     │                        │     map to tunnel      │
     │◀───────────────────────│                        │
     │  UUID: abc-123-def     │                        │
     │                        │                        │
     │                        │                        │
     │                        │  4. Share URL           │
     │                        │◀───────────────────────│
     │                        │  https://share.bennett │
     │                        │    .studio/db/abc-123  │
     │                        │                        │
     │                        │  5. Open tunnel        │
     │                        │◀───────────────────────│
     │                        │  (WebSocket)           │
     │                        │                        │
     │  6. Multiplex traffic  │                        │
     │◀───────────────────────│───────────────────────▶│
     │  (bidirectional pipe)  │                        │
     │                        │                        │
     │  7. Schema policy      │                        │
     │     enforcement        │                        │
     │  (query rewriting)     │                        │
     │                        │                        │
     │  8. Audit log          │                        │
     │───────────────────────▶│                        │
     │  (query, user, time)   │                        │
```

---

## Roadmap

### Phase 1: Foundation (Months 1-3)

- [x] Headless Rust engine with Docker runtime
- [x] Tauri desktop client with React UI
- [x] PostgreSQL, MySQL, MariaDB, SQLite support
- [x] Monaco SQL editor with autocomplete
- [x] Basic query execution and result display
- [x] Export to SQL, CSV, JSON

### Phase 2: Sharing & Teams (Months 4-6)

- [ ] LAN sharing via mDNS discovery
- [ ] Remote sharing via reverse tunnel relay
- [ ] Credential vault with auto-rotation
- [ ] Schema policy engine (table-level permissions)
- [ ] Connection multiplexing (1:N tunnels)
- [ ] Web client (browser-based access to shared DBs)

### Phase 3: Enterprise (Months 7-9)

- [ ] RBAC with team workspaces
- [ ] Audit logging with ClickHouse backend
- [ ] Schema migration runner (migra, skeema integration)
- [ ] Offline resilience with query buffer/queue
- [ ] VS Code extension
- [ ] CLI tool

### Phase 4: Scale & Intelligence (Months 10-12)

- [ ] Plugin marketplace (OCI-compatible)
- [ ] AI assistant for natural language SQL
- [ ] WebAssembly runtime for custom functions
- [ ] Kubernetes operator for cloud deployments
- [ ] Enterprise SSO (SAML, OIDC)
- [ ] SOC 2 Type II compliance

---

## Installation

### Prerequisites

- Docker Engine 24.0+ or Docker Desktop 4.20+
- macOS 12+, Windows 10+, or Linux (Ubuntu 22.04+, Fedora 38+)
- 4GB RAM minimum, 8GB recommended

### Desktop App

```bash
# macOS
brew install bennett-studio

# Windows
winget install BennettStudio

# Linux (AppImage)
curl -fsSL https://get.bennett.studio | sh
```

### From Source

```bash
# Clone repository
git clone https://github.com/bennett-studio/bennett-studio.git
cd bennett-studio

# Build engine (Rust)
cd engine
cargo build --release

# Build desktop client (Tauri + React)
cd ../desktop
npm install
npm run tauri build

# Run engine
./target/release/bennett-engine

# Launch desktop app
./src-tauri/target/release/bennett-studio
```

### Docker (Headless Server)

```bash
docker run -d \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -v bennett-data:/data \
  -p 8080:8080 \
  bennettstudio/engine:latest
```

---

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

### Development Setup

```bash
# 1. Fork and clone
git clone https://github.com/YOUR_USERNAME/bennett-studio.git

# 2. Install Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 3. Install Node.js 20+
nvm install 20

# 4. Install Tauri prerequisites
# See: https://tauri.app/v1/guides/getting-started/prerequisites

# 5. Run development mode
cd engine && cargo run
cd ../desktop && npm run tauri dev
```

### Architecture Decision Records

All major architectural decisions are documented in [docs/adr/](docs/adr/):

- [ADR-001: Headless Engine vs Monolithic](docs/adr/adr-001-headless-engine.md)
- [ADR-002: Rust for Control Plane](docs/adr/adr-002-rust-control-plane.md)
- [ADR-003: Docker as Runtime](docs/adr/adr-003-docker-runtime.md)
- [ADR-004: Tauri over Electron](docs/adr/adr-004-tauri-desktop.md)
- [ADR-005: Reverse Tunnel for Sharing](docs/adr/adr-005-reverse-tunnel.md)
- [ADR-006: Schema Policy Engine](docs/adr/adr-006-schema-policy.md)

---

## License

Bennett Studio is dual-licensed:

- **Community Edition**: [MIT License](LICENSE-MIT) — Free for personal and small team use (< 5 users)
- **Enterprise Edition**: [Commercial License](LICENSE-ENTERPRISE) — SSO, audit, RBAC, SLA support

---

## Acknowledgments

- Inspired by the simplicity of [TablePlus](https://tableplus.com/) and the openness of [DBeaver](https://dbeaver.io/)
- Sharing architecture influenced by [Localtonet](https://localtonet.com/) and [ngrok](https://ngrok.com/)
- Query engine powered by [sqlparser-rs](https://github.com/sqlparser-rs/sqlparser-rs) and [deadpool](https://github.com/bikeshedder/deadpool)
- Desktop shell built with [Tauri](https://tauri.app/) and [React](https://react.dev/)

---

<p align="center">
  <strong>Built with ❤️ by developers, for developers.</strong><br>
  <a href="https://bennett.studio">Website</a> ·
  <a href="https://docs.bennett.studio">Documentation</a> ·
  <a href="https://discord.bennett.studio">Discord</a> ·
  <a href="https://twitter.com/bennettstudio">Twitter</a>
</p>
