# ADR-003: Docker as Database Runtime

## Status

Accepted

## Context

Bennett Studio must install and manage multiple database types and versions on a user's local machine. We evaluated:

1. **Native binaries**: Download and run database binaries directly
2. **Package managers**: Use brew, apt, choco to install databases
3. **Docker containers**: Run databases in isolated, versioned containers
4. **Virtual machines**: Full VM per database (Vagrant, Multipass)

## Decision

We will use **Docker containers** as the primary runtime for databases, with native execution as a fallback for SQLite.

## Consequences

### Positive

- **Version consistency**: Exact PostgreSQL 16.2, not "whatever brew installed"
- **Isolation**: Databases don't conflict with system packages or each other
- **Reproducibility**: Same container image on every machine
- **Cleanup**: `docker rm` removes everything cleanly
- **Multi-version**: Run PostgreSQL 15, 16, 17 simultaneously
- **Security**: Container boundaries provide defense in depth
- **Ecosystem**: Official, maintained images for all major databases

### Negative

- **Docker dependency**: Users must have Docker installed (or we bundle it)
- **Resource overhead**: Containers use more memory than native processes
- **Volume management**: Data persistence requires named volumes or bind mounts
- **Networking**: Port mapping adds complexity
- **Platform limitations**: Docker Desktop licensing on Windows/macOS for enterprise

## Mitigations

- **Docker Desktop auto-install**: Detect absence, offer one-click install
- **Rootless Docker**: Support rootless mode for better security
- **Podman fallback**: Support Podman as alternative container runtime
- **SQLite exception**: Native file access, no container needed
- **Resource limits**: Enforce memory and CPU limits per container

## Alternatives Considered

### Native Binaries

- **Pros**: No Docker dependency, lower resource usage
- **Cons**: Version management nightmare, cleanup difficult, conflicts with system packages
- **Verdict**: Rejected — too fragile for a developer tool

### Package Managers

- **Pros**: Familiar to users, integrates with system
- **Cons**: Inconsistent versions across platforms, requires sudo, difficult to uninstall cleanly
- **Verdict**: Rejected — not portable or reproducible

## References

- [Docker Best Practices for Databases](https://docs.docker.com/storage/volumes/)
- [Testcontainers](https://www.testcontainers.org/)
- [Podman vs Docker](https://developers.redhat.com/blog/2020/11/19/transitioning-from-docker-to-podman)

## Date

2024-06-10

## Author

Bennett Studio Core Team
