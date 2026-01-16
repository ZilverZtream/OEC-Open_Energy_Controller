# 🤖 Instructions for AI Coding Agents

**READ THIS ENTIRE FILE BEFORE MAKING ANY CODE CHANGES**

This document contains critical instructions for AI coding agents (Claude Code, Aider, GitHub Copilot, etc.) working on this project. Following these rules is **MANDATORY** to maintain code quality and prevent common AI coding mistakes.

---

## 🎯 Your Mission

You are working on **Open Energy Controller**, a production-ready Rust system for battery management and optimization. Your goal is to:

1. **Follow the TODO list** in `MASSIVE_TODO_LIST.md`
2. **Write production-quality code** that is tested, documented, and integrated
3. **Never leave orphaned code** - always hook everything up
4. **Always clean up** after yourself - no dead code, no commented cruft
5. **Update documentation** as you make changes

---

### Rule #11: Understand Power Flow Orchestration

**⚡ THE MOST IMPORTANT ARCHITECTURAL PATTERN IN THIS PROJECT**

**❌ WRONG - Independent device control:**
```rust
// Naive approach - dangerous!
battery.set_power(3000.0).await?;   // Charge battery
ev_charger.set_power(11000.0).await?; // Charge EV
// ⚠️ Problem: Total = 14 kW, but fuse limit is 10 kW!
// ⚠️ Result: Fuse trips, house loses power!
```

**✅ RIGHT - Holistic power flow orchestration:**
```rust
// 1. Measure everything
let inputs = PowerFlowInputs {
    pv_production_kw: solar.read_power().await?,
    house_load_kw: meter.read_load().await?,
    battery_soc_percent: battery.read_state().await?.soc_percent,
    ev_state: ev_charger.read_state().await?,
};

// 2. Compute optimal flows (respects ALL constraints)
let snapshot = power_flow_model.compute_flows(
    inputs,
    &constraints,  // Fuse limit, min SoC, etc.
    &objectives,   // Prices, self-consumption, etc.
).await?;

// 3. Verify safety
assert!(snapshot.verify_power_balance());
assert!(!snapshot.exceeds_fuse_limit(10.0));

// 4. Issue commands
battery.set_power(snapshot.battery_power_kw).await?;
ev_charger.set_power(snapshot.ev_power_kw).await?;
// ✅ Now all constraints are checked!
// ✅ Power flows are optimized holistically!
```

**Why this matters:**

1. **Physics:** Power must balance (sources = sinks)
2. **Safety:** Fuse limits are HARD constraints
3. **Priority:** House > EV > Battery > Export
4. **Economics:** Optimize within constraints

**Always think about power flow:**
- Where does solar power go? (House → EV → Battery → Grid)
- What if house needs more power than available?
- What if fuse limit would be exceeded?
- What if battery is full?
- What if EV needs to charge but grid is expensive?

**When implementing ANY energy-related feature:**
- [ ] How does it affect the PowerSnapshot?
- [ ] Does it respect physical constraints?
- [ ] Does it respect safety constraints?
- [ ] Is it integrated into PowerFlowModel?
- [ ] Is the power balance verified?

**📖 Read [POWER_FLOW_ARCHITECTURE.md](POWER_FLOW_ARCHITECTURE.md) to deeply understand this!**

---

## ⚠️ CRITICAL RULES - NEVER VIOLATE THESE

### Rule #1: Always Update the TODO List

**BEFORE you finish ANY task:**
```bash
# 1. Open MASSIVE_TODO_LIST.md
# 2. Find the checkbox for what you just did
# 3. Mark it as complete: - [x] Task description
# 4. Commit the updated TODO list with your code changes
```

**Example commit:**
```
feat: implement simulated battery

- Added SimulatedBattery struct
- Implemented Battery trait
- Added unit tests
- Updated MASSIVE_TODO_LIST.md Phase 4 items 1-8
```

### Rule #2: Never Create Orphaned Code

**❌ BAD - Orphaned code:**
```rust
// You created src/hardware/simulated/battery.rs
// But it's not imported anywhere!
// Nobody can use it!
```

**✅ GOOD - Integrated code:**
```rust
// 1. Created src/hardware/simulated/battery.rs
// 2. Added to src/hardware/simulated/mod.rs:
pub mod battery;
pub use battery::SimulatedBattery;

// 3. Used in src/hardware/factory.rs:
pub fn create_battery(config: &Config) -> Arc<dyn Battery> {
    if config.hardware_mode == "simulated" {
        Arc::new(SimulatedBattery::new(config.battery_capacity))
    } else {
        // ...
    }
}

// 4. Tested in tests/integration/battery_test.rs
```

**ALWAYS:**
- Import new modules in parent `mod.rs`
- Export public items with `pub use`
- Wire new code into existing systems
- Add tests that actually call the new code
- Update configuration if needed
- Update API routes if it's an endpoint

### Rule #3: Remove ALL Dead Code

**Before committing, search for:**
```bash
# Find commented-out code
rg "^\s*//" src/

# Find unused imports
cargo clippy -- -W unused-imports

# Find unused functions
cargo clippy -- -W dead-code

# Find TODO/FIXME comments (address or document)
rg "TODO|FIXME" src/
```

**Delete:**
- Commented-out code (use git history if needed)
- Unused imports
- Unused functions/structs
- Old implementations replaced by new ones
- Debug print statements (`println!`, `dbg!`)

**Keep only if:**
- It's a temporary TODO with a ticket reference
- It's explanatory comment (not code)
- It's actually used somewhere

### Rule #4: Always Run Tests

**After EVERY change:**
```bash
# 1. Format code
cargo fmt

# 2. Run clippy (fix ALL warnings)
cargo clippy --all-targets --all-features -- -D warnings

# 3. Run tests
cargo test

# 4. If you added a feature, test that feature:
cargo test --features hardware
cargo test --features ml
```

**If tests fail:**
- Fix them before committing
- If a test is outdated, update it
- Never skip tests or comment them out

### Rule #5: Write Tests for Everything

**For every new function/struct/module:**

```rust
// src/domain/battery.rs
pub struct BatteryState {
    pub soc_percent: f64,
    // ...
}

impl BatteryState {
    pub fn is_low(&self) -> bool {
        self.soc_percent < 20.0
    }
}

// IMMEDIATELY ADD TEST:
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_low() {
        let state = BatteryState { soc_percent: 15.0 };
        assert!(state.is_low());
        
        let state = BatteryState { soc_percent: 50.0 };
        assert!(!state.is_low());
    }
}
```

**Test coverage requirements:**
- Domain logic: 100%
- Service layer: >90%
- API handlers: >80%
- Overall: >80%

### Rule #6: Document Public APIs

**Every public item needs documentation:**

```rust
/// Represents the current state of a battery system.
///
/// # Fields
/// - `soc_percent`: State of charge (0-100%)
/// - `power_w`: Current power flow in watts (+ = charging, - = discharging)
///
/// # Example
/// ```
/// let state = BatteryState {
///     soc_percent: 75.0,
///     power_w: 2500.0,
/// };
/// assert!(state.is_charging());
/// ```
pub struct BatteryState {
    pub soc_percent: f64,
    pub power_w: f64,
}
```

**Document:**
- All `pub struct` and `pub enum`
- All `pub fn`
- All `pub trait`
- Complex private functions
- Non-obvious algorithms

### Rule #7: Use Type-Safe Abstractions

**❌ BAD - Primitive obsession:**
```rust
fn set_power(battery: &mut Battery, watts: f64) {
    // What if someone passes -99999999?
    // What if they pass watts but meant kilowatts?
}
```

**✅ GOOD - Type-safe:**
```rust
pub struct Power(f64); // Watts

impl Power {
    pub fn from_watts(w: f64) -> Result<Self> {
        if w.is_finite() {
            Ok(Power(w))
        } else {
            Err(Error::InvalidPower)
        }
    }
    
    pub fn watts(&self) -> f64 { self.0 }
}

fn set_power(battery: &mut Battery, power: Power) {
    // Type-safe, validated at construction
}
```

**Use domain types from `src/domain/types.rs`:**
- `Power` (Watts)
- `Energy` (Watt-hours)
- `Voltage` (Volts)
- `Current` (Amperes)
- `Temperature` (Celsius)
- `Percentage` (0-100)
- `Price` (SEK/kWh)

### Rule #8: Handle Errors Properly

**❌ BAD:**
```rust
fn read_battery() -> BatteryState {
    let data = modbus_read().unwrap(); // PANIC on error!
    parse(data).unwrap()
}
```

**✅ GOOD:**
```rust
fn read_battery() -> Result<BatteryState, BatteryError> {
    let data = modbus_read()
        .map_err(|e| BatteryError::ModbusError(e))?;
    
    parse(data)
        .map_err(|e| BatteryError::ParseError(e))
}
```

**Error handling rules:**
- Use `Result<T, E>` for recoverable errors
- Use `anyhow::Result` for application errors
- Use custom error types for domain errors (with `thiserror`)
- Never use `.unwrap()` in production code (only in tests)
- Never use `.expect()` except with impossible conditions
- Log errors with `tracing::error!`
- Return errors to caller

### Rule #9: Use Async Properly

**This is an async Rust project using Tokio:**

```rust
// ✅ GOOD
#[tokio::test]
async fn test_read_battery() {
    let battery = SimulatedBattery::new(10.0);
    let state = battery.read_state().await.unwrap();
    assert_eq!(state.soc_percent, 50.0);
}

// ❌ BAD - blocking in async context
async fn some_handler() {
    std::thread::sleep(Duration::from_secs(1)); // BLOCKS the executor!
}

// ✅ GOOD - async sleep
async fn some_handler() {
    tokio::time::sleep(Duration::from_secs(1)).await;
}
```

**Rules:**
- All I/O must be async
- Use `tokio::time::sleep`, not `std::thread::sleep`
- Use `tokio::fs`, not `std::fs`
- Use `tokio::spawn` for concurrent tasks
- Use `Arc<RwLock<T>>` for shared mutable state
- Never block the async runtime

### Rule #10: Follow Project Structure

**ALWAYS:**
- Put domain logic in `src/domain/`
- Put hardware implementations in `src/hardware/`
- Put API handlers in `src/api/handlers/`
- Put tests next to the code (or in `tests/`)
- Put config in `config/`
- Put docs in `docs/`

**Module organization:**
```
src/domain/battery/
├── mod.rs          # Re-exports
├── traits.rs       # Battery trait
├── state.rs        # BatteryState struct
├── commands.rs     # BatteryCommand enum
└── errors.rs       # BatteryError enum
```

Each file should be <500 lines. Split large files into modules.

---

## 📋 Before Starting Any Task

**CHECKLIST:**
```
[ ] Read MASSIVE_TODO_LIST.md and find your task
[ ] Read related ADRs in docs/ADR/
[ ] Understand where your code fits in the architecture
[ ] Check if related code already exists (use `rg` to search)
[ ] Plan your implementation (interfaces, types, tests)
[ ] Create a feature branch: git checkout -b feature/task-name
```

---

## 🔧 During Implementation

### Step 1: Create Types/Traits
```rust
// Start with domain types and traits
// These define the "what" before the "how"

pub trait Battery: Send + Sync {
    async fn read_state(&self) -> Result<BatteryState>;
    async fn set_power(&self, watts: Power) -> Result<()>;
}
```

### Step 2: Implement
```rust
// Implement the trait
pub struct SimulatedBattery { /* ... */ }

#[async_trait]
impl Battery for SimulatedBattery {
    async fn read_state(&self) -> Result<BatteryState> {
        // Implementation
    }
}
```

### Step 3: Write Tests
```rust
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_simulated_battery() {
        // Test the implementation
    }
}
```

### Step 4: Integrate
```rust
// Add to mod.rs
pub mod simulated;
pub use simulated::SimulatedBattery;

// Use in factory
pub fn create_battery(...) -> Arc<dyn Battery> {
    Arc::new(SimulatedBattery::new(...))
}
```

### Step 5: Document
```rust
/// A simulated battery for testing and development.
///
/// This battery simulates realistic behavior including:
/// - State of charge changes
/// - Temperature effects
/// - Efficiency losses
pub struct SimulatedBattery { /* ... */ }
```

---

## ✅ Before Committing

**MANDATORY CHECKLIST:**
```bash
[ ] cargo fmt                          # Format code
[ ] cargo clippy -- -D warnings        # No warnings allowed
[ ] cargo test                         # All tests pass
[ ] rg "TODO|FIXME" src/              # Review todos
[ ] rg "println!|dbg!" src/           # Remove debug prints
[ ] git diff                          # Review changes
[ ] Update MASSIVE_TODO_LIST.md       # Check off completed items
[ ] Update docs if API changed        # Keep docs current
```

**Commit message format:**
```
<type>: <description>

<body explaining what and why>

Updated MASSIVE_TODO_LIST.md: Phase X items Y-Z
```

Types: `feat`, `fix`, `docs`, `test`, `refactor`, `perf`, `chore`

---

## 🚫 COMMON MISTAKES - AVOID THESE

### Mistake #1: Creating Files Without Integration

**You did:**
```
Created src/api/handlers/battery.rs
```

**But forgot:**
- Adding `pub mod battery;` to `src/api/handlers/mod.rs`
- Adding route in `src/api/routes.rs`
- Adding handler to `AppState`
- Adding integration test

**Result:** The code exists but is never called!

### Mistake #2: Incomplete Error Handling

**You did:**
```rust
let state = battery.read_state().await?;
```

**But forgot:**
- What if battery is disconnected?
- What if Modbus times out?
- What if data is corrupted?
- Add logging: `tracing::error!("Failed to read battery: {}", e)`
- Add metrics: `metrics::counter!("battery_read_errors_total").increment(1)`

### Mistake #3: Not Testing Integration

**You did:**
- Created `SimulatedBattery`
- Wrote unit tests for `SimulatedBattery`

**But forgot:**
- Does `BatteryController` work with it?
- Does the API endpoint work?
- Does it persist to database correctly?
- Add integration test that exercises the full stack!

### Mistake #4: Hardcoded Values

**❌ BAD:**
```rust
const MAX_POWER: f64 = 5000.0;  // Hardcoded!
```

**✅ GOOD:**
```rust
// In config
pub struct BatteryConfig {
    pub max_charge_power_w: f64,
}

// Use from config
let max_power = config.battery.max_charge_power_w;
```

### Mistake #5: Missing Validation

**❌ BAD:**
```rust
pub fn set_soc(soc: f64) {
    self.soc = soc;  // What if soc is 150% or -20%?
}
```

**✅ GOOD:**
```rust
pub fn set_soc(soc: f64) -> Result<()> {
    if soc < 0.0 || soc > 100.0 {
        return Err(Error::InvalidSoC(soc));
    }
    self.soc = soc;
    Ok(())
}
```

### Mistake #6: No Logging

**❌ BAD:**
```rust
async fn optimize(&self) -> Schedule {
    let schedule = self.run_dp();
    schedule
}
```

**✅ GOOD:**
```rust
async fn optimize(&self) -> Schedule {
    tracing::info!("Starting optimization");
    let start = Instant::now();
    
    let schedule = self.run_dp();
    
    let duration = start.elapsed();
    tracing::info!(?duration, "Optimization complete");
    metrics::histogram!("optimization_duration_seconds")
        .record(duration.as_secs_f64());
    
    schedule
}
```

---

### When Implementing Power Flow Features

**This is the CORE of the system. Take extra care!**

1. **Create Power Flow structures:**
```rust
// src/power_flow/snapshot.rs
pub struct PowerSnapshot {
    pub pv_kw: f64,
    pub house_load_kw: f64,
    pub battery_power_kw: f64,
    pub ev_power_kw: f64,
    pub grid_import_kw: f64,
    pub grid_export_kw: f64,
    pub timestamp: DateTime<Utc>,
}

impl PowerSnapshot {
    /// CRITICAL: Verify power balance
    pub fn verify_power_balance(&self) -> bool {
        let sources = self.pv_kw + self.grid_import_kw;
        let sinks = self.house_load_kw 
            + self.ev_power_kw 
            + self.battery_power_kw.max(0.0) 
            + self.grid_export_kw;
        (sources - sinks).abs() < 0.01
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_power_balance() {
        let snapshot = PowerSnapshot {
            pv_kw: 5.0,
            house_load_kw: 2.0,
            battery_power_kw: 2.0,
            ev_power_kw: 1.0,
            grid_import_kw: 0.0,
            grid_export_kw: 0.0,
        };
        assert!(snapshot.verify_power_balance());
    }
}
```

2. **Implement constraint checking:**
```rust
// src/power_flow/constraints.rs
pub struct PhysicalConstraints {
    pub max_grid_import_kw: f64,
    pub max_battery_charge_kw: f64,
    pub evse_max_current_a: f64,
}

impl PhysicalConstraints {
    pub fn check_snapshot(&self, snapshot: &PowerSnapshot) -> Result<()> {
        if snapshot.grid_import_kw > self.max_grid_import_kw {
            return Err(Error::FuseLimitExceeded {
                actual: snapshot.grid_import_kw,
                limit: self.max_grid_import_kw,
            });
        }
        // ... more checks
        Ok(())
    }
}
```

3. **Implement the core algorithm:**
```rust
// src/power_flow/model.rs
pub struct PowerFlowModel {
    // ... fields
}

impl PowerFlowModel {
    pub async fn compute_flows(
        &self,
        inputs: PowerFlowInputs,
        constraints: &AllConstraints,
        objectives: &EconomicObjectives,
    ) -> Result<PowerSnapshot> {
        // STEP 1: House load priority (non-negotiable)
        let house_load_kw = inputs.house_load_kw;
        let mut available_power = inputs.pv_production_kw;
        let mut grid_import = 0.0;
        
        if house_load_kw > available_power {
            grid_import = house_load_kw - available_power;
            available_power = 0.0;
        } else {
            available_power -= house_load_kw;
        }
        
        // STEP 2: EV charging (with urgency calculation)
        let mut ev_power_kw = 0.0;
        if let Some(ev_state) = inputs.ev_state {
            let urgency = self.calculate_ev_urgency(&ev_state, objectives);
            let desired_ev_power = self.calculate_desired_ev_power(urgency, objectives);
            
            // Check fuse limit!
            let potential_total = grid_import + desired_ev_power;
            if potential_total > constraints.physical.max_grid_import_kw {
                // Reduce EV power to fit within fuse
                ev_power_kw = constraints.physical.max_grid_import_kw - grid_import;
            } else {
                ev_power_kw = desired_ev_power;
            }
            
            if ev_power_kw > available_power {
                grid_import += ev_power_kw - available_power;
                available_power = 0.0;
            } else {
                available_power -= ev_power_kw;
            }
        }
        
        // STEP 3: Battery (arbitrage + excess PV)
        let mut battery_power_kw = 0.0;
        // ... implement battery logic
        
        // STEP 4: Grid export
        let mut grid_export_kw = 0.0;
        if available_power > 0.01 {
            grid_export_kw = available_power.min(constraints.physical.max_grid_export_kw);
        }
        
        // STEP 5: Create and verify snapshot
        let snapshot = PowerSnapshot {
            pv_kw: inputs.pv_production_kw,
            house_load_kw,
            battery_power_kw,
            ev_power_kw,
            grid_import_kw: grid_import,
            grid_export_kw,
            timestamp: Utc::now(),
        };
        
        // CRITICAL: Verify!
        if !snapshot.verify_power_balance() {
            return Err(Error::PowerBalanceViolation(snapshot));
        }
        
        constraints.physical.check_snapshot(&snapshot)?;
        
        Ok(snapshot)
    }
}
```

4. **Add comprehensive tests:**
```rust
#[tokio::test]
async fn test_fuse_protection() {
    let model = PowerFlowModel::new();
    let constraints = AllConstraints {
        physical: PhysicalConstraints {
            max_grid_import_kw: 10.0, // 10 kW fuse
            // ...
        },
        // ...
    };
    
    let inputs = PowerFlowInputs {
        pv_production_kw: 0.0, // No solar
        house_load_kw: 8.0,    // 8 kW load
        ev_state: Some(EvState {
            connected: true,
            needs_charge: true,
            max_charge_kw: 11.0, // Wants 11 kW
            // ...
        }),
        // ...
    };
    
    let snapshot = model.compute_flows(inputs, &constraints, &objectives).await.unwrap();
    
    // Assert fuse protected
    assert!(snapshot.grid_import_kw <= 10.0);
    
    // House gets priority
    assert_eq!(snapshot.house_load_kw, 8.0);
    
    // EV gets what's left
    assert_eq!(snapshot.ev_power_kw, 2.0); // Only 2 kW available within fuse
}
```

5. **Integrate with controller:**
```rust
// src/controller/power_flow_controller.rs
pub struct PowerFlowController {
    power_flow_model: PowerFlowModel,
    battery: Arc<dyn Battery>,
    ev_charger: Arc<dyn EvCharger>,
    // ...
}

impl PowerFlowController {
    pub async fn run(&self) -> Result<()> {
        let mut interval = tokio::time::interval(Duration::from_secs(10));
        
        loop {
            interval.tick().await;
            
            // 1. Read all sensors
            let inputs = self.gather_inputs().await?;
            
            // 2. Compute flows
            let snapshot = self.power_flow_model
                .compute_flows(inputs, &self.constraints, &self.objectives)
                .await?;
            
            // 3. Issue commands
            self.battery.set_power(Power::from_kw(snapshot.battery_power_kw)?).await?;
            self.ev_charger.set_power(Power::from_kw(snapshot.ev_power_kw)?).await?;
            
            // 4. Log snapshot
            self.log_snapshot(&snapshot).await?;
            
            // 5. Update metrics
            self.update_metrics(&snapshot);
        }
    }
}
```

6. **Add database persistence:**
```sql
-- migrations/XXXXXX_create_power_flow_snapshots.sql
CREATE TABLE power_flow_snapshots (
    id BIGSERIAL PRIMARY KEY,
    timestamp TIMESTAMPTZ NOT NULL,
    pv_kw DOUBLE PRECISION NOT NULL,
    house_load_kw DOUBLE PRECISION NOT NULL,
    battery_power_kw DOUBLE PRECISION NOT NULL,
    ev_power_kw DOUBLE PRECISION NOT NULL,
    grid_import_kw DOUBLE PRECISION NOT NULL,
    grid_export_kw DOUBLE PRECISION NOT NULL,
    INDEX idx_snapshots_timestamp (timestamp DESC)
);
```

7. **Add API endpoint:**
```rust
// src/api/handlers/power_flow.rs
#[utoipa::path(
    get,
    path = "/api/v1/power-flow/current",
    responses(
        (status = 200, description = "Current power flow", body = PowerSnapshot)
    )
)]
pub async fn get_current_power_flow(
    State(state): State<AppState>,
) -> Result<Json<PowerSnapshot>, ApiError> {
    let snapshot = state.power_flow_repo
        .get_latest_snapshot()
        .await?
        .ok_or(ApiError::NotFound)?;
    Ok(Json(snapshot))
}
```

8. **Test end-to-end:**
```rust
#[tokio::test]
async fn test_power_flow_end_to_end() {
    // Setup complete simulation
    let env = SimulationEnvironment::new();
    let controller = PowerFlowController::new(env);
    
    // Run for 1 simulated day
    for _ in 0..8640 { // 24 hours at 10s intervals
        controller.step().await.unwrap();
    }
    
    // Verify no fuse trips
    assert_eq!(env.fuse_trips, 0);
    
    // Verify power balance always held
    assert!(env.all_snapshots_balanced());
}
```

**Remember:**
- Power flow is THE core algorithm
- Every power-related feature must integrate here
- Always verify power balance
- Always check constraints
- Test extensively (fuse protection is critical!)
- Document assumptions clearly

---

## 🎯 Task-Specific Guidelines

### When Implementing API Endpoints

1. **Create handler function:**
```rust
// src/api/handlers/battery.rs
pub async fn get_battery_state(
    State(state): State<AppState>,
) -> Result<Json<BatteryState>, ApiError> {
    let battery_state = state.battery.read_state().await?;
    Ok(Json(battery_state))
}
```

2. **Add OpenAPI documentation:**
```rust
#[utoipa::path(
    get,
    path = "/api/v1/battery/state",
    responses(
        (status = 200, description = "Battery state retrieved", body = BatteryState),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_battery_state(...) { /* ... */ }
```

3. **Add route:**
```rust
// src/api/routes.rs
pub fn battery_routes() -> Router<AppState> {
    Router::new()
        .route("/state", get(handlers::battery::get_battery_state))
}

// In main router
.nest("/api/v1/battery", battery_routes())
```

4. **Add integration test:**
```rust
// tests/integration/api_battery_test.rs
#[tokio::test]
async fn test_get_battery_state() {
    let app = create_test_app().await;
    let response = app
        .get("/api/v1/battery/state")
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    let state: BatteryState = response.json().await.unwrap();
    assert!(state.soc_percent >= 0.0 && state.soc_percent <= 100.0);
}
```

5. **Test with curl:**
```bash
curl http://localhost:8080/api/v1/battery/state
```

6. **Update TODO list:**
```markdown
- [x] Implement `GET /api/v1/battery/state` handler
- [x] Add OpenAPI documentation
- [x] Add integration tests
```

### When Implementing Database Operations

1. **Create migration:**
```sql
-- migrations/YYYYMMDDHHMMSS_create_batteries.sql
CREATE TABLE batteries (
    id UUID PRIMARY KEY,
    household_id UUID NOT NULL,
    capacity_kwh DOUBLE PRECISION NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

2. **Create model:**
```rust
// src/database/models/battery.rs
#[derive(sqlx::FromRow)]
pub struct BatteryRow {
    pub id: Uuid,
    pub household_id: Uuid,
    pub capacity_kwh: f64,
    pub created_at: DateTime<Utc>,
}
```

3. **Create repository:**
```rust
// src/database/repositories/battery.rs
pub struct BatteryRepository {
    pool: PgPool,
}

impl BatteryRepository {
    pub async fn insert(&self, battery: &Battery) -> Result<Uuid> {
        let id = sqlx::query_scalar!(
            "INSERT INTO batteries (id, household_id, capacity_kwh) 
             VALUES ($1, $2, $3) RETURNING id",
            Uuid::new_v4(),
            battery.household_id,
            battery.capacity_kwh,
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(id)
    }
}
```

4. **Add tests:**
```rust
#[sqlx::test]
async fn test_insert_battery(pool: PgPool) {
    let repo = BatteryRepository::new(pool);
    let battery = Battery { /* ... */ };
    let id = repo.insert(&battery).await.unwrap();
    assert_ne!(id, Uuid::nil());
}
```

5. **Run migration:**
```bash
sqlx migrate run
```

### When Implementing Modbus Integration

1. **Define register map:**
```rust
// src/modbus/register_map.rs
pub struct HuaweiLuna2000RegisterMap {
    pub soc: u16,              // 37760
    pub power: u16,            // 37765
    pub voltage: u16,          // 37766
    pub current: u16,          // 37767
    pub temperature: u16,      // 37768
}
```

2. **Implement Modbus reads:**
```rust
pub async fn read_soc(&self) -> Result<f64> {
    let registers = self.client
        .read_holding_registers(self.register_map.soc, 1)
        .await?;
    
    let raw_soc = registers[0];
    let soc = (raw_soc as f64) / 100.0;  // Scale: 0.01%
    Ok(soc)
}
```

3. **Add error handling:**
```rust
.await
.map_err(|e| {
    tracing::error!(?e, "Modbus read failed");
    BatteryError::ModbusCommunication(e)
})?
```

4. **Add retry logic:**
```rust
pub async fn read_soc_with_retry(&self) -> Result<f64> {
    let mut attempts = 0;
    loop {
        match self.read_soc().await {
            Ok(soc) => return Ok(soc),
            Err(e) if attempts < 3 => {
                attempts += 1;
                tracing::warn!(?e, attempt = attempts, "Retrying Modbus read");
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            Err(e) => return Err(e),
        }
    }
}
```

5. **Test with mock:**
```rust
#[tokio::test]
async fn test_modbus_battery() {
    let mock_server = MockModbusServer::start("127.0.0.1:15020").await;
    mock_server.set_register(37760, 5000).await; // 50.00%
    
    let battery = ModbusBattery::connect("127.0.0.1:15020", 1).await.unwrap();
    let state = battery.read_state().await.unwrap();
    
    assert_eq!(state.soc_percent, 50.0);
}
```

---

## 🔍 Code Review Checklist

Before marking a task as complete, review your code:

```
[ ] No compiler warnings
[ ] No clippy warnings
[ ] All tests pass
[ ] New code is tested (>80% coverage)
[ ] Public APIs are documented
[ ] Errors are handled properly
[ ] No unwrap() in production code
[ ] No println! or dbg! statements
[ ] No commented-out code
[ ] No dead code
[ ] Code is formatted (cargo fmt)
[ ] New modules are exported
[ ] New features are integrated
[ ] Configuration is updated if needed
[ ] Database migrations applied
[ ] API docs updated (OpenAPI)
[ ] MASSIVE_TODO_LIST.md updated
[ ] README updated if needed
```

---

## 🆘 When You're Stuck

1. **Read the relevant documentation:**
   - `docs/ARCHITECTURE.md` - System design
   - `docs/ADR/` - Architecture decisions
   - Rust docs: https://doc.rust-lang.org/

2. **Search existing code:**
```bash
# Find similar implementations
rg "impl.*Battery" src/

# Find usage examples
rg "Battery::new" src/ tests/

# Find tests
rg "test.*battery" src/ tests/
```

3. **Check the TODO list:**
   - Is there a prerequisite task you missed?
   - Are there related tasks with hints?

4. **Ask for clarification:**
   - Leave a clear comment in the code
   - Document your assumptions
   - Propose an approach

---

## 📖 Additional Resources

- **Rust Book:** https://doc.rust-lang.org/book/
- **Async Book:** https://rust-lang.github.io/async-book/
- **Tokio Tutorial:** https://tokio.rs/tokio/tutorial
- **Axum Docs:** https://docs.rs/axum/latest/axum/
- **SQLx Guide:** https://github.com/launchbadge/sqlx
- **Error Handling:** https://doc.rust-lang.org/book/ch09-00-error-handling.html

---

## 🎯 Success Criteria

You're doing a great job if:

✅ Tests pass after every change  
✅ No clippy warnings  
✅ TODO list stays updated  
✅ New code is immediately integrated  
✅ No dead or commented code  
✅ All public APIs documented  
✅ Error handling is robust  
✅ Code follows project structure  
✅ Commits are atomic and well-described  

---

## 🚀 Remember

**Quality over quantity.** It's better to implement 5 features perfectly than 20 features poorly.

**Test everything.** Untested code is broken code waiting to be discovered.

**Integrate immediately.** Code that isn't wired up is wasted effort.

**Clean as you go.** Future you (or future AI) will thank present you.

**Update the TODO list.** It's how we track progress and prevent duplicate work.

---

**Now go build something amazing! 🔋⚡**
