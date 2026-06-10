# ADR-001: Headless Engine vs Monolithic Architecture

## Status

Accepted

## Context

Bennett Studio needs to support multiple client interfaces (desktop, web, CLI, VS Code extension) while maintaining a consistent database management experience. We evaluated two architectural approaches:

1. **Monolithic**: Each client bundles its own database management logic
2. **Headless Engine**: A single runtime engine with multiple thin clients

## Decision

We will adopt a **headless engine architecture** with a Rust-based control plane that runs independently, and multiple client interfaces that communicate via gRPC and WebSocket protocols.

## Consequences

### Positive

- **Client flexibility**: New clients (mobile, IDE plugins, web dashboards) can be built without engine changes
- **Consistency**: All clients share the same database runtime, query engine, and sharing logic
- **Team collaboration**: The engine can run on a server while clients connect remotely
- **Testing**: Engine can be tested independently of UI concerns
- **Performance**: Rust engine provides memory safety and async performance without UI overhead

### Negative

- **Complexity**: Requires protocol design, versioning, and backward compatibility
- **Initial overhead**: More moving parts than a monolithic desktop app
- **Distribution**: Engine must be distributed alongside clients (embedded in Tauri, or installed separately)

## Alternatives Considered

### Monolithic (Electron-only)

- **Pros**: Simpler initial development, single codebase
- **Cons**: Beekeeper Studio followed this path and now struggles to add web/team features; difficult to share logic across platforms
- **Verdict**: Rejected — locks us into a single client type

### Server-Rendered Web App

- **Pros**: Single deployment, no client distribution
- **Cons**: Requires always-on server, poor offline support, difficult to manage local Docker containers from a remote server
- **Verdict**: Rejected — contradicts local-first philosophy

## References

- [Beekeeper Studio Architecture](https://github.com/beekeeper-studio/beekeeper-studio)
- [DBeaver Architecture](https://dbeaver.io/)
- [Local-first Software](https://www.inkandswitch.com/local-first/)

## Date

2024-06-10

## Author

Bennett Studio Core Team
