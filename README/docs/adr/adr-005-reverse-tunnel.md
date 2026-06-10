# ADR-005: Reverse Tunnel for Database Sharing

## Status

Accepted

## Context

Users need to share local databases with teammates across different networks (LAN, remote, cloud). We evaluated:

1. **Direct port forwarding**: Expose local port to internet
2. **VPN mesh**: Tailscale, WireGuard, ZeroTier
3. **P2P hole punching**: WebRTC, STUN/TURN
4. **Reverse tunnel**: Outbound connection to relay server
5. **Cloud sync**: Replicate data to cloud database

## Decision

We will implement **reverse tunnel sharing** via WebSocket connections to a relay server, with LAN direct connection as an optimization.

## Consequences

### Positive

- **No firewall holes**: Host initiates outbound connection; no port forwarding needed
- **NAT traversal**: Works behind corporate firewalls, CGNAT, university networks
- **Easy sharing**: UUID-based URLs (`https://share.bennett.studio/db/abc-123`)
- **Centralized control**: Relay can enforce rate limits, audit logging, access revocation
- **Multi-peer**: Single tunnel supports multiple guests via multiplexing
- **Encryption**: TLS over WebSocket by default

### Negative

- **Infrastructure cost**: Relay servers require hosting (Fly.io, Hetzner, Railway)
- **Bandwidth cost**: We pay for relayed traffic (mitigated by LAN direct mode)
- **Dependency**: Users depend on our relay availability (mitigated by self-host option)
- **Latency**: Relay adds hop compared to direct connection
- **Privacy**: Traffic passes through our servers (end-to-end encryption mitigates)

## Architecture

```
Host (User A)          Relay Server           Guest (User B)
     │                      │                       │
     │ 1. Auth (JWT)        │                       │
     │─────────────────────▶│                       │
     │                      │                       │
     │ 2. Open tunnel       │                       │
     │ (WebSocket outbound) │                       │
     │─────────────────────▶│                       │
     │                      │ 3. Generate UUID      │
     │◀─────────────────────│    map to tunnel      │
     │                      │                       │
     │                      │ 4. Share URL          │
     │                      │◀─────────────────────│
     │                      │                       │
     │                      │ 5. Guest connects     │
     │                      │◀─────────────────────│
     │                      │    (WebSocket)        │
     │                      │                       │
     │ 6. Multiplex traffic │                       │
     │◀─────────────────────│─────────────────────▶│
     │   (bidirectional)    │                       │
```

## Mitigations

- **LAN direct mode**: Same network = direct connection, no relay
- **Self-hosted relay**: Enterprise users can run their own relay
- **WireGuard option**: High-throughput scenarios use VPN instead of TCP relay
- **End-to-end encryption**: Host and guest encrypt payloads; relay sees only metadata
- **Rate limiting**: Per-share bandwidth and query limits
- **Audit logging**: All queries logged with user attribution

## Alternatives Considered

### VPN Mesh (Tailscale)

- **Pros**: Direct P2P when possible, excellent performance, stable IPs
- **Cons**: Requires separate app install, complex for non-technical users, not zero-config
- **Verdict**: Optional integration, not default

### P2P Hole Punching (WebRTC)

- **Pros**: Direct connection when possible, no relay bandwidth cost
- **Cons**: Unreliable with corporate firewalls, complex signaling, STUN/TURN servers still needed
- **Verdict**: Too unreliable for developer tools; 20% failure rate unacceptable

### Cloud Sync

- **Pros**: Data always available, no host dependency
- **Cons**: Expensive, slow for large datasets, conflicts with local-first philosophy
- **Verdict**: Rejected — contradicts our local-first, ephemeral sharing model

## References

- [Localtonet Architecture](https://localtonet.com/)
- [ngrok Documentation](https://ngrok.com/docs/)
- [Cloudflare Tunnel](https://developers.cloudflare.com/cloudflare-one/connections/connect-networks/)
- [WebRTC NAT Traversal](https://webrtcforthecurious.com/docs/03-connecting/)

## Date

2024-06-10

## Author

Bennett Studio Core Team
