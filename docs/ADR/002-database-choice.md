# ADR 002: PostgreSQL for persistence

## Status
Accepted

## Context
The system stores time-series telemetry, power-flow snapshots, schedules, and
configuration data. We need relational integrity, robust time-series querying,
and compatibility with existing Rust tooling.

## Decision
Use PostgreSQL as the primary datastore, with SQLx for type-safe queries and
migration support.

## Consequences
- Strong relational guarantees and rich SQL for analytics.
- Works well with time-series extensions if needed (TimescaleDB).
- SQLx compile-time checks improve safety.
- Requires running and managing a database service.

## Alternatives Considered
- SQLite: Simple but lacks concurrency and scaling features for production.
- InfluxDB/Timescale only: Great for time-series but less suitable for
  relational entities (users, devices, configs).
- DynamoDB: Managed service but less aligned with edge-first, local deployment.

## References
- docs/ARCHITECTURE.md
- migrations/ directory
