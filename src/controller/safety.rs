#![allow(dead_code)]
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use tracing::{error, warn};

use crate::domain::{BatteryState, GridConnection, GridStatus};

/// Safety event types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SafetyEvent {
    /// Battery temperature exceeded safe limits
    BatteryOverTemperature {
        temperature_c: f64,
        limit_c: f64,
    },
    /// Battery temperature below safe operating range
    BatteryUnderTemperature {
        temperature_c: f64,
        limit_c: f64,
    },
    /// Battery SoC below emergency minimum
    BatteryLowSoC {
        soc_percent: f64,
        limit_percent: f64,
    },
    /// Battery SoC above safe maximum
    BatteryHighSoC {
        soc_percent: f64,
        limit_percent: f64,
    },
    /// Grid frequency out of acceptable range
    GridFrequencyFault {
        frequency_hz: f64,
        min_hz: f64,
        max_hz: f64,
    },
    /// Grid voltage out of acceptable range
    GridVoltageFault {
        voltage_v: f64,
        min_v: f64,
        max_v: f64,
    },
    /// Grid blackout detected
    GridBlackout,
    /// Battery power command exceeds physical limits
    PowerLimitViolation {
        commanded_w: f64,
        limit_w: f64,
    },
}

/// Safety monitor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyConfig {
    /// Maximum safe battery temperature (°C)
    pub max_battery_temp_c: f64,
    /// Minimum safe battery temperature (°C)
    pub min_battery_temp_c: f64,
    /// Emergency minimum SoC (%)
    pub emergency_min_soc_percent: f64,
    /// Emergency maximum SoC (%)
    pub emergency_max_soc_percent: f64,
    /// Minimum grid frequency (Hz)
    pub min_grid_frequency_hz: f64,
    /// Maximum grid frequency (Hz)
    pub max_grid_frequency_hz: f64,
    /// Minimum grid voltage (V)
    pub min_grid_voltage_v: f64,
    /// Maximum grid voltage (V)
    pub max_grid_voltage_v: f64,
}

impl Default for SafetyConfig {
    fn default() -> Self {
        Self {
            max_battery_temp_c: 60.0,
            min_battery_temp_c: -10.0,
            emergency_min_soc_percent: 5.0,
            emergency_max_soc_percent: 98.0,
            min_grid_frequency_hz: 49.5,
            max_grid_frequency_hz: 50.5,
            min_grid_voltage_v: 207.0, // -10% of 230V
            max_grid_voltage_v: 253.0, // +10% of 230V
        }
    }
}

/// Safety monitor for battery and grid systems
pub struct SafetyMonitor {
    config: SafetyConfig,
    last_events: VecDeque<(DateTime<Utc>, SafetyEvent)>,
    max_event_history: usize,
}

impl SafetyMonitor {
    /// Create a new safety monitor with default configuration
    pub fn new() -> Self {
        Self::with_config(SafetyConfig::default())
    }

    /// Create a new safety monitor with custom configuration
    pub fn with_config(config: SafetyConfig) -> Self {
        Self {
            config,
            last_events: VecDeque::new(),
            max_event_history: 100,
        }
    }

    /// Check battery state for safety violations
    /// Returns Ok(()) if safe, Err with shutdown command if unsafe
    pub fn check_battery_safety(&mut self, state: &BatteryState) -> Result<()> {
        let now = Utc::now();

        // Check battery temperature
        if state.temperature_c > self.config.max_battery_temp_c {
            let event = SafetyEvent::BatteryOverTemperature {
                temperature_c: state.temperature_c,
                limit_c: self.config.max_battery_temp_c,
            };
            self.record_event(now, event.clone());
            error!(
                temperature_c = state.temperature_c,
                limit_c = self.config.max_battery_temp_c,
                "SAFETY VIOLATION: Battery over-temperature detected - initiating emergency shutdown"
            );
            return Err(anyhow::anyhow!(
                "Battery temperature {:.1}°C exceeds maximum safe limit {:.1}°C",
                state.temperature_c,
                self.config.max_battery_temp_c
            ));
        }

        if state.temperature_c < self.config.min_battery_temp_c {
            let event = SafetyEvent::BatteryUnderTemperature {
                temperature_c: state.temperature_c,
                limit_c: self.config.min_battery_temp_c,
            };
            self.record_event(now, event.clone());
            warn!(
                temperature_c = state.temperature_c,
                limit_c = self.config.min_battery_temp_c,
                "Battery temperature below minimum operating range"
            );
        }

        // Check battery SoC bounds
        if state.soc_percent < self.config.emergency_min_soc_percent {
            let event = SafetyEvent::BatteryLowSoC {
                soc_percent: state.soc_percent,
                limit_percent: self.config.emergency_min_soc_percent,
            };
            self.record_event(now, event.clone());
            error!(
                soc_percent = state.soc_percent,
                limit_percent = self.config.emergency_min_soc_percent,
                "SAFETY VIOLATION: Battery SoC critically low - initiating emergency stop"
            );
            return Err(anyhow::anyhow!(
                "Battery SoC {:.1}% below emergency minimum {:.1}%",
                state.soc_percent,
                self.config.emergency_min_soc_percent
            ));
        }

        if state.soc_percent > self.config.emergency_max_soc_percent {
            let event = SafetyEvent::BatteryHighSoC {
                soc_percent: state.soc_percent,
                limit_percent: self.config.emergency_max_soc_percent,
            };
            self.record_event(now, event.clone());
            error!(
                soc_percent = state.soc_percent,
                limit_percent = self.config.emergency_max_soc_percent,
                "SAFETY VIOLATION: Battery SoC critically high - initiating emergency stop"
            );
            return Err(anyhow::anyhow!(
                "Battery SoC {:.1}% exceeds emergency maximum {:.1}%",
                state.soc_percent,
                self.config.emergency_max_soc_percent
            ));
        }

        Ok(())
    }

    /// Check grid connection for safety violations
    pub fn check_grid_safety(&mut self, grid: &GridConnection) -> Result<()> {
        let now = Utc::now();

        // Check for blackout
        if grid.status == GridStatus::Blackout {
            let event = SafetyEvent::GridBlackout;
            self.record_event(now, event.clone());
            error!("SAFETY VIOLATION: Grid blackout detected - switching to backup mode");
            return Err(anyhow::anyhow!("Grid blackout detected"));
        }

        // Check grid frequency
        if grid.frequency_hz < self.config.min_grid_frequency_hz
            || grid.frequency_hz > self.config.max_grid_frequency_hz
        {
            let event = SafetyEvent::GridFrequencyFault {
                frequency_hz: grid.frequency_hz,
                min_hz: self.config.min_grid_frequency_hz,
                max_hz: self.config.max_grid_frequency_hz,
            };
            self.record_event(now, event.clone());
            error!(
                frequency_hz = grid.frequency_hz,
                min_hz = self.config.min_grid_frequency_hz,
                max_hz = self.config.max_grid_frequency_hz,
                "SAFETY VIOLATION: Grid frequency out of range"
            );
            return Err(anyhow::anyhow!(
                "Grid frequency {:.2}Hz out of acceptable range ({:.2}-{:.2}Hz)",
                grid.frequency_hz,
                self.config.min_grid_frequency_hz,
                self.config.max_grid_frequency_hz
            ));
        }

        // Check grid voltage
        if grid.voltage_v < self.config.min_grid_voltage_v
            || grid.voltage_v > self.config.max_grid_voltage_v
        {
            let event = SafetyEvent::GridVoltageFault {
                voltage_v: grid.voltage_v,
                min_v: self.config.min_grid_voltage_v,
                max_v: self.config.max_grid_voltage_v,
            };
            self.record_event(now, event.clone());
            error!(
                voltage_v = grid.voltage_v,
                min_v = self.config.min_grid_voltage_v,
                max_v = self.config.max_grid_voltage_v,
                "SAFETY VIOLATION: Grid voltage out of range"
            );
            return Err(anyhow::anyhow!(
                "Grid voltage {:.1}V out of acceptable range ({:.1}-{:.1}V)",
                grid.voltage_v,
                self.config.min_grid_voltage_v,
                self.config.max_grid_voltage_v
            ));
        }

        Ok(())
    }

    /// Validate a power command before execution
    pub fn validate_power_command(
        &mut self,
        commanded_w: f64,
        max_charge_w: f64,
        max_discharge_w: f64,
    ) -> Result<()> {
        let now = Utc::now();

        // Check if power is finite
        if !commanded_w.is_finite() {
            return Err(anyhow::anyhow!(
                "Invalid power command: value is not finite"
            ));
        }

        // Check charge limit
        if commanded_w > 0.0 && commanded_w > max_charge_w {
            let event = SafetyEvent::PowerLimitViolation {
                commanded_w,
                limit_w: max_charge_w,
            };
            self.record_event(now, event.clone());
            return Err(anyhow::anyhow!(
                "Power command {}W exceeds maximum charge power {}W",
                commanded_w,
                max_charge_w
            ));
        }

        // Check discharge limit
        if commanded_w < 0.0 && commanded_w.abs() > max_discharge_w {
            let event = SafetyEvent::PowerLimitViolation {
                commanded_w,
                limit_w: max_discharge_w,
            };
            self.record_event(now, event.clone());
            return Err(anyhow::anyhow!(
                "Power command {}W exceeds maximum discharge power {}W",
                commanded_w.abs(),
                max_discharge_w
            ));
        }

        Ok(())
    }

    /// Record a safety event
    fn record_event(&mut self, timestamp: DateTime<Utc>, event: SafetyEvent) {
        self.last_events.push_back((timestamp, event));

        // Keep only the most recent events - O(1) with VecDeque
        if self.last_events.len() > self.max_event_history {
            self.last_events.pop_front();
        }
    }

    /// Get recent safety events
    pub fn get_recent_events(&self, count: usize) -> Vec<(DateTime<Utc>, SafetyEvent)> {
        let start = self.last_events.len().saturating_sub(count);
        self.last_events.iter()
            .skip(start)
            .cloned()
            .collect()
    }

    /// Clear all recorded events
    pub fn clear_events(&mut self) {
        self.last_events.clear();
    }

    /// Get safety configuration
    pub fn config(&self) -> &SafetyConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::BatteryStatus;

    fn make_safe_battery_state() -> BatteryState {
        BatteryState {
            soc_percent: 50.0,
            power_w: 0.0,
            voltage_v: 48.0,
            temperature_c: 25.0,
            health_percent: 100.0,
            status: BatteryStatus::Idle,
        }
    }

    fn make_safe_grid_connection() -> GridConnection {
        GridConnection {
            status: GridStatus::Normal,
            import_power_w: 0.0,
            export_power_w: 0.0,
            frequency_hz: 50.0,
            voltage_v: 230.0,
            current_a: 0.0,
        }
    }

    #[test]
    fn test_safe_battery_state() {
        let mut monitor = SafetyMonitor::new();
        let state = make_safe_battery_state();
        assert!(monitor.check_battery_safety(&state).is_ok());
    }

    #[test]
    fn test_battery_over_temperature() {
        let mut monitor = SafetyMonitor::new();
        let mut state = make_safe_battery_state();
        state.temperature_c = 65.0; // Above 60°C limit
        assert!(monitor.check_battery_safety(&state).is_err());
        assert_eq!(monitor.last_events.len(), 1);
    }

    #[test]
    fn test_battery_low_soc() {
        let mut monitor = SafetyMonitor::new();
        let mut state = make_safe_battery_state();
        state.soc_percent = 3.0; // Below 5% limit
        assert!(monitor.check_battery_safety(&state).is_err());
    }

    #[test]
    fn test_battery_high_soc() {
        let mut monitor = SafetyMonitor::new();
        let mut state = make_safe_battery_state();
        state.soc_percent = 99.0; // Above 98% limit
        assert!(monitor.check_battery_safety(&state).is_err());
    }

    #[test]
    fn test_safe_grid_connection() {
        let mut monitor = SafetyMonitor::new();
        let grid = make_safe_grid_connection();
        assert!(monitor.check_grid_safety(&grid).is_ok());
    }

    #[test]
    fn test_grid_blackout() {
        let mut monitor = SafetyMonitor::new();
        let mut grid = make_safe_grid_connection();
        grid.status = GridStatus::Blackout;
        assert!(monitor.check_grid_safety(&grid).is_err());
    }

    #[test]
    fn test_grid_frequency_fault() {
        let mut monitor = SafetyMonitor::new();
        let mut grid = make_safe_grid_connection();
        grid.frequency_hz = 49.0; // Below 49.5Hz limit
        assert!(monitor.check_grid_safety(&grid).is_err());
    }

    #[test]
    fn test_grid_voltage_fault() {
        let mut monitor = SafetyMonitor::new();
        let mut grid = make_safe_grid_connection();
        grid.voltage_v = 200.0; // Below 207V limit
        assert!(monitor.check_grid_safety(&grid).is_err());
    }

    #[test]
    fn test_power_command_validation() {
        let mut monitor = SafetyMonitor::new();

        // Valid power command
        assert!(monitor.validate_power_command(3000.0, 5000.0, 5000.0).is_ok());

        // Charge power too high
        assert!(monitor.validate_power_command(6000.0, 5000.0, 5000.0).is_err());

        // Discharge power too high
        assert!(monitor.validate_power_command(-6000.0, 5000.0, 5000.0).is_err());

        // Non-finite power
        assert!(monitor.validate_power_command(f64::NAN, 5000.0, 5000.0).is_err());
    }

    #[test]
    fn test_event_history() {
        let mut monitor = SafetyMonitor::new();
        let state = make_safe_battery_state();

        // Generate multiple events
        for i in 0..5 {
            let mut test_state = state.clone();
            test_state.temperature_c = 65.0 + i as f64;
            let _ = monitor.check_battery_safety(&test_state);
        }

        assert_eq!(monitor.last_events.len(), 5);

        let recent = monitor.get_recent_events(3);
        assert_eq!(recent.len(), 3);

        monitor.clear_events();
        assert_eq!(monitor.last_events.len(), 0);
    }
}
