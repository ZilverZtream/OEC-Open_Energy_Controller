# ADR 004: Modbus abstraction layer

## Status
Accepted

## Context
The controller must support multiple hardware vendors with different Modbus
register layouts. We need a uniform interface so the rest of the system can
remain hardware-agnostic.

## Decision
Implement a Modbus abstraction layer with vendor-specific register maps and a
common device trait interface. Hardware integrations translate raw register
values into domain types and expose a consistent API.

## Consequences
- Simplifies adding new vendors by updating register maps.
- Keeps domain logic independent of hardware details.
- Requires careful mapping and validation for each vendor.

## Alternatives Considered
- Direct register usage in each device implementation (increases duplication
  and risk of inconsistencies).
- Vendor SDK integration (limited availability and reduced portability).

## References
- docs/MODBUS.md
- src/modbus/
- src/hardware/modbus/
