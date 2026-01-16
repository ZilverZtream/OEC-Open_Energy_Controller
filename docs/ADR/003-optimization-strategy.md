# ADR 003: Optimization strategy layering

## Status
Accepted

## Context
We need optimization that respects physical constraints while still providing
cost-efficient schedules. The system must work with varying data availability
and hardware capabilities.

## Decision
Adopt a layered optimization strategy:
- Default to deterministic dynamic programming or greedy heuristics for
  reliability and speed.
- Add pluggable advanced strategies (MILP/MPC) for higher fidelity.
- Always enforce physical and safety constraints prior to economic objectives.

## Consequences
- Baseline strategies provide predictable runtime on edge devices.
- Advanced optimizers can be introduced without changing the controller core.
- Clear separation between constraint validation and objective optimization.

## Alternatives Considered
- Single MILP optimizer for all cases (too heavy for edge devices).
- Heuristic-only approach (might leave significant economic value unrealized).

## References
- POWER_FLOW_ARCHITECTURE.md
- src/optimizer/strategies/ (strategy implementations)
