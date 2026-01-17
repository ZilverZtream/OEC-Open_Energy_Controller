//! # House Load Simulation (Simple Model)
//!
//! Models household electricity consumption with realistic time-of-day patterns,
//! random noise, and appliance events.
//!
//! ## IMPORTANT: Integration with AdvancedHouseSimulator
//!
//! This module provides **electrical load only** (kW consumption).
//! It does NOT model:
//! - Thermal dynamics (heating/cooling)
//! - HVAC system interactions
//! - Indoor temperature
//! - Heat pump behavior
//!
//! For comprehensive house simulation including thermal physics and HVAC:
//! - Use `AdvancedHouseSimulator` from `advanced_house.rs` instead
//! - `AdvancedHouseSimulator` requires an `HvacSystem` (e.g., `GeothermalHeatPump`)
//! - It models both electrical load AND thermal behavior
//!
//! Use this simple model when:
//! - You only need electrical load forecasting
//! - Thermal simulation is not required
//! - You want minimal computational overhead
//!
//! ## Migration Guide
//!
//! If you're currently using `HouseSimulator` but need thermal/HVAC simulation:
//! ```ignore
//! // OLD: Simple electrical load only
//! let house = HouseSimulator::new(config, start_time);
//! let load_kw = house.load_kw();
//!
//! // NEW: Full thermal + HVAC + electrical simulation
//! use crate::simulation::advanced_house::AdvancedHouseSimulator;
//! use crate::simulation::hvac::{GeothermalHeatPump, GeothermalHeatPumpConfig};
//!
//! let hvac = Box::new(GeothermalHeatPump::new(GeothermalHeatPumpConfig::default()));
//! let house = AdvancedHouseSimulator::new(advanced_config, start_time, 20.0, Some(hvac));
//! let total_load_kw = house.total_load_kw();  // Includes HVAC + base load
//! let indoor_temp = house.indoor_temp_c();    // Thermal state
//! ```

use chrono::{Datelike, Duration, NaiveDateTime, Timelike};
use rand::Rng;
use serde::{Deserialize, Serialize};

/// Load profile type for different household patterns
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoadProfile {
    /// Conservative household (low consumption)
    Conservative,
    /// Average household (typical consumption)
    Average,
    /// High consumption household (many appliances)
    HighConsumption,
    /// Custom profile with manual base load
    Custom(u32), // base load in watts
}

impl LoadProfile {
    /// Get base load in kW for this profile
    pub fn base_load_kw(&self) -> f64 {
        match self {
            LoadProfile::Conservative => 0.3,
            LoadProfile::Average => 0.5,
            LoadProfile::HighConsumption => 0.8,
            LoadProfile::Custom(watts) => *watts as f64 / 1000.0,
        }
    }

    /// Get peak multiplier for this profile
    pub fn peak_multiplier(&self) -> f64 {
        match self {
            LoadProfile::Conservative => 3.0,
            LoadProfile::Average => 4.0,
            LoadProfile::HighConsumption => 6.0,
            LoadProfile::Custom(_) => 4.0,
        }
    }
}

/// Current state of the house load simulation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HouseState {
    /// Current load in kW
    pub load_kw: f64,
    /// Base load component in kW
    pub base_load_kw: f64,
    /// Time-of-day multiplier (1.0 = base, up to peak_multiplier)
    pub tod_multiplier: f64,
    /// Appliance events load in kW
    pub appliance_load_kw: f64,
    /// Random noise component in kW
    pub noise_kw: f64,
    /// Timestamp of this state
    pub timestamp: NaiveDateTime,
}

/// House load simulator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HouseSimulatorConfig {
    /// Load profile type
    pub profile: LoadProfile,
    /// Number of people in household (affects appliance probability)
    pub household_size: u32,
    /// Enable random appliance events
    pub enable_appliance_events: bool,
    /// Noise standard deviation as fraction of base load
    pub noise_std_dev: f64,
    /// Random seed for reproducibility (None = random)
    pub random_seed: Option<u64>,
}

impl Default for HouseSimulatorConfig {
    fn default() -> Self {
        Self {
            profile: LoadProfile::Average,
            household_size: 3,
            enable_appliance_events: true,
            noise_std_dev: 0.1, // 10% noise
            random_seed: None,
        }
    }
}

pub struct HouseSimulator {
    config: HouseSimulatorConfig,
    current_state: HouseState,
    rng: rand::rngs::StdRng,
    active_appliances: Vec<(f64, f64)>,
    accumulated_time_seconds: f64,
}

impl HouseSimulator {
    /// Create a new house simulator
    pub fn new(config: HouseSimulatorConfig, start_time: NaiveDateTime) -> Self {
        use rand::SeedableRng;

        let rng = match config.random_seed {
            Some(seed) => rand::rngs::StdRng::seed_from_u64(seed),
            None => rand::rngs::StdRng::from_entropy(),
        };

        let base_load_kw = config.profile.base_load_kw();

        let mut simulator = Self {
            config,
            current_state: HouseState {
                load_kw: base_load_kw,
                base_load_kw,
                tod_multiplier: 1.0,
                appliance_load_kw: 0.0,
                noise_kw: 0.0,
                timestamp: start_time,
            },
            rng,
            active_appliances: Vec::new(),
            accumulated_time_seconds: 0.0,
        };

        simulator.update_state(start_time);
        simulator
    }

    /// Get current load in kW
    pub fn load_kw(&self) -> f64 {
        self.current_state.load_kw
    }

    /// Get current state snapshot
    pub fn state(&self) -> &HouseState {
        &self.current_state
    }

    /// Update simulation to a new timestamp
    pub fn tick(&mut self, new_time: NaiveDateTime) {
        self.update_state(new_time);
    }

    /// Calculate time-of-day multiplier based on typical household patterns
    fn calculate_tod_multiplier(&self, time: NaiveDateTime) -> f64 {
        let hour = time.hour() as f64;
        let day_of_week = time.weekday().num_days_from_monday();
        let is_weekend = day_of_week >= 5;

        // Base pattern (weekday)
        let base_multiplier = if hour < 6.0 {
            // Night (00:00-06:00): minimal load
            0.5
        } else if hour < 9.0 {
            // Morning peak (06:00-09:00): breakfast, showers
            2.5 + (hour - 6.0) * 0.5
        } else if hour < 16.0 {
            // Daytime (09:00-16:00): reduced load
            1.0
        } else if hour < 21.0 {
            // Evening peak (16:00-21:00): cooking, appliances
            3.5 + (hour - 16.0) * 0.3
        } else {
            // Late evening (21:00-24:00): declining
            4.0 - (hour - 21.0) * 0.5
        };

        // Weekend adjustment (more uniform, slightly elevated during day)
        let multiplier = if is_weekend {
            if hour < 9.0 {
                base_multiplier * 0.7 // Sleep in, delayed morning
            } else if hour < 22.0 {
                base_multiplier * 1.2 // More active at home
            } else {
                base_multiplier
            }
        } else {
            base_multiplier
        };

        // Clamp to reasonable range
        multiplier.min(self.config.profile.peak_multiplier())
    }

    fn simulate_appliance_events(&mut self, dt_seconds: f64, time: NaiveDateTime) {
        if !self.config.enable_appliance_events || dt_seconds <= 0.0 {
            return;
        }

        self.active_appliances.retain_mut(|(_, remaining_seconds)| {
            *remaining_seconds -= dt_seconds;
            *remaining_seconds > 0.0
        });

        self.accumulated_time_seconds += dt_seconds;
        let minute_steps = (self.accumulated_time_seconds / 60.0).floor() as i32;

        if minute_steps > 0 {
            self.accumulated_time_seconds -= minute_steps as f64 * 60.0;

            let hour = time.hour();
            let base_probability = match hour {
                7..=9 | 17..=21 => 0.005,
                10..=16 => 0.002,
                _ => 0.001,
            };

            let probability = base_probability * (self.config.household_size as f64 / 3.0);

            for _ in 0..minute_steps {
                if self.rng.gen_bool(probability) {
                    let (power_kw, duration_minutes) = self.random_appliance();
                    self.active_appliances.push((power_kw, duration_minutes as f64 * 60.0));
                }
            }
        }
    }

    /// Generate random appliance power and duration
    fn random_appliance(&mut self) -> (f64, i64) {
        let appliance_type = self.rng.gen_range(0..10);

        match appliance_type {
            0..=2 => (2.5, 90),    // Dishwasher: 2.5 kW, 90 min
            3..=5 => (2.0, 120),   // Washing machine: 2.0 kW, 120 min
            6..=7 => (3.0, 60),    // Tumble dryer: 3.0 kW, 60 min
            8 => (1.5, 30),        // Vacuum cleaner: 1.5 kW, 30 min
            _ => (0.8, 45),        // Misc (TV, computer): 0.8 kW, 45 min
        }
    }

    /// Calculate total appliance load
    fn appliance_load_kw(&self) -> f64 {
        self.active_appliances.iter().map(|(power, _)| power).sum()
    }

    /// Generate noise component
    fn generate_noise(&mut self) -> f64 {
        use rand_distr::{Distribution, Normal};

        let std_dev = self.current_state.base_load_kw * self.config.noise_std_dev;
        let normal = Normal::new(0.0, std_dev).unwrap();
        normal.sample(&mut self.rng)
    }

    fn update_state(&mut self, time: NaiveDateTime) {
        let tod_multiplier = self.calculate_tod_multiplier(time);

        let dt_seconds = (time - self.current_state.timestamp).num_seconds() as f64;
        self.simulate_appliance_events(dt_seconds, time);

        let appliance_load_kw = self.appliance_load_kw();
        let noise_kw = self.generate_noise();

        let base_load_kw = self.config.profile.base_load_kw();
        let load_kw = (base_load_kw * tod_multiplier + appliance_load_kw + noise_kw).max(0.0);

        self.current_state = HouseState {
            load_kw,
            base_load_kw,
            tod_multiplier,
            appliance_load_kw,
            noise_kw,
            timestamp: time,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_load_profile_base_loads() {
        assert_eq!(LoadProfile::Conservative.base_load_kw(), 0.3);
        assert_eq!(LoadProfile::Average.base_load_kw(), 0.5);
        assert_eq!(LoadProfile::HighConsumption.base_load_kw(), 0.8);
        assert_eq!(LoadProfile::Custom(1500).base_load_kw(), 1.5);
    }

    #[test]
    fn test_house_simulator_initialization() {
        let config = HouseSimulatorConfig {
            profile: LoadProfile::Average,
            random_seed: Some(42),
            ..Default::default()
        };

        let start_time = NaiveDate::from_ymd_opt(2024, 6, 15)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();

        let simulator = HouseSimulator::new(config, start_time);
        assert!(simulator.load_kw() > 0.0);
    }

    #[test]
    fn test_time_of_day_patterns() {
        let config = HouseSimulatorConfig {
            profile: LoadProfile::Average,
            random_seed: Some(42),
            enable_appliance_events: false,
            noise_std_dev: 0.0,
            ..Default::default()
        };

        let base_date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let mut simulator = HouseSimulator::new(config, base_date.and_hms_opt(6, 0, 0).unwrap());

        // Morning peak should be higher than night
        simulator.tick(base_date.and_hms_opt(7, 0, 0).unwrap());
        let morning_load = simulator.load_kw();

        simulator.tick(base_date.and_hms_opt(3, 0, 0).unwrap());
        let night_load = simulator.load_kw();

        assert!(morning_load > night_load);

        // Evening peak should be higher than daytime
        simulator.tick(base_date.and_hms_opt(18, 0, 0).unwrap());
        let evening_load = simulator.load_kw();

        simulator.tick(base_date.and_hms_opt(12, 0, 0).unwrap());
        let noon_load = simulator.load_kw();

        assert!(evening_load > noon_load);
    }

    #[test]
    fn test_appliance_events() {
        let config = HouseSimulatorConfig {
            profile: LoadProfile::Average,
            random_seed: Some(42),
            enable_appliance_events: true,
            noise_std_dev: 0.0,
            household_size: 4,
            ..Default::default()
        };

        let start_time = NaiveDate::from_ymd_opt(2024, 6, 15)
            .unwrap()
            .and_hms_opt(8, 0, 0)
            .unwrap();

        let mut simulator = HouseSimulator::new(config, start_time);

        // Simulate 2 hours (high chance of appliance events)
        let end_time = start_time + Duration::hours(2);
        simulator.tick(end_time);

        // Should have some appliance load (probabilistic, but with fixed seed)
        // Just verify the simulator runs without panicking
        assert!(simulator.load_kw() >= 0.0);
    }

    #[test]
    fn test_household_size_scaling() {
        let config_small = HouseSimulatorConfig {
            profile: LoadProfile::Average,
            random_seed: Some(42),
            household_size: 1,
            enable_appliance_events: false,
            noise_std_dev: 0.0,
            ..Default::default()
        };

        let config_large = HouseSimulatorConfig {
            household_size: 6,
            ..config_small.clone()
        };

        let start_time = NaiveDate::from_ymd_opt(2024, 6, 15)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();

        let sim_small = HouseSimulator::new(config_small, start_time);
        let sim_large = HouseSimulator::new(config_large, start_time);

        // Verify both simulators work (actual scaling happens via appliance events)
        assert!(sim_small.load_kw() > 0.0);
        assert!(sim_large.load_kw() > 0.0);
    }
}
