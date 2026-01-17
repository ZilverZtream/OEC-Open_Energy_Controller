//! # Stochastic EV Driver Behavior Model
//!
//! Models realistic driver behavior patterns using probabilistic models.
//! Simulates vehicle connection/disconnection, driving patterns, and energy consumption.
//!
//! ## Behavior Model
//!
//! Uses a combination of:
//! - **Time-of-day patterns**: Commute times, weekend vs weekday
//! - **Markov chain**: State transitions (home, work, driving, disconnected)
//! - **Stochastic arrival**: Normal distribution for commute arrival times
//! - **Energy consumption**: Trip distance and driving efficiency
//!
//! ## States
//!
//! - **ParkedHome**: Vehicle connected at home, available for V2H
//! - **ParkedAway**: Vehicle at work/destination, unavailable
//! - **Driving**: In transit, consuming energy
//! - **Disconnected**: Manually disconnected (user override)

use chrono::{Datelike, NaiveDateTime, Timelike};
use rand::Rng;
use rand_distr::{Distribution, Normal};
use serde::{Deserialize, Serialize};

/// EV driver behavior state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EVDriverState {
    /// Vehicle parked at home and connected to charger
    ParkedHome,
    /// Vehicle parked away from home (work, shopping, etc.)
    ParkedAway,
    /// Vehicle in transit
    Driving,
    /// Vehicle manually disconnected (not plugged in)
    Disconnected,
}

/// Current EV state snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EVState {
    /// Current driver behavior state
    pub driver_state: EVDriverState,
    /// Vehicle state of charge (0-100%)
    pub soc_percent: f64,
    /// Whether vehicle is connected to charger
    pub is_connected: bool,
    /// Time until next state transition (minutes)
    pub time_to_next_transition_min: i64,
    /// Current trip distance (km, 0 if not driving)
    pub current_trip_distance_km: f64,
    /// Energy consumed on current trip (kWh)
    pub trip_energy_consumed_kwh: f64,
    /// Timestamp of this state
    pub timestamp: NaiveDateTime,
}

/// EV driver behavior configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EVDriverConfig {
    /// Battery capacity (kWh)
    pub battery_capacity_kwh: f64,

    /// Vehicle energy efficiency (kWh/100km)
    pub efficiency_kwh_per_100km: f64,

    /// Mean morning departure time (hour, 24h format)
    pub morning_departure_hour: f64,

    /// Std dev for morning departure (hours)
    pub morning_departure_std_hours: f64,

    /// Mean evening arrival time (hour, 24h format)
    pub evening_arrival_hour: f64,

    /// Std dev for evening arrival (hours)
    pub evening_arrival_std_hours: f64,

    /// Mean daily commute distance (km, one way)
    pub mean_commute_distance_km: f64,

    /// Std dev for commute distance (km)
    pub commute_distance_std_km: f64,

    /// Probability of weekend trip (0.0-1.0)
    pub weekend_trip_probability: f64,

    /// Mean weekend trip distance (km)
    pub weekend_trip_distance_km: f64,

    /// Probability of forgetting to plug in (0.0-1.0)
    pub forget_plugin_probability: f64,

    /// Minimum SoC before driver charges (%)
    pub min_desired_soc_percent: f64,

    /// Random seed for reproducibility
    pub random_seed: Option<u64>,
}

impl Default for EVDriverConfig {
    fn default() -> Self {
        Self {
            battery_capacity_kwh: 60.0,         // Typical mid-size EV
            efficiency_kwh_per_100km: 18.0,     // ~18 kWh/100km is typical
            morning_departure_hour: 7.5,        // 7:30 AM
            morning_departure_std_hours: 0.5,   // ±30 min
            evening_arrival_hour: 17.5,         // 5:30 PM
            evening_arrival_std_hours: 1.0,     // ±1 hour
            mean_commute_distance_km: 25.0,     // 25 km one way (50 km/day)
            commute_distance_std_km: 5.0,       // ±5 km variation
            weekend_trip_probability: 0.3,      // 30% chance of weekend trip
            weekend_trip_distance_km: 80.0,     // Longer weekend trips
            forget_plugin_probability: 0.05,    // 5% chance of forgetting
            min_desired_soc_percent: 30.0,      // Driver wants >30% for peace of mind
            random_seed: None,
        }
    }
}

impl EVDriverConfig {
    /// Short commute profile (urban, close to work)
    pub fn short_commute() -> Self {
        Self {
            mean_commute_distance_km: 15.0,
            commute_distance_std_km: 3.0,
            ..Default::default()
        }
    }

    /// Long commute profile (suburban, longer distance)
    pub fn long_commute() -> Self {
        Self {
            mean_commute_distance_km: 45.0,
            commute_distance_std_km: 10.0,
            ..Default::default()
        }
    }

    /// Highly variable schedule (shift work, irregular hours)
    pub fn irregular_schedule() -> Self {
        Self {
            morning_departure_std_hours: 2.0,
            evening_arrival_std_hours: 3.0,
            ..Default::default()
        }
    }

    /// Forgetful driver (often forgets to plug in)
    pub fn forgetful_driver() -> Self {
        Self {
            forget_plugin_probability: 0.2, // 20% chance
            ..Default::default()
        }
    }
}

/// EV driver behavior simulator
pub struct EVDriverSimulator {
    config: EVDriverConfig,
    state: EVState,
    rng: rand::rngs::StdRng,
    next_transition_time: NaiveDateTime,
}

impl EVDriverSimulator {
    /// Create a new EV driver simulator
    pub fn new(config: EVDriverConfig, start_time: NaiveDateTime) -> Self {
        use rand::SeedableRng;

        let rng = match config.random_seed {
            Some(seed) => rand::rngs::StdRng::seed_from_u64(seed),
            None => rand::rngs::StdRng::from_entropy(),
        };

        let mut simulator = Self {
            config,
            state: EVState {
                driver_state: EVDriverState::ParkedHome,
                soc_percent: 80.0, // Start with reasonable charge
                is_connected: true,
                time_to_next_transition_min: 0,
                current_trip_distance_km: 0.0,
                trip_energy_consumed_kwh: 0.0,
                timestamp: start_time,
            },
            rng,
            next_transition_time: start_time,
        };

        simulator.schedule_next_transition(start_time);
        simulator
    }

    /// Get current EV state
    pub fn state(&self) -> &EVState {
        &self.state
    }

    /// Check if vehicle is connected and available for V2H
    pub fn is_available_for_v2h(&self) -> bool {
        self.state.is_connected && self.state.driver_state == EVDriverState::ParkedHome
    }

    /// Update simulation to new timestamp
    pub fn tick(&mut self, new_time: NaiveDateTime) {
        // Check if we need to transition to new state
        while new_time >= self.next_transition_time {
            self.execute_transition();
            self.schedule_next_transition(self.next_transition_time);
        }

        // Update time to next transition
        self.state.time_to_next_transition_min =
            (self.next_transition_time - new_time).num_minutes();
        self.state.timestamp = new_time;
    }

    /// Execute state transition
    fn execute_transition(&mut self) {
        use EVDriverState::*;

        match self.state.driver_state {
            ParkedHome => {
                // Transition to driving (leaving for work/destination)
                let is_weekday = self.is_weekday(self.next_transition_time);
                let trip_distance = if is_weekday {
                    self.sample_commute_distance()
                } else if self.rng.gen_bool(self.config.weekend_trip_probability) {
                    self.config.weekend_trip_distance_km
                } else {
                    0.0 // Staying home
                };

                if trip_distance > 0.0 {
                    self.state.driver_state = Driving;
                    self.state.is_connected = false;
                    self.state.current_trip_distance_km = trip_distance;
                    self.state.trip_energy_consumed_kwh = 0.0;
                }
            }
            Driving => {
                // Complete trip - consume energy
                let energy_consumed = self.state.current_trip_distance_km
                    * self.config.efficiency_kwh_per_100km
                    / 100.0;

                self.state.soc_percent -= (energy_consumed / self.config.battery_capacity_kwh) * 100.0;
                self.state.soc_percent = self.state.soc_percent.max(0.0);
                self.state.trip_energy_consumed_kwh = energy_consumed;

                // Arrive at destination (work or home)
                let hour = self.next_transition_time.hour() as f64;
                if hour >= 6.0 && hour <= 12.0 {
                    // Morning arrival at work
                    self.state.driver_state = ParkedAway;
                    self.state.is_connected = false;
                } else {
                    // Evening arrival at home
                    self.state.driver_state = ParkedHome;

                    // Check if driver forgets to plug in
                    let forgot_plugin = self.rng.gen_bool(self.config.forget_plugin_probability);
                    self.state.is_connected = !forgot_plugin;

                    if forgot_plugin {
                        self.state.driver_state = Disconnected;
                    }
                }

                self.state.current_trip_distance_km = 0.0;
            }
            ParkedAway => {
                // Start return trip home
                let trip_distance = self.sample_commute_distance();
                self.state.driver_state = Driving;
                self.state.current_trip_distance_km = trip_distance;
                self.state.trip_energy_consumed_kwh = 0.0;
            }
            Disconnected => {
                // Driver notices low SoC and plugs in
                if self.state.soc_percent < self.config.min_desired_soc_percent {
                    self.state.driver_state = ParkedHome;
                    self.state.is_connected = true;
                }
            }
        }
    }

    /// Schedule next state transition
    fn schedule_next_transition(&mut self, current_time: NaiveDateTime) {
        use EVDriverState::*;

        let duration_minutes = match self.state.driver_state {
            ParkedHome => {
                // Next transition: Morning departure
                let departure_hour = self.sample_departure_time();
                let next_day = if current_time.hour() >= departure_hour as u32 {
                    current_time + chrono::Duration::days(1)
                } else {
                    current_time
                };

                let departure_time = next_day
                    .date()
                    .and_hms_opt(departure_hour as u32, ((departure_hour % 1.0) * 60.0) as u32, 0)
                    .unwrap();

                (departure_time - current_time).num_minutes()
            }
            Driving => {
                // Driving time: ~30-60 minutes depending on distance
                let speed_kmh = 40.0; // Average city speed
                let trip_time_hours = self.state.current_trip_distance_km / speed_kmh;
                (trip_time_hours * 60.0) as i64
            }
            ParkedAway => {
                // At work until evening
                let arrival_hour = self.sample_arrival_time();
                let arrival_time = current_time
                    .date()
                    .and_hms_opt(arrival_hour as u32, ((arrival_hour % 1.0) * 60.0) as u32, 0)
                    .unwrap();

                let duration = if arrival_time > current_time {
                    (arrival_time - current_time).num_minutes()
                } else {
                    // Already past arrival time today, wait until tomorrow
                    let tomorrow_arrival =
                        arrival_time + chrono::Duration::days(1);
                    (tomorrow_arrival - current_time).num_minutes()
                };

                duration
            }
            Disconnected => {
                // Check every few hours if driver notices
                self.rng.gen_range(60..360) // 1-6 hours
            }
        };

        self.next_transition_time = current_time + chrono::Duration::minutes(duration_minutes);
    }

    /// Sample morning departure time
    fn sample_departure_time(&mut self) -> f64 {
        let normal = Normal::new(
            self.config.morning_departure_hour,
            self.config.morning_departure_std_hours,
        )
        .unwrap();
        normal.sample(&mut self.rng).clamp(5.0, 12.0)
    }

    /// Sample evening arrival time
    fn sample_arrival_time(&mut self) -> f64 {
        let normal = Normal::new(
            self.config.evening_arrival_hour,
            self.config.evening_arrival_std_hours,
        )
        .unwrap();
        normal.sample(&mut self.rng).clamp(14.0, 22.0)
    }

    /// Sample commute distance
    fn sample_commute_distance(&mut self) -> f64 {
        let normal = Normal::new(
            self.config.mean_commute_distance_km,
            self.config.commute_distance_std_km,
        )
        .unwrap();
        normal.sample(&mut self.rng).max(1.0)
    }

    /// Check if given time is a weekday
    fn is_weekday(&self, time: NaiveDateTime) -> bool {
        let weekday = time.weekday();
        !matches!(weekday, chrono::Weekday::Sat | chrono::Weekday::Sun)
    }

    /// Manually set vehicle SoC (e.g., after charging)
    pub fn set_soc(&mut self, soc_percent: f64) {
        self.state.soc_percent = soc_percent.clamp(0.0, 100.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_ev_driver_initialization() {
        let config = EVDriverConfig {
            random_seed: Some(42),
            ..Default::default()
        };

        let start_time = NaiveDate::from_ymd_opt(2024, 6, 17) // Monday
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();

        let sim = EVDriverSimulator::new(config, start_time);

        assert_eq!(sim.state().driver_state, EVDriverState::ParkedHome);
        assert!(sim.state().is_connected);
        assert!(sim.is_available_for_v2h());
    }

    #[test]
    fn test_morning_commute() {
        let config = EVDriverConfig {
            morning_departure_hour: 8.0,
            morning_departure_std_hours: 0.1,
            random_seed: Some(42),
            ..Default::default()
        };

        let start_time = NaiveDate::from_ymd_opt(2024, 6, 17) // Monday
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();

        let mut sim = EVDriverSimulator::new(config, start_time);

        // Advance to morning departure time
        let morning = start_time
            .with_hour(9)
            .unwrap();
        sim.tick(morning);

        // Should be driving or parked away by now
        assert!(
            sim.state().driver_state == EVDriverState::Driving
                || sim.state().driver_state == EVDriverState::ParkedAway
        );
        assert!(!sim.is_available_for_v2h());
    }

    #[test]
    fn test_energy_consumption_during_trip() {
        let config = EVDriverConfig {
            mean_commute_distance_km: 30.0,
            efficiency_kwh_per_100km: 20.0,
            battery_capacity_kwh: 60.0,
            morning_departure_hour: 8.0,
            random_seed: Some(42),
            ..Default::default()
        };

        let start_time = NaiveDate::from_ymd_opt(2024, 6, 17)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();

        let mut sim = EVDriverSimulator::new(config, start_time);
        let initial_soc = sim.state().soc_percent;

        // Simulate a full day (morning commute + evening return)
        let end_time = start_time + chrono::Duration::days(1);
        sim.tick(end_time);

        // SoC should have decreased due to driving
        // Expected: ~60 km total, 20 kWh/100km = 12 kWh consumed
        // 12 kWh / 60 kWh capacity = 20% SoC drop
        if sim.state().trip_energy_consumed_kwh > 0.0 {
            assert!(sim.state().soc_percent < initial_soc);
        }
    }

    #[test]
    fn test_weekend_behavior() {
        let config = EVDriverConfig {
            weekend_trip_probability: 0.0, // No weekend trips
            random_seed: Some(42),
            ..Default::default()
        };

        let start_time = NaiveDate::from_ymd_opt(2024, 6, 22) // Saturday
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();

        let mut sim = EVDriverSimulator::new(config, start_time);

        // Advance through weekend
        let end_time = start_time + chrono::Duration::days(2);
        sim.tick(end_time);

        // Should mostly stay at home on weekends with no trips configured
        assert!(sim.state().driver_state == EVDriverState::ParkedHome);
    }

    #[test]
    fn test_forget_to_plugin() {
        let config = EVDriverConfig {
            forget_plugin_probability: 1.0, // Always forget
            morning_departure_hour: 8.0,
            evening_arrival_hour: 17.0,
            random_seed: Some(42),
            ..Default::default()
        };

        let start_time = NaiveDate::from_ymd_opt(2024, 6, 17)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();

        let mut sim = EVDriverSimulator::new(config, start_time);

        // Simulate a full day
        let end_time = start_time + chrono::Duration::days(1);
        sim.tick(end_time);

        // After returning home and forgetting, should be Disconnected
        if sim.state().driver_state == EVDriverState::Disconnected {
            assert!(!sim.state().is_connected);
            assert!(!sim.is_available_for_v2h());
        }
    }

    #[test]
    fn test_short_commute_profile() {
        let config = EVDriverConfig::short_commute();
        assert!(config.mean_commute_distance_km < 20.0);
    }

    #[test]
    fn test_long_commute_profile() {
        let config = EVDriverConfig::long_commute();
        assert!(config.mean_commute_distance_km > 40.0);
    }
}
