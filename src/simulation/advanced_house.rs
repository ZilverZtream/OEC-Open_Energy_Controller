//! # Advanced House Simulation with Thermal Physics and HVAC
//!
//! This module provides comprehensive house simulation including:
//! - Thermal zone modeling (building physics, heat transfer)
//! - HVAC system integration (heat pumps, heating, cooling)
//! - Three-phase electrical load distribution
//! - Electrical base load
//!
//! ## vs. Simple HouseSimulator
//!
//! This is the **advanced** simulator for Swedish houses with heat pumps.
//! Unlike the simple `HouseSimulator` which only models electrical load,
//! `AdvancedHouseSimulator` models:
//! ✓ Indoor temperature dynamics
//! ✓ Heat pump operation (COP, defrost cycles, DHW priority)
//! ✓ Building thermal mass and insulation
//! ✓ Passive solar gain through windows
//! ✓ Three-phase load balancing
//!
//! ## CRITICAL: HVAC System Required
//!
//! To use `AdvancedHouseSimulator` properly, you MUST inject an HvacSystem:
//! - `GeothermalHeatPump` for bergvärme (ground source)
//! - `AirHeatPump` for luftvärmepump (air source)
//!
//! If you skip the HVAC system (pass `None`), the simulation will have:
//! - NO heating/cooling
//! - Indoor temperature will drift with outdoor temperature
//! - This defeats the purpose of using the advanced simulator!
//!
//! ## Example Usage
//!
//! ```ignore
//! use crate::simulation::advanced_house::{AdvancedHouseSimulator, AdvancedHouseConfig};
//! use crate::simulation::hvac::{GeothermalHeatPump, GeothermalHeatPumpConfig};
//!
//! // Create HVAC system (REQUIRED for realistic simulation)
//! let hvac_config = GeothermalHeatPumpConfig::default();
//! let hvac = Box::new(GeothermalHeatPump::new(hvac_config));
//!
//! // Create house simulator with HVAC
//! let house_config = AdvancedHouseConfig::default();
//! let mut house = AdvancedHouseSimulator::new(
//!     house_config,
//!     start_time,
//!     20.0,  // Initial indoor temp (°C)
//!     Some(hvac),  // CRITICAL: Inject HVAC system
//! );
//!
//! // Run simulation
//! house.tick(new_time, outdoor_temp, passive_solar_gain, base_electrical_load);
//!
//! // Get results
//! println!("Indoor temp: {}°C", house.indoor_temp_c());
//! println!("Total load: {} kW", house.total_load_kw());
//! println!("HVAC power: {} kW", house.state().hvac_power_kw);
//! ```

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use tracing;
use super::hvac::{HvacSystem, ThreePhaseLoad};
use super::thermal::{ThermalZone, ThermalZoneConfig};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedHouseConfig {
    pub thermal_zone: ThermalZoneConfig,
    pub enable_hvac: bool,
    pub target_temp_c: f64,
    pub nominal_voltage_v: f64,
}

impl Default for AdvancedHouseConfig {
    fn default() -> Self {
        Self {
            thermal_zone: ThermalZoneConfig::default(),
            enable_hvac: true,
            target_temp_c: 21.0,
            nominal_voltage_v: 230.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedHouseState {
    pub indoor_temp_c: f64,
    pub outdoor_temp_c: f64,
    pub hvac_power_kw: f64,
    pub hvac_heat_output_kw: f64,
    pub base_load_kw: f64,
    pub total_load_kw: f64,
    pub phase_loads: ThreePhaseLoad,
    pub timestamp: NaiveDateTime,
}

pub struct AdvancedHouseSimulator {
    config: AdvancedHouseConfig,
    thermal_zone: ThermalZone,
    hvac_system: Option<Box<dyn HvacSystem>>,
    state: AdvancedHouseState,
}

impl AdvancedHouseSimulator {
    pub fn new(
        config: AdvancedHouseConfig,
        start_time: NaiveDateTime,
        initial_indoor_temp: f64,
        hvac_system: Option<Box<dyn HvacSystem>>,
    ) -> Self {
        let thermal_zone = ThermalZone::new(config.thermal_zone.clone(), initial_indoor_temp);

        Self {
            config,
            thermal_zone,
            hvac_system,
            state: AdvancedHouseState {
                indoor_temp_c: initial_indoor_temp,
                outdoor_temp_c: 0.0,
                hvac_power_kw: 0.0,
                hvac_heat_output_kw: 0.0,
                base_load_kw: 0.5,
                total_load_kw: 0.5,
                phase_loads: ThreePhaseLoad::new(0.0, 0.0, 0.0),
                timestamp: start_time,
            },
        }
    }

    pub fn state(&self) -> &AdvancedHouseState {
        &self.state
    }

    pub fn indoor_temp_c(&self) -> f64 {
        self.state.indoor_temp_c
    }

    pub fn total_load_kw(&self) -> f64 {
        self.state.total_load_kw
    }

    pub fn phase_loads(&self) -> &ThreePhaseLoad {
        &self.state.phase_loads
    }

    /// Update the house simulation state
    ///
    /// # CRITICAL: Solar Gain vs Solar PV Production
    /// - `solar_irradiance_gain_w`: PASSIVE solar heat gain through windows (typically 0-2000W)
    ///   This is thermal energy from sunlight warming the house interior.
    /// - This is NOT PV production! Do not pass inverter output (kW electricity) here.
    /// - PV production is electrical power (measured at the inverter) and does NOT directly
    ///   heat the house. Only a tiny fraction becomes heat via electronics losses.
    ///
    /// # Arguments
    /// * `new_time` - New simulation timestamp
    /// * `outdoor_temp_c` - Outdoor air temperature in Celsius
    /// * `solar_irradiance_gain_w` - Passive solar heat gain through windows in Watts (0-2000W typical)
    /// * `base_load_kw` - Base electrical load of the house in kW
    ///
    /// # Example
    /// ```ignore
    /// // CORRECT: Passive solar gain from irradiance through windows
    /// let solar_gain_w = window_area_m2 * solar_irradiance_w_per_m2 * transmittance;
    /// house.tick(time, outdoor_temp, solar_gain_w, base_load);
    ///
    /// // WRONG: Do NOT pass PV production here!
    /// // house.tick(time, outdoor_temp, pv_inverter_output_w, base_load);  // BUG!
    /// ```
    pub fn tick(&mut self, new_time: NaiveDateTime, outdoor_temp_c: f64, solar_irradiance_gain_w: f64, base_load_kw: f64) {
        // CRITICAL FIX: Use num_milliseconds() instead of num_seconds() to avoid truncation
        // num_seconds() truncates milliseconds, causing simulation freeze at high control loop frequencies (e.g., 10Hz)
        let dt_seconds = (new_time - self.state.timestamp).num_milliseconds() as f64 / 1000.0;
        if dt_seconds <= 0.0 {
            return;
        }

        // SAFETY: Validate solar gain is in reasonable range for passive solar (not PV output)
        // Typical max: ~2kW for large south-facing windows on sunny winter day
        // If you see >3kW, someone likely passed PV production instead of passive gain!
        if solar_irradiance_gain_w > 3000.0 {
            tracing::warn!(
                "Solar gain {}W exceeds typical passive solar range (0-2000W). \
                 Verify this is passive solar irradiance through windows, not PV inverter output!",
                solar_irradiance_gain_w
            );
        }

        let indoor_temp = self.thermal_zone.indoor_temp_c();

        let (hvac_load, hvac_heat_output) = if let Some(hvac) = &mut self.hvac_system {
            if self.config.enable_hvac {
                hvac.step(dt_seconds, indoor_temp, outdoor_temp_c)
            } else {
                (ThreePhaseLoad::new(0.0, 0.0, 0.0), 0.0)
            }
        } else {
            (ThreePhaseLoad::new(0.0, 0.0, 0.0), 0.0)
        };

        self.thermal_zone.step(dt_seconds, outdoor_temp_c, hvac_heat_output, solar_irradiance_gain_w);

        let hvac_power_kw = hvac_load.total_power_kw(self.config.nominal_voltage_v);

        let base_load_per_phase = base_load_kw / 3.0;
        let base_current_per_phase = (base_load_per_phase * 1000.0) / self.config.nominal_voltage_v;

        let total_phase_loads = ThreePhaseLoad::new(
            hvac_load.l1_amps + base_current_per_phase,
            hvac_load.l2_amps + base_current_per_phase,
            hvac_load.l3_amps + base_current_per_phase,
        );

        let total_load_kw = total_phase_loads.total_power_kw(self.config.nominal_voltage_v);

        self.state = AdvancedHouseState {
            indoor_temp_c: self.thermal_zone.indoor_temp_c(),
            outdoor_temp_c,
            hvac_power_kw,
            hvac_heat_output_kw: hvac_heat_output / 1000.0,
            base_load_kw,
            total_load_kw,
            phase_loads: total_phase_loads,
            timestamp: new_time,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::hvac::{GeothermalHeatPump, GeothermalHeatPumpConfig};
    use chrono::NaiveDate;

    #[test]
    fn test_advanced_house_with_geothermal() {
        let config = AdvancedHouseConfig::default();
        let start_time = NaiveDate::from_ymd_opt(2024, 1, 15)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();

        let hvac = Box::new(GeothermalHeatPump::new(GeothermalHeatPumpConfig::default()));
        let mut house = AdvancedHouseSimulator::new(config, start_time, 18.0, Some(hvac));

        house.tick(
            start_time + chrono::Duration::hours(1),
            -10.0,
            0.0,
            0.5,
        );

        assert!(house.total_load_kw() > 0.0);
        assert!(house.phase_loads().l1_amps > 0.0);
    }
}
