//! # Environment Orchestrator
//!
//! The master simulation component that coordinates all environmental factors
//! (house load, solar production, grid conditions) and advances the simulation clock.

use super::{
    GridSimulator, GridSimulatorConfig, GridState, HouseSimulator, HouseSimulatorConfig,
    HouseState, SolarSimulator, SolarSimulatorConfig, SolarState,
};
use chrono::{Duration, NaiveDateTime};
use serde::{Deserialize, Serialize};

/// Complete environment state snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentState {
    /// House load state
    pub house: HouseState,
    /// Solar production state
    pub solar: SolarState,
    /// Grid state
    pub grid: GridState,
    /// Net load (house - solar) in kW
    pub net_load_kw: f64,
    /// Current timestamp
    pub timestamp: NaiveDateTime,
}

/// Environment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentConfig {
    /// House simulator configuration
    pub house: HouseSimulatorConfig,
    /// Solar simulator configuration
    pub solar: SolarSimulatorConfig,
    /// Grid simulator configuration
    pub grid: GridSimulatorConfig,
    /// Starting timestamp for simulation
    pub start_time: NaiveDateTime,
}

impl Default for EnvironmentConfig {
    fn default() -> Self {
        use chrono::NaiveDate;

        Self {
            house: HouseSimulatorConfig::default(),
            solar: SolarSimulatorConfig::default(),
            grid: GridSimulatorConfig::default(),
            start_time: NaiveDate::from_ymd_opt(2024, 6, 15)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap(),
        }
    }
}

impl EnvironmentConfig {
    /// Create a new configuration for a specific location
    pub fn for_location(
        latitude: f64,
        longitude: f64,
        timezone_offset: i32,
        start_time: NaiveDateTime,
    ) -> Self {
        let mut config = Self::default();
        config.solar.latitude_deg = latitude;
        config.solar.longitude_deg = longitude;
        config.solar.timezone_offset = timezone_offset;
        config.start_time = start_time;
        config
    }

    /// Set PV capacity
    pub fn with_pv_capacity(mut self, capacity_kw: f64) -> Self {
        self.solar.capacity_kw = capacity_kw;
        self
    }

    /// Set household size
    pub fn with_household_size(mut self, size: u32) -> Self {
        self.house.household_size = size;
        self
    }

    /// Set fuse rating
    pub fn with_fuse_rating(mut self, rating_a: f64) -> Self {
        self.grid.fuse_rating_a = rating_a;
        self
    }

    /// Enable/disable faults
    pub fn with_faults(mut self, enable: bool) -> Self {
        self.grid.enable_faults = enable;
        self
    }

    /// Set random seed for all components
    pub fn with_random_seed(mut self, seed: u64) -> Self {
        self.house.random_seed = Some(seed);
        self.solar.random_seed = Some(seed + 1);
        self.grid.random_seed = Some(seed + 2);
        self
    }
}

/// Master environment simulator
///
/// Coordinates house load, solar production, and grid simulations.
/// Provides a unified interface to query environmental conditions.
pub struct Environment {
    house_sim: HouseSimulator,
    solar_sim: SolarSimulator,
    grid_sim: GridSimulator,
    current_time: NaiveDateTime,
}

impl Environment {
    /// Create a new environment simulator
    pub fn new(config: EnvironmentConfig) -> Self {
        let start_time = config.start_time;

        Self {
            house_sim: HouseSimulator::new(config.house, start_time),
            solar_sim: SolarSimulator::new(config.solar, start_time),
            grid_sim: GridSimulator::new(config.grid, start_time),
            current_time: start_time,
        }
    }

    /// Get current timestamp
    pub fn current_time(&self) -> NaiveDateTime {
        self.current_time
    }

    /// Get current house load in kW
    pub fn house_load_kw(&self) -> f64 {
        self.house_sim.load_kw()
    }

    /// Get current solar production in kW
    pub fn solar_production_kw(&self) -> f64 {
        self.solar_sim.production_kw()
    }

    /// Get current grid frequency in Hz
    pub fn grid_frequency_hz(&self) -> f64 {
        self.grid_sim.frequency_hz()
    }

    /// Get current grid voltage in V
    pub fn grid_voltage_v(&self) -> f64 {
        self.grid_sim.voltage_v()
    }

    /// Check if grid is available
    pub fn grid_available(&self) -> bool {
        self.grid_sim.is_available()
    }

    /// Get net load (house - solar) in kW
    pub fn net_load_kw(&self) -> f64 {
        (self.house_load_kw() - self.solar_production_kw()).max(0.0)
    }

    /// Get current complete environment state
    pub fn state(&self) -> EnvironmentState {
        let house_load = self.house_load_kw();
        let solar_production = self.solar_production_kw();
        let net_load = (house_load - solar_production).max(0.0);

        EnvironmentState {
            house: self.house_sim.state().clone(),
            solar: self.solar_sim.state().clone(),
            grid: self.grid_sim.state().clone(),
            net_load_kw: net_load,
            timestamp: self.current_time,
        }
    }

    /// Advance the simulation by a duration
    ///
    /// Updates all sub-simulators (house, solar, grid) and advances the clock.
    ///
    /// # Arguments
    /// * `delta` - Duration to advance
    /// * `grid_import_kw` - Power being imported from grid (for fuse trip detection)
    /// * `grid_export_kw` - Power being exported to grid
    pub fn tick(&mut self, delta: Duration, grid_import_kw: f64, grid_export_kw: f64) {
        self.current_time += delta;

        // Update all sub-simulators
        self.house_sim.tick(self.current_time);
        self.solar_sim.tick(self.current_time);
        self.grid_sim
            .tick(self.current_time, grid_import_kw, grid_export_kw);
    }

    /// Advance to a specific timestamp
    ///
    /// # Arguments
    /// * `new_time` - Target timestamp
    /// * `grid_import_kw` - Power being imported from grid
    /// * `grid_export_kw` - Power being exported to grid
    pub fn advance_to(&mut self, new_time: NaiveDateTime, grid_import_kw: f64, grid_export_kw: f64) {
        if new_time <= self.current_time {
            return;
        }

        let delta = new_time - self.current_time;
        self.tick(delta, grid_import_kw, grid_export_kw);
    }

    /// Run simulation for a full day (24 hours) with a given time step
    ///
    /// Returns a vector of environment states at each time step.
    ///
    /// # Arguments
    /// * `step_minutes` - Time step in minutes
    /// * `power_callback` - Callback function that computes grid import/export for each state
    pub fn simulate_day<F>(
        &mut self,
        step_minutes: i64,
        mut power_callback: F,
    ) -> Vec<EnvironmentState>
    where
        F: FnMut(&EnvironmentState) -> (f64, f64), // Returns (import_kw, export_kw)
    {
        let mut states = Vec::new();
        let end_time = self.current_time + Duration::days(1);

        while self.current_time < end_time {
            // Get current state
            let state = self.state();

            // Compute grid power using callback
            let (grid_import, grid_export) = power_callback(&state);

            // Store state
            states.push(state);

            // Advance simulation
            self.tick(Duration::minutes(step_minutes), grid_import, grid_export);
        }

        states
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, Timelike};

    #[test]
    fn test_environment_initialization() {
        let config = EnvironmentConfig::default().with_random_seed(42);
        let env = Environment::new(config);

        assert!(env.house_load_kw() > 0.0);
        assert!(env.solar_production_kw() >= 0.0);
        assert!(env.grid_available());
    }

    #[test]
    fn test_environment_tick() {
        let config = EnvironmentConfig::default().with_random_seed(42);
        let mut env = Environment::new(config);

        let initial_time = env.current_time();
        env.tick(Duration::hours(1), 2.0, 0.0);

        assert_eq!(env.current_time(), initial_time + Duration::hours(1));
    }

    #[test]
    fn test_net_load_calculation() {
        let start_time = NaiveDate::from_ymd_opt(2024, 6, 15)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap(); // Noon, should have solar

        let config = EnvironmentConfig::default()
            .with_random_seed(42)
            .with_pv_capacity(5.0);

        let mut env = Environment::new(config);
        env.advance_to(start_time, 0.0, 0.0);

        let house_load = env.house_load_kw();
        let solar_prod = env.solar_production_kw();
        let net_load = env.net_load_kw();

        // Net load should be house - solar (or 0 if solar exceeds house)
        let expected_net = (house_load - solar_prod).max(0.0);
        assert!((net_load - expected_net).abs() < 0.01);
    }

    #[test]
    fn test_location_configuration() {
        let start_time = NaiveDate::from_ymd_opt(2024, 6, 21)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();

        // Stockholm coordinates
        let config = EnvironmentConfig::for_location(59.3293, 18.0686, 1, start_time)
            .with_random_seed(42)
            .with_pv_capacity(5.0);

        let env = Environment::new(config);

        // At noon in summer, solar should be producing
        assert!(env.solar_production_kw() > 1.0);
    }

    #[test]
    fn test_simulate_day() {
        let start_time = NaiveDate::from_ymd_opt(2024, 6, 15)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();

        let config = EnvironmentConfig::default()
            .with_random_seed(42)
            .with_pv_capacity(5.0)
            .with_faults(false); // Disable faults for predictable test

        let mut env = Environment::new(config);
        env.advance_to(start_time, 0.0, 0.0);

        // Simulate a day with 1-hour steps
        let states = env.simulate_day(60, |state| {
            // Simple callback: import net load, export surplus
            let net_load = state.net_load_kw;
            if net_load > 0.0 {
                (net_load, 0.0) // Import needed power
            } else {
                (0.0, -net_load) // Export surplus
            }
        });

        // Should have 24 states (one per hour)
        assert_eq!(states.len(), 24);

        // Find the state with maximum solar production
        let max_solar_state = states
            .iter()
            .max_by(|a, b| a.solar.production_kw.partial_cmp(&b.solar.production_kw).unwrap())
            .unwrap();

        // Maximum solar should be around midday
        let max_solar_hour = max_solar_state.timestamp.hour();
        assert!(max_solar_hour >= 11 && max_solar_hour <= 14);
    }

    #[test]
    fn test_daily_cycle() {
        let start_time = NaiveDate::from_ymd_opt(2024, 6, 15)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();

        let config = EnvironmentConfig::default()
            .with_random_seed(42)
            .with_pv_capacity(5.0);

        let mut env = Environment::new(config);
        env.advance_to(start_time, 0.0, 0.0);

        // Sample at different times of day
        let mut samples = Vec::new();

        for hour in [0, 6, 12, 18, 23] {
            let time = start_time
                .checked_add_signed(Duration::hours(hour))
                .unwrap();
            env.advance_to(time, 0.0, 0.0);

            samples.push((hour, env.house_load_kw(), env.solar_production_kw()));
        }

        // Night (hour 0): minimal house load, no solar
        assert!(samples[0].1 < 1.0);
        assert!(samples[0].2 < 0.1);

        // Morning (hour 6): increasing house load, some solar
        assert!(samples[1].1 > samples[0].1);

        // Noon (hour 12): peak solar
        let noon_solar = samples[2].2;
        assert!(noon_solar > samples[1].2);
        assert!(noon_solar > samples[3].2);

        // Evening (hour 18): high house load, declining solar
        assert!(samples[3].1 > samples[2].1);

        // Night (hour 23): moderate house load, no solar
        assert!(samples[4].2 < 0.1);
    }

    #[test]
    fn test_grid_fault_impact() {
        let start_time = NaiveDate::from_ymd_opt(2024, 6, 15)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();

        let config = EnvironmentConfig::default()
            .with_random_seed(42)
            .with_faults(false); // Start with no faults

        let mut env = Environment::new(config);
        env.advance_to(start_time, 0.0, 0.0);

        // Grid should be available
        assert!(env.grid_available());

        // Trigger fuse trip by excessive import
        env.tick(Duration::minutes(1), 30.0, 0.0);

        // Grid should now be unavailable
        let state = env.state();
        assert!(!state.grid.is_available);
    }

    #[test]
    fn test_environment_state_snapshot() {
        let config = EnvironmentConfig::default().with_random_seed(42);
        let env = Environment::new(config);

        let state = env.state();

        // Verify state completeness
        assert!(state.house.load_kw > 0.0);
        assert!(state.solar.production_kw >= 0.0);
        assert!(state.grid.frequency_hz > 0.0);
        assert!(state.grid.voltage_v > 0.0);
        assert_eq!(state.timestamp, env.current_time());
    }
}
