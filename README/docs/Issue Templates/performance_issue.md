---
name: Performance Issue
about: Report slow queries, high memory usage, or UI lag
title: '[PERF] '
labels: performance
triage: true
---

## Performance Issue

- [ ] Slow query execution
- [ ] High memory usage
- [ ] UI lag / unresponsiveness
- [ ] Slow startup
- [ ] High CPU usage

## Environment

- **OS:** [e.g., macOS 14.5]
- **Bennett Studio Version:** [e.g., v0.3.2]
- **Database:** [e.g., PostgreSQL 16 with 10M rows]
- **Hardware:** [e.g., M2 MacBook Air 16GB, Intel i7 32GB]

## Reproduction

### Query (if applicable)

```sql
[Paste query here]
```

### Data Volume

- Table size: [e.g., 10M rows, 2GB]
- Result size: [e.g., 100K rows]

## Metrics

| Metric | Expected | Actual |
|--------|----------|--------|
| Query time | < 1s | 10s |
| Memory usage | < 500MB | 2GB |
| UI response | < 100ms | 2s |

## Profile Data

If possible, attach:
- CPU profile (from DevTools Performance tab)
- Memory heap snapshot
- Query plan (EXPLAIN ANALYZE output)

## Additional Context

- Does it happen with small datasets?
- Does it happen in other database tools?
- Any specific operations (sorting, filtering, export)?
