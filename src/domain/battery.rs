use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{collections::VecDeque, sync::Arc};
use thiserror::Error;
use tokio::sync::RwLock;

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
}

impl SimulatedBattery {
    pub fn new(initial: BatteryState, caps: BatteryCapabilities) -> Self {
        Self {
            state: Arc::new(RwLock::new(initial)),
            caps,
        }
    }
    fn clamp_soc(soc: f64) -> f64 {
        soc.clamp(0.0, 100.0)
    }
}

#[async_trait]
impl Battery for SimulatedBattery {
    async fn read_state(&self) -> Result<BatteryState> {
        Ok(self.state.read().await.clone())
    }

    async fn set_power(&self, watts: f64) -> Result<()> {
        let mut st = self.state.write().await;
        st.power_w = watts;

        // 60s step skeleton
        let dt_h = 60.0 / 3600.0;
        let cap_kwh = self.caps.capacity_kwh.max(0.1);

        let power_kw = watts / 1000.0;
        let eff = self.caps.efficiency.clamp(0.01, 1.0);
        let delta_kwh = if power_kw >= 0.0 {
            power_kw * dt_h * eff
        } else {
            power_kw * dt_h / eff
        };
        let delta_pct = (delta_kwh / cap_kwh) * 100.0;
        st.soc_percent = Self::clamp_soc(st.soc_percent + delta_pct);

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
