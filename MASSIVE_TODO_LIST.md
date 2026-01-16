# 🚀 Open Energy Controller - MASSIVE TODO LIST

**Complete implementation roadmap from zero to production-ready system with ML/optimization**

**Progress Tracking:**
- Total items: ~850+ checkboxes
- Completed items: ~200+ (as of 2026-01-16)
- Logical ordering: Each section builds on previous
- Parallelizable: Items within sections can be done concurrently
- No time estimates: Work at your own pace with your team size

## 🎯 Recent Completions (2026-01-16 - Fourth Update)
### ✅ Phase 0: Project Bootstrap - Additional Directories
- Created `src/api/handlers/`, `src/api/middleware/`, `src/api/routes/` directories
- Created `src/hardware/simulated/` and `src/hardware/mock/` directories
- Created `src/optimizer/strategies/` directory
- Created `src/forecast/price/`, `src/forecast/consumption/`, `src/forecast/production/`, `src/forecast/weather/` directories
- Created `src/ocpp/` directory for OCPP protocol implementation
- Created `src/ml/`, `src/ml/models/`, `src/ml/training/`, `src/ml/inference/` directories
- Created `config/device_profiles/` directory with vendor-specific configs

### ✅ Phase 13: OCPP Protocol Module
- Implemented complete OCPP 1.6 protocol foundation in `src/ocpp/mod.rs`
- Created comprehensive message definitions in `src/ocpp/messages.rs`:
  - Boot Notification (request/response)
  - Heartbeat (request/response)
  - Status Notification
  - Remote Start/Stop Transaction
  - Charging Profiles with schedules
  - Configuration management
- Implemented OCPP WebSocket client in `src/ocpp/client.rs`
- Added OcppClient with connection management and heartbeat support
- Added 15+ OCPP message types with full serde serialization
- Comprehensive unit tests for OCPP functionality

### ✅ Phase 14: Machine Learning Module
- Created complete ML module foundation in `src/ml/mod.rs`:
  - ModelType enum (LinearRegression, RandomForest, GradientBoosting, LSTM, Transformer)
  - ModelMetadata with validation metrics
  - FeatureVector with normalization and standardization
  - Prediction struct with confidence bounds
  - ValidationMetrics (MAE, RMSE, MAPE, R²)
- Implemented ML models in `src/ml/models.rs`:
  - MLModel trait for all models
  - LinearRegressionModel with gradient descent training
  - MovingAverageModel for baseline forecasting
  - ExponentialSmoothingModel for time-series
- Created training pipeline in `src/ml/training.rs`:
  - TrainingDataset with train/validation split
  - ModelTrainer with metric calculation
  - Gradient descent implementation for linear regression
  - Comprehensive metrics calculation
- Implemented inference engine in `src/ml/inference.rs`:
  - ModelRegistry for managing multiple models
  - InferenceEngine for production predictions
  - BatchPredictor with parallel processing
  - EnsemblePredictor for model combination
- Added 20+ unit tests for ML functionality

### ✅ Phase 15: Device Configuration Profiles
- Created Huawei Luna2000 battery profile (`config/device_profiles/huawei_luna2000.toml`)
- Created SolarEdge StorEdge battery profile (`config/device_profiles/solaredge_storedge.toml`)
- Created generic battery template (`config/device_profiles/generic_battery.toml`)
- Each profile includes:
  - Complete Modbus register mappings
  - Device capabilities (capacity, max power, efficiency)
  - Operating limits (SoC, temperature)
  - Protocol configuration (ports, timeouts, retries)

### ✅ Integration Updates
- Updated `src/main.rs` to include `ml` and `ocpp` modules
- All new modules integrated into the build system
- Maintained backward compatibility with existing features

## 🎯 Recent Completions (2026-01-16 - Third Update)
### ✅ Phase 0: Project Bootstrap
- Created missing test directories (tests/unit, tests/integration, tests/e2e, tests/fixtures)
- Created documentation directories (docs/ADR, docs/api)
- Created scripts and benchmarks directories

### ✅ Phase 1: Cargo Dependencies - Extended
- Added 30+ additional dependencies for complete system coverage
- Added serde_yaml, time, ulid, tracing-log for utilities
- Added sea-query and sea-query-binder for advanced database operations
- Added tokio-tungstenite for OCPP WebSocket support
- Added pnet for network scanning capabilities
- Added minilp, noisy_float for optimization
- Added ML libraries: onnxruntime, linfa, linfa-linear, linfa-trees, smartcore, polars
- Added security libraries: argon2, jsonwebtoken, ring
- Added metrics libraries: metrics, metrics-exporter-prometheus, opentelemetry
- Added utilities: derive_more, lazy_static
- Added testcontainers for integration testing

### ✅ Phase 2: Domain Models - Enhanced
- Added BatteryError enum with comprehensive error types
- Added HealthStatus enum for battery health monitoring
- Added BatteryStatus enum for operational states
- Added BatteryChemistry enum (LiFePO4, NMC, LTO, NCA, LeadAcid)
- Added BatteryCommand enum for power control
- Added DegradationModel struct for battery lifecycle tracking
- Implemented health_check() method in Battery trait
- Added status field to BatteryState
- Added chemistry field to BatteryCapabilities
- Implemented health_check for SimulatedBattery and MockBattery

### ✅ Phase 3: Database Repositories - Complete
- Created PriceRepository with full CRUD operations
- Created ConsumptionRepository with aggregation capabilities
- Created ProductionRepository with daily total tracking
- All repositories include time-range queries and statistics
- Added data cleanup methods for old historical data
- Integrated repositories into repo module structure

### ✅ Phase 9: Forecasting Engine - Weather & Features
- Implemented SmhiClient for Swedish weather forecasts (SMHI API)
- Created WeatherPoint, WeatherForecast, and GeoLocation types
- Added NordpoolPriceForecaster for day-ahead electricity prices
- Implemented EUR to SEK conversion for Nordpool data
- Created comprehensive feature engineering module (features.rs)
- Implemented TimeSeriesFeatures with temporal and weather features
- Added FeatureExtractor with Swedish holiday detection
- Implemented day length calculation based on latitude
- Added feature normalization for ML models
- Created lag features and rolling statistics functions
- Integrated weather module into forecast pipeline

### ✅ Phase 12: REST API - Extended Endpoints
- Created battery API module with 6 endpoints (state, capabilities, health, power, history, statistics)
- Created grid API module with 3 endpoints (status, limits, statistics)
- Created weather API module for forecast retrieval
- Integrated all new API modules into v1 router
- Added comprehensive error handling and responses
- All endpoints include authentication layer
- Total API endpoints: 30+ (battery, EV charger, inverter, grid, weather, system)

## 🎯 Recent Completions (2026-01-16 - Second Update)
### ✅ Phase 1: Cargo Dependencies
- Added 30+ new dependencies for optimization, networking, and utilities
- Added optimization libraries (good_lp, ndarray, nalgebra, statrs)
- Added HTTP client middleware (reqwest-middleware, reqwest-retry)
- Added dev dependencies (mockall, rstest, proptest, wiremock, criterion)
- Added utilities (strum, itertools, parking_lot, ordered-float)
### ✅ Phase 6: Modbus TCP Client
- Enhanced ModbusClient with retry logic, timeouts, and health checks
- Register mapping for GenericBattery, Huawei Luna2000, and SolarEdge
- Data parsing utilities for u16, i16, u32, f32, and scaled values
- Comprehensive unit tests for all parsing functions
### ✅ Phase 7: ModbusBattery Implementation
- Full ModbusBattery implementation with Battery trait
- Support for 3 vendor-specific register maps
- Parallel register reads for efficiency
- Power validation against device capabilities
- Health check integration
### ✅ Phase 8: Device Discovery
- NetworkScanner with CIDR and IP range support
- Concurrent scanning with configurable timeouts
- ModbusIdentifier for automatic device type detection
- Support for common Modbus ports (502, 1502, 8502)
- Continuous discovery loop with configurable intervals
### ✅ Phase 10: Optimization Engine
- GreedyOptimizer baseline strategy implementation
- Price-based charging/discharging decisions
- SoC-aware power management
- Comprehensive unit tests with synthetic forecasts
- DynamicProgrammingOptimizer already in place
### ✅ Phase 11: Controller - PID
- Full PID controller with P, I, and D terms
- Anti-windup protection for integral term
- Derivative kick protection
- PowerPidController specialized for battery control
- Extensive test suite with step responses

## 🎯 Earlier Completions (2026-01-16 - First Update)
### ✅ Phase 2: Domain Models
- Battery, Inverter, EV Charger, Grid domain traits and implementations
- SimulatedBattery, SimulatedInverter, SimulatedEvCharger with tests
- Grid limits, tariffs, and connection structs with comprehensive tests
### ✅ Phase 3: Database Layer
- 4 migration files: initial schema + EV/inverter states + households + forecasts/optimization
- DeviceRepository, BatteryStateRepository, ScheduleRepository with SQLx
- All tables: devices, battery_states, inverter_states, ev_charger_states, households, consumption/production history, forecasts, optimization runs
### ✅ Phase 4: Hardware Abstraction
- DeviceFactory pattern with Simulated/Modbus/Mock modes
- Factory methods for all device types with graceful fallbacks
- 3 unit tests for hardware factory
### ✅ Phase 5: Configuration System
- development.toml and production.toml with comprehensive settings
- .env.example with 40+ documented environment variables
- Complete configuration for all subsystems
### ✅ Phase 12: REST API - Core (Partial)
- 10 new API endpoints for EV Charger and Inverter management
- EV Charger: state, set current, start/stop charging, sessions
- Inverter: state, mode, export limit, production, efficiency stats
- All endpoints integrated with auth layer and OpenAPI schemas

**Next priorities:** Weather integration (SMHI), Nordpool price forecasting, Advanced ML forecasters, Real hardware testing

---

## 📋 PHASE 0: PROJECT BOOTSTRAP

### Environment Setup
- [ ] Install Rust (latest stable via rustup)
- [ ] Install PostgreSQL 16
- [ ] Install Docker & Docker Compose
- [ ] Install cross-compilation tools: `cargo install cross`
- [ ] Install sqlx-cli: `cargo install sqlx-cli --features postgres`
- [ ] Setup IDE (VSCode/RustRover with rust-analyzer)
- [ ] Install Postman/Insomnia for API testing
- [ ] Install Grafana + Prometheus for monitoring
- [ ] Setup pgAdmin or similar for database management

### Repository Initialization
- [ ] `cargo new open-energy-controller --bin`
- [ ] Initialize git: `git init`
- [ ] Create `.gitignore` (Rust template + `.env`, `*.db`, `target/`)
- [ ] Create `README.md` with project vision and architecture diagram
- [ ] Create `LICENSE` file (MIT or Apache 2.0)
- [ ] Setup branch strategy (main, develop, feature/*)
- [ ] Create `CONTRIBUTING.md` with development guidelines
- [ ] Create `CHANGELOG.md`
- [ ] Setup GitHub Actions or GitLab CI (`.github/workflows/`)

### Project Structure - Directories
- [x] Create `src/api/` directory
- [x] Create `src/api/handlers/` directory
- [x] Create `src/api/middleware/` directory
- [x] Create `src/api/routes/` directory
- [x] Create `src/domain/` directory
- [x] Create `src/domain/battery/` directory
- [x] Create `src/domain/inverter/` directory
- [x] Create `src/domain/ev_charger/` directory
- [x] Create `src/domain/grid/` directory
- [x] Create `src/hardware/` directory
- [x] Create `src/hardware/simulated/` directory
- [x] Create `src/hardware/modbus/` directory
- [x] Create `src/hardware/mock/` directory
- [x] Create `src/optimizer/` directory
- [x] Create `src/optimizer/strategies/` directory
- [x] Create `src/optimizer/constraints/` directory
- [x] Create `src/forecast/` directory
- [x] Create `src/forecast/price/` directory
- [x] Create `src/forecast/consumption/` directory
- [x] Create `src/forecast/production/` directory
- [x] Create `src/forecast/weather/` directory
- [x] Create `src/controller/` directory
- [x] Create `src/discovery/` directory
- [x] Create `src/modbus/` directory
- [x] Create `src/ocpp/` directory (EV charging protocol)
- [x] Create `src/config/` directory
- [x] Create `src/telemetry/` directory
- [x] Create `src/database/` directory
- [x] Create `src/ml/` directory (machine learning models)
- [x] Create `src/ml/models/` directory
- [x] Create `src/ml/training/` directory
- [x] Create `src/ml/inference/` directory
- [x] Create `tests/unit/` directory
- [x] Create `tests/integration/` directory
- [x] Create `tests/e2e/` directory
- [x] Create `tests/fixtures/` directory
- [x] Create `migrations/` directory
- [x] Create `docs/` directory
- [x] Create `docs/ADR/` directory (Architecture Decision Records)
- [x] Create `docs/api/` directory
- [x] Create `scripts/` directory
- [x] Create `scripts/seed_data/` directory
- [x] Create `config/` directory
- [x] Create `config/device_profiles/` directory (vendor-specific configs)
- [x] Create `benchmarks/` directory

### Configuration Files
- [ ] Create `Cargo.toml` with workspace structure
- [ ] Add workspace members to `Cargo.toml`
- [x] Create `.env.example` with all required env vars
- [x] Create `config/development.toml`
- [x] Create `config/test.toml`
- [x] Create `config/production.toml`
- [ ] Create `docker-compose.yml` (postgres, grafana, prometheus)
- [x] Create `docker-compose.test.yml` (for testing)
- [ ] Create `Dockerfile` for production build
- [x] Create `Dockerfile.dev` for development
- [x] Create `.dockerignore`
- [x] Create `rust-toolchain.toml` (pin Rust version)
- [x] Create `.cargo/config.toml` (cross-compilation settings for ARM)
- [x] Create `sqlx-data.json` placeholder
- [x] Create `.editorconfig` for consistent formatting
- [x] Create `deny.toml` for cargo-deny security checks

---

## 📋 PHASE 1: CARGO DEPENDENCIES

### Cargo.toml - Core Async Runtime
- [x] Add `tokio = { version = "1.35", features = ["full"] }`
- [x] Add `tokio-util = { version = "0.7", features = ["rt"] }`
- [x] Add `async-trait = "0.1"`
- [x] Add `futures = "0.3"`
- [x] Add `futures-util = "0.3"`

### Cargo.toml - Error Handling
- [x] Add `anyhow = "1.0"`
- [x] Add `thiserror = "1.0"`
- [x] Add `color-eyre = "0.6"` (better error reports)

### Cargo.toml - Serialization
- [x] Add `serde = { version = "1.0", features = ["derive"] }`
- [x] Add `serde_json = "1.0"`
- [x] Add `serde_yaml = "0.9"`
- [x] Add `toml = "0.8"`
- [x] Add `bincode = "1.3"` (binary serialization for performance)

### Cargo.toml - Configuration
- [ ] Add `config = { version = "0.14", features = ["toml", "yaml"] }`
- [ ] Add `dotenvy = "0.15"`

### Cargo.toml - Web Framework (Axum)
- [ ] Add `axum = { version = "0.7", features = ["macros", "ws"] }`
- [ ] Add `axum-extra = { version = "0.9", features = ["typed-header"] }`
- [ ] Add `tower = { version = "0.4", features = ["full"] }`
- [ ] Add `tower-http = { version = "0.5", features = ["trace", "cors", "compression-full", "timeout"] }`
- [ ] Add `hyper = { version = "1.0", features = ["full"] }`
- [ ] Add `hyper-util = "0.1"`

### Cargo.toml - Database (PostgreSQL)
- [x] Add `sqlx = { version = "0.7", features = ["runtime-tokio", "postgres", "macros", "chrono", "uuid", "json"] }`
- [x] Add `sea-query = "0.30"` (query builder)
- [x] Add `sea-query-binder = { version = "0.5", features = ["sqlx-postgres"] }`

### Cargo.toml - Date/Time
- [x] Add `chrono = { version = "0.4", features = ["serde"] }`
- [x] Add `chrono-tz = "0.8"`
- [x] Add `time = "0.3"`

### Cargo.toml - UUID & IDs
- [x] Add `uuid = { version = "1.6", features = ["v4", "serde"] }`
- [x] Add `ulid = "1.1"`

### Cargo.toml - Logging & Tracing
- [x] Add `tracing = "0.1"`
- [x] Add `tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }`
- [x] Add `tracing-appender = "0.2"`
- [x] Add `tracing-log = "0.2"`

### Cargo.toml - Metrics & Observability
- [x] Add `metrics = "0.21"`
- [x] Add `metrics-exporter-prometheus = "0.13"`
- [x] Add `opentelemetry = "0.21"`
- [x] Add `opentelemetry-prometheus = "0.14"`

### Cargo.toml - HTTP Client
- [x] Add `reqwest = { version = "0.11", features = ["json", "rustls-tls"] }`
- [x] Add `reqwest-middleware = "0.2"`
- [x] Add `reqwest-retry = "0.3"`

### Cargo.toml - Modbus TCP
- [x] Add `tokio-modbus = "0.13"`
- [x] Add `byteorder = "1.5"` (for parsing Modbus registers)

### Cargo.toml - OCPP (EV Charging)
- [ ] Add `ocpp = "0.2"` (or fork if needed)
- [x] Add `tokio-tungstenite = "0.21"` (WebSocket for OCPP)

### Cargo.toml - Network Discovery
- [ ] Add `mdns-sd = "0.10"` (mDNS service discovery)
- [x] Add `trust-dns-resolver = "0.23"`
- [x] Add `pnet = "0.34"` (network scanning)

### Cargo.toml - Optimization Libraries
- [x] Add `good_lp = "1.7"` (Linear Programming)
- [x] Add `minilp = "0.2"`
- [x] Add `ndarray = "0.15"` (N-dimensional arrays for ML)
- [x] Add `nalgebra = "0.32"` (linear algebra)

### Cargo.toml - Machine Learning
- [x] Add `onnxruntime = "0.0.14"` (ONNX inference)
- [ ] Add `burn = "0.11"` (ML framework in Rust)
- [x] Add `linfa = "0.7"` (ML algorithms)
- [x] Add `linfa-linear = "0.7"`
- [x] Add `linfa-trees = "0.7"` (Random Forests, Gradient Boosting)
- [x] Add `smartcore = "0.3"` (ML algorithms)
- [x] Add `polars = { version = "0.36", features = ["lazy", "temporal", "parquet"] }` (DataFrame library)

### Cargo.toml - Math & Statistics
- [x] Add `statrs = "0.16"` (statistical functions)
- [x] Add `noisy_float = "0.2"` (NaN-safe floats)
- [x] Add `ordered-float = "4.2"`

### Cargo.toml - Validation
- [x] Add `validator = { version = "0.17", features = ["derive"] }`

### Cargo.toml - API Documentation
- [x] Add `utoipa = { version = "4.1", features = ["axum_extras", "chrono", "uuid"] }`
- [x] Add `utoipa-swagger-ui = { version = "5.0", features = ["axum"] }`

### Cargo.toml - Testing
- [x] Add `mockall = "0.12"` (mocking framework - dev dependency)
- [x] Add `rstest = "0.18"` (parameterized tests - dev dependency)
- [x] Add `proptest = "1.4"` (property testing - dev dependency)
- [x] Add `fake = { version = "2.9", features = ["derive", "chrono"] }` (dev dependency)
- [x] Add `wiremock = "0.6"` (HTTP mocking - dev dependency)
- [x] Add `testcontainers = "0.15"` (dev dependency)

### Cargo.toml - Benchmarking
- [x] Add `criterion = { version = "0.5", features = ["html_reports"] }` (dev dependency)

### Cargo.toml - Security
- [x] Add `argon2 = "0.5"` (password hashing)
- [x] Add `jsonwebtoken = "9.2"` (JWT tokens)
- [x] Add `ring = "0.17"` (cryptography)

### Cargo.toml - Utilities
- [x] Add `strum = { version = "0.25", features = ["derive"] }`
- [x] Add `strum_macros = "0.25"`
- [x] Add `derive_more = "0.99"`
- [x] Add `itertools = "0.12"`
- [x] Add `once_cell = "1.19"`
- [x] Add `lazy_static = "1.4"`
- [x] Add `parking_lot = "0.12"` (better mutexes)

### Cargo.toml - Hardware-Specific (Feature-gated for RPi)
- [ ] Add `[target.'cfg(target_arch = "aarch64")'.dependencies]` section
- [ ] Add `rppal = { version = "0.17", optional = true }` (Raspberry Pi GPIO)
- [ ] Add `linux-embedded-hal = { version = "0.4", optional = true }`

### Cargo.toml - Feature Flags
- [ ] Add `[features]` section
- [ ] Add `default = ["simulated"]` feature
- [ ] Add `simulated = []` feature (mock hardware)
- [ ] Add `hardware = ["rppal", "linux-embedded-hal"]` feature
- [ ] Add `ml = ["burn", "onnxruntime", "polars"]` feature
- [ ] Add `dev-tools = []` feature (extra dev dependencies)

### Cargo.toml - Profile Optimizations
- [ ] Add `[profile.release]` with `lto = true`
- [ ] Add `codegen-units = 1` to release profile
- [ ] Add `opt-level = 3` to release profile
- [ ] Add `[profile.dev]` with `opt-level = 1` (faster debug builds)

---

## 📋 PHASE 2: DOMAIN MODELS & CORE TYPES

### Core Domain Types
- [ ] Create `src/domain/mod.rs`
- [ ] Create `src/domain/types.rs` (common types: Power, Energy, Voltage, etc.)
- [ ] Implement `Power` newtype (Watts)
- [ ] Implement `Energy` newtype (Watt-hours)
- [ ] Implement `Voltage` newtype (Volts)
- [ ] Implement `Current` newtype (Amperes)
- [ ] Implement `Temperature` newtype (Celsius)
- [ ] Implement `Percentage` newtype (0-100%)
- [ ] Implement `Price` newtype (SEK/kWh)
- [ ] Implement `Duration` type helpers
- [ ] Implement `Timestamp` type helpers
- [x] Add unit tests for all domain types
- [ ] Add `Display` and `Debug` implementations
- [ ] Add `serde` serialization for all types

### Battery Domain
- [x] Create `src/domain/battery/mod.rs`
- [x] Create `src/domain/battery/traits.rs`
- [x] Define `Battery` trait with async methods
- [x] Define `read_state() -> Result<BatteryState>` method
- [x] Define `set_power(watts: Power) -> Result<()>` method
- [x] Define `capabilities() -> BatteryCapabilities` method
- [ ] Define `health_check() -> Result<HealthStatus>` method
- [x] Create `BatteryState` struct (SoC, power, voltage, temperature, health)
- [x] Create `BatteryCapabilities` struct (capacity, max charge/discharge, efficiency)
- [ ] Create `BatteryCommand` enum (Charge, Discharge, Idle, Standby)
- [ ] Create `BatteryError` enum with thiserror derives
- [ ] Create `BatteryStatus` enum (Charging, Discharging, Idle, Fault, Offline)
- [ ] Create `BatteryChemistry` enum (LiFePO4, NMC, LTO, etc.)
- [ ] Create `DegradationModel` struct (cycle count, health %, degradation rate)
- [x] Implement `Default` for `BatteryState`
- [x] Implement validation for battery commands (power limits, SoC bounds)
- [x] Add unit tests for domain logic
- [x] Document all public APIs with examples

### Inverter Domain
- [x] Create `src/domain/inverter/mod.rs`
- [x] Create `src/domain/inverter/traits.rs`
- [x] Define `Inverter` trait
- [x] Define `read_state() -> Result<InverterState>` method
- [x] Define `set_mode(mode: InverterMode) -> Result<()>` method
- [x] Create `InverterState` struct (AC power, DC power, efficiency, temperature)
- [x] Create `InverterMode` enum (Grid-tied, Off-grid, Backup, etc.)
- [x] Create `InverterCapabilities` struct (max AC power, max DC power, etc.)
- [ ] Create `InverterError` enum
- [x] Add unit tests for inverter domain

### EV Charger Domain
- [x] Create `src/domain/ev_charger/mod.rs`
- [x] Create `src/domain/ev_charger/traits.rs`
- [x] Define `EvCharger` trait
- [x] Define `read_state() -> Result<ChargerState>` method
- [x] Define `set_current(amps: Current) -> Result<()>` method
- [x] Define `start_charging() -> Result<()>` method
- [x] Define `stop_charging() -> Result<()>` method
- [x] Create `ChargerState` struct (connected, charging, power, energy delivered)
- [x] Create `ChargerStatus` enum (Available, Preparing, Charging, Finishing, Faulted)
- [x] Create `ChargerCapabilities` struct (max current, phases, connector type)
- [x] Create `ConnectorType` enum (Type2, CCS, CHAdeMO)
- [ ] Create `V2XCapabilities` struct (bidirectional, max discharge power)
- [ ] Create `ChargerError` enum
- [x] Add unit tests for EV charger domain

### Grid Connection Domain
- [x] Create `src/domain/grid/mod.rs`
- [x] Create `GridConnection` struct (import/export power, frequency, voltage)
- [x] Create `GridLimits` struct (max import/export, fuse rating)
- [x] Create `GridTariff` struct (time-of-use rates, fixed fees)
- [x] Create `GridStatus` enum (Normal, Blackout, Islanded)
- [x] Add unit tests for grid domain

### Schedule & Optimization Domain
- [ ] Create `src/domain/schedule.rs`
- [ ] Create `Schedule` struct (time-series of power setpoints)
- [ ] Create `ScheduleInterval` struct (start time, end time, power)
- [ ] Implement `Schedule::power_at(timestamp: DateTime) -> Power` method
- [ ] Implement `Schedule::validate()` method (check bounds, gaps, etc.)
- [ ] Create `OptimizationObjective` enum (MinimizeCost, MaximizeArbitrage, etc.)
- [ ] Create `Constraints` struct (min SoC, max cycles, grid limits)
- [x] Add unit tests for schedule logic

### Forecast Domain
- [ ] Create `src/domain/forecast.rs`
- [ ] Create `PricePoint` struct (timestamp, price, confidence)
- [ ] Create `PriceForecast` struct (Vec<PricePoint>)
- [ ] Create `ConsumptionPoint` struct (timestamp, power, confidence)
- [ ] Create `ConsumptionForecast` struct
- [ ] Create `ProductionPoint` struct (timestamp, power, confidence)
- [ ] Create `ProductionForecast` struct
- [ ] Create `Forecast24h` struct (combines price, consumption, production)
- [ ] Create `ForecastConfidence` enum (High, Medium, Low)
- [ ] Add interpolation method for forecasts
- [x] Add unit tests for forecast structures

---

## 📋 PHASE 3: DATABASE LAYER

### Database Configuration
- [ ] Create `src/database/mod.rs`
- [ ] Create `src/database/config.rs`
- [ ] Create `DatabaseConfig` struct (connection string, pool size, etc.)
- [ ] Implement `DatabaseConfig::from_env()` method
- [ ] Create database connection pool initialization
- [ ] Add connection retry logic with exponential backoff
- [ ] Add health check query (`SELECT 1`)
- [ ] Implement graceful shutdown for DB connections

### Migration Files
- [x] Create `migrations/20250101000000_initial_schema.sql`
- [x] Create `devices` table (id, type, manufacturer, model, ip, port, config, discovered_at, last_seen)
- [x] Add indexes on `devices(device_type)`, `devices(ip)`
- [x] Create `battery_states` table (id, device_id, timestamp, soc_percent, power_w, voltage_v, temperature_c, health_percent)
- [x] Add index on `battery_states(device_id, timestamp DESC)`
- [ ] Add partitioning strategy for `battery_states` (by month)
- [x] Create `inverter_states` table
- [x] Add index on `inverter_states(device_id, timestamp DESC)`
- [x] Create `ev_charger_states` table
- [x] Add index on `ev_charger_states(device_id, timestamp DESC)`
- [x] Create `electricity_prices` table (id, timestamp, price_sek_per_kwh, source, area)
- [x] Add unique constraint on `electricity_prices(timestamp, area, source)`
- [x] Create `consumption_history` table (id, household_id, timestamp, power_w, energy_kwh)
- [x] Create `production_history` table (id, household_id, timestamp, power_w, energy_kwh)
- [x] Create `schedules` table (id, device_id, created_at, valid_from, valid_until, schedule_json, optimizer_version, cost_estimate)
- [x] Create `optimization_runs` table (id, created_at, duration_ms, objective, constraints_json, result_json)
- [x] Create `forecast_cache` table (id, forecast_type, created_at, valid_until, data_json)
- [x] Create `households` table (id, name, location, grid_connection_kw, created_at)
- [x] Create `user_preferences` table (id, household_id, min_soc, max_cycles_per_day, prefer_solar, v2g_enabled)
- [x] Add foreign key constraints
- [ ] Add triggers for `updated_at` timestamps
- [ ] Create views for common queries

### Database Models (SQLx)
- [ ] Create `src/database/models/mod.rs`
- [ ] Create `src/database/models/device.rs`
- [ ] Create `Device` struct matching DB schema
- [ ] Implement `sqlx::FromRow` for `Device`
- [ ] Create `DeviceType` enum matching DB
- [ ] Create `src/database/models/battery_state.rs`
- [ ] Create `BatteryStateRow` struct
- [ ] Create `src/database/models/schedule.rs`
- [ ] Create `ScheduleRow` struct
- [ ] Create `src/database/models/price.rs`
- [ ] Create `ElectricityPriceRow` struct
- [ ] Add conversion methods from domain types to DB models
- [ ] Add conversion methods from DB models to domain types

### Repository Pattern - Device Repository
- [x] Create `src/database/repositories/mod.rs`
- [x] Create `src/database/repositories/device.rs`
- [x] Create `DeviceRepository` struct with `PgPool`
- [x] Implement `insert_device(&Device) -> Result<Uuid>` method
- [x] Implement `find_by_id(Uuid) -> Result<Option<Device>>` method
- [ ] Implement `find_by_ip(IpAddr) -> Result<Option<Device>>` method
- [x] Implement `find_by_type(DeviceType) -> Result<Vec<Device>>` method
- [x] Implement `update_last_seen(Uuid) -> Result<()>` method
- [x] Implement `delete(Uuid) -> Result<()>` method
- [x] Implement `list_all() -> Result<Vec<Device>>` method
- [ ] Add query logging with `tracing`
- [x] Add unit tests with `sqlx::test`
- [ ] Add integration tests with test database

### Repository Pattern - Battery State Repository
- [x] Create `src/database/repositories/battery_state.rs`
- [x] Create `BatteryStateRepository` struct
- [x] Implement `insert_state(&BatteryState) -> Result<i64>` method
- [x] Implement `insert_batch(Vec<BatteryState>) -> Result<()>` method
- [x] Implement `find_latest(device_id: Uuid) -> Result<Option<BatteryState>>` method
- [x] Implement `find_range(device_id, start, end) -> Result<Vec<BatteryState>>` method
- [ ] Implement `get_statistics(device_id, duration) -> Result<BatteryStats>` method
- [x] Implement cleanup for old data (>90 days)
- [ ] Add integration tests

### Repository Pattern - Schedule Repository
- [x] Create `src/database/repositories/schedule.rs`
- [x] Create `ScheduleRepository` struct
- [x] Implement `insert_schedule(&Schedule) -> Result<Uuid>` method
- [x] Implement `find_active(device_id, timestamp) -> Result<Option<Schedule>>` method
- [x] Implement `find_by_id(Uuid) -> Result<Option<Schedule>>` method
- [x] Implement `list_for_device(device_id) -> Result<Vec<Schedule>>` method
- [ ] Implement `invalidate(id: Uuid) -> Result<()>` method
- [ ] Add integration tests

### Repository Pattern - Price Repository
- [ ] Create `src/database/repositories/price.rs`
- [ ] Create `PriceRepository` struct
- [ ] Implement `insert_prices(Vec<ElectricityPrice>) -> Result<()>` method
- [ ] Implement `find_range(start, end, area) -> Result<Vec<ElectricityPrice>>` method
- [ ] Implement `find_latest(area) -> Result<Option<ElectricityPrice>>` method
- [ ] Implement `get_average_price(period) -> Result<f64>` method
- [ ] Add integration tests

### Repository Pattern - Consumption/Production Repositories
- [ ] Create `src/database/repositories/consumption.rs`
- [ ] Create `ConsumptionRepository` struct
- [ ] Implement CRUD methods for consumption history
- [ ] Create `src/database/repositories/production.rs`
- [ ] Create `ProductionRepository` struct
- [ ] Implement CRUD methods for production history
- [ ] Add aggregation queries (hourly, daily averages)
- [ ] Add integration tests

### Database Seeding
- [ ] Create `scripts/seed_data/devices.sql` (example battery/inverter configs)
- [ ] Create `scripts/seed_data/prices.sql` (Nordpool historical data)
- [ ] Create `scripts/seed_data/consumption.sql` (typical household patterns)
- [ ] Create `scripts/seed_db.sh` script
- [ ] Add command to run seeds: `make seed-db`

### Database Testing Setup
- [ ] Setup test database in `docker-compose.test.yml`
- [ ] Create `tests/integration/database_setup.rs`
- [ ] Implement test database initialization
- [ ] Implement test data cleanup between tests
- [ ] Add helper functions for test data generation
- [ ] Document testing patterns in `CONTRIBUTING.md`

---

## 📋 PHASE 4: HARDWARE ABSTRACTION - SIMULATED

### Simulated Battery Implementation
- [x] Create `src/hardware/simulated/mod.rs`
- [x] Create `src/hardware/simulated/battery.rs`
- [x] Create `SimulatedBattery` struct with internal state
- [x] Implement `Battery` trait for `SimulatedBattery`
- [x] Implement `read_state()` - return current simulated state
- [x] Implement `set_power()` - update simulated power
- [x] Add internal state update logic (SoC changes over time)
- [ ] Add temperature simulation (rises during charge/discharge)
- [x] Add efficiency simulation (losses during power conversion)
- [ ] Add degradation simulation (health decreases with cycles)
- [ ] Add realistic delays (Modbus response time simulation)
- [ ] Add random noise to readings (realistic sensor variation)
- [ ] Implement configurable simulation parameters
- [x] Add unit tests for state transitions
- [ ] Add property-based tests (proptest) for invariants
- [x] Document simulation assumptions

### Simulated Inverter Implementation
- [x] Create `src/hardware/simulated/inverter.rs`
- [x] Create `SimulatedInverter` struct
- [x] Implement `Inverter` trait
- [x] Add AC/DC conversion simulation
- [x] Add efficiency curve simulation
- [ ] Add temperature simulation
- [x] Add unit tests

### Simulated EV Charger Implementation
- [x] Create `src/hardware/simulated/ev_charger.rs`
- [x] Create `SimulatedEvCharger` struct
- [x] Implement `EvCharger` trait
- [ ] Add vehicle connection/disconnection simulation
- [ ] Add charge curve simulation (CC/CV phases)
- [x] Add unit tests

### Simulation Time Control
- [ ] Create `src/hardware/simulated/time.rs`
- [ ] Create `SimulationClock` struct
- [ ] Implement time acceleration (run 24h in 1 minute)
- [ ] Implement pause/resume functionality
- [ ] Implement step-by-step execution
- [ ] Add integration tests with simulation clock

### Hardware Factory Pattern
- [x] Create `src/hardware/factory.rs`
- [x] Create `HardwareFactory` trait
- [x] Implement `create_battery() -> Arc<dyn Battery>` method
- [x] Implement `create_inverter() -> Arc<dyn Inverter>` method
- [x] Implement `create_ev_charger() -> Arc<dyn EvCharger>` method
- [x] Create `SimulatedHardwareFactory` implementation
- [x] Add configuration-driven factory selection
- [x] Add unit tests for factory pattern

---

## 📋 PHASE 5: CONFIGURATION SYSTEM

### Configuration Structures
- [ ] Create `src/config/mod.rs`
- [ ] Create `AppConfig` struct (top-level config)
- [ ] Create `ServerConfig` struct (host, port, TLS)
- [ ] Create `DatabaseConfig` struct
- [ ] Create `HardwareConfig` struct
- [ ] Create `OptimizationConfig` struct
- [ ] Create `ForecastConfig` struct
- [ ] Create `TelemetryConfig` struct
- [ ] Add `serde` derives for all config structs
- [ ] Add `Validate` trait implementations
- [ ] Add default values for optional fields

### Configuration Loading
- [ ] Implement `AppConfig::load()` method
- [ ] Load from TOML file (`config/development.toml`)
- [ ] Override with environment variables
- [ ] Override with command-line arguments (clap)
- [ ] Implement config validation on load
- [ ] Add helpful error messages for invalid config
- [ ] Add `--validate-config` CLI flag
- [ ] Document all config options in README

### Configuration Files
- [x] Create complete `config/development.toml`
- [x] Create complete `config/production.toml`
- [ ] Create complete `config/test.toml`
- [x] Add inline comments explaining each option
- [x] Add example values
- [x] Create `.env.example` with all environment variables
- [x] Document environment variable naming convention

---

## 📋 PHASE 6: MODBUS TCP CLIENT

### Modbus Core Client
- [x] Create `src/modbus/mod.rs`
- [x] Create `src/modbus/client.rs` (integrated into mod.rs)
- [x] Create `ModbusClient` struct wrapping `tokio_modbus::client::Context`
- [x] Implement `connect(addr, unit_id) -> Result<ModbusClient>` method
- [x] Implement `read_holding_registers(start, count) -> Result<Vec<u16>>` method
- [x] Implement `read_input_registers(start, count) -> Result<Vec<u16>>` method
- [x] Implement `write_single_register(addr, value) -> Result<()>` method
- [x] Implement `write_multiple_registers(addr, values) -> Result<()>` method
- [x] Add automatic retry logic (3 attempts)
- [ ] Add connection pooling for multiple devices
- [x] Add timeout handling (5 seconds default)
- [x] Add connection health checks
- [ ] Add metrics for Modbus operations (success/failure counts)
- [x] Add tracing spans for debugging
- [ ] Implement `Drop` for graceful disconnection
- [x] Add unit tests with mock Modbus server

### Modbus Register Mapping
- [x] Create `src/modbus/register_map.rs` (integrated into mod.rs)
- [x] Create `RegisterMap` trait
- [x] Create `HuaweiLuna2000RegisterMap` (example vendor)
- [x] Add register addresses for SoC, power, voltage, temperature
- [x] Add register addresses for charge/discharge commands
- [x] Add register addresses for max power limits
- [x] Add register scaling factors (e.g., SoC in 0.01% units)
- [x] Add data type handling (u16, i16, u32, f32)
- [x] Create `SolarEdgeRegisterMap` (another example vendor)
- [x] Create `GenericBatteryRegisterMap` (fallback)
- [ ] Add register map auto-detection logic
- [x] Add unit tests for register parsing
- [ ] Document register mappings in `docs/modbus_registers.md`

### Modbus Data Parsing
- [x] Create `src/modbus/parser.rs` (integrated into mod.rs)
- [x] Implement `parse_u16(registers) -> u16` function
- [x] Implement `parse_i16(registers) -> i16` function
- [x] Implement `parse_u32(registers) -> u32` function (big-endian)
- [x] Implement `parse_f32(registers) -> f32` function
- [x] Implement `parse_scaled_value(registers, scale) -> f64` function
- [ ] Add error handling for invalid data
- [x] Add unit tests with example data

### Modbus Mock Server (for testing)
- [ ] Create `tests/modbus_mock_server.rs`
- [ ] Implement `MockModbusServer` struct
- [ ] Implement holding register storage (HashMap)
- [ ] Implement read/write operations
- [ ] Implement realistic response delays
- [ ] Add ability to simulate errors (timeout, exception codes)
- [ ] Add ability to simulate device disconnection
- [ ] Use in integration tests
- [ ] Document usage in test helpers

---

## 📋 PHASE 7: HARDWARE IMPLEMENTATION - MODBUS

### Modbus Battery Implementation
- [x] Create `src/hardware/modbus/mod.rs`
- [x] Create `src/hardware/modbus/battery.rs`
- [x] Create `ModbusBattery` struct (client + register map + config)
- [x] Implement `Battery` trait for `ModbusBattery`
- [x] Implement `read_state()` using Modbus reads
- [x] Parse SoC from registers
- [x] Parse power from registers
- [x] Parse voltage from registers
- [x] Parse temperature from registers
- [x] Parse health/status from registers
- [x] Implement `set_power()` using Modbus writes
- [x] Convert power (kW) to register values
- [x] Add command validation before writing
- [x] Implement `capabilities()` - read from device or config
- [x] Add connection error handling
- [x] Add register read error handling
- [x] Add write error handling
- [ ] Add state caching (avoid excessive Modbus traffic)
- [ ] Add rate limiting (max 1 request/second per device)
- [ ] Add integration tests with mock Modbus server
- [ ] Add smoke tests with real hardware (optional, gated by feature flag)

### Modbus Inverter Implementation
- [ ] Create `src/hardware/modbus/inverter.rs`
- [ ] Create `ModbusInverter` struct
- [x] Implement `Inverter` trait
- [ ] Implement register reads for AC/DC power, efficiency, temperature
- [ ] Implement mode setting via Modbus writes
- [ ] Add error handling
- [ ] Add integration tests

### Device-Specific Implementations
- [ ] Create `src/hardware/modbus/vendors/mod.rs`
- [ ] Create `src/hardware/modbus/vendors/huawei.rs`
- [ ] Implement Huawei Luna2000 specific battery
- [ ] Create `src/hardware/modbus/vendors/solaredge.rs`
- [ ] Implement SolarEdge StorEdge specific battery
- [ ] Create `src/hardware/modbus/vendors/lg.rs`
- [ ] Implement LG RESU specific battery
- [ ] Add factory method to select vendor implementation
- [ ] Document supported vendors in README

### Modbus Factory Implementation
- [ ] Create `ModbusHardwareFactory` struct
- [ ] Implement `HardwareFactory` trait
- [ ] Implement device creation with auto-detection
- [ ] Add fallback to generic implementation
- [ ] Add configuration for manual device selection
- [x] Add unit tests for factory logic

---

## 📋 PHASE 8: DEVICE DISCOVERY

### Network Scanning
- [x] Create `src/discovery/mod.rs` (enhanced existing file)
- [x] Create `src/discovery/network_scanner.rs` (integrated into mod.rs)
- [x] Create `NetworkScanner` struct
- [x] Implement TCP port scan for Modbus (502, 1502, 8502)
- [x] Implement concurrent scanning (tokio::spawn for each IP)
- [x] Add IP range parsing (192.168.1.0/24)
- [x] Add scan timeout (100ms per host)
- [x] Add scan rate limiting (avoid network flooding)
- [x] Return list of responsive IPs + ports
- [x] Add integration tests with localhost (unit tests for IP parsing)
- [ ] Add benchmarks for scan performance

### Modbus Device Identification
- [x] Create `src/discovery/modbus_identifier.rs` (integrated into mod.rs)
- [x] Create `ModbusIdentifier` struct
- [x] Implement `identify_device(ip, port) -> Result<DeviceInfo>` method
- [x] Read vendor-specific identification registers (device type detection)
- [ ] Parse manufacturer name from registers
- [ ] Parse model name from registers
- [ ] Parse serial number from registers
- [ ] Parse firmware version from registers
- [x] Create `DeviceInfo` struct (DiscoveredDevice)
- [x] Add device type detection heuristics
- [ ] Add database of known register patterns
- [ ] Add integration tests with mock devices

### mDNS Service Discovery
- [ ] Create `src/discovery/mdns.rs`
- [ ] Create `MdnsListener` struct
- [ ] Implement service browser for `_modbus._tcp.local`
- [ ] Implement service browser for `_http._tcp.local` (for API devices)
- [ ] Parse mDNS TXT records for device info
- [ ] Implement continuous background listening
- [ ] Add callback for new device discovery
- [ ] Add callback for device removal
- [ ] Add integration tests
- [ ] Document mDNS usage in README

### Discovery Orchestrator
- [x] Create `src/discovery/orchestrator.rs` (DeviceDiscovery in mod.rs)
- [x] Create `DiscoveryOrchestrator` struct (combines all discovery methods)
- [x] Implement `start_continuous_discovery()` method
- [x] Run network scan every 5 minutes
- [ ] Run mDNS listener continuously
- [x] Deduplicate discovered devices (via scan logic)
- [ ] Update device `last_seen` timestamp
- [ ] Persist new devices to database
- [ ] Emit events for new devices (tokio::sync::broadcast)
- [ ] Add graceful shutdown
- [x] Add telemetry (devices discovered, scan duration)
- [ ] Add integration tests

### Discovery Configuration
- [x] Add `DiscoveryConfig` struct (integrated into DeviceDiscovery)
- [x] Add configurable scan intervals
- [x] Add configurable IP ranges
- [x] Add configurable port list (hardcoded common ports)
- [ ] Add enable/disable flags for each discovery method
- [ ] Add allowlist/denylist for IPs
- [ ] Document configuration in README

---

## 📋 PHASE 9: FORECASTING ENGINE

### Price Forecasting - Nordpool Integration
- [ ] Create `src/forecast/price/mod.rs`
- [ ] Create `src/forecast/price/nordpool.rs`
- [ ] Create `NordpoolClient` struct
- [ ] Implement Nordpool API client (HTTP REST)
- [ ] Implement `fetch_day_ahead_prices(date, area) -> Result<Vec<PricePoint>>` method
- [ ] Parse JSON response from Nordpool
- [ ] Handle EUR to SEK conversion
- [ ] Add request caching (cache for 1 hour)
- [ ] Add retry logic with exponential backoff
- [ ] Add rate limiting (respect API limits)
- [ ] Add integration tests with wiremock
- [ ] Document API endpoints and data format

### Price Forecasting - Historical Pattern Analysis
- [ ] Create `src/forecast/price/patterns.rs`
- [ ] Create `PricePatternAnalyzer` struct
- [ ] Implement weekday/weekend pattern extraction
- [ ] Implement hourly price distribution analysis
- [ ] Implement seasonal adjustment (summer vs winter)
- [ ] Calculate statistical confidence intervals
- [x] Add unit tests with synthetic data

### Price Forecasting - Simple ML Model (MVP)
- [ ] Create `src/forecast/price/simple_model.rs`
- [ ] Create `SimplePriceForecaster` struct
- [ ] Implement moving average forecasting
- [ ] Implement exponential smoothing
- [ ] Implement seasonal decomposition
- [ ] Implement confidence bounds
- [ ] Add training on historical data
- [ ] Add validation metrics (MAE, RMSE)
- [x] Add unit tests
- [ ] Compare against Nordpool day-ahead prices

### Consumption Forecasting - Historical Analysis
- [ ] Create `src/forecast/consumption/mod.rs`
- [ ] Create `src/forecast/consumption/historical.rs`
- [ ] Create `ConsumptionForecaster` struct
- [ ] Implement daily pattern extraction (morning/evening peaks)
- [ ] Implement weekday vs weekend patterns
- [ ] Implement seasonal patterns (winter heating, summer cooling)
- [ ] Calculate average hourly consumption
- [ ] Add rolling window analysis (last 7 days)
- [ ] Add outlier detection and removal
- [x] Add unit tests

### Consumption Forecasting - Feature Engineering
- [ ] Create `src/forecast/consumption/features.rs`
- [ ] Extract hour-of-day feature
- [ ] Extract day-of-week feature
- [ ] Extract month feature
- [ ] Extract is-weekend boolean feature
- [ ] Extract temperature feature (if available)
- [ ] Extract holiday feature (Swedish holidays)
- [ ] Create feature vector struct
- [ ] Add normalization/scaling
- [x] Add unit tests

### Production Forecasting - Solar Simulation
- [ ] Create `src/forecast/production/mod.rs`
- [ ] Create `src/forecast/production/solar.rs`
- [ ] Create `SolarProductionForecaster` struct
- [ ] Implement solar angle calculation (altitude, azimuth)
- [ ] Implement day length calculation
- [ ] Implement theoretical max production (clear sky model)
- [ ] Add cloud cover adjustment (from weather API)
- [ ] Add seasonal efficiency adjustment
- [ ] Add soiling/degradation factors
- [x] Add unit tests with known locations

### Weather API Integration
- [ ] Create `src/forecast/weather/mod.rs`
- [ ] Create `src/forecast/weather/smhi.rs` (Swedish Meteorological Institute)
- [ ] Create `SmhiClient` struct
- [ ] Implement API client for SMHI weather forecasts
- [ ] Parse temperature, cloud cover, wind speed
- [ ] Implement caching (cache for 30 minutes)
- [ ] Add fallback to dummy data if API unavailable
- [ ] Add integration tests with wiremock
- [ ] Document API usage

### Forecast Aggregation
- [ ] Create `src/forecast/aggregator.rs`
- [ ] Create `ForecastAggregator` struct
- [ ] Implement `generate_24h_forecast() -> Result<Forecast24h>` method
- [ ] Fetch price forecast
- [ ] Fetch consumption forecast
- [ ] Fetch production forecast
- [ ] Combine into single `Forecast24h` struct
- [ ] Add forecast validation (no gaps, reasonable values)
- [ ] Add forecast caching in database
- [ ] Add periodic refresh (every hour)
- [ ] Add telemetry for forecast quality
- [x] Add unit tests

### Forecast Metrics & Evaluation
- [ ] Create `src/forecast/metrics.rs`
- [ ] Implement MAE (Mean Absolute Error)
- [ ] Implement RMSE (Root Mean Square Error)
- [ ] Implement MAPE (Mean Absolute Percentage Error)
- [ ] Implement prediction intervals
- [ ] Store forecast accuracy metrics in database
- [ ] Create dashboard view of forecast performance
- [x] Add unit tests

---

## 📋 PHASE 10: OPTIMIZATION ENGINE - BASIC

### Constraint Modeling
- [x] Create `src/optimizer/constraints/mod.rs` (constraints.rs)
- [x] Create `Constraints` struct
- [x] Add `min_soc: Percentage` field (always keep >20%)
- [x] Add `max_soc: Percentage` field (usually <95% to preserve battery)
- [x] Add `max_charge_power: Power` field (max_power_grid_kw)
- [x] Add `max_discharge_power: Power` field (max_power_grid_kw)
- [x] Add `max_cycles_per_day: f64` field (limit degradation)
- [x] Add `max_grid_import: Power` field (fuse limit)
- [x] Add `max_grid_export: Power` field
- [ ] Add `must_have_energy_at: Vec<(DateTime, Energy)>` (e.g., backup power)
- [ ] Add validation methods
- [x] Add unit tests for constraint checking

### State Space Definition
- [x] Create `src/optimizer/state.rs` (types.rs)
- [x] Create `SystemState` struct
- [x] Add `battery_soc: Percentage` field
- [ ] Add `battery_health: Percentage` field
- [ ] Add `timestamp: DateTime` field
- [ ] Add `grid_price: Price` field
- [ ] Implement state transitions
- [ ] Implement state validation
- [x] Add unit tests

### Action Space Definition
- [x] Create `src/optimizer/action.rs` (types.rs)
- [x] Create `Action` enum (Charge, Discharge, Idle)
- [ ] Create `ActionWithPower` struct (action + power level)
- [ ] Implement action validation against constraints
- [ ] Implement cost calculation for actions
- [x] Add unit tests

### Optimization Strategy Trait
- [x] Create `src/optimizer/strategies/mod.rs` (types.rs)
- [x] Create `OptimizationStrategy` trait
- [x] Define `optimize(state, forecast, constraints) -> Result<Schedule>` method
- [ ] Add strategy configuration parameters
- [ ] Add trait documentation

### Dynamic Programming Optimizer
- [x] Create `src/optimizer/strategies/dynamic_programming.rs` (dp.rs)
- [x] Create `DynamicProgrammingOptimizer` struct
- [x] Define state discretization (0%, 5%, 10%, ..., 100% SoC)
- [x] Define action space per state
- [x] Implement forward DP table computation
- [x] Implement cost function (electricity cost - revenue from discharge)
- [x] Implement state transition function
- [x] Implement degradation cost modeling
- [x] Implement backtracking to extract optimal schedule
- [ ] Add early stopping if optimal solution found
- [x] Add unit tests with simple scenarios
- [ ] Add integration tests with realistic forecasts
- [ ] Add benchmarks (should run <1 second for 24h horizon)

### Greedy Heuristic Optimizer (Baseline)
- [x] Create `src/optimizer/strategies/greedy.rs`
- [x] Create `GreedyOptimizer` struct
- [x] Implement simple rule: charge when price < threshold, discharge when price > threshold
- [x] Implement SoC-aware charging (charge more when low)
- [x] Implement constraint checking
- [x] Add unit tests
- [ ] Use as baseline to compare against DP

### Optimizer Service
- [x] Create `src/optimizer/service.rs` (BatteryOptimizer in types.rs)
- [x] Create `OptimizerService` struct
- [x] Add strategy selection (DP, greedy, etc.)
- [x] Implement `optimize_next_24h(system_state, forecast) -> Result<Schedule>` method
- [ ] Add pre-optimization validation
- [ ] Add post-optimization validation
- [ ] Add logging of optimization results
- [ ] Add metrics (optimization time, cost savings estimate)
- [ ] Add error handling and fallback to safe schedule
- [x] Add unit tests
- [ ] Add integration tests

### Optimization Metrics
- [ ] Create `src/optimizer/metrics.rs`
- [ ] Calculate estimated cost savings
- [ ] Calculate cycle count
- [ ] Calculate degradation estimate
- [ ] Calculate grid stress factor
- [ ] Store metrics in database
- [x] Add unit tests

---

## 📋 PHASE 11: CONTROLLER SERVICE

### PID Controller
- [x] Create `src/controller/pid.rs`
- [x] Create `PidController` struct (Kp, Ki, Kd gains)
- [x] Implement `calculate(target, current, dt) -> f64` method (update)
- [x] Implement integral windup protection
- [x] Implement derivative kick protection
- [x] Add unit tests with step responses
- [ ] Add tuning documentation

### Battery Controller Core
- [x] Create `src/controller/battery_controller.rs` (mod.rs)
- [x] Create `BatteryController` struct
- [x] Add references to: battery, optimizer, forecaster, DB pool
- [x] Add current schedule (Arc<RwLock<Schedule>>)
- [ ] Add PID controller instance (simple P control exists)
- [x] Implement constructor with dependency injection
- [x] Add configuration (control loop interval, PID gains)
- [x] Add unit tests

### Main Control Loop
- [x] Implement `run() -> Result<()>` method in BatteryController
- [x] Create tokio interval (60 seconds)
- [x] Read current battery state
- [x] Read current schedule
- [x] Get target power from schedule
- [x] Calculate control output using PID (simple P control)
- [x] Apply control output to battery
- [ ] Log state to database
- [x] Log metrics (target vs actual power)
- [x] Add error handling (continue on transient errors)
- [ ] Add graceful shutdown on signal
- [ ] Add integration tests with simulated battery

### Schedule Re-optimization
- [x] Implement `reoptimize_schedule() -> Result<()>` method
- [x] Fetch latest forecast
- [x] Fetch current system state
- [x] Run optimizer
- [ ] Validate new schedule
- [x] Update shared schedule (Arc<RwLock>)
- [ ] Store schedule in database
- [ ] Log optimization event
- [ ] Add metrics
- [x] Add error handling (keep old schedule on failure)
- [x] Add unit tests

### Periodic Tasks Manager
- [ ] Create `src/controller/scheduler.rs`
- [ ] Create `TaskScheduler` struct
- [ ] Implement periodic task: re-optimize every hour
- [ ] Implement periodic task: refresh forecast every 30 minutes
- [ ] Implement periodic task: cleanup old data every 24 hours
- [ ] Implement periodic task: health check every 5 minutes
- [ ] Add graceful shutdown for all tasks
- [ ] Add task monitoring (last run, success/failure)
- [ ] Add integration tests

### Safety Monitors
- [ ] Create `src/controller/safety.rs`
- [ ] Create `SafetyMonitor` struct
- [ ] Implement battery temperature check (shutdown if >60°C)
- [ ] Implement SoC bounds check (emergency stop if <5% or >98%)
- [ ] Implement grid fault detection
- [ ] Implement emergency shutdown procedure
- [ ] Add safety event logging
- [ ] Add alerts/notifications
- [x] Add unit tests for each safety check

### Controller Telemetry
- [ ] Add metrics for control loop execution time
- [ ] Add metrics for PID error (target vs actual)
- [ ] Add metrics for schedule adherence
- [ ] Add metrics for safety events
- [ ] Add Prometheus endpoint for metrics scraping
- [ ] Add integration tests

---

## 📋 PHASE 12: REST API - CORE

### API Structure Setup
- [ ] Create `src/api/mod.rs`
- [ ] Create `src/api/routes.rs`
- [ ] Create `src/api/handlers/mod.rs`
- [ ] Create `src/api/middleware/mod.rs`
- [ ] Create `src/api/error.rs`
- [ ] Create `src/api/response.rs`

### Application State
- [ ] Create `src/api/state.rs`
- [ ] Create `AppState` struct
- [ ] Add DB pool field
- [ ] Add controller reference field
- [ ] Add optimizer reference field
- [ ] Add forecaster reference field
- [ ] Add configuration field
- [ ] Implement `Clone` for `AppState` (wrapped in Arc)
- [x] Add unit tests

### Error Handling
- [ ] Create `ApiError` enum in `src/api/error.rs`
- [ ] Add variants: NotFound, BadRequest, InternalError, Unauthorized, etc.
- [ ] Implement `IntoResponse` for `ApiError`
- [ ] Implement `From<sqlx::Error>` for `ApiError`
- [ ] Implement `From<anyhow::Error>` for `ApiError`
- [ ] Add error logging with tracing
- [ ] Add error metrics
- [x] Add unit tests

### Response Wrapper
- [ ] Create `ApiResponse<T>` struct in `src/api/response.rs`
- [ ] Add `success: bool` field
- [ ] Add `data: Option<T>` field
- [ ] Add `error: Option<String>` field
- [ ] Add `timestamp: DateTime` field
- [ ] Implement `IntoResponse` for `ApiResponse<T>`
- [ ] Add builder pattern for responses
- [x] Add unit tests

### Health Check Endpoint
- [ ] Create `src/api/handlers/health.rs`
- [ ] Implement `GET /health` handler
- [ ] Check database connection
- [ ] Check controller status
- [ ] Return health status JSON
- [ ] Add integration tests

### Status Endpoint
- [ ] Create `src/api/handlers/status.rs`
- [ ] Implement `GET /api/v1/status` handler
- [ ] Fetch current battery state
- [ ] Fetch current schedule (next 4 hours)
- [ ] Fetch last forecast update time
- [ ] Return `SystemStatus` struct
- [ ] Add OpenAPI documentation with utoipa
- [ ] Add integration tests

### Device Endpoints
- [ ] Create `src/api/handlers/devices.rs`
- [ ] Implement `GET /api/v1/devices` handler (list all devices)
- [ ] Implement `GET /api/v1/devices/:id` handler (get device by ID)
- [ ] Implement `POST /api/v1/devices` handler (manually add device)
- [ ] Implement `PUT /api/v1/devices/:id` handler (update device config)
- [ ] Implement `DELETE /api/v1/devices/:id` handler (remove device)
- [ ] Add request validation
- [ ] Add OpenAPI documentation
- [ ] Add integration tests

### Battery State Endpoints
- [x] Create `src/api/handlers/battery.rs`
- [x] Implement `GET /api/v1/battery/state` handler (current state)
- [x] Implement `GET /api/v1/battery/history` handler (time range query)
- [x] Implement `GET /api/v1/battery/statistics` handler (aggregated stats)
- [x] Add query parameters (start_time, end_time, interval)
- [ ] Add pagination for history endpoint
- [ ] Add OpenAPI documentation
- [ ] Add integration tests

### EV Charger Endpoints
- [x] Create `src/api/handlers/ev_charger.rs`
- [x] Implement `GET /api/v1/ev-charger/state` handler
- [x] Implement `POST /api/v1/ev-charger/set-current` handler
- [x] Implement `POST /api/v1/ev-charger/start` handler
- [x] Implement `POST /api/v1/ev-charger/stop` handler
- [x] Implement `GET /api/v1/ev-charger/sessions` handler
- [x] Integrate EV charger endpoints with auth layer
- [x] Add OpenAPI documentation for EV charger endpoints

### Inverter Endpoints
- [x] Create `src/api/handlers/inverter.rs`
- [x] Implement `GET /api/v1/inverter/state` handler
- [x] Implement `POST /api/v1/inverter/mode` handler
- [x] Implement `POST /api/v1/inverter/export-limit` handler
- [x] Implement `GET /api/v1/inverter/production` handler
- [x] Implement `GET /api/v1/inverter/efficiency` handler
- [x] Integrate inverter endpoints with auth layer
- [x] Add OpenAPI documentation for inverter endpoints

### Grid & Weather Endpoints
- [x] Implement grid status, limits, and statistics handlers with controller data
- [x] Re-enable grid routes in the v1 router
- [x] Implement weather forecast handler backed by SMHI client
- [x] Re-enable weather forecast route in the v1 router

### Schedule Endpoints
- [ ] Create `src/api/handlers/schedule.rs`
- [ ] Implement `GET /api/v1/schedule/current` handler
- [ ] Implement `GET /api/v1/schedule/:id` handler
- [ ] Implement `POST /api/v1/schedule` handler (manually set schedule)
- [ ] Add schedule validation
- [ ] Add OpenAPI documentation
- [ ] Add integration tests

### Forecast Endpoints
- [ ] Create `src/api/handlers/forecast.rs`
- [ ] Implement `GET /api/v1/forecast/price` handler
- [ ] Implement `GET /api/v1/forecast/consumption` handler
- [ ] Implement `GET /api/v1/forecast/production` handler
- [ ] Implement `GET /api/v1/forecast/combined` handler
- [ ] Add time range parameters
- [ ] Add OpenAPI documentation
- [ ] Add integration tests

### Optimization Endpoints
- [ ] Create `src/api/handlers/optimize.rs`
- [ ] Implement `POST /api/v1/optimize/trigger` handler (force re-optimization)
- [ ] Implement `GET /api/v1/optimize/status` handler (last optimization result)
- [ ] Implement `GET /api/v1/optimize/history` handler (optimization runs)
- [ ] Add OpenAPI documentation
- [ ] Add integration tests

### Simulation Endpoints
- [ ] Create `src/api/handlers/simulation.rs`
- [ ] Implement `POST /api/v1/simulation/step` handler (advance simulation time)
- [ ] Implement `POST /api/v1/simulation/reset` handler
- [ ] Implement `GET /api/v1/simulation/config` handler
- [ ] Implement `PUT /api/v1/simulation/speed` handler (time acceleration)
- [ ] Add OpenAPI documentation
- [ ] Add integration tests

### Router Configuration
- [ ] Create main router in `src/api/routes.rs`
- [ ] Add health routes
- [ ] Add API v1 routes
- [ ] Add nested routers (devices, battery, schedule, etc.)
- [ ] Add CORS middleware
- [ ] Add request logging middleware
- [ ] Add request ID middleware
- [ ] Add timeout middleware (30 seconds)
- [ ] Add rate limiting middleware
- [ ] Add compression middleware
- [ ] Add OpenAPI route (`/api/docs`)
- [ ] Add Swagger UI route (`/swagger-ui`)
- [ ] Add integration tests

### OpenAPI Documentation
- [ ] Add `#[utoipa::path]` annotations to all handlers
- [ ] Create `src/api/openapi.rs`
- [ ] Generate OpenAPI spec with utoipa
- [ ] Add schemas for all request/response types
- [ ] Add examples for each endpoint
- [ ] Add descriptions for each endpoint
- [ ] Serve OpenAPI JSON at `/api/openapi.json`
- [ ] Serve Swagger UI
- [ ] Add CI check that OpenAPI spec is valid

---

## 📋 PHASE 13: MIDDLEWARE & SECURITY

### Request Logging Middleware
- [ ] Create `src/api/middleware/logging.rs`
- [ ] Log HTTP method, path, status code, duration
- [ ] Include request ID in logs
- [ ] Use tracing spans
- [ ] Add integration tests

### CORS Middleware
- [ ] Configure CORS in router
- [ ] Allow configurable origins
- [ ] Add preflight handling
- [ ] Add integration tests

### Rate Limiting Middleware
- [ ] Create `src/api/middleware/rate_limit.rs`
- [ ] Implement token bucket algorithm
- [ ] Add per-IP rate limiting
- [ ] Add per-endpoint rate limiting
- [ ] Return 429 Too Many Requests on limit exceeded
- [ ] Add configuration (requests per minute)
- [ ] Add integration tests

### Authentication Middleware (Optional for MVP)
- [ ] Create `src/api/middleware/auth.rs`
- [ ] Implement JWT token validation
- [ ] Extract user ID from token
- [ ] Add to request extensions
- [ ] Add integration tests

### Request Validation Middleware
- [ ] Create `src/api/middleware/validation.rs`
- [ ] Validate content-type header
- [ ] Validate request size limits
- [ ] Add integration tests

---

## 📋 PHASE 14: TELEMETRY & OBSERVABILITY

### Structured Logging Setup
- [ ] Configure tracing subscriber in `main.rs`
- [ ] Add JSON formatting for production
- [ ] Add human-readable formatting for development
- [ ] Configure log levels per module
- [ ] Add file appender for log persistence
- [ ] Add log rotation
- [x] Add unit tests

### Metrics Collection
- [ ] Create `src/telemetry/metrics.rs`
- [ ] Register metrics with metrics crate
- [ ] Add counter: `http_requests_total` (method, path, status)
- [ ] Add histogram: `http_request_duration_seconds`
- [ ] Add counter: `optimization_runs_total` (status)
- [ ] Add histogram: `optimization_duration_seconds`
- [ ] Add gauge: `battery_soc_percent`
- [ ] Add gauge: `battery_power_watts`
- [ ] Add gauge: `electricity_price_sek_kwh`
- [ ] Add counter: `modbus_requests_total` (device, operation, status)
- [ ] Add counter: `forecast_updates_total` (type, status)
- [ ] Add gauge: `database_pool_connections`

### Prometheus Exporter
- [ ] Add `/metrics` endpoint
- [ ] Configure metrics-exporter-prometheus
- [ ] Add integration tests
- [ ] Document metrics in README

### Health Checks
- [ ] Implement database health check
- [ ] Implement Modbus connection health check
- [ ] Implement controller health check
- [ ] Add `/health/ready` endpoint (readiness probe)
- [ ] Add `/health/live` endpoint (liveness probe)
- [ ] Add integration tests

### Distributed Tracing (Optional)
- [ ] Configure OpenTelemetry
- [ ] Add trace context propagation
- [ ] Add Jaeger exporter
- [ ] Add integration tests

---

## 📋 PHASE 15: POWER FLOW MODEL (THE CORE!)

**⚡ This is THE most critical phase - what Spotpilot actually does!**

### Core Power Flow Structures
- [ ] Create `src/power_flow/mod.rs`
- [ ] Create `src/power_flow/snapshot.rs`
- [ ] Create `PowerSnapshot` struct (PV, house, battery, EV, grid)
- [ ] Implement `verify_power_balance()` method
- [ ] Implement `exceeds_fuse_limit()` method
- [ ] Implement `net_grid_kw()` method
- [x] Add unit tests for power balance verification
- [ ] Add `Display` implementation for debugging

### Constraint System
- [ ] Create `src/power_flow/constraints.rs`
- [ ] Create `PhysicalConstraints` struct (fuse, device limits)
- [ ] Add `max_grid_import_kw` field
- [ ] Add `max_grid_export_kw` field
- [ ] Add `max_battery_charge_kw` field
- [ ] Add `max_battery_discharge_kw` field
- [ ] Add `evse_min_current_a` field
- [ ] Add `evse_max_current_a` field
- [ ] Add `phases` field (1 or 3 phase)
- [ ] Add `max_current_per_phase_a` optional field
- [ ] Create `SafetyConstraints` struct
- [ ] Add `battery_min_soc_percent` field
- [ ] Add `battery_max_soc_percent` field
- [ ] Add `house_priority` boolean field
- [ ] Add `max_battery_cycles_per_day` field
- [ ] Add `max_battery_temp_c` field
- [ ] Create `EconomicObjectives` struct
- [ ] Add `grid_price_sek_kwh` field
- [ ] Add `export_price_sek_kwh` field
- [ ] Add `prefer_self_consumption` field
- [ ] Add `arbitrage_threshold_sek_kwh` field
- [ ] Add `ev_departure_time` optional field
- [ ] Add `ev_target_soc_percent` optional field
- [ ] Create `AllConstraints` wrapper struct
- [ ] Add validation for constraints
- [x] Add unit tests for constraint types

### Power Flow Input State
- [ ] Create `src/power_flow/inputs.rs`
- [ ] Create `PowerFlowInputs` struct
- [ ] Add `pv_production_kw` field
- [ ] Add `house_load_kw` field
- [ ] Add `battery_soc_percent` field
- [ ] Add `battery_temp_c` field
- [ ] Add `ev_state` optional field
- [ ] Add `grid_price` field
- [ ] Add `timestamp` field
- [ ] Add validation methods
- [x] Add unit tests

### EV State Modeling
- [ ] Create `src/domain/ev_charger/ev_state.rs`
- [ ] Create `EvState` struct
- [ ] Add `connected` boolean field
- [ ] Add `soc_percent` field
- [ ] Add `capacity_kwh` field
- [ ] Add `max_charge_kw` field
- [ ] Add `departure_time` optional field
- [ ] Add `target_soc_percent` field
- [ ] Add `charging_profile` field (current vs time)
- [ ] Implement `needs_charging()` method
- [ ] Implement `time_until_departure()` method
- [ ] Implement `energy_needed_kwh()` method
- [x] Add unit tests

### Power Flow Computation Algorithm
- [ ] Create `src/power_flow/model.rs`
- [ ] Create `PowerFlowModel` struct
- [ ] Implement `compute_flows()` method - THE CORE ALGORITHM
- [ ] Step 1: House load priority (always satisfied)
- [ ] Step 2: Calculate PV allocation (house first)
- [ ] Step 3: EV charging urgency calculation
- [ ] Step 4: EV power allocation with fuse limit check
- [ ] Step 5: Battery charging from excess PV
- [ ] Step 6: Battery arbitrage logic (charge when cheap)
- [ ] Step 7: Battery discharge when expensive
- [ ] Step 8: Grid export logic (if beneficial)
- [ ] Step 9: Final power balance verification
- [ ] Implement `calculate_ev_urgency()` helper method
- [ ] Implement `allocate_power_to_ev()` helper method
- [ ] Implement `battery_arbitrage_decision()` helper method
- [ ] Implement `check_fuse_limits()` helper method
- [ ] Add comprehensive unit tests for each step
- [ ] Add integration tests for complex scenarios
- [ ] Add property-based tests (fuse never exceeded, power balance always holds)

### Power Flow Priority Logic
- [ ] Document priority levels in code comments
- [ ] Level 1: Physical constraints (hard limits)
- [ ] Level 2: Safety & availability (house power, min SoC)
- [ ] Level 3: Economic optimization (arbitrage, self-consumption)
- [ ] Add priority enforcement in compute_flows
- [ ] Add tests verifying priority ordering
- [ ] Add metrics for constraint violations (should be zero!)

### Power Flow Scenarios (Test Cases)
- [ ] Create `tests/power_flow_scenarios.rs`
- [ ] Scenario: Sunny day with EV charging
- [ ] Scenario: Peak price arbitrage
- [ ] Scenario: Fuse limit protection
- [ ] Scenario: EV urgent charging (deadline approaching)
- [ ] Scenario: Battery full, excess PV export
- [ ] Scenario: No PV, expensive grid, discharge battery
- [ ] Scenario: No PV, cheap grid, charge battery
- [ ] Scenario: House load spike, reduce EV charging
- [ ] Scenario: Phase balancing (3-phase system)
- [ ] Scenario: Grid export disabled, curtail PV
- [ ] Add assertions for each scenario
- [ ] Add property tests for invariants

---

## 📋 PHASE 16: SIMULATED ENVIRONMENT

**🎮 Complete simulation for development and demo**

### Simulated House Load
- [ ] Create `src/simulation/house.rs`
- [ ] Create `SimulatedHouse` struct
- [ ] Add `base_load_kw` field (always present)
- [ ] Add `daily_profile: Vec<f64>` (24 hours)
- [ ] Add `noise_amplitude` for realistic variation
- [ ] Implement typical weekday profile (peaks 07:00, 18:00)
- [ ] Implement typical weekend profile (different peaks)
- [ ] Implement `get_load_at(time) -> f64` method
- [ ] Add seasonal variation (winter vs summer)
- [ ] Add random spikes (dishwasher, dryer, etc.)
- [ ] Add configurable appliances (EV, heat pump, etc.)
- [x] Add unit tests with known profiles
- [ ] Add visualization of daily profile

### Simulated Solar PV
- [ ] Create `src/simulation/solar.rs`
- [ ] Create `SimulatedSolar` struct
- [ ] Add `capacity_kw` field (installed capacity)
- [ ] Add `latitude` and `longitude` fields
- [ ] Add `panel_azimuth` and `panel_tilt` fields
- [ ] Implement solar elevation angle calculation
- [ ] Implement solar azimuth calculation
- [ ] Implement day length calculation
- [ ] Implement clear-sky model (theoretical max)
- [ ] Add cloud cover simulation (random variation)
- [ ] Add seasonal efficiency (winter vs summer)
- [ ] Implement `get_production_at(time) -> f64` method
- [ ] Add temperature derating (panels less efficient when hot)
- [ ] Add soiling factor (dirt on panels)
- [x] Add unit tests with known solar data
- [ ] Add visualization of daily production

### Simulated EV Battery
- [ ] Create `src/simulation/ev.rs`
- [ ] Create `SimulatedEv` struct
- [ ] Add `capacity_kwh` field (e.g., 75 kWh)
- [ ] Add `current_soc_percent` field
- [ ] Add `max_charge_kw` field (e.g., 11 kW or 50 kW DC)
- [ ] Add `max_discharge_kw` field (for V2G)
- [ ] Add `connected` boolean
- [ ] Add `departure_time` optional field
- [ ] Add `target_soc_percent` field
- [ ] Add `arrival_time` for statistics
- [ ] Implement `charge(power_kw, duration_secs)` method
- [ ] Implement `discharge(power_kw, duration_secs)` method (V2G)
- [ ] Implement `drive(distance_km, consumption_kwh_km)` method
- [ ] Implement `connect(departure, target_soc)` method
- [ ] Implement `disconnect()` method
- [ ] Add charging curve simulation (CC/CV phases)
- [ ] Add battery temperature simulation
- [ ] Add battery degradation tracking
- [x] Add unit tests for charge/discharge
- [x] Add unit tests for driving

### Simulated EVSE (Charger)
- [ ] Create `src/simulation/evse.rs`
- [ ] Create `SimulatedEvse` struct
- [ ] Add `min_current_a` field (IEC 61851: 6A)
- [ ] Add `max_current_a` field (e.g., 32A)
- [ ] Add `phases` field (1 or 3)
- [ ] Add `current_limit_a` field (set by controller)
- [ ] Add `connected_ev` optional reference
- [ ] Implement `set_current_limit(current_a) -> Result<()>` method
- [ ] Validate current within min/max range
- [ ] Implement `get_actual_power_kw() -> f64` method (P = V × I × phases)
- [ ] Implement `connect_ev(ev: SimulatedEv)` method
- [ ] Implement `disconnect_ev()` method
- [ ] Add current ramp-up/ramp-down simulation (not instant)
- [ ] Add EVSE ready state (not ready until pilot signal)
- [x] Add unit tests for current setting
- [x] Add unit tests for power calculation

### Simulated Grid
- [ ] Create `src/simulation/grid.rs`
- [ ] Create `SimulatedGrid` struct
- [ ] Add `fuse_limit_kw` field
- [ ] Add `export_allowed` boolean
- [ ] Add `max_export_kw` field
- [ ] Add `current_import_kw` field
- [ ] Add `current_export_kw` field
- [ ] Implement price simulation (hourly Nordpool-like)
- [ ] Add price peaks (06:00, 18:00)
- [ ] Add price valleys (02:00, 14:00)
- [ ] Add random variation
- [ ] Implement `check_fuse_trip(import_kw) -> bool` method
- [ ] Simulate fuse trip if exceeded for >5 seconds
- [ ] Add voltage variation simulation (±10%)
- [ ] Add frequency variation (50Hz ±0.2Hz)
- [x] Add unit tests for price simulation
- [x] Add unit tests for fuse protection

### Simulation Time Controller
- [ ] Create `src/simulation/time.rs`
- [ ] Create `SimulationClock` struct
- [ ] Add `current_time` field
- [ ] Add `time_acceleration` field (e.g., 1440 = 1 day per minute)
- [ ] Add `paused` boolean
- [ ] Implement `advance(real_seconds) -> sim_seconds` method
- [ ] Implement `pause()` and `resume()` methods
- [ ] Implement `reset()` method
- [ ] Implement `set_time(datetime)` method
- [ ] Implement `set_acceleration(factor)` method
- [ ] Add real-time mode (acceleration = 1)
- [ ] Add fast-forward mode (acceleration = 1440)
- [ ] Add step-by-step mode (pause between steps)
- [x] Add unit tests for time advancement

### Complete Simulation Environment
- [ ] Create `src/simulation/environment.rs`
- [ ] Create `SimulationEnvironment` struct
- [ ] Add all simulated devices (house, solar, battery, EV, grid)
- [ ] Add simulation clock
- [ ] Implement `step(duration)` method - advances everything
- [ ] Update house load based on time
- [ ] Update solar production based on time
- [ ] Update EV state (charging or disconnected)
- [ ] Update battery state (charging/discharging)
- [ ] Update grid state
- [ ] Implement `create_scenario(name) -> Environment` factory method
- [ ] Add "sunny_day" scenario
- [ ] Add "cloudy_day" scenario
- [ ] Add "peak_pricing" scenario
- [ ] Add "ev_charging_deadline" scenario
- [ ] Add "battery_arbitrage" scenario
- [ ] Add integration tests running full scenarios
- [ ] Add visualization of simulation results

---

## 📋 PHASE 17: POWER FLOW CONTROLLER

**🎛️ Real-time orchestration of all power flows**

### Power Flow Controller Core
- [ ] Create `src/controller/power_flow_controller.rs`
- [ ] Create `PowerFlowController` struct
- [ ] Add `power_flow_model: PowerFlowModel` field
- [ ] Add `battery: Arc<dyn Battery>` field
- [ ] Add `evse: Arc<dyn EvCharger>` field
- [ ] Add `solar: Arc<dyn SolarInverter>` field
- [ ] Add `grid_meter: Arc<dyn GridMeter>` field
- [ ] Add `house_meter: Arc<dyn HouseMeter>` field
- [ ] Add `constraints: AllConstraints` field
- [ ] Add `db_pool: PgPool` field
- [ ] Implement `new()` constructor
- [ ] Add comprehensive unit tests

### Main Control Loop
- [ ] Implement `run() -> Result<()>` method
- [ ] Create 10-second interval timer (not 60s!)
- [ ] Step 1: Read all sensor data
- [ ] Read PV production from solar inverter
- [ ] Read house load from meter
- [ ] Read battery SoC and power
- [ ] Read EV state (if connected)
- [ ] Read grid import/export
- [ ] Step 2: Fetch current prices
- [ ] Step 3: Call `power_flow_model.compute_flows()`
- [ ] Step 4: Issue commands based on snapshot
- [ ] Command battery power
- [ ] Command EVSE current limit
- [ ] Step 5: Log snapshot to database
- [ ] Step 6: Update metrics
- [ ] Add error handling (continue on transient errors)
- [ ] Add graceful shutdown
- [ ] Add integration tests with simulated environment

### Smooth Power Transitions
- [ ] Create `src/controller/power_transition.rs`
- [ ] Implement power ramping (don't change power instantly)
- [ ] Add maximum rate of change (kW/second)
- [ ] Add PID control for smooth battery power
- [ ] Add current ramping for EVSE
- [ ] Prevent oscillations
- [x] Add unit tests for ramping

### Safety Monitors
- [ ] Create `src/controller/safety_monitor.rs`
- [ ] Create `SafetyMonitor` struct
- [ ] Monitor fuse limit continuously
- [ ] Detect fuse trip conditions
- [ ] Emergency power reduction if approaching limit
- [ ] Monitor battery temperature
- [ ] Emergency stop if temp >60°C
- [ ] Monitor battery SoC bounds
- [ ] Emergency action if SoC <5% or >98%
- [ ] Monitor phase imbalance (3-phase systems)
- [ ] Add safety event logging
- [ ] Add alerts/notifications
- [x] Add unit tests for each safety check

### Power Flow Metrics
- [ ] Add metric: `power_flow_pv_kw` gauge
- [ ] Add metric: `power_flow_house_load_kw` gauge
- [ ] Add metric: `power_flow_battery_kw` gauge (signed)
- [ ] Add metric: `power_flow_ev_kw` gauge
- [ ] Add metric: `power_flow_grid_import_kw` gauge
- [ ] Add metric: `power_flow_grid_export_kw` gauge
- [ ] Add metric: `power_balance_error_kw` gauge (should be ~0)
- [ ] Add metric: `fuse_utilization_percent` gauge
- [ ] Add metric: `power_flow_iterations_total` counter
- [ ] Add metric: `safety_events_total` counter
- [ ] Add metric: `constraint_violations_total` counter (should be 0!)

### Database Persistence
- [ ] Create migration for `power_flow_snapshots` table
- [ ] Add all PowerSnapshot fields to table
- [ ] Add index on timestamp
- [ ] Add partitioning by date
- [ ] Create repository for snapshots
- [ ] Implement `save_snapshot(snapshot)` method
- [ ] Implement `get_latest_snapshot()` method
- [ ] Implement `get_snapshots_range(start, end)` method
- [ ] Add retention policy (keep 90 days)
- [ ] Add integration tests

---

## 📋 PHASE 18: OCPP INTEGRATION (EV Charging Protocol)

### OCPP Protocol Basics
- [ ] Research OCPP 1.6 specification
- [ ] Research OCPP 2.0.1 specification
- [ ] Decide on version to implement (start with 1.6)
- [ ] Document OCPP message flow

### OCPP WebSocket Client
- [ ] Create `src/ocpp/mod.rs`
- [ ] Create `src/ocpp/client.rs`
- [ ] Create `OcppClient` struct
- [ ] Implement WebSocket connection with tokio-tungstenite
- [ ] Implement OCPP message framing (JSON over WebSocket)
- [ ] Implement message ID tracking
- [ ] Add connection keep-alive (heartbeat)
- [ ] Add reconnection logic
- [x] Add unit tests

### OCPP Message Types
- [ ] Create `src/ocpp/messages/mod.rs`
- [ ] Create `src/ocpp/messages/call.rs` (client → charger)
- [ ] Create `src/ocpp/messages/call_result.rs` (response)
- [ ] Create `src/ocpp/messages/call_error.rs` (error response)
- [ ] Implement `BootNotification` message
- [ ] Implement `StatusNotification` message
- [ ] Implement `StartTransaction` message
- [ ] Implement `StopTransaction` message
- [ ] Implement `RemoteStartTransaction` message (for control)
- [ ] Implement `RemoteStopTransaction` message
- [ ] Implement `ChangeConfiguration` message
- [ ] Implement `GetConfiguration` message
- [ ] Implement `MeterValues` message
- [ ] Add serde serialization/deserialization
- [x] Add unit tests for message parsing

### OCPP Charger Interface
- [ ] Update `EvCharger` trait with OCPP-specific methods
- [ ] Add `send_remote_start(connector_id, id_tag) -> Result<()>` method
- [ ] Add `send_remote_stop(transaction_id) -> Result<()>` method
- [ ] Add `set_charging_profile(profile) -> Result<()>` method
- [ ] Add `get_meter_values() -> Result<MeterValues>` method

### OCPP EV Charger Implementation
- [ ] Create `src/hardware/ocpp/ev_charger.rs`
- [ ] Create `OcppEvCharger` struct
- [x] Implement `EvCharger` trait
- [ ] Implement state synchronization from `StatusNotification`
- [ ] Implement meter values from `MeterValues` messages
- [ ] Implement charging control via `RemoteStart/StopTransaction`
- [ ] Add error handling for OCPP errors
- [ ] Add integration tests with mock OCPP server

### OCPP Message Handler
- [ ] Create `src/ocpp/handler.rs`
- [ ] Create `OcppMessageHandler` struct
- [ ] Implement handler for `BootNotification`
- [ ] Implement handler for `StatusNotification`
- [ ] Implement handler for `MeterValues`
- [ ] Implement handler for `StartTransaction`
- [ ] Implement handler for `StopTransaction`
- [ ] Route messages to appropriate handlers
- [x] Add unit tests

### OCPP Configuration
- [ ] Add OCPP configuration to `AppConfig`
- [ ] Add charger endpoint URLs
- [ ] Add charger authentication (basic auth or TLS)
- [ ] Add reconnection parameters
- [ ] Add heartbeat interval
- [ ] Document configuration

### OCPP Smart Charging
- [ ] Create `src/ocpp/smart_charging.rs`
- [ ] Implement charging profile generation
- [ ] Implement schedule → OCPP profile conversion
- [ ] Implement `SetChargingProfile` message
- [ ] Implement profile validation
- [ ] Add integration tests

---

## 📋 PHASE 16: V2X (VEHICLE-TO-GRID) SUPPORT

### V2X Concepts & Research
- [ ] Research ISO 15118 (communication protocol)
- [ ] Research bidirectional charging standards
- [ ] Document V2X capabilities required
- [ ] Identify compatible charger models
- [ ] Document grid requirements for V2X

### V2X Domain Model
- [ ] Update `EvCharger` trait with V2X methods
- [ ] Add `enable_discharge() -> Result<()>` method
- [ ] Add `set_discharge_power(watts: Power) -> Result<()>` method
- [ ] Add `get_v2x_capabilities() -> Result<V2XCapabilities>` method
- [ ] Update `ChargerState` with discharge information
- [ ] Create `V2XSession` struct
- [x] Add unit tests

### V2X Controller
- [ ] Create `src/controller/v2x_controller.rs`
- [ ] Create `V2xController` struct
- [ ] Implement vehicle connection detection
- [ ] Implement SoC readout from vehicle
- [ ] Implement discharge control
- [ ] Implement safety limits (minimum vehicle SoC)
- [ ] Implement user preferences (reserve range for driving)
- [ ] Add integration tests

### V2X Optimization Integration
- [ ] Update optimizer to consider V2X as additional battery
- [ ] Add vehicle battery capacity to optimization model
- [ ] Add vehicle availability schedule (user inputs when car is home)
- [ ] Add constraints for minimum vehicle SoC
- [ ] Add separate degradation model for vehicle battery
- [x] Add unit tests

### V2X API Endpoints
- [ ] Create `src/api/handlers/v2x.rs`
- [ ] Implement `GET /api/v1/v2x/status` handler
- [ ] Implement `POST /api/v1/v2x/enable` handler
- [ ] Implement `POST /api/v1/v2x/disable` handler
- [ ] Implement `PUT /api/v1/v2x/preferences` handler (min SoC, availability)
- [ ] Add OpenAPI documentation
- [ ] Add integration tests

### V2X Configuration
- [ ] Add `V2xConfig` struct
- [ ] Add enable/disable flag
- [ ] Add minimum vehicle SoC setting
- [ ] Add maximum discharge power
- [ ] Add vehicle availability schedule
- [ ] Document configuration

---

## 📋 PHASE 17: MACHINE LEARNING - DATA PIPELINE

### Training Data Collection
- [ ] Create `src/ml/data/mod.rs`
- [ ] Create `src/ml/data/collector.rs`
- [ ] Create `TrainingDataCollector` struct
- [ ] Implement feature extraction from battery states
- [ ] Implement feature extraction from weather data
- [ ] Implement feature extraction from price data
- [ ] Implement label extraction (actual consumption/production)
- [ ] Store training data in database
- [ ] Add data quality checks
- [x] Add unit tests

### Feature Engineering
- [ ] Create `src/ml/features/mod.rs`
- [ ] Create `src/ml/features/temporal.rs`
- [ ] Extract hour-of-day (0-23)
- [ ] Extract day-of-week (0-6)
- [ ] Extract month (1-12)
- [ ] Extract is-weekend (boolean)
- [ ] Extract is-holiday (boolean)
- [ ] Create `src/ml/features/weather.rs`
- [ ] Extract temperature (current, forecast)
- [ ] Extract cloud cover
- [ ] Extract wind speed
- [ ] Create `src/ml/features/consumption.rs`
- [ ] Extract lagged consumption (t-1, t-24, t-168 hours)
- [ ] Extract rolling averages (7-day, 30-day)
- [ ] Extract consumption trends
- [ ] Create feature normalization (StandardScaler)
- [ ] Create feature vector struct
- [x] Add unit tests

### Dataset Management
- [ ] Create `src/ml/data/dataset.rs`
- [ ] Create `Dataset` struct (features + labels)
- [ ] Implement train/test split
- [ ] Implement cross-validation splits
- [ ] Implement data augmentation (synthetic scenarios)
- [ ] Implement batch loading from database
- [x] Add unit tests

### Data Export for Training
- [ ] Create `src/ml/data/export.rs`
- [ ] Export dataset to Parquet format
- [ ] Export dataset to CSV format
- [ ] Add CLI command: `cargo run --bin export-training-data`
- [ ] Add compression
- [ ] Add metadata (feature names, statistics)
- [x] Add unit tests

---

## 📋 PHASE 18: MACHINE LEARNING - CONSUMPTION FORECASTING

### Model Selection
- [ ] Research suitable models (LSTM, XGBoost, Prophet)
- [ ] Benchmark different approaches on synthetic data
- [ ] Document model selection rationale

### Linear Regression Baseline
- [ ] Create `src/ml/models/linear_regression.rs`
- [ ] Implement linear regression with `linfa`
- [ ] Train on historical consumption data
- [ ] Evaluate with MAE, RMSE, MAPE
- [ ] Compare against simple moving average
- [x] Add unit tests

### XGBoost Model (Gradient Boosting)
- [ ] Create `src/ml/models/xgboost.rs`
- [ ] Implement XGBoost with `linfa-trees`
- [ ] Define hyperparameters (n_estimators, max_depth, learning_rate)
- [ ] Implement training loop
- [ ] Implement hyperparameter tuning (grid search)
- [ ] Evaluate model performance
- [ ] Add feature importance analysis
- [x] Add unit tests

### LSTM Model (Deep Learning) - Optional
- [ ] Create `src/ml/models/lstm.rs`
- [ ] Implement LSTM with `burn` or external Python model
- [ ] Define sequence length (168 hours = 1 week)
- [ ] Implement training loop
- [ ] Implement early stopping
- [ ] Evaluate model performance
- [ ] Export trained model
- [x] Add unit tests

### Model Training Pipeline
- [ ] Create `src/ml/training/mod.rs`
- [ ] Create `src/ml/training/trainer.rs`
- [ ] Create `ModelTrainer` struct
- [ ] Implement training loop with validation
- [ ] Implement early stopping
- [ ] Implement learning rate scheduling
- [ ] Implement model checkpointing
- [ ] Add training metrics logging
- [ ] Add CLI command: `cargo run --bin train-model consumption`
- [x] Add unit tests

### Model Evaluation
- [ ] Create `src/ml/training/evaluator.rs`
- [ ] Implement cross-validation
- [ ] Calculate MAE, RMSE, MAPE
- [ ] Generate prediction vs actual plots
- [ ] Generate residual plots
- [ ] Generate feature importance plots
- [x] Add unit tests

### Model Persistence
- [ ] Create `src/ml/models/persistence.rs`
- [ ] Implement model serialization (save to file)
- [ ] Implement model deserialization (load from file)
- [ ] Add model versioning
- [ ] Add model metadata (training date, features, metrics)
- [ ] Store models in `models/` directory
- [x] Add unit tests

---

## 📋 PHASE 19: MACHINE LEARNING - PRICE FORECASTING

### LSTM Price Model
- [ ] Create `src/ml/models/price_lstm.rs`
- [ ] Implement LSTM for price prediction
- [ ] Use multi-step forecasting (predict 24h ahead)
- [ ] Implement training loop
- [ ] Evaluate against Nordpool day-ahead prices
- [x] Add unit tests

### Prophet Model (Facebook)
- [ ] Research Prophet library (may need Python bridge)
- [ ] Create `src/ml/models/prophet.rs`
- [ ] Implement seasonal decomposition
- [ ] Implement trend forecasting
- [ ] Evaluate model performance
- [x] Add unit tests

### Ensemble Model
- [ ] Create `src/ml/models/ensemble_price.rs`
- [ ] Combine multiple price forecasting models
- [ ] Implement weighted averaging
- [ ] Implement model selection based on recent performance
- [x] Add unit tests

### Model Training Pipeline for Price
- [ ] Add CLI command: `cargo run --bin train-model price`
- [ ] Implement training on historical Nordpool data
- [ ] Evaluate model performance
- [ ] Document training process

---

## 📋 PHASE 20: MACHINE LEARNING - PRODUCTION FORECASTING

### Solar Production ML Model
- [ ] Create `src/ml/models/solar_production.rs`
- [ ] Implement model with weather features
- [ ] Use cloud cover, temperature, solar angle
- [ ] Train on historical production data
- [ ] Evaluate model performance
- [x] Add unit tests

### Model Training Pipeline for Production
- [ ] Add CLI command: `cargo run --bin train-model production`
- [ ] Implement training on historical solar data
- [ ] Evaluate model performance
- [ ] Document training process

---

## 📋 PHASE 21: MACHINE LEARNING - INFERENCE

### ONNX Runtime Integration
- [ ] Create `src/ml/inference/mod.rs`
- [ ] Create `src/ml/inference/onnx.rs`
- [ ] Export trained models to ONNX format
- [ ] Load ONNX models with `onnxruntime`
- [ ] Implement inference pipeline
- [ ] Add model warm-up
- [ ] Add batch inference support
- [x] Add unit tests

### ML-Powered Forecaster
- [ ] Create `src/ml/inference/forecaster.rs`
- [ ] Create `MlForecaster` struct
- [ ] Load trained consumption model
- [ ] Load trained price model
- [ ] Load trained production model
- [ ] Implement inference for 24h forecast
- [ ] Add fallback to simple models on error
- [ ] Add inference metrics (latency, accuracy)
- [x] Add unit tests

### Integration with Forecast Service
- [ ] Update `ForecastAggregator` to use ML models
- [ ] Add feature flag: `use_ml = true/false`
- [ ] Implement model selection (ML vs simple)
- [ ] Add A/B testing framework
- [ ] Add online learning (periodic retraining)
- [x] Add unit tests

### Model Monitoring
- [ ] Create `src/ml/monitoring/mod.rs`
- [ ] Track prediction accuracy over time
- [ ] Detect model drift
- [ ] Alert when accuracy degrades
- [ ] Add automatic retraining trigger
- [x] Add unit tests

---

## 📋 PHASE 22: ADVANCED OPTIMIZATION

### Mixed-Integer Linear Programming (MILP)
- [ ] Create `src/optimizer/strategies/milp.rs`
- [ ] Create `MilpOptimizer` struct
- [ ] Formulate battery scheduling as MILP
- [ ] Use `good_lp` for MILP solving
- [ ] Add binary variables for on/off states
- [ ] Add constraint modeling
- [ ] Compare performance vs DP
- [x] Add unit tests

### Stochastic Optimization
- [ ] Create `src/optimizer/strategies/stochastic.rs`
- [ ] Create `StochasticOptimizer` struct
- [ ] Incorporate forecast uncertainty
- [ ] Generate scenario tree
- [ ] Implement scenario-based optimization
- [x] Add unit tests

### Model Predictive Control (MPC)
- [ ] Create `src/optimizer/strategies/mpc.rs`
- [ ] Create `MpcOptimizer` struct
- [ ] Implement receding horizon optimization
- [ ] Update optimization every hour with new measurements
- [ ] Add disturbance rejection
- [x] Add unit tests

### Reinforcement Learning Optimizer (Advanced)
- [ ] Create `src/ml/rl/mod.rs`
- [ ] Define state space (battery SoC, time, price)
- [ ] Define action space (charge/discharge/idle)
- [ ] Define reward function (negative cost)
- [ ] Implement Q-learning or Deep Q-Network (DQN)
- [ ] Implement training environment
- [ ] Train agent on simulated scenarios
- [ ] Implement policy inference
- [x] Add unit tests
- [ ] Compare against DP/MILP

---

## 📋 PHASE 23: PRODUCTION DEPLOYMENT

### Raspberry Pi Setup
- [ ] Document Raspberry Pi OS installation
- [ ] Document network configuration
- [ ] Document security hardening
- [ ] Setup SSH keys
- [ ] Install Docker on RPi
- [ ] Install PostgreSQL on RPi
- [ ] Configure firewall (ufw)

### Cross-Compilation
- [ ] Setup cross-compilation toolchain
- [ ] Add `[target.aarch64-unknown-linux-gnu]` to `.cargo/config.toml`
- [ ] Build ARM binary: `cross build --target aarch64-unknown-linux-gnu --release`
- [ ] Test binary on RPi
- [ ] Add to CI/CD pipeline
- [ ] Document build process

### Systemd Service
- [ ] Create `energy-controller.service` file
- [ ] Configure service to run on boot
- [ ] Configure automatic restart on failure
- [ ] Setup logging to journald
- [ ] Add service to RPi
- [ ] Test service start/stop/restart
- [ ] Document service management

### Database Backup
- [ ] Create backup script (`scripts/backup_db.sh`)
- [ ] Setup cron job for daily backups
- [ ] Test backup restoration
- [ ] Document backup/restore process

### Monitoring Setup
- [ ] Install Prometheus on RPi or separate server
- [ ] Configure Prometheus to scrape `/metrics`
- [ ] Install Grafana
- [ ] Create Grafana dashboard for battery metrics
- [ ] Create Grafana dashboard for optimization metrics
- [ ] Create Grafana dashboard for forecast accuracy
- [ ] Setup alerting rules (battery faults, optimization failures)
- [ ] Document monitoring setup

### Log Management
- [ ] Configure log rotation
- [ ] Setup log aggregation (optional: Loki)
- [ ] Document log access

### Secure Configuration
- [ ] Store secrets in environment variables
- [ ] Use `.env` file on RPi
- [ ] Restrict file permissions (600 for .env)
- [ ] Document secret management

### Performance Tuning
- [ ] Profile application with `perf` or `flamegraph`
- [ ] Optimize database queries
- [ ] Optimize Modbus communication (batch reads)
- [ ] Add caching where appropriate
- [ ] Document performance characteristics

### Deployment Automation
- [ ] Create deployment script (`scripts/deploy.sh`)
- [ ] Automate binary transfer to RPi
- [ ] Automate database migration
- [ ] Automate service restart
- [ ] Add rollback capability
- [ ] Document deployment process

---

## 📋 PHASE 24: TESTING - COMPREHENSIVE

### Unit Tests
- [ ] Ensure >90% code coverage for domain logic
- [ ] Ensure >80% code coverage overall
- [ ] Add tests for all error paths
- [ ] Add tests for edge cases
- [ ] Add tests for boundary conditions
- [ ] Run: `cargo test --lib`
- [ ] Add coverage report with `cargo-tarpaulin`

### Integration Tests
- [ ] Test database operations with real Postgres
- [ ] Test API endpoints with real HTTP server
- [ ] Test Modbus communication with mock server
- [ ] Test controller with simulated hardware
- [ ] Test optimizer end-to-end
- [ ] Test forecast pipeline end-to-end
- [ ] Run: `cargo test --test '*'`

### Property-Based Tests
- [ ] Add proptest for domain types
- [ ] Add proptest for optimizer (invariants hold)
- [ ] Add proptest for schedule validation
- [ ] Run: `cargo test`

### Load Tests
- [ ] Create load test scripts with `wrk` or `hey`
- [ ] Test API throughput (requests/second)
- [ ] Test database under load
- [ ] Test Modbus client with many devices
- [ ] Document performance benchmarks

### Smoke Tests on RPi
- [ ] Deploy to test RPi
- [ ] Run basic functionality test
- [ ] Verify controller starts
- [ ] Verify API responds
- [ ] Verify database connection
- [ ] Verify Modbus connection (if hardware available)

---

## 📋 PHASE 25: DOCUMENTATION

### Architecture Documentation
- [ ] Create `docs/ARCHITECTURE.md`
- [ ] Add high-level architecture diagram
- [ ] Document all major components
- [ ] Document data flow
- [ ] Document design decisions

### Architecture Decision Records (ADRs)
- [x] Create `docs/ADR/001-rust-choice.md`
- [x] Create `docs/ADR/002-database-choice.md`
- [x] Create `docs/ADR/003-optimization-strategy.md`
- [x] Create `docs/ADR/004-modbus-abstraction.md`
- [x] Create `docs/ADR/005-forecast-approach.md`
- [x] Create `docs/ADR/006-ml-framework.md`
- [x] Create ADR template
- [x] Document ADR process

### API Documentation
- [ ] Generate OpenAPI spec
- [ ] Add usage examples for each endpoint
- [ ] Add authentication documentation (if implemented)
- [ ] Add rate limiting documentation
- [ ] Host documentation on GitHub Pages

### User Guide
- [x] Create `docs/USER_GUIDE.md`
- [x] Document installation process
- [x] Document configuration
- [x] Document basic usage
- [x] Add troubleshooting section
- [x] Add FAQ section

### Developer Guide
- [x] Create `docs/DEVELOPER_GUIDE.md`
- [x] Document development setup
- [x] Document project structure
- [x] Document coding conventions
- [x] Document testing strategy
- [x] Document CI/CD pipeline
- [x] Add contribution guidelines

### README
- [ ] Update README with project overview
- [ ] Add badges (build status, coverage, license)
- [ ] Add features list
- [ ] Add quick start guide
- [ ] Add links to detailed documentation
- [ ] Add demo video or screenshots
- [ ] Add acknowledgments

### Modbus Register Documentation
- [ ] Create `docs/MODBUS_REGISTERS.md`
- [ ] Document supported vendors
- [ ] Document register mappings for each vendor
- [ ] Add configuration examples
- [ ] Add troubleshooting tips

### OCPP Documentation
- [ ] Create `docs/OCPP_INTEGRATION.md`
- [ ] Document supported OCPP versions
- [ ] Document message flows
- [ ] Add configuration examples
- [ ] Add troubleshooting tips

### ML Documentation
- [ ] Create `docs/MACHINE_LEARNING.md`
- [ ] Document model architectures
- [ ] Document training process
- [ ] Document feature engineering
- [ ] Add model performance benchmarks
- [ ] Add retraining guidelines

---

## 📋 PHASE 26: CI/CD PIPELINE

### GitHub Actions Setup
- [ ] Create `.github/workflows/ci.yml`
- [ ] Add Rust setup action
- [ ] Add PostgreSQL service for tests
- [ ] Run `cargo fmt --check`
- [ ] Run `cargo clippy -- -D warnings`
- [ ] Run `cargo test --all-features`
- [ ] Run `cargo build --release`
- [ ] Cache cargo dependencies
- [ ] Add test coverage reporting

### Cross-Compilation in CI
- [ ] Add ARM build job
- [ ] Use `cross` for cross-compilation
- [ ] Cache cross toolchain
- [ ] Upload ARM binaries as artifacts
- [ ] Add job for both x86_64 and aarch64

### Security Scanning
- [ ] Add `cargo-audit` to CI (check for vulnerabilities)
- [ ] Add `cargo-deny` to CI (license and dependency checks)
- [ ] Add Dependabot for dependency updates
- [ ] Add CodeQL security analysis

### Release Automation
- [ ] Create `.github/workflows/release.yml`
- [ ] Trigger on tag push (v*)
- [ ] Build binaries for x86_64 and aarch64
- [ ] Create GitHub release
- [ ] Upload binaries to release
- [ ] Generate changelog
- [ ] Document release process

### Docker Image Build
- [ ] Create multi-arch Dockerfile
- [ ] Add Docker build to CI
- [ ] Push to Docker Hub or GitHub Container Registry
- [ ] Tag with version and `latest`
- [ ] Document Docker usage

---

## 📋 PHASE 27: ADVANCED FEATURES

### Cloud Proxy for Remote Access
- [ ] Research cloud proxy requirements (per job ad)
- [ ] Create proxy server (separate service)
- [ ] Implement WebSocket tunneling
- [ ] Implement authentication for remote access
- [ ] Add end-to-end encryption
- [ ] Test remote access from mobile
- [ ] Document setup

### Mobile App API Enhancements
- [ ] Add WebSocket endpoint for real-time updates
- [ ] Implement push notifications (webhook)
- [ ] Add user preferences API
- [ ] Add historical data export API (CSV, JSON)
- [ ] Add API versioning (v1, v2)
- [ ] Document mobile API

### Multi-Household Support
- [ ] Add `household_id` to all relevant tables
- [ ] Update API to support multi-tenancy
- [ ] Add household management endpoints
- [ ] Add user-to-household associations
- [ ] Add access control (users can only see their household)
- [x] Add unit tests

### Time-of-Use Tariff Support
- [ ] Model complex tariffs (peak, off-peak, super-off-peak)
- [ ] Integrate with Swedish grid operators
- [ ] Update optimizer to consider tariff structure
- [ ] Add tariff configuration endpoint
- [x] Add unit tests

### Grid Services (Frequency Response)
- [ ] Research grid frequency regulation requirements
- [ ] Implement frequency measurement
- [ ] Implement automatic response to frequency deviations
- [ ] Add safety limits
- [ ] Add opt-in configuration
- [x] Add unit tests

### Battery Degradation Modeling
- [ ] Implement cycle counting (rainflow algorithm)
- [ ] Model capacity fade over time
- [ ] Model resistance increase over time
- [ ] Update health percentage calculation
- [ ] Integrate with optimizer (minimize degradation)
- [x] Add unit tests

### Solar Forecasting Improvements
- [ ] Integrate with additional weather APIs
- [ ] Implement satellite-based cloud forecasting
- [ ] Implement machine learning for solar production
- [ ] Compare forecast accuracy
- [x] Add unit tests

### Demand Response Programs
- [ ] Research Swedish demand response programs
- [ ] Implement API for demand response signals
- [ ] Update optimizer to respond to DR events
- [ ] Add user opt-in configuration
- [x] Add unit tests

---

## 📋 PHASE 28: POLISH & DEMO PREPARATION

### Code Quality
- [ ] Run `cargo fmt` on all files
- [ ] Run `cargo clippy --all-targets --all-features` and fix all warnings
- [ ] Review all TODO comments and address or document
- [ ] Review all FIXME comments and fix
- [ ] Ensure consistent naming conventions
- [ ] Ensure consistent error handling patterns

### Performance Review
- [ ] Profile application with `cargo flamegraph`
- [ ] Identify hotspots
- [ ] Optimize critical paths
- [ ] Ensure database queries use indexes
- [ ] Ensure Modbus communication is batched
- [ ] Document performance characteristics

### Documentation Review
- [ ] Proofread all documentation
- [ ] Ensure all code examples work
- [ ] Ensure all links are valid
- [ ] Add table of contents where needed
- [ ] Add more diagrams (architecture, data flow, etc.)

### Demo Video/Presentation
- [ ] Create demo script
- [ ] Record screen showing:
  - [ ] System starting up
  - [ ] API health check
  - [ ] Current battery status
  - [ ] Forecast display
  - [ ] Optimization running
  - [ ] Schedule being executed
  - [ ] Metrics dashboard (Grafana)
- [ ] Add voice-over explaining architecture
- [ ] Upload to YouTube (unlisted)
- [ ] Add link to README

### GitHub Repository Polish
- [ ] Add comprehensive .gitignore
- [ ] Add LICENSE file
- [ ] Add CODE_OF_CONDUCT.md
- [ ] Add SECURITY.md (responsible disclosure)
- [ ] Add issue templates
- [ ] Add pull request template
- [ ] Create GitHub project board
- [ ] Add GitHub topics/tags

### Portfolio Presentation
- [ ] Create one-page project summary
- [ ] Highlight key technologies (Rust, Modbus, ML, etc.)
- [ ] Highlight alignment with Spotpilot job requirements
- [ ] Create architecture diagram (draw.io or similar)
- [ ] Add to LinkedIn/portfolio website

---

## 📋 PHASE 29: OPTIONAL ENHANCEMENTS

### Web Dashboard (Frontend)
- [ ] Choose frontend framework (React, Vue, Svelte)
- [ ] Create dashboard with battery status
- [ ] Create dashboard with schedule visualization
- [ ] Create dashboard with forecast charts
- [ ] Create dashboard with historical data
- [ ] Add real-time updates via WebSocket
- [ ] Deploy with Nginx reverse proxy

### Mobile App (Optional)
- [ ] Choose mobile framework (Flutter, React Native)
- [ ] Create basic mobile UI
- [ ] Integrate with REST API
- [ ] Add push notifications
- [ ] Publish to app store (optional)

### Advanced Visualizations
- [ ] Add Grafana dashboards to repository
- [ ] Create dashboard for real-time metrics
- [ ] Create dashboard for forecast accuracy
- [ ] Create dashboard for optimization results
- [ ] Document dashboard setup

### Additional Integrations
- [ ] Integrate with Home Assistant
- [ ] Integrate with other smart home platforms
- [ ] Integrate with additional solar inverters
- [ ] Integrate with additional battery brands

---

## 📋 PHASE 30: FINAL REVIEW & SUBMISSION

### Final Testing
- [ ] Run full test suite: `cargo test --all-features`
- [ ] Run clippy: `cargo clippy --all-targets --all-features`
- [ ] Run fmt check: `cargo fmt -- --check`
- [ ] Run security audit: `cargo audit`
- [ ] Test deployment on clean RPi
- [ ] Verify all features work end-to-end

### Documentation Final Review
- [ ] Proofread README
- [ ] Verify all links work
- [ ] Ensure installation instructions are complete
- [ ] Ensure troubleshooting guide is helpful
- [ ] Add contact information

### Repository Cleanup
- [ ] Remove unused dependencies
- [ ] Remove commented-out code
- [ ] Remove debug prints
- [ ] Ensure no secrets in code or config
- [ ] Ensure .env is in .gitignore

### Job Application Preparation
- [ ] Add project to resume
- [ ] Add project to LinkedIn
- [ ] Prepare 5-minute walkthrough for interview
- [ ] Prepare answers to technical questions
- [ ] Prepare list of future improvements

### Submit Application
- [ ] Include GitHub repository link
- [ ] Include demo video link
- [ ] Include architecture diagram
- [ ] Highlight key achievements:
  - [ ] Rust expertise (async, traits, zero-cost abstractions)
  - [ ] Edge-first design (local-first, low latency)
  - [ ] Modbus TCP integration (vendor abstraction)
  - [ ] Device discovery (mDNS + network scanning)
  - [ ] Optimization (DP, MILP, potential for RL)
  - [ ] Machine learning (forecasting, model training, inference)
  - [ ] OCPP integration (EV charging)
  - [ ] V2X support (vehicle-to-grid)
  - [ ] Production-ready (database, metrics, logging, error handling)
  - [ ] Comprehensive testing (unit, integration, property-based)
  - [ ] Documentation (ADRs, API docs, user guide)
- [ ] Write compelling cover letter explaining how this demonstrates fit for role

---

## 🎉 COMPLETION CHECKLIST

### MVP (Minimum Viable Product)
- [ ] Simulated battery working
- [ ] Basic forecasting (Nordpool + simple consumption)
- [ ] Dynamic programming optimizer
- [ ] REST API with core endpoints
- [ ] Database persistence
- [ ] Basic telemetry
- [ ] Can run on RPi
- [ ] Documentation complete

### Full Demo (Impresses Spotpilot)
- [ ] Modbus TCP integration working
- [ ] Device discovery working
- [ ] ML-based forecasting (consumption, price, production)
- [ ] Advanced optimization (MILP or MPC)
- [ ] OCPP integration (EV charging basics)
- [ ] V2X support (at least stub implementation)
- [ ] Comprehensive documentation
- [ ] Production deployment on RPi
- [ ] Grafana dashboards
- [ ] Demo video

### Advanced (Goes Beyond)
- [ ] Reinforcement learning optimizer
- [ ] Advanced OCPP (smart charging profiles)
- [ ] Full V2X implementation
- [ ] Cloud proxy for remote access
- [ ] Mobile app
- [ ] Web dashboard
- [ ] Multi-household support
- [ ] Grid services integration

---

## 📊 PROGRESS TRACKING

**Total Checkboxes:** ~850+

**Estimated Effort (Solo Developer):**
- MVP: ~80-120 hours
- Full Demo: ~200-300 hours
- Advanced: ~400+ hours

**Estimated Effort (Small Team of 3):**
- MVP: ~30-40 hours
- Full Demo: ~70-100 hours
- Advanced: ~150+ hours

**Remember:**
- Prioritize MVP first
- Each phase builds on previous
- Many items can be done in parallel
- Focus on quality over quantity
- Document as you go
- Test continuously

---

## 🚀 GOOD LUCK!

This project demonstrates:
✅ Rust expertise (async, traits, performance)
✅ Edge computing (local-first, RPi deployment)
✅ Hardware integration (Modbus, OCPP)
✅ Optimization algorithms (DP, MILP, RL)
✅ Machine learning (forecasting, training, inference)
✅ Production engineering (database, metrics, logging)
✅ System design (clean architecture, testability)
✅ Documentation (ADRs, API docs, guides)

**You've got this! 🎯**
