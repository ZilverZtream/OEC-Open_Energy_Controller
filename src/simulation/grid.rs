//! # Grid Simulation
//!
//! Models grid behavior including frequency/voltage fluctuations, fuse limits,
//! and fault conditions.

use chrono::NaiveDateTime;
use rand::Rng;
use rand_distr::{Distribution, Normal};
use serde::{Deserialize, Serialize};

/// Grid fault type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GridFaultType {
    /// No fault
    Normal,
    /// Under-frequency event
    UnderFrequency,
    /// Over-frequency event
    OverFrequency,
    /// Under-voltage event
    UnderVoltage,
    /// Over-voltage event
    OverVoltage,
    /// Fuse tripped (overcurrent)
    FuseTripped,
    /// Complete grid outage
    Outage,
}

/// Current state of the grid simulation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridState {
    /// Grid frequency in Hz (nominal 50 Hz in Europe)
    pub frequency_hz: f64,
    /// Grid voltage in V (nominal 230 V in Europe)
    pub voltage_v: f64,
    /// Current fault state
    pub fault: GridFaultType,
    /// Current import power in kW
    pub import_kw: f64,
    /// Current export power in kW
    pub export_kw: f64,
    /// Whether grid is available
    pub is_available: bool,
    /// Time remaining in fault condition (minutes)
    pub fault_duration_minutes: i64,
    /// Timestamp of this state
    pub timestamp: NaiveDateTime,
}

/// Grid simulator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridSimulatorConfig {
    /// Nominal frequency in Hz (50 Hz for Europe, 60 Hz for North America)
    pub nominal_frequency_hz: f64,
    /// Nominal voltage in V (230 V for Europe, 120 V for North America)
    pub nominal_voltage_v: f64,
    /// Frequency standard deviation for normal operation
    pub frequency_std_dev_hz: f64,
    /// Voltage standard deviation for normal operation
    pub voltage_std_dev_v: f64,
    /// Fuse rating in Amperes
    pub fuse_rating_a: f64,
    /// Enable random fault events
    pub enable_faults: bool,
    /// Probability of fault per hour (0.0-1.0)
    pub fault_probability_per_hour: f64,
    /// Random seed for reproducibility (None = random)
    pub random_seed: Option<u64>,

    // SIMULATION IMPROVEMENT #2: Grid Impedance Model
    /// Grid connection point (GCP) impedance in Ohms
    /// Typical values: 0.1-0.5 Ω for strong grid, 0.5-2.0 Ω for weak grid
    pub gcp_impedance_ohm: f64,
    /// Enable voltage sag/rise simulation based on power flow
    pub enable_voltage_impedance: bool,
}

impl Default for GridSimulatorConfig {
    fn default() -> Self {
        Self {
            nominal_frequency_hz: 50.0, // European grid
            nominal_voltage_v: 230.0,   // European grid
            frequency_std_dev_hz: 0.02, // ±0.02 Hz is typical
            voltage_std_dev_v: 2.0,     // ±2 V is typical
            fuse_rating_a: 25.0,        // 25A main fuse (typical household)
            enable_faults: true,
            fault_probability_per_hour: 0.01, // 1% chance per hour (~90 hours MTBF)
            random_seed: None,
            gcp_impedance_ohm: 0.3,     // Medium grid strength
            enable_voltage_impedance: true,
        }
    }
}

impl GridSimulatorConfig {
    /// Configuration for strong grid (urban area, close to substation)
    pub fn strong_grid() -> Self {
        Self {
            gcp_impedance_ohm: 0.1,
            ..Default::default()
        }
    }

    /// Configuration for weak grid (rural area, far from substation)
    pub fn weak_grid() -> Self {
        Self {
            gcp_impedance_ohm: 1.5,
            ..Default::default()
        }
    }
}

/// Simulates grid behavior
pub struct GridSimulator {
    config: GridSimulatorConfig,
    current_state: GridState,
    rng: rand::rngs::StdRng,
    fault_duration_remaining: i64,
}

impl GridSimulator {
    /// Create a new grid simulator
    pub fn new(config: GridSimulatorConfig, start_time: NaiveDateTime) -> Self {
        use rand::SeedableRng;

        let rng = match config.random_seed {
            Some(seed) => rand::rngs::StdRng::seed_from_u64(seed),
            None => rand::rngs::StdRng::from_entropy(),
        };

        let mut simulator = Self {
            config,
            current_state: GridState {
                frequency_hz: 50.0,
                voltage_v: 230.0,
                fault: GridFaultType::Normal,
                import_kw: 0.0,
                export_kw: 0.0,
                is_available: true,
                fault_duration_minutes: 0,
                timestamp: start_time,
            },
            rng,
            fault_duration_remaining: 0,
        };

        simulator.update_state(start_time, 0.0, 0.0);
        simulator
    }

    /// Get current grid state
    pub fn state(&self) -> &GridState {
        &self.current_state
    }

    /// Get current frequency
    pub fn frequency_hz(&self) -> f64 {
        self.current_state.frequency_hz
    }

    /// Get current voltage
    pub fn voltage_v(&self) -> f64 {
        self.current_state.voltage_v
    }

    /// Check if grid is available
    pub fn is_available(&self) -> bool {
        self.current_state.is_available
    }

    /// Update simulation to a new timestamp with power flow
    ///
    /// # Arguments
    /// * `new_time` - New timestamp
    /// * `import_kw` - Power being imported from grid
    /// * `export_kw` - Power being exported to grid
    pub fn tick(&mut self, new_time: NaiveDateTime, import_kw: f64, export_kw: f64) {
        self.update_state(new_time, import_kw, export_kw);
    }

    /// Generate frequency noise
    fn generate_frequency_noise(&mut self) -> f64 {
        let normal = Normal::new(0.0, self.config.frequency_std_dev_hz).unwrap();
        normal.sample(&mut self.rng)
    }

    /// Generate voltage noise
    fn generate_voltage_noise(&mut self) -> f64 {
        let normal = Normal::new(0.0, self.config.voltage_std_dev_v).unwrap();
        normal.sample(&mut self.rng)
    }

    /// Check if fuse should trip based on current
    fn check_fuse_trip(&mut self, import_kw: f64) -> bool {
        // Calculate current: I = P / V
        let current_a = import_kw * 1000.0 / self.config.nominal_voltage_v;

        // Fuse trips if current exceeds rating
        // Add small random variation (±2%) to simulate thermal characteristics
        let trip_threshold = self.config.fuse_rating_a * (1.0 + self.rng.gen_range(-0.02..0.02));

        current_a > trip_threshold
    }

    /// Simulate random fault events
    fn simulate_faults(&mut self, minutes_elapsed: i64) {
        if !self.config.enable_faults {
            return;
        }

        // Check if currently in a fault
        if self.fault_duration_remaining > 0 {
            self.fault_duration_remaining -= minutes_elapsed;
            if self.fault_duration_remaining <= 0 {
                // Fault cleared
                self.current_state.fault = GridFaultType::Normal;
                self.current_state.is_available = true;
            }
            return;
        }

        // Check for new fault
        let probability_per_minute = self.config.fault_probability_per_hour / 60.0;
        for _ in 0..minutes_elapsed {
            if self.rng.gen_bool(probability_per_minute) {
                // Fault occurred
                self.trigger_random_fault();
                break;
            }
        }
    }

    /// Calculate voltage at connection point including impedance effects
    ///
    /// V_measured = V_source - I * Z_line (for import)
    /// V_measured = V_source + I * Z_line (for export)
    ///
    /// Where:
    /// - V_source = grid source voltage
    /// - I = current flow
    /// - Z_line = grid connection point impedance
    ///
    /// This models voltage sag during import and voltage rise during export
    fn calculate_voltage_with_impedance(
        &self,
        source_voltage_v: f64,
        import_kw: f64,
        export_kw: f64,
    ) -> f64 {
        // Net power flow (positive = import, negative = export)
        let net_power_kw = import_kw - export_kw;

        // Calculate current: I = P / V
        let current_a = (net_power_kw * 1000.0) / source_voltage_v;

        // Voltage drop: V_drop = I * Z
        let voltage_drop_v = current_a * self.config.gcp_impedance_ohm;

        // Voltage sag during import (positive current)
        // Voltage rise during export (negative current)
        let measured_voltage = source_voltage_v - voltage_drop_v;

        // Clamp to realistic range
        measured_voltage.clamp(180.0, 260.0)
    }

    /// Trigger a random fault
    fn trigger_random_fault(&mut self) {
        let fault_type = self.rng.gen_range(0..100);

        let (fault, duration_minutes, affects_availability) = match fault_type {
            0..=20 => (GridFaultType::UnderFrequency, self.rng.gen_range(1..5), false),
            21..=40 => (GridFaultType::OverFrequency, self.rng.gen_range(1..5), false),
            41..=55 => (GridFaultType::UnderVoltage, self.rng.gen_range(2..10), false),
            56..=70 => (GridFaultType::OverVoltage, self.rng.gen_range(2..10), false),
            71..=85 => (GridFaultType::FuseTripped, self.rng.gen_range(10..60), true),
            _ => (GridFaultType::Outage, self.rng.gen_range(30..180), true),
        };

        self.current_state.fault = fault;
        self.fault_duration_remaining = duration_minutes;
        self.current_state.fault_duration_minutes = duration_minutes;
        self.current_state.is_available = !affects_availability;
    }

    /// Update the grid state
    fn update_state(&mut self, time: NaiveDateTime, import_kw: f64, export_kw: f64) {
        let minutes_elapsed = (time - self.current_state.timestamp).num_minutes();

        // Simulate fault events
        self.simulate_faults(minutes_elapsed);

        // Check for fuse trip due to overcurrent
        if self.current_state.fault == GridFaultType::Normal && self.check_fuse_trip(import_kw) {
            self.current_state.fault = GridFaultType::FuseTripped;
            self.fault_duration_remaining = self.rng.gen_range(10..60);
            self.current_state.fault_duration_minutes = self.fault_duration_remaining;
            self.current_state.is_available = false;
        }

        // Calculate frequency and voltage based on fault state
        let (frequency_hz, voltage_v) = match self.current_state.fault {
            GridFaultType::Normal => {
                // Normal operation with small fluctuations
                let freq = self.config.nominal_frequency_hz + self.generate_frequency_noise();

                // SIMULATION IMPROVEMENT #2: Voltage sag/rise from grid impedance
                let base_voltage = self.config.nominal_voltage_v + self.generate_voltage_noise();
                let volt = if self.config.enable_voltage_impedance {
                    self.calculate_voltage_with_impedance(base_voltage, import_kw, export_kw)
                } else {
                    base_voltage
                };
                (freq, volt)
            }
            GridFaultType::UnderFrequency => {
                // Frequency drops to 49.5-49.9 Hz
                let freq = self.rng.gen_range(49.5..49.9);
                let volt = self.config.nominal_voltage_v + self.generate_voltage_noise();
                (freq, volt)
            }
            GridFaultType::OverFrequency => {
                // Frequency rises to 50.1-50.5 Hz
                let freq = self.rng.gen_range(50.1..50.5);
                let volt = self.config.nominal_voltage_v + self.generate_voltage_noise();
                (freq, volt)
            }
            GridFaultType::UnderVoltage => {
                // Voltage drops to 200-220 V
                let freq = self.config.nominal_frequency_hz + self.generate_frequency_noise();
                let volt = self.rng.gen_range(200.0..220.0);
                (freq, volt)
            }
            GridFaultType::OverVoltage => {
                // Voltage rises to 240-250 V
                let freq = self.config.nominal_frequency_hz + self.generate_frequency_noise();
                let volt = self.rng.gen_range(240.0..250.0);
                (freq, volt)
            }
            GridFaultType::FuseTripped | GridFaultType::Outage => {
                // No grid connection
                (0.0, 0.0)
            }
        };

        self.current_state = GridState {
            frequency_hz,
            voltage_v,
            fault: self.current_state.fault,
            import_kw: if self.current_state.is_available {
                import_kw
            } else {
                0.0
            },
            export_kw: if self.current_state.is_available {
                export_kw
            } else {
                0.0
            },
            is_available: self.current_state.is_available,
            fault_duration_minutes: self.fault_duration_remaining.max(0),
            timestamp: time,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, NaiveDate};

    #[test]
    fn test_grid_simulator_initialization() {
        let config = GridSimulatorConfig {
            random_seed: Some(42),
            enable_faults: false,
            ..Default::default()
        };

        let start_time = NaiveDate::from_ymd_opt(2024, 6, 15)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();

        let simulator = GridSimulator::new(config, start_time);
        assert_eq!(simulator.state().fault, GridFaultType::Normal);
        assert!(simulator.is_available());
    }

    #[test]
    fn test_normal_operation() {
        let config = GridSimulatorConfig {
            random_seed: Some(42),
            enable_faults: false,
            ..Default::default()
        };

        let start_time = NaiveDate::from_ymd_opt(2024, 6, 15)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();

        let mut simulator = GridSimulator::new(config, start_time);

        // Simulate 1 hour
        let end_time = start_time + Duration::hours(1);
        simulator.tick(end_time, 2.0, 0.0);

        // Should remain normal
        assert_eq!(simulator.state().fault, GridFaultType::Normal);
        assert!(simulator.is_available());
        assert!((simulator.frequency_hz() - 50.0).abs() < 0.1); // Within 0.1 Hz
        assert!((simulator.voltage_v() - 230.0).abs() < 10.0); // Within 10 V
    }

    #[test]
    fn test_fuse_trip() {
        let config = GridSimulatorConfig {
            random_seed: Some(42),
            enable_faults: false,
            fuse_rating_a: 25.0,
            nominal_voltage_v: 230.0,
            ..Default::default()
        };

        let start_time = NaiveDate::from_ymd_opt(2024, 6, 15)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();

        let mut simulator = GridSimulator::new(config, start_time);

        // Try to import 30 kW (130 A at 230 V, way over 25 A fuse)
        let end_time = start_time + Duration::minutes(1);
        simulator.tick(end_time, 30.0, 0.0);

        // Fuse should trip
        assert_eq!(simulator.state().fault, GridFaultType::FuseTripped);
        assert!(!simulator.is_available());
    }

    #[test]
    fn test_fault_recovery() {
        let config = GridSimulatorConfig {
            random_seed: Some(42),
            enable_faults: false,
            fuse_rating_a: 25.0,
            nominal_voltage_v: 230.0,
            ..Default::default()
        };

        let start_time = NaiveDate::from_ymd_opt(2024, 6, 15)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();

        let mut simulator = GridSimulator::new(config, start_time);

        // Trip the fuse
        simulator.tick(start_time + Duration::minutes(1), 30.0, 0.0);
        assert!(!simulator.is_available());

        let fault_duration = simulator.state().fault_duration_minutes;

        // Wait for fault to clear
        let recovery_time = start_time + Duration::minutes(fault_duration + 5);
        simulator.tick(recovery_time, 2.0, 0.0);

        // Should be recovered
        assert_eq!(simulator.state().fault, GridFaultType::Normal);
        assert!(simulator.is_available());
    }

    #[test]
    fn test_frequency_voltage_fluctuations() {
        let config = GridSimulatorConfig {
            random_seed: Some(42),
            enable_faults: false,
            frequency_std_dev_hz: 0.02,
            voltage_std_dev_v: 2.0,
            ..Default::default()
        };

        let start_time = NaiveDate::from_ymd_opt(2024, 6, 15)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();

        let mut simulator = GridSimulator::new(config, start_time);

        let mut frequencies = Vec::new();
        let mut voltages = Vec::new();

        // Collect 100 samples
        for i in 0..100 {
            let time = start_time + Duration::minutes(i);
            simulator.tick(time, 2.0, 0.0);
            frequencies.push(simulator.frequency_hz());
            voltages.push(simulator.voltage_v());
        }

        // Check that values fluctuate around nominal
        let avg_freq = frequencies.iter().sum::<f64>() / frequencies.len() as f64;
        let avg_volt = voltages.iter().sum::<f64>() / voltages.len() as f64;

        assert!((avg_freq - 50.0).abs() < 0.05); // Average near nominal
        assert!((avg_volt - 230.0).abs() < 5.0); // Average near nominal
    }

    #[test]
    fn test_power_tracking() {
        let config = GridSimulatorConfig {
            random_seed: Some(42),
            enable_faults: false,
            ..Default::default()
        };

        let start_time = NaiveDate::from_ymd_opt(2024, 6, 15)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();

        let mut simulator = GridSimulator::new(config, start_time);

        // Import 2 kW, export 1 kW
        simulator.tick(start_time + Duration::minutes(1), 2.0, 1.0);

        assert!((simulator.state().import_kw - 2.0).abs() < 0.01);
        assert!((simulator.state().export_kw - 1.0).abs() < 0.01);
    }
}
