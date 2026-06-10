# ADR-002: Rust for Control Plane

## Status

Accepted

## Context

The control plane must manage Docker containers, handle concurrent database connections, process SQL queries, and maintain persistent tunnels — all while running as a long-lived background process. We evaluated:

1. **Node.js/TypeScript**: Familiar to frontend team, large ecosystem
2. **Go**: Strong concurrency, Docker ecosystem
3. **Rust**: Memory safety, zero-cost abstractions, async-native
4. **Python**: Rapid prototyping, rich data libraries

## Decision

We will use **Rust** for the control plane, engine, and Tauri backend.

## Consequences

### Positive

- **Memory safety**: Eliminates entire classes of bugs (use-after-free, data races) without GC pauses
- **Async performance**: tokio provides high-performance concurrent I/O for database connections and WebSocket tunnels
- **Single binary**: Easy distribution and deployment
- **FFI friendly**: Can embed SQLite, WebAssembly runtime, and native database drivers
- **Docker ecosystem**: bollard crate provides first-class Docker API integration
- **Team confidence**: Senior developers can confidently build infrastructure in Rust

### Negative

- **Learning curve**: Steeper than Go or Node.js for new contributors
- **Compilation time**: Slower than interpreted languages during development
- **Ecosystem gaps**: Some specialized libraries less mature than Python/Node.js equivalents
- **Hiring**: Smaller talent pool than Go or JavaScript

## Alternatives Considered

### Go

- **Pros**: Excellent concurrency, fast compilation, strong Docker ecosystem
- **Cons**: No memory safety guarantees, verbose error handling, less expressive type system
- **Verdict**: Strong alternative, but Rust's safety guarantees win for infrastructure code

### Node.js

- **Pros**: Same language as frontend, rapid iteration, npm ecosystem
- **Cons**: Memory leaks in long-running processes, callback complexity, single-threaded event loop limitations
- **Verdict**: Rejected — control plane requires predictable performance and resource usage

## References

- [Rust Async Book](https://rust-lang.github.io/async-book/)
- [tokio documentation](https://tokio.rs/)
- [bollard crate](https://docs.rs/bollard/)
- [Why Rust for Infrastructure](https://www.infoq.com/articles/rust-infrastructure/)

## Date

2024-06-10

## Author

Bennett Studio Core Team
