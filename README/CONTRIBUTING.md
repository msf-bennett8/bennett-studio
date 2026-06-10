# Contributing to Bennett Studio

Thank you for your interest in contributing! This document outlines the process and guidelines for contributing to Bennett Studio.

## Table of Contents

1. [Code of Conduct](#code-of-conduct)
2. [Getting Started](#getting-started)
3. [Development Environment](#development-environment)
4. [Project Structure](#project-structure)
5. [Contribution Workflow](#contribution-workflow)
6. [Coding Standards](#coding-standards)
7. [Testing](#testing)
8. [Commit Message Convention](#commit-message-convention)
9. [Pull Request Process](#pull-request-process)
10. [Architecture Decision Records](#architecture-decision-records)
11. [Community](#community)

---

## Code of Conduct

This project adheres to a strict code of conduct. By participating, you are expected to uphold this code:

- Be respectful and inclusive in all interactions
- Welcome newcomers and help them learn
- Focus on constructive criticism, not personal attacks
- Respect differing viewpoints and experiences
- Prioritize the community's well-being over individual preferences

Harassment, trolling, or discriminatory behavior will not be tolerated.

---

## Getting Started

### Prerequisites

Before you begin, ensure you have the following installed:

| Tool | Version | Purpose |
|------|---------|---------|
| Rust | 1.78+ | Control plane, engine, Tauri backend |
| Node.js | 20.x | Desktop UI (React), web client |
| pnpm | 9.x | Package management (faster than npm/yarn) |
| Docker | 24.0+ | Database runtime, integration tests |
| Docker Compose | 2.20+ | Multi-service test environments |
| Git | 2.40+ | Version control |

### Platform-Specific Requirements

#### macOS

```bash
# Install Homebrew (if not installed)
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# Install prerequisites
brew install rust node pnpm docker docker-compose

# Install Tauri dependencies
brew install pkg-config cairo pango libpng librsvg pixman
```

#### Linux (Ubuntu/Debian)

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Install Node.js 20
curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash -
sudo apt-get install -y nodejs

# Install pnpm
npm install -g pnpm

# Install Docker
sudo apt-get update
sudo apt-get install -y docker.io docker-compose

# Install Tauri dependencies
sudo apt-get install -y libwebkit2gtk-4.1-dev build-essential curl wget libssl-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev
```

#### Windows

```powershell
# Install Rust (via rustup)
Invoke-WebRequest -Uri https://win.rustup.rs -OutFile rustup-init.exe
.ustup-init.exe

# Install Node.js 20 (via nvm-windows or installer)
# Download from https://nodejs.org/dist/v20.x.x/node-v20.x.x-x64.msi

# Install pnpm
npm install -g pnpm

# Install Docker Desktop
# Download from https://www.docker.com/products/docker-desktop

# Install Tauri dependencies (via vcpkg or MSYS2)
# See: https://tauri.app/v1/guides/getting-started/prerequisites
```

---

## Development Environment

### 1. Fork and Clone

```bash
# Fork the repository on GitHub, then clone your fork
git clone https://github.com/YOUR_USERNAME/bennett-studio.git
cd bennett-studio

# Add upstream remote
git remote add upstream https://github.com/bennett-studio/bennett-studio.git
```

### 2. Install Dependencies

```bash
# Install Rust dependencies and tools
cd engine
cargo install cargo-watch cargo-nextest cargo-audit cargo-deny

# Install Node.js dependencies
cd ../desktop
pnpm install

cd ../web
pnpm install
```

### 3. Build the Project

```bash
# Build the engine (Rust)
cd engine
cargo build --release

# Build the desktop client (Tauri + React)
cd ../desktop
pnpm tauri build

# Build the web client (React)
cd ../web
pnpm build
```

### 4. Run Development Mode

```bash
# Terminal 1: Run the engine with hot reload
cd engine
cargo watch -x run

# Terminal 2: Run the desktop client with hot reload
cd desktop
pnpm tauri dev

# Terminal 3: Run the web client (optional)
cd web
pnpm dev
```

### 5. Verify Setup

```bash
# Run the full test suite
cd engine
cargo test --all-features

# Run linting
cargo clippy --all-targets --all-features -- -D warnings

# Check formatting
cargo fmt --check
```

---

## Project Structure

```
bennett-studio/
├── .github/                  # GitHub Actions, issue templates
├── docs/                     # Documentation, ADRs, RFCs
│   ├── adr/                  # Architecture Decision Records
│   ├── rfcs/                 # Request for Comments
│   └── guides/               # User and developer guides
├── engine/                     # Rust control plane and runtime
│   ├── src/
│   │   ├── main.rs           # Entry point
│   │   ├── api/              # gRPC/HTTP API handlers
│   │   ├── auth/             # Authentication, JWT, RBAC
│   │   ├── control_plane/    # Core business logic
│   │   │   ├── connection/   # Connection pool manager
│   │   │   ├── query/        # Query engine, SQL parser
│   │   │   ├── export/       # Export orchestrator
│   │   │   ├── migration/    # Schema migration runner
│   │   │   └── vault/        # Credential vault
│   │   ├── runtime/          # Docker runtime, process supervisor
│   │   │   ├── container/    # Docker API wrappers
│   │   │   ├── process/      # Process management
│   │   │   ├── volume/       # Volume management
│   │   │   ├── network/      # Network isolation
│   │   │   └── port/         # Port allocation
│   │   ├── sharing/          # Sharing layer
│   │   │   ├── lan/          # mDNS discovery, direct connections
│   │   │   ├── relay/        # Reverse tunnel client
│   │   │   ├── session/      # Session manager
│   │   │   ├── policy/       # Schema policy engine
│   │   │   └── multiplex/    # Connection multiplexing
│   │   ├── plugins/          # Plugin system
│   │   ├── telemetry/        # OpenTelemetry, metrics
│   │   └── wasm/             # WebAssembly runtime
│   ├── proto/                # gRPC protobuf definitions
│   ├── tests/                # Integration tests
│   └── Cargo.toml
├── desktop/                  # Tauri desktop application
│   ├── src/
│   │   ├── App.tsx           # Main React component
│   │   ├── components/       # Reusable UI components
│   │   ├── pages/            # Page-level components
│   │   ├── hooks/            # Custom React hooks
│   │   ├── stores/           # Zustand state management
│   │   └── services/         # API client, gRPC/WebSocket
│   ├── src-tauri/            # Rust backend for Tauri
│   │   ├── src/
│   │   │   ├── main.rs       # Tauri entry point
│   │   │   ├── commands/     # Tauri command handlers
│   │   │   └── engine/       # Embedded engine launcher
│   │   └── Cargo.toml
│   └── package.json
├── web/                      # Web client (React)
│   ├── src/
│   ├── public/
│   └── package.json
├── cli/                      # CLI tool (Rust)
│   └── src/
├── vscode-ext/               # VS Code extension
│   └── src/
├── shared/                   # Shared types, utilities
│   ├── proto/                # Shared protobuf definitions
│   └── types/                # TypeScript type definitions
├── scripts/                  # Build, release, and utility scripts
├── docker/                   # Docker images for engine, relay
├── infra/                    # Infrastructure as Code (Terraform)
│   ├── relay/                # Relay server deployment
│   └── monitoring/           # Prometheus, Grafana
└── Cargo.toml                # Workspace root
```

---

## Contribution Workflow

### 1. Find or Create an Issue

- Check [existing issues](https://github.com/bennett-studio/bennett-studio/issues) for something you'd like to work on
- If you have a new idea, [open an issue](https://github.com/bennett-studio/bennett-studio/issues/new/choose) first to discuss it
- Wait for maintainer approval before starting significant work

### 2. Create a Branch

```bash
# Sync with upstream
git fetch upstream
git checkout main
git merge upstream/main

# Create a feature branch
git checkout -b feat/your-feature-name
# or
git checkout -b fix/your-bug-fix
# or
git checkout -b docs/your-documentation-update
```

**Branch naming conventions:**

| Prefix | Purpose | Example |
|--------|---------|---------|
| `feat/` | New feature | `feat/schema-policy-engine` |
| `fix/` | Bug fix | `fix/connection-pool-leak` |
| `docs/` | Documentation | `docs/sharing-architecture` |
| `refactor/` | Code refactoring | `refactor/query-engine` |
| `test/` | Test additions/changes | `test/relay-multiplex` |
| `chore/` | Maintenance tasks | `chore/update-dependencies` |
| `perf/` | Performance improvements | `perf/connection-pooling` |
| `security/` | Security fixes | `security/credential-rotation` |

### 3. Make Changes

- Write code following our [coding standards](#coding-standards)
- Add or update tests for your changes
- Update documentation as needed
- Ensure all tests pass locally

### 4. Commit Your Changes

```bash
# Stage changes
git add .

# Commit with conventional message
git commit -m "feat(sharing): add connection multiplexing for 1:N tunnels

- Implement multiplex protocol over single WebSocket
- Add session manager with UUID mapping
- Include rate limiting per peer connection
- Add integration tests for 10 concurrent peers

Closes #123"
```

See [Commit Message Convention](#commit-message-convention) for details.

### 5. Push and Open Pull Request

```bash
# Push to your fork
git push origin feat/your-feature-name

# Open a pull request on GitHub
```

---

## Coding Standards

### Rust

- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `cargo fmt` for formatting (enforced in CI)
- Use `cargo clippy` for linting (enforced in CI, deny warnings)
- Document all public APIs with rustdoc
- Use `Result` and `Option` explicitly; avoid `unwrap()` in production code
- Prefer `async/await` with `tokio` for concurrency
- Use `tracing` for structured logging
- Use `thiserror` for error types, `anyhow` for application errors

**Example:**

```rust
use tracing::{info, error};
use thiserror::Error;

/// Manages database container lifecycle.
#[derive(Debug)]
pub struct ContainerManager {
    docker: Docker,
    port_allocator: PortAllocator,
}

#[derive(Error, Debug)]
pub enum ContainerError {
    #[error("Docker daemon unavailable: {0}")]
    DaemonUnavailable(String),
    #[error("Port allocation failed: {0}")]
    PortAllocationFailed(#[from] PortError),
    #[error("Image pull failed: {0}")]
    ImagePullFailed(String),
}

impl ContainerManager {
    /// Creates a new container manager with the given Docker client.
    ///
    /// # Errors
    ///
    /// Returns `ContainerError::DaemonUnavailable` if Docker is not running.
    pub async fn new(docker: Docker) -> Result<Self, ContainerError> {
        docker.ping().await.map_err(|e| {
            ContainerError::DaemonUnavailable(e.to_string())
        })?;

        info!("Docker daemon connected successfully");

        Ok(Self {
            docker,
            port_allocator: PortAllocator::new(),
        })
    }
}
```

### TypeScript / React

- Follow the [TypeScript Style Guide](https://google.github.io/styleguide/tsguide.html)
- Use strict TypeScript (`strict: true` in tsconfig)
- Use functional components with hooks
- Use Zustand for state management (not Redux)
- Use TanStack Query for server state
- Use Tailwind CSS for styling (no inline styles)
- Write tests with Vitest and React Testing Library

**Example:**

```typescript
import { useQuery } from '@tanstack/react-query';
import { useDatabaseStore } from '@/stores/database';

interface QueryResultProps {
  connectionId: string;
  query: string;
}

export function QueryResult({ connectionId, query }: QueryResultProps) {
  const { executeQuery } = useDatabaseStore();

  const { data, isLoading, error } = useQuery({
    queryKey: ['query', connectionId, query],
    queryFn: () => executeQuery(connectionId, query),
    enabled: !!query,
  });

  if (isLoading) return <QuerySkeleton />;
  if (error) return <QueryError error={error} />;

  return (
    <DataGrid
      data={data.rows}
      columns={data.columns}
      rowCount={data.rowCount}
    />
  );
}
```

### Protocol Buffers (gRPC)

- Use `proto3` syntax
- Follow [Google's Protocol Buffer Style Guide](https://developers.google.com/protocol-buffers/docs/style)
- Version all APIs (e.g., `v1`, `v2`)
- Document all messages, fields, and services

---

## Testing

### Test Pyramid

```
        /\
       /  \
      / E2E \      <- Tauri integration, full user flows (slow, few)
     /─────────\
    /  Integration \  <- API endpoints, database operations (medium, moderate)
   /─────────────────\
  /      Unit          \ <- Pure functions, business logic (fast, many)
 /─────────────────────────\
```

### Running Tests

```bash
# Unit tests (fast)
cd engine
cargo test --lib

# Integration tests (medium)
cargo test --test '*'

# All tests with coverage
cargo tarpaulin --out Html

# E2E tests (slow, requires Docker)
cd desktop
pnpm test:e2e

# Web client tests
cd web
pnpm test
```

### Test Guidelines

- **Unit tests**: Test pure functions, edge cases, error paths
- **Integration tests**: Test database operations, API endpoints, Docker interactions
- **E2E tests**: Test critical user flows (install DB, run query, share tunnel)
- Use `testcontainers` for integration tests with real databases
- Mock external services (Docker API, relay server) in unit tests
- Use `insta` for snapshot testing of SQL output and query plans

### Writing Integration Tests

```rust
#[tokio::test]
async fn test_postgres_install_and_query() {
    let engine = TestEngine::new().await;

    // Install PostgreSQL 16
    let db = engine
        .install_database(DatabaseType::Postgres, "16")
        .await
        .expect("install should succeed");

    // Verify health
    assert!(db.is_healthy().await);

    // Run a query
    let result = engine
        .query(db.id(), "SELECT version()")
        .await
        .expect("query should succeed");

    assert!(result.rows[0][0].contains("PostgreSQL 16"));

    // Cleanup
    engine.remove_database(db.id()).await;
}
```

---

## Commit Message Convention

We follow [Conventional Commits](https://www.conventionalcommits.org/) with the following types:

### Format

```
<type>(<scope>): <description>

[optional body]

[optional footer(s)]
```

### Types

| Type | Description | Example |
|------|-------------|---------|
| `feat` | New feature | `feat(sharing): add LAN discovery via mDNS` |
| `fix` | Bug fix | `fix(engine): resolve port allocation race condition` |
| `docs` | Documentation | `docs(readme): update sharing architecture diagram` |
| `style` | Formatting | `style(rust): run cargo fmt` |
| `refactor` | Code restructuring | `refactor(query): split parser into modules` |
| `perf` | Performance | `perf(pool): reduce connection acquisition time` |
| `test` | Tests | `test(relay): add multiplex stress tests` |
| `chore` | Maintenance | `chore(deps): update tokio to 1.38` |
| `ci` | CI/CD | `ci(github): add ARM64 build target` |
| `security` | Security | `security(vault): rotate credentials on share revoke` |

### Scopes

Common scopes: `engine`, `desktop`, `web`, `cli`, `sharing`, `relay`, `api`, `auth`, `query`, `runtime`, `docker`, `vault`, `policy`, `telemetry`, `wasm`, `plugins`

### Breaking Changes

```
feat(api)!: remove deprecated v1 endpoints

BREAKING CHANGE: v1 API endpoints are removed. Migrate to v2.
```

---

## Pull Request Process

### Before Submitting

- [ ] Branch is up-to-date with `main`
- [ ] All tests pass locally
- [ ] Code is formatted (`cargo fmt`, `pnpm format`)
- [ ] Linting passes (`cargo clippy`, `pnpm lint`)
- [ ] Documentation is updated (README, rustdoc, TS doc)
- [ ] Commit messages follow convention
- [ ] PR description includes context and closes related issues

### PR Template

```markdown
## Description

Brief description of the changes and why they're needed.

## Related Issue

Closes #123

## Type of Change

- [ ] Bug fix (non-breaking)
- [ ] New feature (non-breaking)
- [ ] Breaking change
- [ ] Documentation update
- [ ] Performance improvement
- [ ] Security fix

## Testing

- [ ] Unit tests added/updated
- [ ] Integration tests added/updated
- [ ] E2E tests added/updated
- [ ] Manual testing performed

## Checklist

- [ ] Code follows style guidelines
- [ ] Self-review completed
- [ ] Comments added for complex logic
- [ ] Documentation updated
- [ ] No new warnings introduced
```

### Review Process

1. **Automated checks** must pass (CI, tests, linting, coverage)
2. **Code review** by at least one maintainer
3. **Approval** required before merge
4. **Squash merge** to main with conventional commit message

---

## Architecture Decision Records

For significant architectural changes, you must write an ADR before implementation:

1. Copy the [ADR template](docs/adr/template.md)
2. Name it `adr-XXX-short-title.md`
3. Submit as part of your PR or as a separate PR
4. Discuss in the issue before coding

See [docs/adr/](docs/adr/) for existing ADRs.

---

## Community

- **Discord**: [discord.bennett.studio](https://discord.bennett.studio) — Real-time chat, help, announcements
- **GitHub Discussions**: [github.com/bennett-studio/bennett-studio/discussions](https://github.com/bennett-studio/bennett-studio/discussions) — Long-form discussions, RFCs
- **Twitter/X**: [@bennettstudio](https://twitter.com/bennettstudio) — Updates, tips, community highlights
- **Dev.to**: [dev.to/bennettstudio](https://dev.to/bennettstudio) — Tutorials, deep dives

### Becoming a Maintainer

Consistent, high-quality contributors may be invited to become maintainers:

1. Demonstrate expertise in a domain (Rust, React, Docker, networking)
2. Review others' PRs constructively
3. Help triage issues and answer questions
4. Follow the project's values and code of conduct

---

## License

By contributing, you agree that your contributions will be licensed under the same license as the project (MIT for community code, subject to CLA for enterprise features).

---

**Thank you for contributing to Bennett Studio! 🚀**
