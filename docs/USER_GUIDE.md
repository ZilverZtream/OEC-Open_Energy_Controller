# User Guide

This guide explains how to install, configure, and run the Open Energy Controller in a development or home lab setup using simulated hardware. It also covers common troubleshooting steps and FAQ answers.

## Prerequisites

- Rust toolchain (see `rust-toolchain.toml`)
- PostgreSQL 16 (local install or Docker)
- Docker (optional, for database and observability stack)

## Installation

1. Clone the repository and enter it:

   ```bash
   git clone https://github.com/yourusername/open-energy-controller.git
   cd open-energy-controller
   ```

2. Copy the environment template and adjust as needed:

   ```bash
   cp .env.example .env
   ```

3. Start PostgreSQL (Docker option):

   ```bash
   docker-compose up -d postgres
   ```

4. Run database migrations:

   ```bash
   sqlx migrate run
   ```

## Configuration

Configuration loads in the following order (last wins):

1. `config/default.toml`
2. Environment variables prefixed with `OEC__`

Example overrides:

```bash
export OEC__SERVER__HOST=0.0.0.0
export OEC__SERVER__PORT=8080
```

Key configuration files:

- `config/default.toml` for baseline defaults
- `config/development.toml` for local dev overrides
- `config/production.toml` for production deployments

## Running the Controller

Start the service in simulated mode:

```bash
cargo run
```

The HTTP server should be available on the configured host/port (defaults in `config/default.toml`).

## Verifying the API

Quick checks once the service is running:

```bash
curl http://localhost:8080/health
curl http://localhost:8080/api/v1/status
curl http://localhost:8080/api/v1/battery/state
```

OpenAPI docs are exposed at:

```text
http://localhost:8080/swagger-ui
```

## Observability

If you started the monitoring stack via Docker, you can access:

- Metrics: `http://localhost:8080/metrics`
- Grafana: `http://localhost:3000`

## Troubleshooting

### Database connection errors

- Confirm PostgreSQL is running and reachable.
- Verify `OEC__DB__URL` or `DATABASE_URL` matches your local database.
- Re-run migrations with `sqlx migrate run`.

### API returns 401 Unauthorized

- Ensure `OEC__AUTH__TOKEN` matches the token you send in the request.
- If using cURL, include `Authorization: Bearer <token>`.

### Service exits immediately

- Run with `RUST_LOG=debug` to check startup logs.
- Confirm `config/default.toml` exists and is readable.

## FAQ

**Q: Can I run without hardware?**

Yes. The default configuration uses simulated devices for battery, inverter, and EV charger.

**Q: How often does the controller optimize?**

The control loop runs on the `controller.tick_seconds` interval, and re-optimizes every `controller.reoptimize_every_minutes` minutes.

**Q: Where is the data stored?**

Power snapshots, device states, and forecasts are stored in PostgreSQL using the SQLx migrations in `migrations/`.

**Q: How do I reset my database?**

Drop and recreate the database, then run migrations again:

```bash
dropdb energy_controller
createdb energy_controller
sqlx migrate run
```
