use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Solar/Hybrid Inverter trait - abstraction for Modbus or simulated inverters
#[async_trait]
pub trait Inverter: Send + Sync {
    async fn read_state(&self) -> Result<InverterState>;
    async fn set_mode(&self, mode: InverterMode) -> Result<()>;
    async fn set_export_limit(&self, watts: f64) -> Result<()>;
    fn capabilities(&self) -> InverterCapabilities;
}

#[cfg_attr(feature = "swagger", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InverterState {
    pub mode: InverterMode,
    pub pv_power_w: f64,
    pub ac_output_power_w: f64,
    pub dc_input_power_w: f64,
    pub grid_frequency_hz: f64,
    pub ac_voltage_v: f64,
    pub dc_voltage_v: f64,
    pub temperature_c: f64,
    pub efficiency_percent: f64,
    pub status: InverterStatus,
    pub daily_energy_kwh: f64,
    pub total_energy_kwh: f64,
}

#[cfg_attr(feature = "swagger", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum InverterMode {
    GridTied,
    OffGrid,
    Hybrid,
    Backup,
    Standby,
}

#[cfg_attr(feature = "swagger", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum InverterStatus {
    Normal,
    Fault,
    Standby,
    InitialStandby,
    Shutdown,
    Warning,
}

#[cfg_attr(feature = "swagger", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InverterCapabilities {
    pub rated_power_w: f64,
    pub max_dc_input_w: f64,
    pub max_ac_output_w: f64,
    pub max_efficiency_percent: f64,
    pub mppt_channels: u8,
    pub supports_export_limit: bool,
    pub supports_frequency_regulation: bool,
}

/// Simulated Solar Inverter for development and testing
#[derive(Debug)]
pub struct SimulatedInverter {
    state: Arc<RwLock<InverterState>>,
    caps: InverterCapabilities,
}

impl SimulatedInverter {
    pub fn new(initial: InverterState, caps: InverterCapabilities) -> Self {
        Self {
            state: Arc::new(RwLock::new(initial)),
            caps,
        }
    }

    pub fn default_inverter() -> Self {
        let caps = InverterCapabilities {
            rated_power_w: 10000.0,
            max_dc_input_w: 15000.0,
            max_ac_output_w: 10000.0,
            max_efficiency_percent: 97.5,
            mppt_channels: 2,
            supports_export_limit: true,
            supports_frequency_regulation: true,
        };
        let initial = InverterState {
            mode: InverterMode::GridTied,
            pv_power_w: 0.0,
            ac_output_power_w: 0.0,
            dc_input_power_w: 0.0,
            grid_frequency_hz: 50.0,
            ac_voltage_v: 230.0,
            dc_voltage_v: 400.0,
            temperature_c: 35.0,
            efficiency_percent: 97.0,
            status: InverterStatus::Normal,
            daily_energy_kwh: 0.0,
            total_energy_kwh: 0.0,
        };
        Self::new(initial, caps)
    }

    /// Simulate PV production based on time of day (for testing)
    pub async fn simulate_pv_production(&self, hour: u8) {
        let mut st = self.state.write().await;

        // Simple solar curve: 0 at night, peak at noon
        let production_factor = if hour >= 6 && hour <= 20 {
            let normalized_hour = (hour as f64 - 6.0) / 14.0; // 0..1 over daylight hours
            let curve = (normalized_hour * std::f64::consts::PI).sin(); // Sine curve
            curve.max(0.0)
        } else {
            0.0
        };

        st.pv_power_w = self.caps.rated_power_w * production_factor;
        st.dc_input_power_w = st.pv_power_w;

        // Apply efficiency to get AC output
        st.ac_output_power_w = st.pv_power_w * (st.efficiency_percent / 100.0);
    }
}

#[async_trait]
impl Inverter for SimulatedInverter {
    async fn read_state(&self) -> Result<InverterState> {
        Ok(self.state.read().await.clone())
    }

    async fn set_mode(&self, mode: InverterMode) -> Result<()> {
        let mut st = self.state.write().await;

        // Validate mode transition
        match (st.mode, mode) {
            (InverterMode::GridTied, InverterMode::OffGrid) => {
                // Might need to wait for grid disconnect
            }
            (InverterMode::OffGrid, InverterMode::GridTied) => {
                // Might need to sync with grid frequency
            }
            _ => {}
        }

        st.mode = mode;
        Ok(())
    }

    async fn set_export_limit(&self, watts: f64) -> Result<()> {
        if !self.caps.supports_export_limit {
            anyhow::bail!("Inverter does not support export limiting");
        }

        let clamped = watts.clamp(0.0, self.caps.max_ac_output_w);

        let mut st = self.state.write().await;
        // In a real implementation, this would set the export limit register
        // For simulation, we just clamp the AC output
        if st.ac_output_power_w > clamped {
            st.ac_output_power_w = clamped;
        }

        Ok(())
    }

    fn capabilities(&self) -> InverterCapabilities {
        self.caps.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_simulated_inverter_modes() {
        let inverter = SimulatedInverter::default_inverter();

        // Initial mode
        let state = inverter.read_state().await.unwrap();
        assert_eq!(state.mode, InverterMode::GridTied);

        // Change to off-grid
        inverter.set_mode(InverterMode::OffGrid).await.unwrap();
        let state = inverter.read_state().await.unwrap();
        assert_eq!(state.mode, InverterMode::OffGrid);
    }

    #[tokio::test]
    async fn test_export_limit() {
        let inverter = SimulatedInverter::default_inverter();

        // Set export limit
        inverter.set_export_limit(5000.0).await.unwrap();

        // Capabilities should support it
        let caps = inverter.capabilities();
        assert!(caps.supports_export_limit);
    }

    #[tokio::test]
    async fn test_pv_production_simulation() {
        let inverter = SimulatedInverter::default_inverter();

        // Midnight - no production
        inverter.simulate_pv_production(0).await;
        let state = inverter.read_state().await.unwrap();
        assert_eq!(state.pv_power_w, 0.0);

        // Noon - peak production
        inverter.simulate_pv_production(13).await;
        let state = inverter.read_state().await.unwrap();
        assert!(state.pv_power_w > 0.0);
        assert!(state.ac_output_power_w > 0.0);
        assert!(state.ac_output_power_w < state.pv_power_w); // Due to efficiency loss
    }
}
