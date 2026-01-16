# ADR 005: Forecasting approach

## Status
Accepted

## Context
Forecasting is required for production, consumption, and pricing. The system
must operate with variable data availability and support both simple and ML
models while keeping the controller reliable.

## Decision
Use a hybrid forecasting approach:
- Provide baseline deterministic forecasts (moving average, exponential
  smoothing) for reliability.
- Integrate optional ML-based models for improved accuracy when data is
  available.
- Provide a forecast aggregator that selects the best available model and
  exposes a unified output format.

## Consequences
- Ensures forecasts are always available (fallback models).
- Enables incremental improvements without breaking the controller.
- Requires monitoring to ensure ML models do not degrade unexpectedly.

## Alternatives Considered
- ML-only forecasting (risk of outages or poor performance in low-data cases).
- Rule-based heuristics only (lower accuracy for pricing/production).

## References
- src/forecast/
- docs/ML.md
