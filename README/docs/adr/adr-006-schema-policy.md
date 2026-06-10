# ADR-006: Schema Policy Engine for Shared Connections

## Status

Accepted

## Context

When users share databases via tunnels, guests must not have unrestricted access. We need granular permissions at table and row level. We evaluated:

1. **Database-native RLS**: PostgreSQL RLS, MySQL grants
2. **Proxy SQL rewriting**: Intercept and modify queries
3. **Read-only replicas**: Create replica with limited data
4. **Application-level filtering**: Filter results in the engine

## Decision

We will implement a **Schema Policy Engine** that combines proxy SQL rewriting with database-native RLS, depending on the database type and share configuration.

## Consequences

### Positive

- **Granular control**: Table allowlists, blocklists, column masks, row filters
- **Database agnostic**: Works across PostgreSQL, MySQL, MariaDB with consistent API
- **No data duplication**: No replicas or snapshots needed
- **Audit friendly**: All rewrites logged for compliance
- **Performance**: Minimal overhead for simple policies; native RLS for complex ones

### Negative

- **Complexity**: SQL parsing and rewriting is non-trivial
- **Edge cases**: Complex queries (CTEs, subqueries, window functions) may bypass policies
- **Maintenance**: Must keep up with SQL dialect changes

## Architecture

```
Guest Query
     │
     ▼
┌───────────────┐
│ SQL Parser    │──Parse to AST (sqlparser-rs)
│ (sqlparser-rs)│
└───────────────┘
     │
     ▼
┌───────────────┐
│ Policy Engine │──Check against share policies:
│               │   - Table access allowed?
│               │   - Columns masked?
│               │   - Row filter applies?
└───────────────┘
     │
     ▼
┌───────────────┐
│ Query Rewrite │──Inject WHERE clauses, mask columns,
│               │   rewrite table references
└───────────────┘
     │
     ▼
┌───────────────┐
│ Native RLS    │──For PostgreSQL, enable RLS as
│ (fallback)    │   defense in depth
└───────────────┘
     │
     ▼
Database
```

## Policy Types

| Policy | Description | Implementation |
|--------|-------------|----------------|
| **Table Allowlist** | Only specified tables accessible | AST validation, reject if table not in list |
| **Table Blocklist** | Specified tables inaccessible | AST validation, reject if table in list |
| **Column Mask** | Hide sensitive columns | Rewrite SELECT to exclude columns; return NULLs |
| **Row Filter** | Only matching rows visible | Inject WHERE clause with policy condition |
| **Read-Only** | No DML or DDL | Reject INSERT, UPDATE, DELETE, CREATE, DROP |
| **Query Limit** | Max rows returned | Append LIMIT clause |
| **Time Window** | Access only during business hours | Check timestamp before executing |

## Example: Row Filter Injection

**Original query:**
```sql
SELECT * FROM orders WHERE status = 'pending';
```

**Policy:** Guest can only see orders from region = 'US'

**Rewritten query:**
```sql
SELECT * FROM orders WHERE status = 'pending' AND region = 'US';
```

## Mitigations

- **Defense in depth**: Combine proxy rewriting with database-native RLS where available
- **Query validation**: Reject queries that cannot be safely rewritten (complex CTEs, dynamic SQL)
- **Audit logging**: Log original and rewritten queries for compliance review
- **Fuzz testing**: Automated tests for policy bypass attempts

## Alternatives Considered

### Database-Native RLS Only

- **Pros**: Zero overhead, guaranteed enforcement, no parsing complexity
- **Cons**: Database-specific (PostgreSQL only), requires superuser to configure, not portable
- **Verdict**: Used as fallback, not primary solution

### Read-Only Replicas

- **Pros**: Physical isolation, no query rewriting needed
- **Cons**: Data duplication, sync lag, storage overhead, slow for large databases
- **Verdict**: Rejected — too expensive and slow for ephemeral sharing

## References

- [PostgreSQL Row-Level Security](https://www.postgresql.org/docs/current/ddl-rowsecurity.html)
- [sqlparser-rs](https://github.com/sqlparser-rs/sqlparser-rs)
- [Database Proxy Patterns](https://www.cockroachlabs.com/blog/sql-proxy/)
- [Policy Enforcement in SQL](https://arxiv.org/abs/2103.02142)

## Date

2024-06-10

## Author

Bennett Studio Core Team
