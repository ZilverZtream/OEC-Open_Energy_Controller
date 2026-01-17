#![allow(dead_code)]
use std::sync::Arc;

use crate::domain::{
    Battery, BatteryCapabilities, BatteryState, BatteryStatus, EvCharger,
    Inverter, InverterCapabilities, InverterMode, InverterState, InverterStatus,
    SimulatedBattery, SimulatedEvCharger, SimulatedInverter,
};

/// Hardware mode configuration
///
/// CRITICAL SAFETY FIX: Modbus mode is only available when the 'modbus' feature is enabled
/// This prevents accidental hardware actuation during simulation/development builds
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HardwareMode {
    /// Simulated devices for development and testing
    Simulated,
    /// Real hardware via Modbus TCP
    /// SAFETY: Only available with 'modbus' feature flag to prevent accidental hardware actuation
    #[cfg(feature = "modbus")]
    #[allow(dead_code)]
    Modbus,
    /// Mock devices with pre-programmed responses
    #[allow(dead_code)]
    Mock,
}

/// Factory for creating hardware device instances
pub struct DeviceFactory {
    mode: HardwareMode,
    config: Option<crate::config::Config>,
}

impl DeviceFactory {
    pub fn new(mode: HardwareMode) -> Self {
        Self { mode, config: None }
    }

    pub fn with_config(mode: HardwareMode, config: crate::config::Config) -> Self {
        Self { mode, config: Some(config) }
    }

    /// Create a battery instance based on hardware mode
    pub async fn create_battery(
        &self,
        caps: BatteryCapabilities,
        initial_soc: f64,
        ambient_temp_c: f64,
    ) -> Arc<dyn Battery> {
        match self.mode {
            HardwareMode::Simulated => {
                let initial = BatteryState {
                    soc_percent: initial_soc,
                    power_w: 0.0,
                    voltage_v: 48.0,
                    temperature_c: ambient_temp_c,
                    health_percent: 100.0,
                    status: BatteryStatus::Idle,
                };
                Arc::new(SimulatedBattery::new_with_ambient(initial, caps, ambient_temp_c))
            }
            #[cfg(feature = "modbus")]
            HardwareMode::Modbus => {
                if let Some(ref config) = self.config {
                    if let Some(ref modbus_config) = config.hardware.modbus {
                        // For now, use localhost with configured port
                        // TODO: Add device discovery or explicit host configuration
                        let addr = format!("127.0.0.1:{}", modbus_config.default_port);
                        match crate::hardware::modbus::ModbusBattery::new(
                            &addr,
                            modbus_config.default_unit_id,
                        )
                        .await
                        {
                            Ok(battery) => {
                                tracing::info!("Successfully connected to Modbus battery at {}", addr);
                                return Arc::new(battery);
                            }
                            Err(e) => {
                                tracing::error!(
                                    error = %e,
                                    "Failed to connect to Modbus battery, falling back to simulated"
                                );
                            }
                        }
                    }
                }

                tracing::warn!("Modbus battery not configured, falling back to simulated");
                let initial = BatteryState {
                    soc_percent: initial_soc,
                    power_w: 0.0,
                    voltage_v: 48.0,
                    temperature_c: ambient_temp_c,
                    health_percent: 100.0,
                    status: BatteryStatus::Idle,
                };
                Arc::new(SimulatedBattery::new_with_ambient(initial, caps, ambient_temp_c))
            }
            HardwareMode::Mock => {
                // TODO: Implement Mock battery with pre-programmed responses
                tracing::warn!("Mock battery not yet implemented, falling back to simulated");
                let initial = BatteryState {
                    soc_percent: initial_soc,
                    power_w: 0.0,
                    voltage_v: 48.0,
                    temperature_c: ambient_temp_c,
                    health_percent: 100.0,
                    status: BatteryStatus::Idle,
                };
                Arc::new(SimulatedBattery::new_with_ambient(initial, caps, ambient_temp_c))
            }
        }
    }

    /// Create an EV charger instance based on hardware mode
    pub fn create_ev_charger(&self) -> Arc<dyn EvCharger> {
        match self.mode {
            HardwareMode::Simulated => Arc::new(SimulatedEvCharger::default_charger()),
            #[cfg(feature = "modbus")]
            HardwareMode::Modbus => {
                // TODO: Implement OCPP EV charger
                tracing::warn!("OCPP charger not yet implemented, falling back to simulated");
                Arc::new(SimulatedEvCharger::default_charger())
            }
            HardwareMode::Mock => {
                tracing::warn!("Mock charger not yet implemented, falling back to simulated");
                Arc::new(SimulatedEvCharger::default_charger())
            }
        }
    }

    /// Create an inverter instance based on hardware mode
    pub fn create_inverter(&self, caps: InverterCapabilities) -> Arc<dyn Inverter> {
        match self.mode {
            HardwareMode::Simulated => {
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
                Arc::new(SimulatedInverter::new(initial, caps))
            }
            #[cfg(feature = "modbus")]
            HardwareMode::Modbus => {
                // TODO: Implement Modbus inverter
                tracing::warn!("Modbus inverter not yet implemented, falling back to simulated");
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
                Arc::new(SimulatedInverter::new(initial, caps))
            }
            HardwareMode::Mock => {
                tracing::warn!("Mock inverter not yet implemented, falling back to simulated");
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
                Arc::new(SimulatedInverter::new(initial, caps))
            }
        }
    }
}

impl Default for DeviceFactory {
    fn default() -> Self {
        Self::new(HardwareMode::Simulated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::BatteryChemistry;

    #[tokio::test]
    async fn test_factory_creates_simulated_battery() {
        let factory = DeviceFactory::new(HardwareMode::Simulated);
        let caps = BatteryCapabilities {
            capacity_kwh: 10.0,
            max_charge_kw: 5.0,
            max_discharge_kw: 5.0,
            efficiency: 0.92,
            degradation_per_cycle: 0.001,
            chemistry: BatteryChemistry::LiFePO4,
        };

        let battery = factory.create_battery(caps, 50.0, 25.0).await;
        let state = battery.read_state().await.unwrap();

        assert_eq!(state.soc_percent, 50.0);
        assert_eq!(state.power_w, 0.0);
    }

    #[tokio::test]
    async fn test_factory_creates_ev_charger() {
        let factory = DeviceFactory::new(HardwareMode::Simulated);
        let charger = factory.create_ev_charger();
        let state = charger.read_state().await.unwrap();

        assert!(!state.connected);
        assert!(!state.charging);
    }

    #[tokio::test]
    async fn test_factory_creates_inverter() {
        let factory = DeviceFactory::new(HardwareMode::Simulated);
        let caps = InverterCapabilities {
            rated_power_w: 10000.0,
            max_dc_input_w: 15000.0,
            max_ac_output_w: 10000.0,
            max_efficiency_percent: 97.5,
            mppt_channels: 2,
            supports_export_limit: true,
            supports_frequency_regulation: true,
        };

        let inverter = factory.create_inverter(caps);
        let state = inverter.read_state().await.unwrap();

        assert_eq!(state.mode, InverterMode::GridTied);
    }
}
