use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThermalZoneConfig {
    pub floor_area_m2: f64,
    pub wall_u_value: f64,
    pub window_area_m2: f64,
    pub window_u_value: f64,
    pub ceiling_u_value: f64,
    pub floor_u_value: f64,
    pub air_changes_per_hour: f64,
    pub internal_gains_w: f64,
}

impl Default for ThermalZoneConfig {
    fn default() -> Self {
        Self {
            floor_area_m2: 150.0,
            wall_u_value: 0.18,
            window_area_m2: 20.0,
            window_u_value: 1.2,
            ceiling_u_value: 0.12,
            floor_u_value: 0.15,
            air_changes_per_hour: 0.5,
            internal_gains_w: 300.0,
        }
    }
}

impl ThermalZoneConfig {
    pub fn well_insulated() -> Self {
        Self {
            wall_u_value: 0.12,
            window_u_value: 0.8,
            ceiling_u_value: 0.08,
            floor_u_value: 0.10,
            air_changes_per_hour: 0.3,
            ..Default::default()
        }
    }

    pub fn poorly_insulated() -> Self {
        Self {
            wall_u_value: 0.40,
            window_u_value: 2.5,
            ceiling_u_value: 0.25,
            floor_u_value: 0.30,
            air_changes_per_hour: 1.0,
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThermalZoneState {
    pub indoor_temp_c: f64,
    pub heat_loss_w: f64,
    pub heat_gain_w: f64,
    pub net_heat_flow_w: f64,
}

pub struct ThermalZone {
    config: ThermalZoneConfig,
    state: ThermalZoneState,
}

impl ThermalZone {
    pub fn new(config: ThermalZoneConfig, initial_temp_c: f64) -> Self {
        Self {
            config,
            state: ThermalZoneState {
                indoor_temp_c: initial_temp_c,
                heat_loss_w: 0.0,
                heat_gain_w: 0.0,
                net_heat_flow_w: 0.0,
            },
        }
    }

    pub fn state(&self) -> &ThermalZoneState {
        &self.state
    }

    pub fn indoor_temp_c(&self) -> f64 {
        self.state.indoor_temp_c
    }

    pub fn step(&mut self, dt_seconds: f64, outdoor_temp_c: f64, hvac_heat_w: f64, solar_gain_w: f64) {
        const CEILING_HEIGHT_M: f64 = 2.5;
        const AIR_DENSITY: f64 = 1.2;
        const AIR_SPECIFIC_HEAT: f64 = 1005.0;

        let wall_area_m2 = (self.config.floor_area_m2.sqrt() * 4.0) * CEILING_HEIGHT_M;
        let ceiling_area_m2 = self.config.floor_area_m2;

        let temp_delta = self.state.indoor_temp_c - outdoor_temp_c;

        let wall_loss = wall_area_m2 * self.config.wall_u_value * temp_delta;
        let window_loss = self.config.window_area_m2 * self.config.window_u_value * temp_delta;
        let ceiling_loss = ceiling_area_m2 * self.config.ceiling_u_value * temp_delta;
        let floor_loss = ceiling_area_m2 * self.config.floor_u_value * temp_delta;

        let volume_m3 = self.config.floor_area_m2 * CEILING_HEIGHT_M;
        let ventilation_loss = (volume_m3 * self.config.air_changes_per_hour / 3600.0)
            * AIR_DENSITY
            * AIR_SPECIFIC_HEAT
            * temp_delta;

        let total_heat_loss = wall_loss + window_loss + ceiling_loss + floor_loss + ventilation_loss;

        let total_heat_gain = hvac_heat_w + self.config.internal_gains_w + solar_gain_w;

        let net_heat_flow = total_heat_gain - total_heat_loss;

        let thermal_mass_j_per_k = volume_m3 * AIR_DENSITY * AIR_SPECIFIC_HEAT * 50.0;
        let temp_change = (net_heat_flow * dt_seconds) / thermal_mass_j_per_k;

        self.state.indoor_temp_c = (self.state.indoor_temp_c + temp_change).clamp(-20.0, 40.0);
        self.state.heat_loss_w = total_heat_loss;
        self.state.heat_gain_w = total_heat_gain;
        self.state.net_heat_flow_w = net_heat_flow;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thermal_zone_cooling() {
        let config = ThermalZoneConfig::default();
        let mut zone = ThermalZone::new(config, 20.0);

        zone.step(3600.0, -10.0, 0.0, 0.0);

        assert!(zone.indoor_temp_c() < 20.0);
    }

    #[test]
    fn test_thermal_zone_heating() {
        let config = ThermalZoneConfig::default();
        let mut zone = ThermalZone::new(config, 15.0);

        zone.step(3600.0, -10.0, 5000.0, 0.0);

        assert!(zone.indoor_temp_c() > 15.0);
    }
}
