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

/// Dual-Node Hydronic Thermal Model for Swedish Floor Heating Systems
///
/// Models the critical 4-8 hour thermal lag of hydronic floor heating
/// (vattenburen golvvärme) embedded in concrete slabs. This is essential
/// for realistic simulation of 90% of modern Swedish homes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HydronicZoneConfig {
    /// Thermal mass of concrete slab (kWh/K)
    /// Typical: 5-10 kWh/K for 100m² concrete floor
    pub slab_thermal_mass_kwh_k: f64,

    /// Thermal mass of room air (kWh/K)
    /// Typical: 0.1-0.5 kWh/K (much smaller than slab)
    pub air_thermal_mass_kwh_k: f64,

    /// Thermal resistance from slab to air (K/kW)
    /// Controls how fast heat transfers from floor to room
    pub r_slab_to_air_k_per_kw: f64,

    /// Thermal resistance from air to outside (K/kW)
    /// Building envelope insulation
    pub r_air_to_out_k_per_kw: f64,

    /// Ground temperature below slab (°C)
    pub ground_temp_c: f64,

    /// Thermal resistance from slab to ground (K/kW)
    /// Heat loss downward through floor insulation
    pub r_slab_to_ground_k_per_kw: f64,

    /// Internal heat gains from occupants, appliances (kW)
    pub internal_gains_kw: f64,
}

impl Default for HydronicZoneConfig {
    fn default() -> Self {
        // Realistic Swedish house with 100m² floor, good insulation
        Self {
            slab_thermal_mass_kwh_k: 6.0,    // ~20 tons concrete
            air_thermal_mass_kwh_k: 0.3,     // Small air volume
            r_slab_to_air_k_per_kw: 2.0,     // Moderate heat transfer
            r_air_to_out_k_per_kw: 15.0,     // Well insulated
            ground_temp_c: 5.0,               // Typical Swedish ground temp
            r_slab_to_ground_k_per_kw: 40.0, // Good floor insulation
            internal_gains_kw: 0.3,           // 300W from people/devices
        }
    }
}

impl HydronicZoneConfig {
    /// Configuration for a well-insulated modern Swedish house
    pub fn well_insulated() -> Self {
        Self {
            r_air_to_out_k_per_kw: 20.0,     // Better insulation
            r_slab_to_ground_k_per_kw: 50.0, // Better floor insulation
            ..Default::default()
        }
    }

    /// Configuration for an older, poorly insulated house
    pub fn poorly_insulated() -> Self {
        Self {
            r_air_to_out_k_per_kw: 8.0,      // Worse insulation
            r_slab_to_ground_k_per_kw: 20.0, // Worse floor insulation
            ..Default::default()
        }
    }

    /// Configuration for a large house with massive concrete slab
    pub fn large_house() -> Self {
        Self {
            slab_thermal_mass_kwh_k: 12.0,   // ~40 tons concrete
            air_thermal_mass_kwh_k: 0.5,     // Larger volume
            internal_gains_kw: 0.5,           // More occupants/devices
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HydronicZoneState {
    /// Concrete slab temperature (°C)
    pub slab_temp_c: f64,
    /// Room air temperature (°C)
    pub air_temp_c: f64,
    /// Heat flow from slab to air (W)
    pub heat_flow_slab_to_air_w: f64,
    /// Heat loss from air to outside (W)
    pub heat_loss_air_to_out_w: f64,
    /// Heat loss from slab to ground (W)
    pub heat_loss_slab_to_ground_w: f64,
}

/// Dual-Node Hydronic Zone
///
/// This model captures the critical thermal lag of Swedish hydronic floor heating.
/// Heat input goes into the SLAB, which then slowly transfers heat to the AIR.
///
/// Physics:
/// - C_slab * dT_slab/dt = Q_hvac - (T_slab - T_air)/R_slab_air - (T_slab - T_ground)/R_slab_ground
/// - C_air * dT_air/dt = (T_slab - T_air)/R_slab_air - (T_air - T_out)/R_air_out + Q_internal
///
/// The large C_slab and resistance creates realistic 4-8 hour lag.
pub struct HydronicZone {
    config: HydronicZoneConfig,
    state: HydronicZoneState,
}

impl HydronicZone {
    pub fn new(config: HydronicZoneConfig, initial_slab_temp_c: f64, initial_air_temp_c: f64) -> Self {
        Self {
            config,
            state: HydronicZoneState {
                slab_temp_c: initial_slab_temp_c,
                air_temp_c: initial_air_temp_c,
                heat_flow_slab_to_air_w: 0.0,
                heat_loss_air_to_out_w: 0.0,
                heat_loss_slab_to_ground_w: 0.0,
            },
        }
    }

    pub fn state(&self) -> &HydronicZoneState {
        &self.state
    }

    /// Air temperature is what occupants feel
    pub fn air_temp_c(&self) -> f64 {
        self.state.air_temp_c
    }

    /// Slab temperature (usually 2-5°C warmer than air when heating)
    pub fn slab_temp_c(&self) -> f64 {
        self.state.slab_temp_c
    }

    /// Step the dual-node thermal model
    ///
    /// # Arguments
    /// * `dt_seconds` - Time step in seconds
    /// * `outdoor_temp_c` - Outside air temperature
    /// * `hvac_heat_w` - Heat power from HVAC system (goes into slab)
    /// * `solar_gain_w` - Solar heat gain (goes into air through windows)
    pub fn step(&mut self, dt_seconds: f64, outdoor_temp_c: f64, hvac_heat_w: f64, solar_gain_w: f64) {
        if dt_seconds <= 0.0 {
            return;
        }

        let dt_hours = dt_seconds / 3600.0;

        // Calculate heat flows (all in kW for easier math)
        let hvac_heat_kw = hvac_heat_w / 1000.0;
        let solar_gain_kw = solar_gain_w / 1000.0;

        // Heat flow from slab to air (kW)
        let heat_flow_slab_to_air_kw =
            (self.state.slab_temp_c - self.state.air_temp_c) / self.config.r_slab_to_air_k_per_kw;

        // Heat loss from slab to ground (kW)
        let heat_loss_slab_to_ground_kw =
            (self.state.slab_temp_c - self.config.ground_temp_c) / self.config.r_slab_to_ground_k_per_kw;

        // Heat loss from air to outside (kW)
        let heat_loss_air_to_out_kw =
            (self.state.air_temp_c - outdoor_temp_c) / self.config.r_air_to_out_k_per_kw;

        // Slab energy balance: Heat input - heat transfer to air - heat loss to ground
        let net_heat_slab_kw = hvac_heat_kw - heat_flow_slab_to_air_kw - heat_loss_slab_to_ground_kw;
        let delta_slab_temp = (net_heat_slab_kw * dt_hours) / self.config.slab_thermal_mass_kwh_k;

        // Air energy balance: Heat from slab + internal gains + solar - heat loss outside
        let net_heat_air_kw = heat_flow_slab_to_air_kw + self.config.internal_gains_kw + solar_gain_kw - heat_loss_air_to_out_kw;
        let delta_air_temp = (net_heat_air_kw * dt_hours) / self.config.air_thermal_mass_kwh_k;

        // Update temperatures
        self.state.slab_temp_c = (self.state.slab_temp_c + delta_slab_temp).clamp(-20.0, 50.0);
        self.state.air_temp_c = (self.state.air_temp_c + delta_air_temp).clamp(-30.0, 40.0);

        // Store heat flows for diagnostics (convert back to W)
        self.state.heat_flow_slab_to_air_w = heat_flow_slab_to_air_kw * 1000.0;
        self.state.heat_loss_air_to_out_w = heat_loss_air_to_out_kw * 1000.0;
        self.state.heat_loss_slab_to_ground_w = heat_loss_slab_to_ground_kw * 1000.0;
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

    #[test]
    fn test_hydronic_zone_thermal_lag() {
        // Test the critical 4-8 hour thermal lag
        let config = HydronicZoneConfig::default();
        let mut zone = HydronicZone::new(config, 18.0, 18.0);

        // Turn on heat pump at 3kW
        // With instant model, temperature would rise immediately
        // With hydronic model, temperature rises slowly

        // After 1 hour
        zone.step(3600.0, -10.0, 3000.0, 0.0);

        // Air temp should rise only slightly (< 2°C) due to thermal mass
        assert!(zone.air_temp_c() < 20.0, "Air temp should rise slowly due to slab thermal mass");
        assert!(zone.air_temp_c() > 18.0, "Air temp should rise a bit");

        // Slab should be warming up more than air
        assert!(zone.slab_temp_c() > zone.air_temp_c(), "Slab should be warmer than air when heating");
    }

    #[test]
    fn test_hydronic_zone_steady_state() {
        // Test that system reaches steady state
        let config = HydronicZoneConfig::default();
        let mut zone = HydronicZone::new(config, 18.0, 18.0);

        // Run for 24 hours with constant outdoor temp and heat input
        for _ in 0..24 {
            zone.step(3600.0, -5.0, 2000.0, 0.0);
        }

        // After 24 hours, should be near steady state
        // Slab and air temps should be closer together
        let temp_diff = (zone.slab_temp_c() - zone.air_temp_c()).abs();
        assert!(temp_diff < 3.0, "Slab and air temps should converge at steady state");

        // Both should be above initial temp
        assert!(zone.air_temp_c() > 18.0);
        assert!(zone.slab_temp_c() > 18.0);
    }

    #[test]
    fn test_hydronic_zone_cooling_lag() {
        // Test thermal lag when heating stops
        let config = HydronicZoneConfig::default();
        let mut zone = HydronicZone::new(config, 25.0, 22.0);

        // Turn off heating, outdoor temp is cold
        zone.step(3600.0, -10.0, 0.0, 0.0);

        // Slab releases stored heat to air, so air temp doesn't drop as fast
        assert!(zone.air_temp_c() > 21.0, "Air temp should drop slowly due to slab thermal buffer");

        // Slab should cool faster than air initially
        assert!(zone.slab_temp_c() < 25.0, "Slab should cool down");
    }
}
