# 🔋 Open Energy Controller

**A comprehensive edge-based energy management system with advanced power flow orchestration, battery optimization, and EV charging coordination.**

[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)]()

---

## 🎯 Overview

Open Energy Controller is a sophisticated energy management system designed for residential and small commercial installations. It provides real-time power flow orchestration, coordinating solar production, battery storage, EV charging, and grid interaction to minimize costs while respecting all physical and safety constraints.

**Key capabilities:**
- **Holistic Power Flow Management** - Coordinates all energy flows with constraint-aware optimization
- **EV Charging Coordination** - Deadline-aware charging with fuse protection and solar prioritization
- **Battery Optimization** - Arbitrage trading and self-consumption maximization
- **Hardware Abstraction** - Works with simulated devices for development and testing
- **Production Ready** - Comprehensive observability, error handling, and deployment tooling

---

## ✨ Features

### Core Capabilities
- **Power Flow Orchestration** - Holistic coordination of all energy flows with constraint verification
- **Constraint-Based Control** - Three-tier priority system (Physical → Safety → Economic)
- **EV Charging Management** - Deadline-aware charging with dynamic power allocation
- **Battery Optimization** - Arbitrage trading and solar self-consumption
- **Fuse Protection** - Automatic load management to prevent grid connection overload
- **Device Abstraction** - Hardware-agnostic interfaces (Modbus TCP, OCPP, simulated)
- **Real-Time Forecasting** - Price, consumption, and production prediction
- **Comprehensive Simulation** - Test scenarios without physical hardware

### Hardware Support
- **Battery Systems** - Modbus TCP communication with major manufacturers
- **Solar Inverters** - Production monitoring and control
- **EV Chargers** - OCPP 1.6 protocol support
- **Grid Meters** - Import/export monitoring
- **Simulated Devices** - Full environment simulation for development

### Production Features
- **PostgreSQL Persistence** - Time-series data with partitioning
- **Prometheus Metrics** - Comprehensive observability
- **Structured Logging** - JSON output with tracing spans
- **OpenAPI Documentation** - Auto-generated REST API docs
- **Health Checks** - Kubernetes-ready liveness/readiness probes
- **Graceful Shutdown** - Clean resource cleanup

---

## 🏗️ Architecture

### Power Flow Orchestration

The system uses a holistic power flow model that coordinates all energy sources and sinks simultaneously, ensuring physical constraints are never violated while optimizing for economic objectives.

```
Control Loop (10s interval):
┌─────────────────────────────────────────────┐
│ 1. Sensor Reading                           │
│    • Solar PV production                    │
│    • Household consumption                  │
│    • Battery state                          │
│    • EV charging state                      │
│    • Grid pricing                           │
└─────────────────────────────────────────────┘
                    ↓
┌─────────────────────────────────────────────┐
│ 2. Power Flow Computation                   │
│                                             │
│    Constraint Hierarchy:                    │
│    1. Physical (fuse limits, device caps)   │
│    2. Safety (min SoC, load priority)       │
│    3. Economic (cost minimization)          │
│                                             │
│    Output: PowerSnapshot                     │
│    • Battery command (-5kW to +5kW)         │
│    • EV charger command (6A to 32A)         │
│    • Expected grid flow                     │
└─────────────────────────────────────────────┘
                    ↓
┌─────────────────────────────────────────────┐
│ 3. Command Execution                        │
│    • Battery inverter (Modbus TCP)          │
│    • EV charger (OCPP 1.6)                  │
│    • Monitoring & logging                   │
└─────────────────────────────────────────────┘
```

### System Components

```
┌─────────────────────────────────────────────────────────┐
│                    REST API (Axum)                      │
│    /api/v1/{power-flow, battery, ev, schedule...}      │
└─────────────────────────────────────────────────────────┘
                           │
┌─────────────────────────────────────────────────────────┐
│              PowerFlowController (10s loop)              │
│                                                         │
│  ┌──────────────────────────────────────────────────┐  │
│  │       PowerFlowModel.compute_flows()             │  │
│  │                                                  │  │
│  │  Inputs:                                         │  │
│  │   • PV, House, Battery, EV, Grid, Prices        │  │
│  │                                                  │  │
│  │  Constraints:                                    │  │
│  │   • Physical (fuse, device limits)              │  │
│  │   • Safety (min SoC, house priority)            │  │
│  │   • Economic (prices, self-consumption)         │  │
│  │                                                  │  │
│  │  Output: PowerSnapshot                           │  │
│  │   → Battery command                              │  │
│  │   → EV charger command                           │  │
│  └──────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
                           │
┌─────────────────────────────────────────────────────────┐
│             Service Layer (Forecasting, Optimization)    │
│  ┌─────────────┐  ┌──────────────┐  ┌────────────────┐ │
│  │ Forecaster  │  │  Optimizer   │  │  Safety Monitor│ │
│  │ (ML/Simple) │  │  (DP/MILP)   │  │  (Constraints) │ │
│  └─────────────┘  └──────────────┘  └────────────────┘ │
└─────────────────────────────────────────────────────────┘
                           │
┌─────────────────────────────────────────────────────────┐
│               Hardware Abstraction Layer                 │
│  ┌─────────────┐  ┌──────────────┐  ┌────────────────┐ │
│  │ Simulated   │  │  Modbus TCP  │  │  OCPP Client   │ │
│  │ (Dev/Test)  │  │  (Battery)   │  │  (EV Charging) │ │
│  └─────────────┘  └──────────────┘  └────────────────┘ │
└─────────────────────────────────────────────────────────┘
                           │
┌─────────────────────────────────────────────────────────┐
│                PostgreSQL + Timescale                    │
│     (PowerSnapshots, Devices, States, Schedules)        │
└─────────────────────────────────────────────────────────┘
```

**Key Design Principles:**
- **⚡ Power Flow Orchestration:** Holistic optimization of all energy flows
- **🎯 Priority-Based Control:** Physics → Safety → Economics
- **🔋 EV Deadline Awareness:** Urgency-based charging coordination
- **⚙️ Fuse Protection:** Never exceed grid connection limits
- **🌞 Solar Priority:** Use PV locally before grid import/export
- **Edge-first:** All computation runs locally (Raspberry Pi capable)
- **Trait-based abstraction:** Easy to swap implementations (simulated ↔ real hardware)
- **Async Rust:** Efficient I/O with Tokio
- **Type safety:** Compile-time guarantees for physical units (Power, Energy, Voltage)
- **Production-ready:** Metrics, logging, error handling, testing, CI/CD

**📖 Read [POWER_FLOW_ARCHITECTURE.md](POWER_FLOW_ARCHITECTURE.md) for deep dive on the core algorithm!**

---

## 🚀 Quick Start

### Prerequisites
- **Rust 1.75+** - Install via [rustup](https://rustup.rs/)
- **PostgreSQL 16** - Database server
- **Docker** (optional) - For containerized development

### Installation

```bash
# Clone repository
git clone https://github.com/yourusername/open-energy-controller.git
cd open-energy-controller

# Copy environment template
cp .env.example .env

# Start PostgreSQL (via Docker)
docker-compose up -d postgres

# Run database migrations
sqlx migrate run

# Build and run (development mode with simulated hardware)
cargo run

# Run tests
cargo test

# Run with real hardware (requires Raspberry Pi + Modbus devices)
cargo run --release --features hardware --no-default-features
```

### API Quick Test

```bash
# Check health
curl http://localhost:8080/health

# Get system status
curl http://localhost:8080/api/v1/status

# Get current battery state
curl http://localhost:8080/api/v1/battery/state

# Get forecast
curl http://localhost:8080/api/v1/forecast/combined

# View OpenAPI docs
open http://localhost:8080/swagger-ui
```

---

## 📁 Project Structure

```
open-energy-controller/
├── src/
│   ├── main.rs                    # Application entry point
│   ├── api/                       # REST API (Axum)
│   │   ├── handlers/              # Request handlers
│   │   ├── middleware/            # Auth, logging, rate limiting
│   │   └── routes.rs              # Router configuration
│   ├── domain/                    # Domain models & traits
│   │   ├── battery/               # Battery trait & types
│   │   ├── inverter/              # Inverter trait & types
│   │   └── ev_charger/            # EV charger trait & types
│   ├── hardware/                  # Hardware implementations
│   │   ├── simulated/             # Simulated devices (dev/test)
│   │   ├── modbus/                # Modbus TCP devices
│   │   └── factory.rs             # Device factory pattern
│   ├── optimizer/                 # Optimization algorithms
│   │   └── strategies/            # DP, MILP, MPC, RL
│   ├── forecast/                  # Forecasting pipeline
│   │   ├── price/                 # Electricity price forecasts
│   │   ├── consumption/           # Household consumption
│   │   └── production/            # Solar production
│   ├── controller/                # Real-time control loop
│   ├── discovery/                 # Device discovery (mDNS, Modbus scan)
│   ├── modbus/                    # Modbus client & register maps
│   ├── ocpp/                      # OCPP protocol (EV charging)
│   ├── ml/                        # Machine learning models
│   │   ├── models/                # Model implementations
│   │   ├── training/              # Training pipeline
│   │   └── inference/             # Production inference
│   ├── database/                  # Database layer
│   │   ├── models/                # SQLx models
│   │   └── repositories/          # Repository pattern
│   ├── config/                    # Configuration management
│   └── telemetry/                 # Metrics & logging
├── tests/
│   ├── unit/                      # Unit tests
│   ├── integration/               # Integration tests
│   └── e2e/                       # End-to-end tests
├── migrations/                    # Database migrations
├── docs/
│   ├── ARCHITECTURE.md            # Architecture deep-dive
│   ├── ADR/                       # Architecture Decision Records
│   ├── API.md                     # API documentation
│   ├── MODBUS.md                  # Modbus integration guide
│   └── ML.md                      # Machine learning guide
├── scripts/
│   ├── deploy.sh                  # Deployment script
│   └── seed_db.sh                 # Seed test data
├── config/
│   ├── development.toml           # Dev configuration
│   ├── production.toml            # Prod configuration
│   └── device_profiles/           # Vendor-specific configs
├── Cargo.toml                     # Dependencies
├── docker-compose.yml             # Docker services
├── MASSIVE_TODO_LIST.md           # 850+ item checklist
├── AGENTS.md                      # Instructions for AI agents
├── CLAUDE_CODE.md                 # Claude Code specific guide
└── README.md                      # This file
```

---

## 🔧 Configuration

Configuration is loaded in this order (later overrides earlier):
1. `config/{environment}.toml` file
2. Environment variables (prefix `APP_`)
3. Command-line arguments

### Example Configuration

```toml
# config/development.toml
[server]
host = "127.0.0.1"
port = 8080

[database]
url = "postgres://localhost/energy_controller"
max_connections = 10

[hardware]
mode = "simulated"  # or "real" for Modbus devices

[modbus]
scan_enabled = true
scan_interval_secs = 300

[optimization]
strategy = "dynamic_programming"  # or "greedy", "milp", "mpc"
horizon_hours = 24

[forecasting]
use_ml_models = false  # Set to true when ML models are trained
```

### Environment Variables

```bash
# .env
DATABASE_URL=postgres://user:pass@localhost/energy_controller
RUST_LOG=info,energy_controller=debug
NORDPOOL_API_KEY=your_api_key_here
SMHI_API_URL=https://opendata-download-metfcst.smhi.se
```

---

## 🧪 Testing

```bash
# Run all tests
cargo test

# Run unit tests only
cargo test --lib

# Run integration tests
cargo test --test '*'

# Run specific test
cargo test test_battery_charging

# Run with logging
RUST_LOG=debug cargo test

# Run benchmarks
cargo bench

# Generate coverage report
cargo tarpaulin --out Html
```

---

## 📊 Monitoring

### Prometheus Metrics

Access metrics at: `http://localhost:8080/metrics`

Key metrics:
- `http_requests_total` - API request counter
- `http_request_duration_seconds` - Request latency histogram
- `battery_soc_percent` - Battery state of charge
- `optimization_duration_seconds` - Optimization runtime
- `forecast_accuracy_mape` - Forecast accuracy (MAPE)

### Grafana Dashboards

```bash
# Start monitoring stack
docker-compose up -d grafana prometheus

# Access Grafana
open http://localhost:3000
# Default login: admin/admin
```

Pre-built dashboards available in `config/grafana/dashboards/`

### Logs

Structured JSON logs in production:
```bash
# View logs
journalctl -u energy-controller -f

# Filter by level
journalctl -u energy-controller -p err

# Export logs
journalctl -u energy-controller --since "1 hour ago" > logs.txt
```

---

## 🚢 Deployment

### Raspberry Pi Deployment

```bash
# Cross-compile for ARM
cross build --target aarch64-unknown-linux-gnu --release --features hardware

# Copy to Raspberry Pi
scp target/aarch64-unknown-linux-gnu/release/open-energy-controller pi@raspberrypi.local:/home/pi/

# SSH and setup service
ssh pi@raspberrypi.local
sudo cp energy-controller.service /etc/systemd/system/
sudo systemctl enable energy-controller
sudo systemctl start energy-controller
```

See [docs/DEPLOYMENT.md](docs/DEPLOYMENT.md) for detailed instructions.

### Docker Deployment

```bash
# Build image
docker build -t open-energy-controller .

# Run container
docker-compose up -d
```

## 📚 Documentation

- **[POWER_FLOW_ARCHITECTURE.md](POWER_FLOW_ARCHITECTURE.md)** - Core power flow orchestration system
- **[DEVELOPMENT.md](DEVELOPMENT.md)** - Development guidelines and patterns
- **[AGENTS.md](AGENTS.md)** - Coding standards and best practices  
- **[MASSIVE_TODO_LIST.md](MASSIVE_TODO_LIST.md)** - Complete implementation checklist

---

## 🤝 Contributing

Contributions are welcome! Please read the development guidelines before submitting PRs:

1. Fork the repository
2. Create a feature branch
3. Follow coding standards in `AGENTS.md`
4. Add tests for new functionality
5. Update documentation as needed
6. Submit a pull request

---

## 📝 License

This project is licensed under the MIT License - see [LICENSE](LICENSE) file for details.
