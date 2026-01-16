# 🛠️ Development Guide

**Comprehensive development guidelines for Open Energy Controller**

This guide is for both human and AI developers working on the project.

---

## 🚀 Getting Started

### Prerequisites

- **Rust 1.75+** via rustup
- **PostgreSQL 16**
- **Docker & Docker Compose**
- **Git**
- **Make** (optional but recommended)

### First-Time Setup

```bash
# 1. Clone and enter
git clone https://github.com/yourusername/open-energy-controller.git
cd open-energy-controller

# 2. Install Rust tools
cargo install sqlx-cli --features postgres
cargo install cargo-watch
cargo install cargo-tarpaulin  # For coverage

# 3. Copy environment file
cp .env.example .env
# Edit .env with your database URL

# 4. Start PostgreSQL
docker-compose up -d postgres

# 5. Run migrations
sqlx migrate run

# 6. Build and run
cargo build
cargo run

# 7. Verify it works
curl http://localhost:8080/health
```

---

## 📁 Project Organization

### Directory Structure

```
src/
├── main.rs              # Entry point - minimal, calls lib.rs
├── lib.rs               # Library root - exports public API
├── api/                 # HTTP API layer (Axum)
│   ├── mod.rs
│   ├── handlers/        # Request handlers (one file per resource)
│   ├── middleware/      # Custom middleware
│   ├── routes.rs        # Route definitions
│   ├── state.rs         # Shared application state
│   ├── error.rs         # API error types
│   └── response.rs      # Response wrappers
├── domain/              # Domain models (business logic)
│   ├── battery/
│   │   ├── mod.rs
│   │   ├── traits.rs    # Trait definitions
│   │   ├── state.rs     # State structs
│   │   ├── commands.rs  # Command enums
│   │   └── errors.rs    # Domain errors
│   ├── inverter/
│   ├── ev_charger/
│   └── types.rs         # Shared domain types (Power, Energy, etc.)
├── hardware/            # Hardware implementations
│   ├── simulated/       # For dev/test
│   ├── modbus/          # Real Modbus devices
│   ├── ocpp/            # OCPP EV chargers
│   └── factory.rs       # Device creation
├── optimizer/           # Optimization algorithms
│   ├── strategies/      # Different algorithms (DP, MILP, etc.)
│   ├── constraints.rs
│   └── service.rs
├── forecast/            # Forecasting pipeline
│   ├── price/
│   ├── consumption/
│   └── production/
├── controller/          # Real-time control
│   ├── battery_controller.rs
│   ├── pid.rs
│   └── safety.rs
├── discovery/           # Device discovery
├── modbus/              # Modbus client
├── ocpp/                # OCPP protocol
├── ml/                  # Machine learning
│   ├── models/
│   ├── training/
│   └── inference/
├── database/            # Database layer
│   ├── models/          # DB models
│   ├── repositories/    # Repository pattern
│   └── migrations/      # SQL migrations
├── config/              # Configuration
└── telemetry/           # Metrics & logging

tests/
├── unit/                # Unit tests (if not in src/)
├── integration/         # Integration tests
└── e2e/                 # End-to-end tests
```

### File Naming Conventions

- **Modules:** `snake_case.rs`
- **Types:** `PascalCase`
- **Functions:** `snake_case`
- **Constants:** `SCREAMING_SNAKE_CASE`
- **Lifetimes:** `'a`, `'b`, etc.

### Module Organization Rules

**Each module should:**
- Be <500 lines (split if larger)
- Have a clear single responsibility
- Export only what's needed (`pub` vs `pub(crate)`)
- Include tests in `#[cfg(test)] mod tests { }`

**Example module structure:**
```rust
// src/domain/battery/mod.rs

mod traits;      // Battery trait
mod state;       // BatteryState struct
mod commands;    // BatteryCommand enum
mod errors;      // BatteryError enum

pub use traits::Battery;
pub use state::{BatteryState, BatteryCapabilities};
pub use commands::BatteryCommand;
pub use errors::BatteryError;
```

---

## 🏗️ Architecture Patterns

### Trait-Based Abstraction

**Use traits for polymorphism:**

```rust
// Define the interface
#[async_trait]
pub trait Battery: Send + Sync {
    async fn read_state(&self) -> Result<BatteryState>;
    async fn set_power(&self, power: Power) -> Result<()>;
    fn capabilities(&self) -> BatteryCapabilities;
}

// Multiple implementations
pub struct SimulatedBattery { /* ... */ }
pub struct ModbusBattery { /* ... */ }
pub struct MockBattery { /* ... */ }

// All implement the same trait
impl Battery for SimulatedBattery { /* ... */ }
impl Battery for ModbusBattery { /* ... */ }
impl Battery for MockBattery { /* ... */ }

// Use via trait objects
fn create_battery(config: &Config) -> Arc<dyn Battery> {
    match config.mode {
        Mode::Simulated => Arc::new(SimulatedBattery::new()),
        Mode::Real => Arc::new(ModbusBattery::new()),
    }
}
```

### Repository Pattern

**Separate database logic from business logic:**

```rust
// Domain model (business logic)
pub struct Battery {
    pub id: Uuid,
    pub capacity_kwh: f64,
}

// Database model (persistence)
#[derive(sqlx::FromRow)]
pub struct BatteryRow {
    pub id: Uuid,
    pub capacity_kwh: f64,
    pub created_at: DateTime<Utc>,
}

// Repository (data access)
pub struct BatteryRepository {
    pool: PgPool,
}

impl BatteryRepository {
    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<Battery>> {
        let row = sqlx::query_as!(BatteryRow, "SELECT * FROM batteries WHERE id = $1", id)
            .fetch_optional(&self.pool)
            .await?;
        
        Ok(row.map(|r| Battery {
            id: r.id,
            capacity_kwh: r.capacity_kwh,
        }))
    }
}
```

### Service Layer

**Business logic orchestration:**

```rust
pub struct BatteryService {
    repository: BatteryRepository,
    battery: Arc<dyn Battery>,
}

impl BatteryService {
    pub async fn get_current_state(&self) -> Result<BatteryState> {
        // 1. Read from hardware
        let state = self.battery.read_state().await?;
        
        // 2. Validate
        if !state.is_valid() {
            return Err(Error::InvalidState);
        }
        
        // 3. Persist
        self.repository.save_state(&state).await?;
        
        // 4. Return
        Ok(state)
    }
}
```

### Error Handling

**Use typed errors with `thiserror`:**

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BatteryError {
    #[error("Battery communication failed: {0}")]
    CommunicationError(String),
    
    #[error("Invalid SoC value: {0}")]
    InvalidSoC(f64),
    
    #[error("Battery not found: {0}")]
    NotFound(Uuid),
    
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
}

pub type Result<T> = std::result::Result<T, BatteryError>;
```

### Configuration

**Layered configuration (file → env → args):**

```rust
use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub hardware: HardwareConfig,
}

impl AppConfig {
    pub fn load() -> Result<Self, ConfigError> {
        Config::builder()
            .add_source(File::with_name("config/default"))
            .add_source(File::with_name(&format!("config/{}", env)).required(false))
            .add_source(Environment::with_prefix("APP"))
            .build()?
            .try_deserialize()
    }
}
```

---

## 🧪 Testing Strategy

### Test Levels

1. **Unit Tests** - Test individual functions/methods
2. **Integration Tests** - Test module interactions
3. **End-to-End Tests** - Test full system workflows

### Unit Tests

**Place tests in the same file as the code:**

```rust
// src/domain/battery/state.rs

impl BatteryState {
    pub fn is_low(&self) -> bool {
        self.soc_percent < 20.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_is_low() {
        let state = BatteryState { soc_percent: 15.0, ..Default::default() };
        assert!(state.is_low());
        
        let state = BatteryState { soc_percent: 50.0, ..Default::default() };
        assert!(!state.is_low());
    }
    
    #[test]
    fn test_is_low_boundary() {
        let state = BatteryState { soc_percent: 20.0, ..Default::default() };
        assert!(!state.is_low()); // Exactly 20% is not low
    }
}
```

### Integration Tests

**Place in `tests/` directory:**

```rust
// tests/integration/battery_test.rs

use open_energy_controller::*;

#[tokio::test]
async fn test_battery_read_and_store() {
    // Setup
    let config = Config::test();
    let pool = setup_test_db().await;
    let battery = SimulatedBattery::new(10.0);
    let repo = BatteryRepository::new(pool);
    
    // Execute
    let state = battery.read_state().await.unwrap();
    repo.save_state(&state).await.unwrap();
    
    // Verify
    let saved = repo.get_latest_state().await.unwrap().unwrap();
    assert_eq!(saved.soc_percent, state.soc_percent);
}
```

### Test Database Setup

```rust
// tests/common/mod.rs

use sqlx::PgPool;

pub async fn setup_test_db() -> PgPool {
    let database_url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://localhost/energy_controller_test".to_string());
    
    let pool = PgPool::connect(&database_url).await.unwrap();
    
    // Run migrations
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    
    pool
}

pub async fn cleanup_test_db(pool: &PgPool) {
    sqlx::query("TRUNCATE TABLE batteries CASCADE")
        .execute(pool)
        .await
        .unwrap();
}
```

### Test Fixtures

```rust
// tests/fixtures/battery.rs

pub fn sample_battery() -> Battery {
    Battery {
        id: Uuid::new_v4(),
        capacity_kwh: 10.0,
        max_charge_kw: 5.0,
        max_discharge_kw: 5.0,
    }
}

pub fn sample_battery_state() -> BatteryState {
    BatteryState {
        soc_percent: 50.0,
        power_w: 0.0,
        voltage_v: 51.2,
        temperature_c: 25.0,
        health_percent: 100.0,
    }
}
```

### Mocking

**Use `mockall` for mocking traits:**

```rust
#[cfg(test)]
use mockall::predicate::*;
#[cfg(test)]
use mockall::mock;

mock! {
    pub Battery {}
    
    #[async_trait]
    impl Battery for Battery {
        async fn read_state(&self) -> Result<BatteryState>;
        async fn set_power(&self, power: Power) -> Result<()>;
        fn capabilities(&self) -> BatteryCapabilities;
    }
}

#[tokio::test]
async fn test_controller_with_mock() {
    let mut mock_battery = MockBattery::new();
    mock_battery
        .expect_read_state()
        .returning(|| Ok(BatteryState { /* ... */ }));
    
    let controller = BatteryController::new(Arc::new(mock_battery));
    // Test controller logic...
}
```

---

## 🔧 Development Workflow

### Daily Workflow

```bash
# 1. Pull latest changes
git pull origin main

# 2. Create feature branch
git checkout -b feature/my-feature

# 3. Make changes (following TODO list)

# 4. Run tests continuously
cargo watch -x test

# 5. Check code quality
cargo fmt
cargo clippy -- -D warnings

# 6. Update TODO list
# Mark completed items in MASSIVE_TODO_LIST.md

# 7. Commit
git add .
git commit -m "feat: implement feature X"

# 8. Push
git push origin feature/my-feature

# 9. Create PR
```

### Pre-Commit Checklist

```bash
# Format code
cargo fmt

# Check for warnings
cargo clippy --all-targets --all-features -- -D warnings

# Run tests
cargo test

# Check for common mistakes
rg "println!|dbg!" src/
rg "unwrap\(\)" src/ | grep -v test
rg "TODO" src/

# Update TODO list
# Edit MASSIVE_TODO_LIST.md

# Review changes
git diff

# Commit
git commit
```

### Git Commit Messages

**Follow Conventional Commits:**

```
<type>(<scope>): <subject>

<body>

<footer>
```

**Types:**
- `feat:` - New feature
- `fix:` - Bug fix
- `docs:` - Documentation changes
- `test:` - Test additions/changes
- `refactor:` - Code refactoring
- `perf:` - Performance improvements
- `chore:` - Build/tooling changes

**Examples:**
```
feat(battery): add SimulatedBattery implementation

- Implement Battery trait
- Add state simulation with realistic behavior
- Add comprehensive unit tests

Updated MASSIVE_TODO_LIST.md Phase 4 items 1-8
```

```
fix(modbus): handle connection timeout gracefully

Previously the code would panic on timeout. Now it returns
a proper error that can be handled by the caller.

Fixes #42
```

---

## 🔍 Code Review Guidelines

### What to Look For

**Architecture:**
- [ ] Does it fit the overall design?
- [ ] Is it in the right module?
- [ ] Does it use appropriate abstractions?

**Code Quality:**
- [ ] Is it readable?
- [ ] Are names descriptive?
- [ ] Is it well-documented?
- [ ] No commented-out code?

**Testing:**
- [ ] Are there tests?
- [ ] Do tests cover edge cases?
- [ ] Are tests readable?

**Error Handling:**
- [ ] Are errors handled properly?
- [ ] No unwraps in production code?
- [ ] Are errors logged?

**Performance:**
- [ ] Any obvious inefficiencies?
- [ ] Unnecessary allocations?
- [ ] Blocking in async code?

### Review Process

1. **Read the PR description** - Understand the goal
2. **Review the TODO list updates** - Verify completeness
3. **Review the tests** - Do they pass? Are they sufficient?
4. **Review the code** - Line by line
5. **Test locally** - Pull and run it
6. **Approve or request changes**

---

## 📊 Performance Guidelines

### Async Best Practices

**❌ BAD:**
```rust
async fn process_batteries(batteries: Vec<Battery>) {
    for battery in batteries {
        // Sequential! Slow!
        let state = battery.read_state().await;
        process(state);
    }
}
```

**✅ GOOD:**
```rust
async fn process_batteries(batteries: Vec<Battery>) {
    let futures = batteries.iter().map(|b| b.read_state());
    let states = futures::future::join_all(futures).await;
    // Parallel! Fast!
}
```

### Database Performance

**Use indexes:**
```sql
CREATE INDEX idx_battery_states_timestamp ON battery_states(device_id, timestamp DESC);
```

**Batch inserts:**
```rust
// ❌ BAD - N queries
for state in states {
    repo.insert(state).await?;
}

// ✅ GOOD - 1 query
repo.insert_batch(states).await?;
```

**Use connection pooling:**
```rust
let pool = PgPoolOptions::new()
    .max_connections(10)
    .connect(&database_url)
    .await?;
```

### Memory Management

**Prefer `Arc<T>` over `Rc<T>` for concurrent code:**
```rust
let battery: Arc<dyn Battery> = Arc::new(SimulatedBattery::new());
tokio::spawn(async move {
    // battery can be safely shared across threads
});
```

**Use `Cow` for strings that might be borrowed:**
```rust
use std::borrow::Cow;

fn process_name(name: Cow<str>) {
    // Can accept both &str and String without cloning unnecessarily
}
```

---

## 🛡️ Security Guidelines

### Secrets Management

**Never commit secrets:**
```bash
# .gitignore
.env
*.pem
*.key
secrets.toml
```

**Use environment variables:**
```rust
let api_key = std::env::var("API_KEY")
    .expect("API_KEY must be set");
```

### Input Validation

**Always validate user input:**
```rust
pub fn set_soc(soc: f64) -> Result<()> {
    if !(0.0..=100.0).contains(&soc) {
        return Err(Error::InvalidSoC(soc));
    }
    // ...
}
```

### SQL Injection Prevention

**Use parameterized queries:**
```rust
// ✅ GOOD - sqlx prevents SQL injection
sqlx::query!("SELECT * FROM batteries WHERE id = $1", id)
    .fetch_one(&pool)
    .await?;

// ❌ BAD - vulnerable!
let query = format!("SELECT * FROM batteries WHERE id = '{}'", id);
```

---

## 📈 Monitoring & Observability

### Logging

**Use structured logging with tracing:**

```rust
use tracing::{info, warn, error, debug, instrument};

#[instrument(skip(battery))]
pub async fn read_battery_state(battery: &Battery) -> Result<BatteryState> {
    debug!("Reading battery state");
    
    let state = battery.read_state().await.map_err(|e| {
        error!(?e, "Failed to read battery state");
        e
    })?;
    
    info!(soc = %state.soc_percent, "Battery state read successfully");
    Ok(state)
}
```

### Metrics

**Instrument code with metrics:**

```rust
use metrics::{counter, histogram, gauge};

pub async fn optimize(&self) -> Schedule {
    let start = Instant::now();
    counter!("optimization_runs_total").increment(1);
    
    let schedule = self.run_optimization();
    
    histogram!("optimization_duration_seconds").record(start.elapsed().as_secs_f64());
    gauge!("schedule_horizon_hours").set(schedule.horizon_hours() as f64);
    
    schedule
}
```

---

## 🐛 Debugging

### Enabling Logs

```bash
# All debug logs
RUST_LOG=debug cargo run

# Specific module
RUST_LOG=energy_controller::battery=trace cargo run

# Multiple modules
RUST_LOG=energy_controller=debug,sqlx=info cargo run
```

### Using the Debugger

**With VS Code:**
1. Install rust-analyzer extension
2. Add breakpoint
3. Press F5 to start debugging

**With LLDB:**
```bash
rust-lldb target/debug/energy-controller
(lldb) breakpoint set --file battery.rs --line 42
(lldb) run
```

### Common Issues

**Issue: Database connection refused**
```bash
# Check if PostgreSQL is running
docker-compose ps

# Check DATABASE_URL
echo $DATABASE_URL

# Try connecting manually
psql $DATABASE_URL
```

**Issue: Modbus connection timeout**
```bash
# Check if device is reachable
ping 192.168.1.100

# Check if Modbus port is open
nc -zv 192.168.1.100 502

# Check firewall
sudo ufw status
```

**Issue: Tests failing randomly**
```bash
# Run single test
cargo test test_name -- --nocapture

# Run with more threads
cargo test -- --test-threads=1

# Check for race conditions
cargo test -- --test-threads=1 --nocapture
```

---

## 📚 Resources

### Rust Learning
- [The Rust Book](https://doc.rust-lang.org/book/)
- [Rust by Example](https://doc.rust-lang.org/rust-by-example/)
- [Async Book](https://rust-lang.github.io/async-book/)
- [Tokio Tutorial](https://tokio.rs/tokio/tutorial)

### Project-Specific
- [Architecture Docs](docs/ARCHITECTURE.md)
- [ADRs](docs/ADR/)
- [API Documentation](docs/API.md)
- [Modbus Guide](docs/MODBUS.md)

### Tools
- [cargo-watch](https://github.com/watchexec/cargo-watch) - Auto-rebuild
- [cargo-tarpaulin](https://github.com/xd009642/tarpaulin) - Coverage
- [cargo-audit](https://github.com/RustSec/rustsec/tree/main/cargo-audit) - Security
- [cargo-deny](https://github.com/EmbarkStudios/cargo-deny) - Lints

---

## 🤝 Getting Help

**Found a bug?**
1. Check if it's already reported
2. Create an issue with reproduction steps
3. Include error messages and logs

**Need clarification?**
1. Check the documentation
2. Search existing issues
3. Ask in discussions

**Want to contribute?**
1. Read CONTRIBUTING.md
2. Pick a task from TODO list
3. Read AGENTS.md for guidelines
4. Submit a PR

---

**Happy coding! 🚀**
