use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{collections::VecDeque, sync::Arc};
use tokio::sync::RwLock;

#[async_trait]
pub trait Battery: Send + Sync {
    async fn read_state(&self) -> Result<BatteryState>;
    async fn set_power(&self, watts: f64) -> Result<()>;
    fn capabilities(&self) -> BatteryCapabilities;
}

#[cfg_attr(feature = "swagger", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatteryState {
    pub soc_percent: f64,
    pub power_w: f64,
    pub voltage_v: f64,
    pub temperature_c: f64,
    pub health_percent: f64,
}

#[cfg_attr(feature = "swagger", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatteryCapabilities {
    pub capacity_kwh: f64,
    pub max_charge_kw: f64,
    pub max_discharge_kw: f64,
    pub efficiency: f64,
    pub degradation_per_cycle: f64,
}

#[derive(Debug)]
pub struct SimulatedBattery {
    state: Arc<RwLock<BatteryState>>,
    caps: BatteryCapabilities,
}

impl SimulatedBattery {
    pub fn new(initial: BatteryState, caps: BatteryCapabilities) -> Self {
        Self { state: Arc::new(RwLock::new(initial)), caps }
    }
    fn clamp_soc(soc: f64) -> f64 { soc.clamp(0.0, 100.0) }
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
        let delta_kwh = if power_kw >= 0.0 { power_kw * dt_h * eff } else { power_kw * dt_h / eff };
        let delta_pct = (delta_kwh / cap_kwh) * 100.0;
        st.soc_percent = Self::clamp_soc(st.soc_percent + delta_pct);

        Ok(())
    }

    fn capabilities(&self) -> BatteryCapabilities { self.caps.clone() }
}

pub struct MockBattery {
    pub states: Arc<RwLock<VecDeque<BatteryState>>>,
    caps: BatteryCapabilities,
}

impl MockBattery {
    pub fn new(states: VecDeque<BatteryState>, caps: BatteryCapabilities) -> Self {
        Self { states: Arc::new(RwLock::new(states)), caps }
    }
}

#[async_trait]
impl Battery for MockBattery {
    async fn read_state(&self) -> Result<BatteryState> {
        let mut q = self.states.write().await;
        Ok(q.pop_front().unwrap_or(BatteryState { soc_percent: 50.0, power_w: 0.0, voltage_v: 48.0, temperature_c: 25.0, health_percent: 100.0 }))
    }
    async fn set_power(&self, _watts: f64) -> Result<()> { Ok(()) }
    fn capabilities(&self) -> BatteryCapabilities { self.caps.clone() }
}
