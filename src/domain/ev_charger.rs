use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// EV Charger trait - abstraction for OCPP-based or simulated EV chargers
#[async_trait]
pub trait EvCharger: Send + Sync {
    async fn read_state(&self) -> Result<ChargerState>;
    async fn set_current(&self, amps: f64) -> Result<()>;
    async fn start_charging(&self) -> Result<()>;
    async fn stop_charging(&self) -> Result<()>;
    fn capabilities(&self) -> ChargerCapabilities;
}

#[cfg_attr(feature = "swagger", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChargerState {
    pub status: ChargerStatus,
    pub connected: bool,
    pub charging: bool,
    pub current_amps: f64,
    pub power_w: f64,
    pub energy_delivered_kwh: f64,
    pub session_duration_seconds: u64,
    pub vehicle_soc_percent: Option<f64>,
}

#[cfg_attr(feature = "swagger", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub enum ChargerStatus {
    Available,
    Preparing,
    Charging,
    SuspendedEV,
    SuspendedEVSE,
    Finishing,
    Reserved,
    Unavailable,
    Faulted,
}

#[cfg_attr(feature = "swagger", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChargerCapabilities {
    pub max_current_amps: f64,
    pub min_current_amps: f64,
    pub phases: u8,
    pub voltage_v: f64,
    pub connector_type: ConnectorType,
    pub power_max_kw: f64,
    pub supports_v2g: bool,
}

#[cfg_attr(feature = "swagger", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ConnectorType {
    Type1,
    Type2,
    CCS,
    CHAdeMO,
    Tesla,
}

/// Simulated EV Charger for development and testing
#[derive(Debug)]
pub struct SimulatedEvCharger {
    state: Arc<RwLock<ChargerState>>,
    caps: ChargerCapabilities,
}

impl SimulatedEvCharger {
    pub fn new(initial: ChargerState, caps: ChargerCapabilities) -> Self {
        Self {
            state: Arc::new(RwLock::new(initial)),
            caps,
        }
    }

    pub fn default_charger() -> Self {
        let caps = ChargerCapabilities {
            max_current_amps: 32.0,
            min_current_amps: 6.0,
            phases: 3,
            voltage_v: 230.0,
            connector_type: ConnectorType::Type2,
            power_max_kw: 22.0, // 32A * 230V * 3 phases / 1000
            supports_v2g: false,
        };
        let initial = ChargerState {
            status: ChargerStatus::Available,
            connected: false,
            charging: false,
            current_amps: 0.0,
            power_w: 0.0,
            energy_delivered_kwh: 0.0,
            session_duration_seconds: 0,
            vehicle_soc_percent: None,
        };
        Self::new(initial, caps)
    }
}

#[async_trait]
impl EvCharger for SimulatedEvCharger {
    async fn read_state(&self) -> Result<ChargerState> {
        Ok(self.state.read().await.clone())
    }

    async fn set_current(&self, amps: f64) -> Result<()> {
        let mut st = self.state.write().await;

        // Clamp to capabilities
        let clamped = amps.clamp(0.0, self.caps.max_current_amps);
        st.current_amps = clamped;

        // Calculate power (3-phase)
        if st.charging && st.connected {
            st.power_w = clamped * self.caps.voltage_v * self.caps.phases as f64;
        } else {
            st.power_w = 0.0;
        }

        Ok(())
    }

    async fn start_charging(&self) -> Result<()> {
        let mut st = self.state.write().await;

        if !st.connected {
            anyhow::bail!("Cannot start charging: no vehicle connected");
        }

        if st.status == ChargerStatus::Faulted {
            anyhow::bail!("Cannot start charging: charger is faulted");
        }

        st.charging = true;
        st.status = ChargerStatus::Charging;

        Ok(())
    }

    async fn stop_charging(&self) -> Result<()> {
        let mut st = self.state.write().await;

        st.charging = false;
        st.current_amps = 0.0;
        st.power_w = 0.0;

        if st.connected {
            st.status = ChargerStatus::SuspendedEVSE;
        } else {
            st.status = ChargerStatus::Available;
        }

        Ok(())
    }

    fn capabilities(&self) -> ChargerCapabilities {
        self.caps.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_simulated_charger_start_stop() {
        let charger = SimulatedEvCharger::default_charger();

        // Initially available
        let state = charger.read_state().await.unwrap();
        assert_eq!(state.status, ChargerStatus::Available);
        assert!(!state.charging);

        // Can't start without connection
        assert!(charger.start_charging().await.is_err());

        // Simulate connection
        {
            let mut st = charger.state.write().await;
            st.connected = true;
            st.status = ChargerStatus::Preparing;
        }

        // Now can start
        charger.start_charging().await.unwrap();
        let state = charger.read_state().await.unwrap();
        assert!(state.charging);
        assert_eq!(state.status, ChargerStatus::Charging);

        // Stop charging
        charger.stop_charging().await.unwrap();
        let state = charger.read_state().await.unwrap();
        assert!(!state.charging);
        assert_eq!(state.power_w, 0.0);
    }

    #[tokio::test]
    async fn test_set_current_calculates_power() {
        let charger = SimulatedEvCharger::default_charger();

        // Simulate connected and charging
        {
            let mut st = charger.state.write().await;
            st.connected = true;
            st.charging = true;
        }

        // Set current to 16A
        charger.set_current(16.0).await.unwrap();
        let state = charger.read_state().await.unwrap();
        assert_eq!(state.current_amps, 16.0);

        // Power = 16A * 230V * 3 phases = 11,040W
        assert_eq!(state.power_w, 16.0 * 230.0 * 3.0);
    }
}
