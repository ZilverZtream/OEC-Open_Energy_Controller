use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ThreePhaseLoad {
    pub l1_amps: f64,
    pub l2_amps: f64,
    pub l3_amps: f64,
}

impl ThreePhaseLoad {
    pub fn new(l1: f64, l2: f64, l3: f64) -> Self {
        Self {
            l1_amps: l1,
            l2_amps: l2,
            l3_amps: l3,
        }
    }

    pub fn balanced(total_amps: f64) -> Self {
        let per_phase = total_amps / 3.0;
        Self::new(per_phase, per_phase, per_phase)
    }

    pub fn single_phase(phase: u8, amps: f64) -> Self {
        match phase {
            1 => Self::new(amps, 0.0, 0.0),
            2 => Self::new(0.0, amps, 0.0),
            3 => Self::new(0.0, 0.0, amps),
            _ => Self::new(0.0, 0.0, 0.0),
        }
    }

    pub fn total_power_kw(&self, voltage_v: f64) -> f64 {
        (self.l1_amps + self.l2_amps + self.l3_amps) * voltage_v / 1000.0
    }
}

pub trait HvacSystem: Send + Sync {
    fn step(&mut self, dt_seconds: f64, indoor_temp: f64, outdoor_temp: f64) -> (ThreePhaseLoad, f64);
    fn name(&self) -> &str;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeothermalHeatPumpConfig {
    pub compressor_power_kw: f64,
    pub circulation_pump_power_kw: f64,
    pub ground_loop_temp_c: f64,
    pub cop_at_nominal: f64,
    pub target_temp_c: f64,
    pub hysteresis_c: f64,
    pub nominal_voltage_v: f64,
}

impl Default for GeothermalHeatPumpConfig {
    fn default() -> Self {
        Self {
            compressor_power_kw: 3.0,
            circulation_pump_power_kw: 0.15,
            ground_loop_temp_c: 4.0,
            cop_at_nominal: 4.5,
            target_temp_c: 21.0,
            hysteresis_c: 1.0,
            nominal_voltage_v: 230.0,
        }
    }
}

pub struct GeothermalHeatPump {
    config: GeothermalHeatPumpConfig,
    is_running: bool,
}

impl GeothermalHeatPump {
    pub fn new(config: GeothermalHeatPumpConfig) -> Self {
        Self {
            config,
            is_running: false,
        }
    }
}

impl HvacSystem for GeothermalHeatPump {
    fn step(&mut self, _dt_seconds: f64, indoor_temp: f64, _outdoor_temp: f64) -> (ThreePhaseLoad, f64) {
        if self.is_running {
            if indoor_temp > self.config.target_temp_c + self.config.hysteresis_c {
                self.is_running = false;
            }
        } else {
            if indoor_temp < self.config.target_temp_c - self.config.hysteresis_c {
                self.is_running = true;
            }
        }

        if self.is_running {
            let total_power = self.config.compressor_power_kw + self.config.circulation_pump_power_kw;
            let compressor_current = (self.config.compressor_power_kw * 1000.0) / self.config.nominal_voltage_v;
            let pump_current = (self.config.circulation_pump_power_kw * 1000.0) / self.config.nominal_voltage_v;

            let load = ThreePhaseLoad::new(
                compressor_current / 3.0 + pump_current,
                compressor_current / 3.0,
                compressor_current / 3.0,
            );

            let heat_output = total_power * self.config.cop_at_nominal;

            (load, heat_output)
        } else {
            let idle_power = self.config.circulation_pump_power_kw * 0.3;
            let idle_current = (idle_power * 1000.0) / self.config.nominal_voltage_v;
            let load = ThreePhaseLoad::single_phase(1, idle_current);

            (load, 0.0)
        }
    }

    fn name(&self) -> &str {
        "Geothermal Heat Pump (Bergvärme)"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AirHeatPumpConfig {
    pub nominal_power_kw: f64,
    pub target_temp_c: f64,
    pub hysteresis_c: f64,
    pub nominal_voltage_v: f64,
    pub electric_element_power_kw: f64,
    pub electric_element_threshold_c: f64,
}

impl Default for AirHeatPumpConfig {
    fn default() -> Self {
        Self {
            nominal_power_kw: 2.5,
            target_temp_c: 21.0,
            hysteresis_c: 1.0,
            nominal_voltage_v: 230.0,
            electric_element_power_kw: 5.0,
            electric_element_threshold_c: -15.0,
        }
    }
}

pub struct AirHeatPump {
    config: AirHeatPumpConfig,
    is_running: bool,
}

impl AirHeatPump {
    pub fn new(config: AirHeatPumpConfig) -> Self {
        Self {
            config,
            is_running: false,
        }
    }

    fn calculate_cop(&self, outdoor_temp: f64) -> f64 {
        if outdoor_temp >= 7.0 {
            4.2
        } else if outdoor_temp >= 0.0 {
            3.5 + (outdoor_temp / 7.0) * 0.7
        } else if outdoor_temp >= -10.0 {
            2.5 + ((outdoor_temp + 10.0) / 10.0) * 1.0
        } else if outdoor_temp >= -20.0 {
            1.5 + ((outdoor_temp + 20.0) / 10.0) * 1.0
        } else {
            1.2
        }
    }
}

impl HvacSystem for AirHeatPump {
    fn step(&mut self, _dt_seconds: f64, indoor_temp: f64, outdoor_temp: f64) -> (ThreePhaseLoad, f64) {
        if self.is_running {
            if indoor_temp > self.config.target_temp_c + self.config.hysteresis_c {
                self.is_running = false;
            }
        } else {
            if indoor_temp < self.config.target_temp_c - self.config.hysteresis_c {
                self.is_running = true;
            }
        }

        if self.is_running {
            let cop = self.calculate_cop(outdoor_temp);
            let mut power_draw = self.config.nominal_power_kw;
            let mut heat_output = power_draw * cop;

            if outdoor_temp < self.config.electric_element_threshold_c {
                power_draw += self.config.electric_element_power_kw;
                heat_output += self.config.electric_element_power_kw;
            }

            let current = (power_draw * 1000.0) / self.config.nominal_voltage_v;
            let load = ThreePhaseLoad::single_phase(1, current);

            (load, heat_output)
        } else {
            let load = ThreePhaseLoad::new(0.0, 0.0, 0.0);
            (load, 0.0)
        }
    }

    fn name(&self) -> &str {
        "Air-to-Air Heat Pump (Luftvärmepump)"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_geothermal_heat_pump() {
        let config = GeothermalHeatPumpConfig::default();
        let mut hp = GeothermalHeatPump::new(config);

        let (load, heat) = hp.step(60.0, 18.0, -10.0);
        assert!(load.total_power_kw(230.0) > 0.0);
        assert!(heat > 0.0);
    }

    #[test]
    fn test_air_heat_pump_cop_variation() {
        let config = AirHeatPumpConfig::default();
        let hp = AirHeatPump::new(config);

        let cop_warm = hp.calculate_cop(7.0);
        let cop_cold = hp.calculate_cop(-10.0);
        let cop_very_cold = hp.calculate_cop(-25.0);

        assert!(cop_warm > cop_cold);
        assert!(cop_cold > cop_very_cold);
        assert!(cop_very_cold >= 1.0);
    }

    #[test]
    fn test_three_phase_balanced() {
        let load = ThreePhaseLoad::balanced(30.0);
        assert_eq!(load.l1_amps, 10.0);
        assert_eq!(load.l2_amps, 10.0);
        assert_eq!(load.l3_amps, 10.0);
    }
}
