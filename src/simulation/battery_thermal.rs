//! # Battery Thermal Model
//!
//! Physics-based thermal simulation for battery temperature dynamics.
//! Models heat generation during charge/discharge and thermal dissipation.
//!
//! ## Physics Model
//!
//! The battery thermal behavior follows a simple lumped thermal mass model:
//!
//! dT/dt = (Q_gen - Q_loss) / (m * c_p)
//!
//! Where:
//! - T = battery temperature (°C)
//! - Q_gen = heat generation from resistive losses (W)
//! - Q_loss = heat dissipation to ambient (W)
//! - m = battery thermal mass (kg)
//! - c_p = specific heat capacity (J/kg·K)
//!
//! Heat generation: Q_gen = I² * R_internal
//! Heat loss: Q_loss = h * A * (T_battery - T_ambient)
//!
//! ## Temperature Effects
//!
//! - **Efficiency**: Drops at low temps (<10°C), optimal at 15-25°C
//! - **Degradation**: Accelerates significantly above 35°C
//! - **Safety**: Critical threshold at 60°C (thermal runaway risk)

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Battery thermal model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatteryThermalConfig {
    /// Battery thermal mass (kg) - typical LiFePO4: 50-100 kg for home systems
    pub thermal_mass_kg: f64,

    /// Specific heat capacity (J/kg·K) - LiFePO4: ~1100 J/kg·K
    pub specific_heat_j_per_kg_k: f64,

    /// Internal resistance (Ohm) - affects heat generation
    pub internal_resistance_ohm: f64,

    /// Heat transfer coefficient (W/K) - depends on enclosure and cooling
    pub heat_transfer_coeff_w_per_k: f64,

    /// Ambient temperature (°C)
    pub ambient_temp_c: f64,

    /// Minimum safe operating temperature (°C)
    pub min_operating_temp_c: f64,

    /// Maximum safe operating temperature (°C)
    pub max_operating_temp_c: f64,

    /// Critical overheat threshold (°C) - triggers emergency shutdown
    pub critical_temp_c: f64,
}

impl Default for BatteryThermalConfig {
    fn default() -> Self {
        Self {
            thermal_mass_kg: 75.0,                  // Typical 10 kWh LiFePO4 battery
            specific_heat_j_per_kg_k: 1100.0,       // LiFePO4 specific heat
            internal_resistance_ohm: 0.05,          // Typical for home battery
            heat_transfer_coeff_w_per_k: 15.0,      // Natural convection + forced air
            ambient_temp_c: 15.0,                   // Nordic garage temperature
            min_operating_temp_c: -10.0,            // Cold limit
            max_operating_temp_c: 50.0,             // Hot limit
            critical_temp_c: 60.0,                  // Emergency shutdown
        }
    }
}

impl BatteryThermalConfig {
    /// Create config for cold climate (Nordic winter)
    pub fn cold_climate() -> Self {
        Self {
            ambient_temp_c: -10.0,
            ..Default::default()
        }
    }

    /// Create config for warm climate (Mediterranean summer)
    pub fn warm_climate() -> Self {
        Self {
            ambient_temp_c: 30.0,
            ..Default::default()
        }
    }

    /// Create config for outdoor installation with poor cooling
    pub fn outdoor_poor_cooling() -> Self {
        Self {
            heat_transfer_coeff_w_per_k: 8.0, // Poor cooling
            ambient_temp_c: 20.0,
            ..Default::default()
        }
    }

    /// Create config for indoor with active cooling
    pub fn indoor_active_cooling() -> Self {
        Self {
            heat_transfer_coeff_w_per_k: 30.0, // Good active cooling
            ambient_temp_c: 20.0,
            ..Default::default()
        }
    }
}

/// Current thermal state of the battery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatteryThermalState {
    /// Current battery temperature (°C)
    pub temperature_c: f64,

    /// Rate of temperature change (°C/s)
    pub temp_rate_c_per_s: f64,

    /// Heat generation from current operation (W)
    pub heat_generation_w: f64,

    /// Heat dissipation to ambient (W)
    pub heat_dissipation_w: f64,

    /// Temperature-dependent efficiency factor (0.0-1.0)
    pub efficiency_factor: f64,

    /// Temperature-dependent degradation multiplier (1.0 = normal, >1.0 = accelerated)
    pub degradation_multiplier: f64,

    /// Whether battery is in safe operating range
    pub temp_in_safe_range: bool,

    /// Whether critical temperature reached (emergency stop required)
    pub critical_temp_reached: bool,
}

/// Battery thermal simulator
pub struct BatteryThermalSimulator {
    config: BatteryThermalConfig,
    state: BatteryThermalState,
}

impl BatteryThermalSimulator {
    /// Create a new thermal simulator
    pub fn new(config: BatteryThermalConfig) -> Self {
        let initial_temp = config.ambient_temp_c;

        Self {
            state: BatteryThermalState {
                temperature_c: initial_temp,
                temp_rate_c_per_s: 0.0,
                heat_generation_w: 0.0,
                heat_dissipation_w: 0.0,
                efficiency_factor: Self::calculate_efficiency_factor(initial_temp),
                degradation_multiplier: Self::calculate_degradation_multiplier(initial_temp),
                temp_in_safe_range: true,
                critical_temp_reached: false,
            },
            config,
        }
    }

    /// Get current thermal state
    pub fn state(&self) -> &BatteryThermalState {
        &self.state
    }

    /// Get current battery temperature
    pub fn temperature_c(&self) -> f64 {
        self.state.temperature_c
    }

    /// Update thermal simulation based on battery power flow
    ///
    /// # Arguments
    /// * `power_w` - Current battery power (positive = discharge, negative = charge)
    /// * `voltage_v` - Battery voltage
    /// * `delta` - Time step duration
    pub fn update(&mut self, power_w: f64, voltage_v: f64, delta: Duration) {
        let delta_secs = delta.as_secs_f64();
        if delta_secs <= 0.0 {
            return;
        }

        // Calculate current: I = P / V
        let current_a = if voltage_v > 0.0 {
            power_w / voltage_v
        } else {
            0.0
        };

        // CRITICAL PHYSICS FIX: Include both resistive AND entropic heat generation
        // Previous code only modeled I²R (ohmic) heating
        // Real batteries, especially cold ones, generate massive heat from chemical inefficiency

        // 1. Resistive (I²R) heat from internal resistance
        let resistive_heat_w = current_a.powi(2) * self.config.internal_resistance_ohm;

        // 2. Entropic heat from chemical inefficiency (P × (1 - η))
        //    This is the "missing" heat that was being ignored
        //    At -10°C with 60% efficiency, a 5kW charge generates:
        //      - I²R:      ~156W (negligible)
        //      - Entropic: 2000W (massive!)
        //    This self-heating effect is critical for cold-weather operation
        let temp_c = self.state.temperature_c;
        let efficiency_factor = Self::calculate_efficiency_factor(temp_c);
        let entropic_heat_w = power_w.abs() * (1.0 - efficiency_factor);

        // Total heat generation
        let heat_gen_w = resistive_heat_w + entropic_heat_w;

        // Heat dissipation: Q_loss = h * (T - T_ambient)
        // Simplified model: no explicit area, absorbed into h coefficient
        let temp_diff = self.state.temperature_c - self.config.ambient_temp_c;
        let heat_loss_w = self.config.heat_transfer_coeff_w_per_k * temp_diff;

        // Net heat flow
        let net_heat_w = heat_gen_w - heat_loss_w;

        // Thermal capacity: C = m * c_p
        let thermal_capacity_j_per_k =
            self.config.thermal_mass_kg * self.config.specific_heat_j_per_kg_k;

        // Temperature rate: dT/dt = Q_net / C
        let temp_rate_c_per_s = net_heat_w / thermal_capacity_j_per_k;

        // Update temperature
        let new_temp = self.state.temperature_c + (temp_rate_c_per_s * delta_secs);

        // Calculate temperature-dependent performance factors
        let efficiency_factor = Self::calculate_efficiency_factor(new_temp);
        let degradation_multiplier = Self::calculate_degradation_multiplier(new_temp);

        // Safety checks
        let temp_in_safe_range = new_temp >= self.config.min_operating_temp_c
            && new_temp <= self.config.max_operating_temp_c;
        let critical_temp_reached = new_temp >= self.config.critical_temp_c;

        self.state = BatteryThermalState {
            temperature_c: new_temp,
            temp_rate_c_per_s,
            heat_generation_w: heat_gen_w,
            heat_dissipation_w: heat_loss_w,
            efficiency_factor,
            degradation_multiplier,
            temp_in_safe_range,
            critical_temp_reached,
        };
    }

    /// Calculate efficiency factor based on temperature
    ///
    /// Efficiency degrades at cold temperatures and slightly at hot temperatures:
    /// - Below 0°C: 60% efficiency
    /// - 0-10°C: 70-90% efficiency
    /// - 10-30°C: 95-100% efficiency (optimal)
    /// - 30-40°C: 90-95% efficiency
    /// - Above 40°C: 85-90% efficiency
    fn calculate_efficiency_factor(temp_c: f64) -> f64 {
        match temp_c {
            t if t < 0.0 => 0.6 + (t + 10.0) * 0.01, // 60% at -10°C, rising
            t if t < 10.0 => 0.7 + (t / 10.0) * 0.25, // 70% at 0°C to 95% at 10°C
            t if t < 30.0 => 0.95 + ((20.0 - (t - 10.0).abs()) / 20.0) * 0.05, // Peak 100% at 20°C
            t if t < 40.0 => 0.95 - ((t - 30.0) / 10.0) * 0.05, // 95% at 30°C to 90% at 40°C
            t => 0.90 - ((t - 40.0) / 20.0) * 0.05, // Declining above 40°C
        }
        .clamp(0.5, 1.0)
    }

    /// Calculate degradation multiplier based on temperature
    ///
    /// Battery degradation accelerates at high temperatures:
    /// - Below 25°C: Normal degradation (1.0x)
    /// - 25-35°C: 1.0-2.0x
    /// - 35-45°C: 2.0-4.0x
    /// - Above 45°C: 4.0-8.0x
    fn calculate_degradation_multiplier(temp_c: f64) -> f64 {
        match temp_c {
            t if t < 25.0 => 1.0,
            t if t < 35.0 => 1.0 + ((t - 25.0) / 10.0) * 1.0, // 1x to 2x
            t if t < 45.0 => 2.0 + ((t - 35.0) / 10.0) * 2.0, // 2x to 4x
            t => 4.0 + ((t - 45.0) / 10.0) * 4.0,              // 4x to 8x+
        }
        .clamp(1.0, 10.0)
    }

    /// Set ambient temperature
    pub fn set_ambient_temp(&mut self, temp_c: f64) {
        self.config.ambient_temp_c = temp_c;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thermal_initialization() {
        let config = BatteryThermalConfig::default();
        let sim = BatteryThermalSimulator::new(config.clone());

        assert!((sim.temperature_c() - config.ambient_temp_c).abs() < 0.01);
        assert!(sim.state().temp_in_safe_range);
        assert!(!sim.state().critical_temp_reached);
    }

    #[test]
    fn test_heat_generation_during_charge() {
        let config = BatteryThermalConfig::default();
        let mut sim = BatteryThermalSimulator::new(config);

        let initial_temp = sim.temperature_c();

        // Charge at 5 kW for 1 hour
        for _ in 0..60 {
            sim.update(-5000.0, 400.0, Duration::from_secs(60));
        }

        // Temperature should increase due to resistive heating
        assert!(sim.temperature_c() > initial_temp);
        assert!(sim.state().heat_generation_w > 0.0);
    }

    #[test]
    fn test_heat_generation_during_discharge() {
        let config = BatteryThermalConfig::default();
        let mut sim = BatteryThermalSimulator::new(config);

        let initial_temp = sim.temperature_c();

        // Discharge at 5 kW for 1 hour
        for _ in 0..60 {
            sim.update(5000.0, 400.0, Duration::from_secs(60));
        }

        // Temperature should increase
        assert!(sim.temperature_c() > initial_temp);
        assert!(sim.state().heat_generation_w > 0.0);
    }

    #[test]
    fn test_thermal_equilibrium() {
        let config = BatteryThermalConfig::default();
        let mut sim = BatteryThermalSimulator::new(config.clone());

        // No power flow - should stay at ambient
        for _ in 0..120 {
            sim.update(0.0, 400.0, Duration::from_secs(60));
        }

        // Should remain close to ambient
        assert!((sim.temperature_c() - config.ambient_temp_c).abs() < 1.0);
    }

    #[test]
    fn test_efficiency_factor_at_cold_temp() {
        let mut config = BatteryThermalConfig::default();
        config.ambient_temp_c = -10.0;
        let sim = BatteryThermalSimulator::new(config);

        // At -10°C, efficiency should be reduced
        assert!(sim.state().efficiency_factor < 0.8);
    }

    #[test]
    fn test_efficiency_factor_at_optimal_temp() {
        let mut config = BatteryThermalConfig::default();
        config.ambient_temp_c = 20.0;
        let sim = BatteryThermalSimulator::new(config);

        // At 20°C, efficiency should be optimal (~100%)
        assert!(sim.state().efficiency_factor > 0.95);
    }

    #[test]
    fn test_degradation_at_high_temp() {
        let mut config = BatteryThermalConfig::default();
        config.ambient_temp_c = 40.0;
        let mut sim = BatteryThermalSimulator::new(config);

        // Heat battery to 40°C
        for _ in 0..120 {
            sim.update(5000.0, 400.0, Duration::from_secs(60));
        }

        // Degradation should be accelerated
        if sim.temperature_c() >= 35.0 {
            assert!(sim.state().degradation_multiplier > 1.5);
        }
    }

    #[test]
    fn test_critical_temp_detection() {
        let mut config = BatteryThermalConfig::default();
        config.critical_temp_c = 50.0;
        config.ambient_temp_c = 45.0;
        let critical_temp = config.critical_temp_c;
        let mut sim = BatteryThermalSimulator::new(config);

        // Apply heavy load to heat battery
        for _ in 0..300 {
            sim.update(10000.0, 400.0, Duration::from_secs(60));
            if sim.state().critical_temp_reached {
                break;
            }
        }

        // Should eventually reach critical temperature
        if sim.temperature_c() >= critical_temp {
            assert!(sim.state().critical_temp_reached);
        }
    }

    #[test]
    fn test_cold_climate_config() {
        let config = BatteryThermalConfig::cold_climate();
        let sim = BatteryThermalSimulator::new(config);

        assert!(sim.temperature_c() < 0.0);
        // Cold temp should reduce efficiency
        assert!(sim.state().efficiency_factor < 0.85);
    }

    #[test]
    fn test_thermal_capacity_scaling() {
        // Smaller battery should heat faster
        let mut small_config = BatteryThermalConfig::default();
        small_config.thermal_mass_kg = 30.0; // Half the mass

        let mut large_config = BatteryThermalConfig::default();
        large_config.thermal_mass_kg = 150.0; // Double the mass

        let mut small_sim = BatteryThermalSimulator::new(small_config);
        let mut large_sim = BatteryThermalSimulator::new(large_config);

        // Apply same power for same duration
        small_sim.update(5000.0, 400.0, Duration::from_secs(3600));
        large_sim.update(5000.0, 400.0, Duration::from_secs(3600));

        // Smaller battery should have higher temperature rise
        let small_rise = small_sim.temperature_c() - small_sim.config.ambient_temp_c;
        let large_rise = large_sim.temperature_c() - large_sim.config.ambient_temp_c;

        assert!(small_rise > large_rise);
    }
}
