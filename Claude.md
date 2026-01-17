# 🤖 Claude AI Assistant Guide for Open Energy Controller

**Welcome to Open Energy Controller!** This guide is specifically designed for Claude AI assistants working on this project.

---

## 🎯 What You Need to Know First

You're working on **Open Energy Controller (OEC)** - a production-grade Rust system that manages residential energy flows in real-time. This is NOT a toy project - it's designed to run on edge devices (like Raspberry Pi) and control real hardware that manages significant power (5-20kW).

**Three critical things to understand immediately:**

1. **Safety First**: This system controls real hardware. Bugs can trip circuit breakers, damage batteries, or cause power outages. Always verify power balance and constraint checking.

2. **Power Flow Orchestration**: This is the core concept. Never control devices independently - always compute holistic power flows that respect all constraints simultaneously.

3. **Production Quality**: All code must be tested, documented, integrated, and maintain 80%+ coverage. No orphaned code, no incomplete implementations.

---

## 📚 Essential Reading Order

When you start working on this project, read files in this order:

1. **This file (Claude.md)** - Project introduction and quick reference (you are here)
2. **POWER_FLOW_ARCHITECTURE.md** - The core algorithm that coordinates all energy flows
3. **AGENTS.md** - Critical coding rules and patterns (MANDATORY)
4. **CLAUDE_CODE.md** - Tool-specific workflow guidance (if you have computer use capabilities)
5. **MASSIVE_TODO_LIST.md** - Find your tasks here
6. **DEVELOPMENT.md** - Development setup and guidelines

---

## ⚡ Core Concept: Power Flow Orchestration

### The Problem This Solves

Imagine a house with:
- Solar panels producing 8 kW
- House consuming 3 kW
- Battery that can charge at 5 kW
- EV that wants to charge at 11 kW
- Main fuse limit of 10 kW

**Naive approach (WRONG):**
```rust
// This will trip the circuit breaker!
battery.charge(5000.0).await?;    // 5 kW
ev_charger.charge(11000.0).await?; // 11 kW
// Total demand: 3 + 5 + 11 = 19 kW from 8 kW solar
// Grid needs to import: 11 kW → EXCEEDS 10 kW FUSE! 💥
```

**Power flow orchestration (CORRECT):**
```rust
// 1. Gather all inputs
let inputs = PowerFlowInputs {
    pv_production_kw: 8.0,
    house_load_kw: 3.0,
    battery_soc_percent: 60.0,
    ev_state: connected_and_needs_charge,
};

// 2. Compute optimal flows respecting ALL constraints
let snapshot = power_flow_model.compute_flows(
    inputs,
    &constraints,  // Fuse limit: 10 kW, etc.
    &objectives,   // Minimize cost, etc.
).await?;

// 3. Verify safety
assert!(snapshot.verify_power_balance());
assert!(!snapshot.exceeds_fuse_limit(10.0));

// Result: House gets 3 kW, Battery gets 3 kW, EV gets 2 kW
// All from solar! Grid import: 0 kW ✅
```

### The Golden Rule

**NEVER implement energy-related features that bypass the PowerFlowModel.**

Every power command must flow through the power flow orchestration system.

---

## 🏗️ Architecture Quick Reference

### Key Components

```
┌─────────────────────────────────────────────┐
│  REST API (Axum)                            │
│  Port 8080, OpenAPI docs at /swagger-ui    │
└─────────────────────────────────────────────┘
                    ↓
┌─────────────────────────────────────────────┐
│  PowerFlowModel                             │
│  • Computes optimal power flows             │
│  • Enforces constraints (fuse, SoC, etc.)  │
│  • Returns PowerSnapshot                    │
└─────────────────────────────────────────────┘
                    ↓
┌─────────────────────────────────────────────┐
│  Hardware Abstraction Layer                 │
│  • SimulatedBattery/ModbusBattery          │
│  • OcppEvCharger                           │
│  • Trait-based, swappable                  │
└─────────────────────────────────────────────┘
                    ↓
┌─────────────────────────────────────────────┐
│  Physical Devices or Simulation             │
│  • Real: Modbus TCP, OCPP 1.6              │
│  • Sim: Realistic behavior models          │
└─────────────────────────────────────────────┘
```

### Directory Map

| Path | Purpose | When to modify |
|------|---------|---------------|
| `src/power_flow/` | **Core orchestration system** | Adding power flow logic, constraints |
| `src/domain/` | Domain models (Battery, EvCharger traits) | New device types, domain logic |
| `src/hardware/` | Hardware implementations (Modbus, OCPP, Simulated) | Hardware integration |
| `src/api/` | REST API handlers | New endpoints |
| `src/optimizer/` | Multi-hour optimization (DP, MILP) | Optimization algorithms |
| `src/forecast/` | Price, consumption, production forecasting | Forecasting logic |
| `src/controller/` | Real-time control loops | Control algorithms |
| `src/repo/` | Database repositories | Database queries |
| `src/simulation/` | Advanced simulation (thermal, 3-phase, etc.) | Simulation features |

---

## 🎓 Key Technical Patterns

### 1. Type-Safe Domain Types

**Don't use raw floats for physical quantities:**

```rust
// ❌ WRONG
fn set_power(watts: f64) { }  // No validation, unit confusion

// ✅ RIGHT
use crate::domain::types::Power;

fn set_power(power: Power) { }  // Type-safe, validated
```

Available types: `Power`, `Energy`, `Voltage`, `Current`, `Temperature`, `Percentage`, `Price`

### 2. Async Everything

This is an async Rust project using Tokio:

```rust
// ✅ Use Tokio async functions
#[tokio::test]
async fn test_battery() {
    let battery = SimulatedBattery::new(10.0);
    let state = battery.read_state().await?;
}

// ❌ Never block the executor
std::thread::sleep(Duration::from_secs(1));  // WRONG!
tokio::time::sleep(Duration::from_secs(1)).await;  // RIGHT!
```

### 3. Error Handling

```rust
// ❌ Never panic in production code
let data = modbus.read().unwrap();

// ✅ Return errors with context
let data = modbus.read()
    .map_err(|e| BatteryError::ModbusCommunication(e))?;
```

### 4. Trait-Based Abstraction

```rust
// Domain trait (in src/domain/battery/)
#[async_trait]
pub trait Battery: Send + Sync {
    async fn read_state(&self) -> Result<BatteryState>;
    async fn set_power(&self, power: Power) -> Result<()>;
}

// Implementations (in src/hardware/)
pub struct SimulatedBattery { /* ... */ }  // For dev/test
pub struct ModbusBattery { /* ... */ }     // For production

// Both implement the same trait!
```

---

## 🔧 Common Tasks & Quick Patterns

### Task: Add a New API Endpoint

**Files to touch:**
1. `src/api/[resource].rs` - Create handler function
2. `src/api/v1.rs` or `src/api/mod.rs` - Add route
3. `tests/integration/` - Add integration test

**Pattern:**
```rust
// src/api/battery.rs
#[utoipa::path(
    get,
    path = "/api/v1/battery/state",
    responses((status = 200, body = BatteryState))
)]
pub async fn get_battery_state(
    State(state): State<AppState>,
) -> Result<Json<BatteryState>, ApiError> {
    let battery_state = state.battery.read_state().await?;
    Ok(Json(battery_state))
}
```

### Task: Add a Database Table

**Steps:**
1. Create migration: `sqlx migrate add create_table_name`
2. Write SQL in `migrations/XXXXX_create_table_name.sql`
3. Create model struct in `src/database/models/`
4. Create repository in `src/repo/`
5. Run migration: `sqlx migrate run`

### Task: Implement a Hardware Device

**Files needed:**
1. `src/hardware/[type]/[device].rs` - Implementation
2. `src/hardware/factory.rs` - Wire into factory
3. `tests/integration/` - Integration tests

**Must implement the domain trait from `src/domain/`**

---

## ⚠️ Critical Rules (Never Violate)

### 1. Always Verify Power Balance

```rust
let snapshot = compute_power_flows(...)?;
assert!(snapshot.verify_power_balance(), "Power balance violated!");
```

Every `PowerSnapshot` must satisfy: Sources = Sinks

### 2. Never Exceed Physical Constraints

```rust
// Check fuse limit
assert!(snapshot.grid_import_kw <= constraints.max_grid_import_kw);

// Check battery limits
assert!(snapshot.battery_power_kw.abs() <= constraints.max_battery_charge_kw);
```

### 3. Always Update MASSIVE_TODO_LIST.md

When you complete a task, mark it as done:
```markdown
- [x] Implement SimulatedBattery  # Done!
- [ ] Implement ModbusBattery     # Next
```

### 4. No Orphaned Code

Every file you create must be:
- Imported in parent `mod.rs`
- Used somewhere in the codebase
- Tested with actual tests that run
- Documented with rustdoc comments

### 5. Test Everything

Minimum coverage requirements:
- Domain logic: 100%
- Service layer: >90%
- API handlers: >80%
- Overall: >80%

```bash
cargo test                    # Run all tests
cargo test --doc             # Run doc tests
cargo clippy -- -D warnings  # No warnings allowed
cargo fmt                    # Format code
```

---

## 🚦 Development Workflow

### Before Starting Any Task

```bash
# 1. Read the task in MASSIVE_TODO_LIST.md
# 2. Understand the architecture context
# 3. Search for similar existing code
rg "Battery" src/ --type rust
# 4. Plan what files you'll create/modify
# 5. Consider integration points
```

### While Implementing

```bash
# Run tests frequently
cargo test

# Check for warnings
cargo clippy --all-targets --all-features -- -D warnings

# Format code
cargo fmt
```

### Before Committing

```bash
# Mandatory checklist
cargo fmt                      # Format
cargo clippy -- -D warnings   # Lint
cargo test                    # Test
rg "println!|dbg!" src/       # Remove debug code
# Update MASSIVE_TODO_LIST.md
# Review git diff
```

**Commit message format:**
```
feat: implement simulated battery

- Created SimulatedBattery struct
- Implemented Battery trait with realistic physics
- Added comprehensive unit and integration tests
- Updated MASSIVE_TODO_LIST.md Phase 4 items 1-8

Tests: All passing, 95% coverage on new code
```

---

## 🎯 Project-Specific Concepts

### PowerSnapshot

The fundamental data structure representing system state:

```rust
pub struct PowerSnapshot {
    pub pv_kw: f64,              // Solar production
    pub house_load_kw: f64,      // House consumption
    pub battery_power_kw: f64,   // Battery (+ charge, - discharge)
    pub ev_power_kw: f64,        // EV charging
    pub grid_import_kw: f64,     // Grid import
    pub grid_export_kw: f64,     // Grid export
    pub timestamp: DateTime<Utc>,
}
```

Location: `src/power_flow/snapshot.rs`

### Constraints Hierarchy

Three tiers enforced in strict order:

1. **Physical** - Immutable reality (fuse limits, device capacity)
2. **Safety** - Operational safety (min battery SoC, house priority)
3. **Economic** - Cost optimization (prices, self-consumption)

Location: `src/power_flow/constraints.rs`

### Control Loop Timing

- **Power flow computation**: Every 10 seconds
- **Optimization recomputation**: Every 1-5 minutes
- **Forecast updates**: Every hour
- **Database persistence**: After each computation

---

## 📖 Understanding the Codebase

### Finding Things

```bash
# Find trait definitions
rg "pub trait" src/domain/

# Find implementations
rg "impl.*Battery" src/hardware/

# Find API endpoints
rg "#\[utoipa::path" src/api/

# Find tests
rg "#\[tokio::test\]" src/ tests/

# Find database queries
rg "sqlx::query" src/repo/
```

### Key Files to Know

| File | Critical? | Purpose |
|------|-----------|---------|
| `src/power_flow/model.rs` | ⭐⭐⭐ | Core power flow algorithm |
| `src/power_flow/snapshot.rs` | ⭐⭐⭐ | PowerSnapshot struct and validation |
| `src/domain/battery.rs` | ⭐⭐ | Battery trait definition |
| `src/hardware/factory.rs` | ⭐⭐ | Device instantiation |
| `src/main.rs` | ⭐ | Application entry point |
| `src/config.rs` | ⭐ | Configuration management |

### Dependencies You'll Use

- `tokio` - Async runtime
- `axum` - Web framework
- `sqlx` - Database (async PostgreSQL)
- `serde` - Serialization
- `anyhow`/`thiserror` - Error handling
- `tracing` - Logging
- `tokio-modbus` - Modbus communication
- `async-trait` - Async traits

---

## 🧪 Testing Philosophy

### Test Pyramid

```
          /\
         /  \  E2E (simulation-based scenarios)
        /____\
       /      \  Integration (API + DB + services)
      /________\
     /          \  Unit (domain logic, pure functions)
    /____________\
```

### Test Categories

1. **Unit tests** - In same file as code, under `#[cfg(test)] mod tests`
2. **Integration tests** - In `tests/integration/`
3. **E2E tests** - In `tests/e2e/`, use full simulation environment

### Example: Testing Power Flow

```rust
#[tokio::test]
async fn test_fuse_protection_under_high_demand() {
    // Setup
    let model = PowerFlowModel::new();
    let constraints = PhysicalConstraints {
        max_grid_import_kw: 10.0,  // 10 kW fuse
        // ...
    };

    // Scenario: Low solar, high demand
    let inputs = PowerFlowInputs {
        pv_production_kw: 2.0,
        house_load_kw: 8.0,
        ev_state: Some(EvState::needs_11kw_charge()),
        // ...
    };

    // Execute
    let snapshot = model.compute_flows(inputs, &constraints, &objectives).await?;

    // Verify
    assert!(snapshot.verify_power_balance());
    assert!(snapshot.grid_import_kw <= 10.0, "Fuse limit exceeded!");
    assert_eq!(snapshot.house_load_kw, 8.0, "House must be prioritized");
    // EV gets remaining: 10 - 8 = 2 kW max
    assert!(snapshot.ev_power_kw <= 2.0);
}
```

---

## 🔍 Debugging Tips

### When Tests Fail

```bash
# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_battery_charging

# Run with logging
RUST_LOG=debug cargo test

# Run single-threaded (easier to read output)
cargo test -- --test-threads=1
```

### When Compilation Fails

```bash
# Faster feedback than full build
cargo check

# Detailed error explanations
cargo check --message-format=human --color=always

# See macro expansions (for debugging macros)
cargo expand
```

### When Clippy Complains

```bash
# See all lints with explanations
cargo clippy --all-targets --all-features -- -D warnings

# Explain specific lint
rustc --explain E0308
```

---

## 🌟 Best Practices Specific to This Project

### 1. Device Communication

Always implement retry logic for hardware communication:

```rust
pub async fn read_with_retry(&self) -> Result<Data> {
    let mut attempts = 0;
    loop {
        match self.read().await {
            Ok(data) => return Ok(data),
            Err(e) if attempts < 3 => {
                attempts += 1;
                tracing::warn!(?e, attempt = attempts, "Retrying read");
                tokio::time::sleep(Duration::from_millis(100 * attempts)).await;
            }
            Err(e) => return Err(e),
        }
    }
}
```

### 2. Logging and Metrics

Always add both:

```rust
pub async fn optimize(&self) -> Schedule {
    tracing::info!("Starting optimization");
    let start = Instant::now();

    let schedule = self.run_optimizer().await;

    let duration = start.elapsed();
    tracing::info!(?duration, "Optimization complete");

    // Prometheus metrics
    metrics::histogram!("optimization_duration_seconds")
        .record(duration.as_secs_f64());

    schedule
}
```

### 3. Configuration Over Hardcoding

```rust
// ❌ WRONG - hardcoded value
const MAX_POWER: f64 = 5000.0;

// ✅ RIGHT - from config
pub struct BatteryConfig {
    pub max_charge_power_kw: f64,
}

let max_power = config.battery.max_charge_power_kw;
```

### 4. Database Transactions

Use transactions for multi-step operations:

```rust
pub async fn create_schedule(&self, schedule: &Schedule) -> Result<Uuid> {
    let mut tx = self.pool.begin().await?;

    let schedule_id = sqlx::query_scalar!(
        "INSERT INTO schedules (...) VALUES (...) RETURNING id",
        // ...
    )
    .fetch_one(&mut *tx)
    .await?;

    // Insert related rows
    for slot in &schedule.slots {
        sqlx::query!(/* ... */)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;
    Ok(schedule_id)
}
```

---

## 🚨 Common Mistakes to Avoid

### ❌ Mistake: Independent Device Control

```rust
// DANGEROUS - no coordination!
battery.set_power(5000.0).await?;
ev_charger.set_power(11000.0).await?;
```

### ✅ Correct: Power Flow Orchestration

```rust
let snapshot = power_flow_model.compute_flows(...).await?;
battery.set_power(snapshot.battery_power_kw).await?;
ev_charger.set_power(snapshot.ev_power_kw).await?;
```

---

### ❌ Mistake: Missing Integration

Creating `src/hardware/simulated/battery.rs` but forgetting to:
- Add `pub mod battery;` to `src/hardware/simulated/mod.rs`
- Add it to the factory in `src/hardware/factory.rs`
- Write integration tests

### ✅ Correct: Full Integration

Complete all integration points before marking task done.

---

### ❌ Mistake: No Error Context

```rust
let state = battery.read_state().await?;  // What went wrong?
```

### ✅ Correct: Contextual Errors

```rust
let state = battery.read_state().await
    .map_err(|e| {
        tracing::error!(?e, "Failed to read battery state");
        metrics::counter!("battery_read_errors_total").increment(1);
        e
    })?;
```

---

## 📚 Additional Resources

### Project Documentation

- **POWER_FLOW_ARCHITECTURE.md** - Deep dive on power flow algorithm
- **AGENTS.md** - Complete coding standards and rules
- **DEVELOPMENT.md** - Development setup and guidelines
- **MASSIVE_TODO_LIST.md** - All tasks organized by phase
- **docs/ADR/** - Architecture Decision Records
- **docs/ARCHITECTURE.md** - System architecture overview

### External Resources

- [Rust Book](https://doc.rust-lang.org/book/)
- [Async Book](https://rust-lang.github.io/async-book/)
- [Tokio Tutorial](https://tokio.rs/tokio/tutorial)
- [Axum Documentation](https://docs.rs/axum/)
- [SQLx Guide](https://github.com/launchbadge/sqlx)

---

## 🎯 Quick Command Reference

```bash
# Development
cargo run                     # Run application
cargo test                    # Run all tests
cargo clippy                  # Lint
cargo fmt                     # Format

# Database
sqlx migrate run             # Apply migrations
sqlx migrate add <name>      # Create migration
psql $DATABASE_URL           # Connect to DB

# Specific builds
cargo build --release                          # Production build
cargo build --features hardware                # With hardware
cargo test --features hardware                 # Hardware tests

# Code exploration
rg "pattern" src/                              # Search code
fd "filename"                                  # Find files
cargo tree                                     # Dependency tree
cargo doc --open                               # Generate & view docs
```

---

## 🎓 When You're Stuck

1. **Search the codebase** - Someone likely solved a similar problem
   ```bash
   rg "similar_pattern" src/
   ```

2. **Check tests** - They show how code is meant to be used
   ```bash
   rg "test.*battery" tests/
   ```

3. **Read architecture docs** - Understand the design intent
   - POWER_FLOW_ARCHITECTURE.md
   - docs/ADR/

4. **Check MASSIVE_TODO_LIST.md** - For context on related tasks

5. **Ask for clarification** - Document your question and assumptions

---

## ✅ Success Checklist

You're doing excellent work if:

- ✅ All tests pass (`cargo test`)
- ✅ No clippy warnings (`cargo clippy -- -D warnings`)
- ✅ Code is formatted (`cargo fmt`)
- ✅ Power balance is always verified for power flow code
- ✅ MASSIVE_TODO_LIST.md is updated
- ✅ New code is fully integrated (no orphans)
- ✅ Public APIs have documentation
- ✅ Error handling is robust (no unwrap/expect in prod code)
- ✅ Tests cover new functionality (>80% coverage)
- ✅ Commits have clear messages explaining what and why

---

## 🎯 Your Mission

Build a **reliable, safe, and efficient** energy management system that:

1. **Never violates physical constraints** (fuse limits, device capacity)
2. **Always prioritizes safety** (house power, minimum battery SoC)
3. **Optimizes economics** within safety bounds (minimize costs)
4. **Handles errors gracefully** (retry logic, fallbacks, logging)
5. **Maintains production quality** (tested, documented, monitored)

---

## 🚀 Ready to Start?

**Your typical workflow:**

1. Pick a task from **MASSIVE_TODO_LIST.md**
2. Read related code and documentation
3. Plan your implementation (files to create/modify)
4. Implement with tests
5. Run `cargo fmt && cargo clippy && cargo test`
6. Integrate into existing systems
7. Update MASSIVE_TODO_LIST.md
8. Commit with descriptive message

**Remember:** Quality over quantity. One perfect feature is worth more than ten incomplete ones.

---

**Welcome to the team! Let's build something amazing! ⚡🔋**
