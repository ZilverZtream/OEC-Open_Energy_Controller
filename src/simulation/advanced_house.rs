use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
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

    pub fn tick(&mut self, new_time: NaiveDateTime, outdoor_temp_c: f64, solar_gain_w: f64, base_load_kw: f64) {
        let dt_seconds = (new_time - self.state.timestamp).num_seconds() as f64;
        if dt_seconds <= 0.0 {
            return;
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

        self.thermal_zone.step(dt_seconds, outdoor_temp_c, hvac_heat_output, solar_gain_w);

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
