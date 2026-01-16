# Developer Guide

This guide describes how to set up a development environment, navigate the codebase, and contribute safely.

## Development Setup

1. Install Rust via rustup.
2. Install PostgreSQL 16 (local or Docker).
3. Copy environment template:

   ```bash
   cp .env.example .env
   ```

4. Start PostgreSQL (Docker option):

   ```bash
   docker-compose up -d postgres
   ```

5. Run migrations:

   ```bash
   sqlx migrate run
   ```

6. Run the app:

   ```bash
   cargo run
   ```

## Project Structure

Core directories:

- `src/api/` - HTTP API handlers, routes, middleware
- `src/controller/` - real-time orchestration loop
- `src/domain/` - core domain types and traits
- `src/hardware/` - simulated and Modbus hardware integrations
- `src/forecast/` - forecasting pipeline
- `src/optimizer/` - scheduling and optimization strategies
- `src/database/` - SQLx models and repositories
- `src/telemetry/` - metrics and tracing

## Configuration Conventions

- Configuration is loaded from `config/default.toml` and overridden by `OEC__`-prefixed env vars.
- Keep new configuration fields documented in `config/*.toml` examples.

## Coding Standards

Follow the repository rules in `AGENTS.md`:

- No `unwrap()` or `expect()` in production code.
- All public items must have doc comments.
- Write tests for new logic.
- Integrate new modules (no orphaned code).
- Update `MASSIVE_TODO_LIST.md` when tasks are completed.

## Testing & Quality Checks

Run these before every commit:

```bash
cargo fmt
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

If you add feature-gated behavior, run the relevant feature tests:

```bash
cargo test --features hardware
cargo test --features ml
```

## Database Changes

When adding database tables or columns:

1. Add a new migration in `migrations/`.
2. Update SQLx models in `src/database/models/`.
3. Add repository methods and tests.
4. Run `sqlx migrate run` locally.

## API Changes

When adding API endpoints:

1. Add handler in `src/api/handlers/`.
2. Wire routes in `src/api/routes/`.
3. Update OpenAPI annotations.
4. Add integration tests under `tests/integration/`.

## Documentation Updates

If you modify behavior or introduce new components, update:

- `docs/ARCHITECTURE.md` or relevant module docs
- `docs/API.md` (if endpoints change)
- `README.md` for user-facing changes

## CI/CD Pipeline

CI/CD is planned but not yet fully implemented. When workflows are added under
`.github/workflows/`, keep them consistent with the local checks:

- `cargo fmt --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all-features`

Document any new CI jobs in this guide as they are introduced.

## Pull Requests

- Keep commits focused and atomic.
- Include a summary of changes and tests.
- Ensure all TODO items you completed are checked off in `MASSIVE_TODO_LIST.md`.

## Contribution Guidelines

See `CONTRIBUTING.md` for coding standards, review expectations, and how to
submit changes.
