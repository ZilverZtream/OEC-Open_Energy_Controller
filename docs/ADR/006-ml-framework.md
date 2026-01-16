# ADR 006: ML framework choices

## Status
Accepted

## Context
The ML subsystem should support classic regression models and allow future
expansion to deep learning while remaining deployable on edge devices.

## Decision
Adopt a mixed ML stack:
- Use `linfa` and `smartcore` for classical models and feature-based
  forecasting.
- Use ONNX runtime for deployment of exported models.
- Keep optional deep learning framework usage behind feature flags.

## Consequences
- Provides lightweight models suitable for edge inference.
- ONNX enables portability across training environments.
- Multiple libraries increase maintenance and dependency management.

## Alternatives Considered
- Single deep learning framework (heavier dependencies and resource usage).
- Python-based inference (not aligned with edge-first, Rust-native approach).

## References
- src/ml/
- docs/ML.md
