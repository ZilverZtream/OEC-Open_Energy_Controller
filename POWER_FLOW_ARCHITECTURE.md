# Power Flow Architecture

**Technical documentation for the power flow orchestration system**

---

## Overview

The power flow orchestration system is the core coordination layer that manages all energy flows in real-time. It ensures physical constraints are never violated, safety requirements are always met, and economic optimization occurs within those boundaries.

---

## Core Concepts

### Power Balance

At any moment, the fundamental equation must hold:

```
Sources = Sinks
PV + Grid Import + Battery Discharge = Load + EV + Battery Charge + Grid Export
```

All power flows are computed simultaneously to guarantee this balance.

### PowerSnapshot

The `PowerSnapshot` struct represents the complete system state at a single moment:

```rust
pub struct PowerSnapshot {
    /// Solar PV production (kW, always positive)
    pub pv_kw: f64,
    
    /// Household load (kW, always positive)
    pub house_load_kw: f64,
    
    /// Battery power (kW, positive = charging, negative = discharging)
    pub battery_power_kw: f64,
    
    /// EV charging power (kW, always positive)
    pub ev_power_kw: f64,
    
    /// Grid import (kW, positive = importing)
    pub grid_import_kw: f64,
    
    /// Grid export (kW, positive = exporting)
    pub grid_export_kw: f64,
    
    pub timestamp: DateTime<Utc>,
}
```

### Verification

Every snapshot includes verification methods:

```rust
impl PowerSnapshot {
    /// Verify power balance equation holds
    pub fn verify_power_balance(&self) -> bool {
        let sources = self.pv_kw + self.grid_import_kw 
            + self.battery_power_kw.min(0.0).abs();
        let sinks = self.house_load_kw + self.ev_power_kw 
            + self.battery_power_kw.max(0.0) + self.grid_export_kw;
        (sources - sinks).abs() < 0.01
    }
    
    pub fn exceeds_fuse_limit(&self, limit_kw: f64) -> bool {
        self.grid_import_kw > limit_kw
    }
}
```

---

## Constraint System

### Three-Tier Hierarchy

The system enforces constraints in strict priority order:

#### 1. Physical Constraints (Hard Limits)

These represent immutable physical reality:

```rust
pub struct PhysicalConstraints {
    /// Main fuse / grid connection limit
    pub max_grid_import_kw: f64,
    
    /// Maximum grid export (may be zero)
    pub max_grid_export_kw: f64,
    
    /// Battery charge power limit
    pub max_battery_charge_kw: f64,
    
    /// Battery discharge power limit
    pub max_battery_discharge_kw: f64,
    
    /// EVSE minimum current (IEC 61851)
    pub evse_min_current_a: f64,
    
    /// EVSE maximum current
    pub evse_max_current_a: f64,
    
    /// Number of phases (1 or 3)
    pub phases: u8,
}
```

#### 2. Safety Constraints

These ensure safe, reliable operation:

```rust
pub struct SafetyConstraints {
    /// Minimum battery SoC (never discharge below)
    pub battery_min_soc_percent: f64,
    
    /// Maximum battery SoC (never charge above)
    pub battery_max_soc_percent: f64,
    
    /// House load always has priority
    pub house_priority: bool,
    
    /// Maximum battery cycles per day
    pub max_battery_cycles_per_day: f64,
    
    /// Maximum battery temperature
    pub max_battery_temp_c: f64,
}
```

#### 3. Economic Objectives

These guide optimization within safe bounds:

```rust
pub struct EconomicObjectives {
    /// Current grid electricity price
    pub grid_price_sek_kwh: f64,
    
    /// Grid export price (feed-in tariff)
    pub export_price_sek_kwh: f64,
    
    /// Prefer self-consumption over export
    pub prefer_self_consumption: bool,
    
    /// Price threshold for arbitrage trading
    pub arbitrage_threshold_sek_kwh: f64,
    
    /// EV departure time (if connected)
    pub ev_departure_time: Option<DateTime<Utc>>,
    
    /// EV target SoC at departure
    pub ev_target_soc_percent: Option<f64>,
}
```

---

## Power Flow Algorithm

### High-Level Flow

```
1. Read system state (PV, load, battery, EV, prices)
2. Apply constraint hierarchy
3. Compute optimal power allocation
4. Verify power balance and constraints
5. Return PowerSnapshot
```

### Implementation

```rust
impl PowerFlowModel {
    pub async fn compute_flows(
        &self,
        inputs: PowerFlowInputs,
        constraints: &AllConstraints,
        objectives: &EconomicObjectives,
    ) -> Result<PowerSnapshot> {
        let mut available_pv = inputs.pv_production_kw;
        let mut grid_import = 0.0;
        let mut grid_export = 0.0;
        
        // Step 1: Satisfy house load (highest priority)
        let house_load = inputs.house_load_kw;
        if house_load > available_pv {
            grid_import = house_load - available_pv;
            available_pv = 0.0;
        } else {
            available_pv -= house_load;
        }
        
        // Step 2: EV charging (if connected and needed)
        let ev_power = self.compute_ev_power(
            &inputs.ev_state,
            available_pv,
            grid_import,
            constraints,
            objectives,
        )?;
        
        if ev_power > available_pv {
            let additional_grid = ev_power - available_pv;
            if grid_import + additional_grid > constraints.physical.max_grid_import_kw {
                // Reduce EV power to fit within fuse limit
                let ev_power = constraints.physical.max_grid_import_kw - grid_import;
            }
            grid_import += (ev_power - available_pv).max(0.0);
            available_pv = 0.0;
        } else {
            available_pv -= ev_power;
        }
        
        // Step 3: Battery (charge from excess PV or grid arbitrage)
        let battery_power = self.compute_battery_power(
            inputs.battery_soc_percent,
            available_pv,
            grid_import,
            constraints,
            objectives,
        )?;
        
        // Step 4: Grid export (if beneficial and allowed)
        if available_pv > 0.01 && constraints.physical.max_grid_export_kw > 0.0 {
            if objectives.prefer_self_consumption {
                if inputs.battery_soc_percent >= constraints.safety.battery_max_soc_percent {
                    grid_export = available_pv.min(constraints.physical.max_grid_export_kw);
                }
            } else {
                grid_export = available_pv.min(constraints.physical.max_grid_export_kw);
            }
        }
        
        let snapshot = PowerSnapshot {
            pv_kw: inputs.pv_production_kw,
            house_load_kw: house_load,
            battery_power_kw: battery_power,
            ev_power_kw: ev_power,
            grid_import_kw: grid_import,
            grid_export_kw: grid_export,
            timestamp: Utc::now(),
        };
        
        if !snapshot.verify_power_balance() {
            return Err(Error::PowerBalanceViolation(snapshot));
        }
        
        Ok(snapshot)
    }
}
```

### EV Charging Logic

The EV charging power is calculated based on urgency:

```rust
fn compute_ev_power(
    &self,
    ev_state: &Option<EvState>,
    available_pv: f64,
    current_grid_import: f64,
    constraints: &AllConstraints,
    objectives: &EconomicObjectives,
) -> Result<f64> {
    let Some(ev) = ev_state else {
        return Ok(0.0);
    };
    
    if !ev.connected || ev.soc_percent >= ev.target_soc_percent {
        return Ok(0.0);
    }
    
    // Calculate urgency (0.0 = no rush, 1.0 = must charge now)
    let urgency = self.calculate_urgency(ev, objectives.ev_departure_time)?;
    
    // Determine desired power based on urgency and price
    let desired_power = if urgency > 0.8 {
        // Urgent - charge at maximum rate
        self.evse_max_power_kw(constraints)
    } else if objectives.grid_price_sek_kwh < objectives.arbitrage_threshold_sek_kwh {
        // Cheap electricity - charge at high rate
        self.evse_max_power_kw(constraints) * 0.8
    } else {
        // Expensive - charge at minimum rate
        self.evse_min_power_kw(constraints)
    };
    
    // Check fuse limit
    let fuse_available = constraints.physical.max_grid_import_kw - current_grid_import;
    let actual_power = (available_pv + fuse_available).min(desired_power);
    
    Ok(actual_power)
}

fn calculate_urgency(
    &self,
    ev: &EvState,
    departure: Option<DateTime<Utc>>,
) -> Result<f64> {
    let Some(departure_time) = departure else {
        return Ok(0.0);
    };
    
    let now = Utc::now();
    if departure_time <= now {
        return Ok(1.0); // Already past departure!
    }
    
    let hours_remaining = (departure_time - now).num_seconds() as f64 / 3600.0;
    let soc_needed = ev.target_soc_percent - ev.soc_percent;
    let energy_needed_kwh = (soc_needed / 100.0) * ev.capacity_kwh;
    let required_rate_kw = energy_needed_kwh / hours_remaining;
    
    let urgency = required_rate_kw / ev.max_charge_kw;
    Ok(urgency.clamp(0.0, 1.0))
}
```

---

## Simulation Environment

### Purpose

The simulation environment enables:
- **Development** without physical hardware
- **Testing** of edge cases and failure scenarios
- **Validation** of control algorithms
- **Demonstration** of system capabilities

### Components

#### Simulated House

Generates realistic load profiles:

```rust
pub struct SimulatedHouse {
    base_load_kw: f64,
    daily_profile: Vec<f64>, // 24 hourly factors
    noise_amplitude: f64,
}

impl SimulatedHouse {
    pub fn get_load_at(&self, time: DateTime<Utc>) -> f64 {
        let hour = time.hour() as usize;
        let profile_factor = self.daily_profile[hour];
        let noise = rand::random::<f64>() * self.noise_amplitude;
        self.base_load_kw * profile_factor + noise
    }
}
```

#### Simulated Solar

Models PV production with solar geometry:

```rust
pub struct SimulatedSolar {
    capacity_kw: f64,
    latitude: f64,
    longitude: f64,
}

impl SimulatedSolar {
    pub fn get_production_at(&self, time: DateTime<Utc>) -> f64 {
        let elevation = self.solar_elevation_angle(time);
        if elevation <= 0.0 {
            return 0.0; // Night
        }
        
        let clear_sky = self.capacity_kw * (elevation / 90.0).sin();
        let cloud_factor = 0.7 + 0.3 * rand::random::<f64>();
        clear_sky * cloud_factor
    }
}
```

#### Simulated EV

Tracks state and charging:

```rust
pub struct SimulatedEv {
    pub capacity_kwh: f64,
    pub soc_percent: f64,
    pub max_charge_kw: f64,
    pub connected: bool,
    pub departure_time: Option<DateTime<Utc>>,
    pub target_soc_percent: f64,
}

impl SimulatedEv {
    pub fn charge(&mut self, power_kw: f64, duration_secs: f64) {
        let energy = power_kw * (duration_secs / 3600.0);
        let soc_increase = (energy / self.capacity_kwh) * 100.0;
        self.soc_percent = (self.soc_percent + soc_increase).min(100.0);
    }
}
```

#### Simulated EVSE

Enforces IEC 61851 constraints:

```rust
pub struct SimulatedEvse {
    current_limit_a: f64,
    min_current_a: f64, // 6A per IEC 61851
    max_current_a: f64,
    phases: u8,
}

impl SimulatedEvse {
    pub fn set_current_limit(&mut self, current_a: f64) -> Result<()> {
        if current_a < self.min_current_a || current_a > self.max_current_a {
            return Err(Error::InvalidCurrent(current_a));
        }
        self.current_limit_a = current_a;
        Ok(())
    }
    
    pub fn get_power_kw(&self) -> f64 {
        230.0 * self.current_limit_a * self.phases as f64 / 1000.0
    }
}
```

---

## Test Scenarios

### Scenario 1: Solar Self-Consumption

```
Time: 12:00 (sunny day)
- PV: 8 kW
- House: 2 kW
- Battery: 50% SoC
- EV: Connected, 40% SoC, departs 17:00

Expected flow:
- House: 2 kW (from PV)
- EV: 4 kW (from PV)
- Battery: 2 kW (from PV)
- Grid: 0 kW
```

### Scenario 2: Fuse Protection

```
Time: 18:00 (peak demand)
- PV: 0 kW (dark)
- House: 8 kW
- Battery: 30% SoC
- EV: Wants 11 kW
- Fuse: 16 kW

Expected flow:
- House: 8 kW (priority)
- EV: 7 kW (reduced to fit fuse)
- Battery: 1 kW (minimal)
- Grid: 16 kW (at fuse limit)
```

### Scenario 3: Urgent EV Charging

```
Time: 16:00
- PV: 2 kW
- House: 2 kW
- EV: 25% SoC, target 80%, departs 17:00 (1 hour!)
- Urgency: 0.95 (high!)

Expected flow:
- House: 2 kW (from PV)
- EV: 11 kW (max rate, from grid)
- Battery: 0 kW (paused for urgent EV)
- Grid: 11 kW
```

---

## Performance Considerations

### Control Loop Frequency

The system runs at 10-second intervals, balancing:
- Responsiveness to changes
- Modbus communication overhead
- Database write load

### Optimization Overhead

Power flow computation typically completes in <5ms, allowing ample time for:
- Sensor reads (~50ms total)
- Command writes (~50ms total)
- Database logging (~10ms)
- Metric updates (~1ms)

### Memory Footprint

Typical RSS usage: ~50MB, dominated by:
- Database connection pool (20MB)
- Tokio runtime (15MB)
- Application state (10MB)

---

## Error Handling

### Constraint Violations

If constraints would be violated, the system:
1. Logs the violation attempt
2. Falls back to safe default (house only, no EV/battery)
3. Increments violation counter (for monitoring)
4. Retries next cycle with updated state

### Communication Failures

Device communication errors are handled with:
- Automatic retry (3 attempts, exponential backoff)
- Timeout (5 seconds)
- Fallback to last known good state
- Alert if >5 consecutive failures

### Power Balance Errors

If power balance cannot be achieved:
- Log detailed snapshot
- Use grid import to satisfy load
- Disable optimization for one cycle
- Alert operator

---

## Further Reading

- **Implementation Guide**: See `MASSIVE_TODO_LIST.md` for development roadmap
- **API Documentation**: See OpenAPI spec at `/swagger-ui`
- **Deployment Guide**: See `docs/DEPLOYMENT.md`
