#![allow(dead_code)]
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{collections::VecDeque, sync::Arc};
use thiserror::Error;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};

/// Battery-specific errors
#[derive(Debug, Error)]
pub enum BatteryError {
    #[error("Communication error: {0}")]
    Communication(String),
    #[error("Invalid power command: {0}W (exceeds limits)")]
    InvalidPower(f64),
    #[error("Battery offline or unavailable")]
    Offline,
    #[error("Battery in fault state: {0}")]
    Fault(String),
    #[error("State of charge out of bounds: {0}%")]
    SocOutOfBounds(f64),
}

/// Battery health status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "swagger", derive(utoipa::ToSchema))]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Warning,
    Critical,
    Offline,
}

/// Battery operational status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "swagger", derive(utoipa::ToSchema))]
pub enum BatteryStatus {
    Charging,
    Discharging,
    Idle,
    Standby,
    Fault,
    Offline,
}

impl std::str::FromStr for BatteryStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "charging" => Ok(BatteryStatus::Charging),
            "discharging" => Ok(BatteryStatus::Discharging),
            "idle" => Ok(BatteryStatus::Idle),
            "standby" => Ok(BatteryStatus::Standby),
            "fault" => Ok(BatteryStatus::Fault),
            "offline" => Ok(BatteryStatus::Offline),
            _ => Err(format!("Unknown battery status: {}", s)),
        }
    }
}

impl std::fmt::Display for BatteryStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BatteryStatus::Charging => write!(f, "charging"),
            BatteryStatus::Discharging => write!(f, "discharging"),
            BatteryStatus::Idle => write!(f, "idle"),
            BatteryStatus::Standby => write!(f, "standby"),
            BatteryStatus::Fault => write!(f, "fault"),
            BatteryStatus::Offline => write!(f, "offline"),
        }
    }
}

/// Battery chemistry type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "swagger", derive(utoipa::ToSchema))]
pub enum BatteryChemistry {
    LiFePO4,    // Lithium Iron Phosphate
    NMC,        // Nickel Manganese Cobalt
    LTO,        // Lithium Titanate Oxide
    NCA,        // Nickel Cobalt Aluminum
    LeadAcid,   // Lead Acid
    Unknown,
}

/// Battery command
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "swagger", derive(utoipa::ToSchema))]
pub enum BatteryCommand {
    Charge(f64),     // Charge at specified watts
    Discharge(f64),  // Discharge at specified watts
    Idle,            // Stop charging/discharging
    Standby,         // Enter standby mode
}

/// Battery degradation model
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "swagger", derive(utoipa::ToSchema))]
pub struct DegradationModel {
    pub cycle_count: u32,
    pub health_percent: f64,
    pub degradation_rate_per_cycle: f64,
    pub calendar_age_years: f64,
    pub temperature_impact: f64,
}

impl Default for DegradationModel {
    fn default() -> Self {
        Self {
            cycle_count: 0,
            health_percent: 100.0,
            degradation_rate_per_cycle: 0.01, // 0.01% per cycle
            calendar_age_years: 0.0,
            temperature_impact: 1.0,
        }
    }
}

#[async_trait]
pub trait Battery: Send + Sync {
    async fn read_state(&self) -> Result<BatteryState>;
    async fn set_power(&self, watts: f64) -> Result<()>;
    fn capabilities(&self) -> BatteryCapabilities;
    async fn health_check(&self) -> Result<HealthStatus>;
}

#[cfg_attr(feature = "swagger", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatteryState {
    pub soc_percent: f64,
    pub power_w: f64,
    pub voltage_v: f64,
    pub temperature_c: f64,
    pub health_percent: f64,
    pub status: BatteryStatus,
}

#[cfg_attr(feature = "swagger", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatteryCapabilities {
    pub capacity_kwh: f64,
    pub max_charge_kw: f64,
    pub max_discharge_kw: f64,
    pub efficiency: f64,
    pub degradation_per_cycle: f64,
    pub chemistry: BatteryChemistry,
}

#[derive(Debug)]
pub struct SimulatedBattery {
    state: Arc<RwLock<BatteryState>>,
    caps: BatteryCapabilities,
    /// Enable realistic communication delays (simulates Modbus latency)
    pub simulate_delays: bool,
    /// Enable random noise in sensor readings (realistic sensor variation)
    pub simulate_noise: bool,
    /// Ambient temperature (°C) - configurable based on installation environment
    /// Nordic winter: -10°C, Summer: 20°C, Indoor: 15-25°C
    pub ambient_temp_c: f64,
}

impl SimulatedBattery {
    pub fn new(initial: BatteryState, caps: BatteryCapabilities) -> Self {
        Self::new_with_ambient(initial, caps, 25.0) // Default 25°C for backwards compatibility
    }

    /// Create a new simulated battery with configurable ambient temperature
    pub fn new_with_ambient(initial: BatteryState, caps: BatteryCapabilities, ambient_temp_c: f64) -> Self {
        Self {
            state: Arc::new(RwLock::new(initial)),
            caps,
            simulate_delays: false,
            simulate_noise: false,
            ambient_temp_c,
        }
    }

    /// Create a new simulated battery with realistic delays and noise enabled
    pub fn new_realistic(initial: BatteryState, caps: BatteryCapabilities) -> Self {
        Self::new_realistic_with_ambient(initial, caps, 25.0) // Default 25°C for backwards compatibility
    }

    /// Create a new simulated battery with realistic delays, noise, and configurable ambient temperature
    pub fn new_realistic_with_ambient(initial: BatteryState, caps: BatteryCapabilities, ambient_temp_c: f64) -> Self {
        Self {
            state: Arc::new(RwLock::new(initial)),
            caps,
            simulate_delays: true,
            simulate_noise: true,
            ambient_temp_c,
        }
    }

    fn clamp_soc(soc: f64) -> f64 {
        soc.clamp(0.0, 100.0)
    }

    /// Add random noise to a value (±1% variation)
    fn add_noise(&self, value: f64) -> f64 {
        if !self.simulate_noise {
            return value;
        }

        use std::collections::hash_map::RandomState;
        use std::hash::{BuildHasher, Hash, Hasher};

        // Simple pseudo-random noise using hash of current time
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        let mut hasher = RandomState::new().build_hasher();
        now.hash(&mut hasher);
        let hash = hasher.finish();

        // Convert hash to a value between -0.01 and 0.01 (±1%)
        let noise_factor = ((hash % 200) as f64 / 10000.0) - 0.01;
        value * (1.0 + noise_factor)
    }
}

#[async_trait]
impl Battery for SimulatedBattery {
    async fn read_state(&self) -> Result<BatteryState> {
        // Simulate Modbus communication delay (50-150ms)
        if self.simulate_delays {
            use std::collections::hash_map::RandomState;
            use std::hash::{BuildHasher, Hasher};

            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();

            let mut hasher = RandomState::new().build_hasher();
            std::hash::Hash::hash(&now, &mut hasher);
            let delay_ms = 50 + (hasher.finish() % 100) as u64;

            sleep(Duration::from_millis(delay_ms)).await;
        }

        let state = self.state.read().await;

        // Add noise to readings if enabled
        if self.simulate_noise {
            Ok(BatteryState {
                soc_percent: self.add_noise(state.soc_percent).clamp(0.0, 100.0),
                power_w: self.add_noise(state.power_w),
                voltage_v: self.add_noise(state.voltage_v),
                temperature_c: self.add_noise(state.temperature_c),
                health_percent: self.add_noise(state.health_percent).clamp(0.0, 100.0),
                status: state.status,
            })
        } else {
            Ok(state.clone())
        }
    }

    async fn set_power(&self, watts: f64) -> Result<()> {
        let mut st = self.state.write().await;
        st.power_w = watts;

        // CRITICAL POWER CONVENTION:
        // `watts` parameter is AC-side power (grid perspective)
        // - Positive = charging (drawing from grid)
        // - Negative = discharging (feeding to grid)
        // The battery stores DC power, so we apply efficiency conversion:
        // - Charging: AC * efficiency = DC stored
        // - Discharging: DC / efficiency = AC delivered
        // This matches the optimizer's convention in dp.rs to prevent efficiency double-dip

        // 60s step skeleton
        let dt_h = 60.0 / 3600.0;
        let cap_kwh = self.caps.capacity_kwh.max(0.1);

        let power_kw = watts / 1000.0;
        // Validate efficiency is within acceptable range
        let eff = if self.caps.efficiency >= 0.5 && self.caps.efficiency <= 1.0 {
            self.caps.efficiency
        } else {
            anyhow::bail!("Invalid battery efficiency: {}. Must be between 0.5 and 1.0", self.caps.efficiency);
        };
        // Apply efficiency conversion: AC -> DC
        let delta_kwh = if power_kw >= 0.0 {
            // Charging: AC power * efficiency = DC energy stored
            power_kw * dt_h * eff
        } else {
            // Discharging: DC energy / efficiency = AC power delivered
            power_kw * dt_h / eff
        };
        let delta_pct = (delta_kwh / cap_kwh) * 100.0;
        st.soc_percent = Self::clamp_soc(st.soc_percent + delta_pct);

        // Temperature simulation - rises during charging/discharging
        // CRITICAL FIX: Use configurable ambient temperature based on installation environment
        // Nordic garage in winter might be -10°C, indoor might be 20°C
        const TEMP_RISE_PER_KW: f64 = 2.0; // °C per kW of power
        const COOLING_RATE: f64 = 0.5; // °C per time step when idle
        const MAX_SAFE_TEMP: f64 = 45.0; // Should match SafetyConstraints default

        let power_abs_kw = watts.abs() / 1000.0;
        let target_temp = self.ambient_temp_c + (power_abs_kw * TEMP_RISE_PER_KW);

        // Gradually approach target temperature
        if st.temperature_c < target_temp {
            st.temperature_c = (st.temperature_c + COOLING_RATE).min(target_temp);
        } else {
            st.temperature_c = (st.temperature_c - COOLING_RATE).max(target_temp);
        }

        // CRITICAL: Clamp to safe operating range (respect safety constraints)
        st.temperature_c = st.temperature_c.clamp(0.0, MAX_SAFE_TEMP);

        // Degradation simulation - health decreases with cycles
        // A cycle is a full charge or discharge (100% SoC change)
        let soc_change_pct = delta_pct.abs();
        let partial_cycle = soc_change_pct / 100.0;
        let degradation = self.caps.degradation_per_cycle * partial_cycle;
        st.health_percent = (st.health_percent - degradation).max(0.0);

        // Update status based on power
        st.status = if watts > 10.0 {
            BatteryStatus::Charging
        } else if watts < -10.0 {
            BatteryStatus::Discharging
        } else {
            BatteryStatus::Idle
        };

        Ok(())
    }

    fn capabilities(&self) -> BatteryCapabilities {
        self.caps.clone()
    }

    async fn health_check(&self) -> Result<HealthStatus> {
        let state = self.state.read().await;
        Ok(if state.health_percent >= 90.0 {
            HealthStatus::Healthy
        } else if state.health_percent >= 70.0 {
            HealthStatus::Degraded
        } else if state.health_percent >= 50.0 {
            HealthStatus::Warning
        } else if state.health_percent > 0.0 {
            HealthStatus::Critical
        } else {
            HealthStatus::Offline
        })
    }
}

pub struct MockBattery {
    pub states: Arc<RwLock<VecDeque<BatteryState>>>,
    caps: BatteryCapabilities,
}

impl MockBattery {
    pub fn new(states: VecDeque<BatteryState>, caps: BatteryCapabilities) -> Self {
        Self {
            states: Arc::new(RwLock::new(states)),
            caps,
        }
    }
}

#[async_trait]
impl Battery for MockBattery {
    async fn read_state(&self) -> Result<BatteryState> {
        let mut q = self.states.write().await;
        Ok(q.pop_front().unwrap_or(BatteryState {
            soc_percent: 50.0,
            power_w: 0.0,
            voltage_v: 48.0,
            temperature_c: 25.0,
            health_percent: 100.0,
            status: BatteryStatus::Idle,
        }))
    }
    async fn set_power(&self, _watts: f64) -> Result<()> {
        Ok(())
    }
    fn capabilities(&self) -> BatteryCapabilities {
        self.caps.clone()
    }
    async fn health_check(&self) -> Result<HealthStatus> {
        Ok(HealthStatus::Healthy)
    }
}
