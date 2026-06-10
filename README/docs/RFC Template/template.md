# RFC-XXX: [Title]

**Status:** Draft | Proposed | Accepted | Implemented | Rejected | Superseded

**Author:** [Name] <[email]>

**Date:** YYYY-MM-DD

**Target Version:** [e.g., v0.5.0, v1.0.0]

**Related ADRs:** [ADR-001](adr-001-headless-engine.md), [ADR-005](adr-005-reverse-tunnel.md)

**Related Issues:** #123, #456

---

## Summary

One paragraph explanation of the feature or change.

## Motivation

Why are we doing this? What problems does it solve? What is the expected outcome?

### User Stories

- As a [type of user], I want [goal] so that [benefit].
- As a [type of user], I want [goal] so that [benefit].

### Current Pain Points

Describe the current state and why it's insufficient.

## Detailed Design

### Architecture

```
[Insert architecture diagram or description]
```

### API Changes

```protobuf
// New or modified gRPC/HTTP endpoints
service NewService {
  rpc NewMethod(NewRequest) returns (NewResponse);
}

message NewRequest {
  string field = 1;
}

message NewResponse {
  bool success = 1;
}
```

### Data Model Changes

```sql
-- New tables or migrations
CREATE TABLE new_table (
  id UUID PRIMARY KEY,
  created_at TIMESTAMP NOT NULL DEFAULT NOW()
);
```

### UI/UX Changes

Describe changes to the user interface. Include wireframes or mockups if applicable.

### Configuration Changes

```toml
# New configuration options
[new_feature]
enabled = true
max_connections = 100
```

## Implementation Plan

### Phase 1: Foundation (Week 1-2)

- [ ] Task 1
- [ ] Task 2

### Phase 2: Core Feature (Week 3-4)

- [ ] Task 3
- [ ] Task 4

### Phase 3: Polish & Release (Week 5-6)

- [ ] Task 5
- [ ] Task 6

## Testing Strategy

### Unit Tests

- [ ] Test case 1
- [ ] Test case 2

### Integration Tests

- [ ] Test scenario 1
- [ ] Test scenario 2

### E2E Tests

- [ ] User flow 1
- [ ] User flow 2

## Performance Impact

| Metric | Before | After | Target |
|--------|--------|-------|--------|
| Query latency | X ms | Y ms | < Z ms |
| Memory usage | X MB | Y MB | < Z MB |
| Bundle size | X MB | Y MB | < Z MB |

## Security Considerations

- Threat model updates
- New attack surfaces
- Mitigation strategies

## Backwards Compatibility

- Breaking changes list
- Migration path for existing users
- Deprecation timeline

## Alternatives Considered

### Alternative A: [Name]

- **Pros:**
- **Cons:**
- **Why rejected:**

### Alternative B: [Name]

- **Pros:**
- **Cons:**
- **Why rejected:**

## Open Questions

1. [Question 1]
2. [Question 2]

## Timeline

| Milestone | Date | Deliverable |
|-----------|------|-------------|
| RFC Accepted | YYYY-MM-DD | This document approved |
| Implementation Start | YYYY-MM-DD | PR opened |
| Alpha Release | YYYY-MM-DD | Feature flag enabled |
| GA Release | YYYY-MM-DD | Default enabled |

## Appendix

### Glossary

- **Term 1:** Definition
- **Term 2:** Definition

### References

- [Link 1](url)
- [Link 2](url)
