# ADR 001: Rust for core implementation

## Status
Accepted

## Context
Open Energy Controller requires a reliable, low-latency edge runtime that can
interface with hardware devices, manage concurrent I/O, and run optimization
workloads continuously on constrained devices such as Raspberry Pi.

## Decision
Use Rust as the primary implementation language for the controller core,
including the API server, device interfaces, power-flow orchestration, and
optimization services.

## Consequences
- Strong compile-time guarantees for memory safety and concurrency.
- Async ecosystem (Tokio/Axum) aligns with I/O-heavy workloads.
- Performance suitable for on-device optimization and forecasting.
- Higher development complexity compared to scripting languages.

## Alternatives Considered
- Python: Faster prototyping but weaker concurrency guarantees and heavier
  runtime cost on edge devices.
- Go: Strong concurrency model, but less expressive type system for domain
  units and fewer mature scientific/optimization libraries.
- C++: High performance but higher maintenance cost and safety risk.

## References
- README.md architecture overview
- POWER_FLOW_ARCHITECTURE.md
