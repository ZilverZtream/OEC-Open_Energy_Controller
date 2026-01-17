//! # Safety Monitor Supervisor
//!
//! Independent high-priority safety monitoring system that runs in parallel
//! to the main control loop. Provides defense-in-depth against:
//! - Fuse trips due to overcurrent
//! - Grid frequency/voltage violations
//! - Battery over-temperature
//! - Battery over-discharge / over-charge
//! - Control loop hangs or failures
//!
//! The safety monitor operates on a fast 1-second tick and can trigger
//! emergency stops independently of the main controller.

use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, error, info, warn};

use crate::domain::{Battery, Inverter};

/// Safety monitor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyMonitorConfig {
    /// Monitoring interval in seconds
    pub check_interval_s: u64,

    /// Fuse rating in Amperes
    pub fuse_rating_a: f64,

    /// Fuse trip margin (0.0-1.0)
    /// Trips at fuse_rating * (1 - margin)
    /// e.g., 0.1 = trip at 90% of fuse rating
    pub fuse_trip_margin: f64,

    /// Grid voltage limits (V)
    pub grid_voltage_min_v: f64,
    pub grid_voltage_max_v: f64,

    /// Grid frequency limits (Hz)
    pub grid_frequency_min_hz: f64,
    pub grid_frequency_max_hz: f64,

    /// Battery temperature limits (°C)
    pub battery_temp_min_c: f64,
    pub battery_temp_max_c: f64,

    /// Battery SoC limits (%)
    pub battery_soc_min_percent: f64,
    pub battery_soc_max_percent: f64,

    /// Control loop watchdog timeout (seconds)
    /// Trigger emergency stop if control loop hasn't updated in this time
    pub control_loop_timeout_s: u64,

    /// Enable emergency stop on violations
    pub enable_emergency_stop: bool,
}

impl Default for SafetyMonitorConfig {
    fn default() -> Self {
        Self {
            check_interval_s: 1, // Fast 1-second monitoring
            fuse_rating_a: 25.0,
            fuse_trip_margin: 0.1, // Trip at 90% of rating
            grid_voltage_min_v: 207.0, // 230V -10%
            grid_voltage_max_v: 253.0, // 230V +10%
            grid_frequency_min_hz: 49.5, // 50Hz -1%
            grid_frequency_max_hz: 50.5, // 50Hz +1%
            battery_temp_min_c: -10.0,
            battery_temp_max_c: 55.0, // Conservative for LiFePO4
            battery_soc_min_percent: 5.0, // Protect from deep discharge
            battery_soc_max_percent: 95.0, // Protect from overcharge
            control_loop_timeout_s: 30, // 3x expected loop time
            enable_emergency_stop: true,
        }
    }
}

impl SafetyMonitorConfig {
    /// Conservative profile (tighter safety margins)
    pub fn conservative() -> Self {
        Self {
            fuse_trip_margin: 0.15, // Trip at 85% of rating
            battery_temp_max_c: 45.0, // Lower max temperature
            battery_soc_min_percent: 10.0, // Higher minimum
            battery_soc_max_percent: 90.0, // Lower maximum
            control_loop_timeout_s: 20,
            ..Default::default()
        }
    }

    /// Relaxed profile (looser safety margins, for testing)
    pub fn relaxed() -> Self {
        Self {
            fuse_trip_margin: 0.05, // Trip at 95% of rating
            battery_temp_max_c: 60.0,
            battery_soc_min_percent: 3.0,
            battery_soc_max_percent: 97.0,
            control_loop_timeout_s: 60,
            ..Default::default()
        }
    }
}

/// Safety violation type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SafetyViolationType {
    /// Grid import current exceeds fuse limit
    FuseOvercurrent,
    /// Grid voltage out of range
    GridVoltageViolation,
    /// Grid frequency out of range
    GridFrequencyViolation,
    /// Battery temperature too high
    BatteryOverTemperature,
    /// Battery temperature too low
    BatteryUnderTemperature,
    /// Battery SoC too low
    BatteryUnderCharge,
    /// Battery SoC too high
    BatteryOverCharge,
    /// Control loop hasn't updated recently
    ControlLoopTimeout,
}

impl std::fmt::Display for SafetyViolationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FuseOvercurrent => write!(f, "Fuse Overcurrent"),
            Self::GridVoltageViolation => write!(f, "Grid Voltage Violation"),
            Self::GridFrequencyViolation => write!(f, "Grid Frequency Violation"),
            Self::BatteryOverTemperature => write!(f, "Battery Over-Temperature"),
            Self::BatteryUnderTemperature => write!(f, "Battery Under-Temperature"),
            Self::BatteryUnderCharge => write!(f, "Battery Under-Charge"),
            Self::BatteryOverCharge => write!(f, "Battery Over-Charge"),
            Self::ControlLoopTimeout => write!(f, "Control Loop Timeout"),
        }
    }
}

/// Safety violation event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyViolation {
    pub violation_type: SafetyViolationType,
    pub timestamp: DateTime<Utc>,
    pub value: f64,
    pub limit: f64,
    pub message: String,
}

impl SafetyViolation {
    pub fn new(
        violation_type: SafetyViolationType,
        value: f64,
        limit: f64,
        message: String,
    ) -> Self {
        Self {
            violation_type,
            timestamp: Utc::now(),
            value,
            limit,
            message,
        }
    }
}

/// Safety monitor command
#[derive(Debug, Clone)]
pub enum SafetyCommand {
    /// Emergency stop all devices
    EmergencyStop(SafetyViolation),
    /// Resume normal operation
    Resume,
}

/// Safety monitor state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyMonitorState {
    /// Is monitoring active
    pub active: bool,
    /// Is system in emergency stop state
    pub emergency_stop_active: bool,
    /// Last violation (if any)
    pub last_violation: Option<SafetyViolation>,
    /// Total violations detected
    pub total_violations: u64,
    /// Last check timestamp
    pub last_check: DateTime<Utc>,
    /// Last control loop heartbeat
    pub last_control_loop_heartbeat: DateTime<Utc>,
}

impl Default for SafetyMonitorState {
    fn default() -> Self {
        Self {
            active: false,
            emergency_stop_active: false,
            last_violation: None,
            total_violations: 0,
            last_check: Utc::now(),
            last_control_loop_heartbeat: Utc::now(),
        }
    }
}

/// System measurements for safety monitoring
#[derive(Debug, Clone)]
pub struct SafetyMeasurements {
    /// Grid import power (kW)
    pub grid_import_kw: f64,
    /// Grid voltage (V)
    pub grid_voltage_v: f64,
    /// Grid frequency (Hz)
    pub grid_frequency_hz: f64,
    /// Battery SoC (%)
    pub battery_soc_percent: f64,
    /// Battery temperature (°C)
    pub battery_temperature_c: f64,
    /// Grid nominal voltage (V) for current calculation
    pub grid_nominal_voltage_v: f64,
    /// CRITICAL SAFETY FIX: Timestamp of measurements
    /// Used to detect stale data (e.g., Modbus thread hung)
    pub timestamp: DateTime<Utc>,
}

impl Default for SafetyMeasurements {
    fn default() -> Self {
        Self {
            grid_import_kw: 0.0,
            grid_voltage_v: 230.0,
            grid_frequency_hz: 50.0,
            battery_soc_percent: 50.0,
            battery_temperature_c: 25.0,
            grid_nominal_voltage_v: 230.0,
            timestamp: Utc::now(),
        }
    }
}

/// Safety monitor supervisor
pub struct SafetyMonitor {
    config: SafetyMonitorConfig,
    state: Arc<RwLock<SafetyMonitorState>>,
    command_tx: broadcast::Sender<SafetyCommand>,
    /// Direct hardware references for emergency shutdown
    /// CRITICAL: These bypass message passing to ensure shutdown even if control loop hangs
    battery: Option<Arc<dyn Battery>>,
    inverter: Option<Arc<dyn Inverter>>,
    /// CRITICAL FIX: Rate limiting for violation warnings to prevent log spam / SD card wear
    /// Maps violation type to last warning timestamp
    last_warning_time: Arc<RwLock<HashMap<SafetyViolationType, DateTime<Utc>>>>,
}

impl SafetyMonitor {
    /// Create a new safety monitor
    pub fn new(config: SafetyMonitorConfig) -> (Self, broadcast::Receiver<SafetyCommand>) {
        let (command_tx, command_rx) = broadcast::channel(32);

        let monitor = Self {
            config,
            state: Arc::new(RwLock::new(SafetyMonitorState::default())),
            command_tx,
            battery: None,
            inverter: None,
            last_warning_time: Arc::new(RwLock::new(HashMap::new())),
        };

        (monitor, command_rx)
    }

    /// Set hardware references for direct emergency shutdown
    /// CRITICAL: Must be called during initialization to enable hardware-level safety
    pub fn set_hardware(
        &mut self,
        battery: Option<Arc<dyn Battery>>,
        inverter: Option<Arc<dyn Inverter>>,
    ) {
        self.battery = battery;
        self.inverter = inverter;
        info!("Safety monitor hardware references configured");
    }

    /// Get current state
    pub async fn state(&self) -> SafetyMonitorState {
        self.state.read().await.clone()
    }

    /// Update control loop heartbeat (call from main control loop)
    pub async fn heartbeat(&self) {
        let mut state = self.state.write().await;
        state.last_control_loop_heartbeat = Utc::now();
        debug!("Safety monitor received control loop heartbeat");
    }

    /// Check for safety violations
    pub async fn check_safety(&self, measurements: &SafetyMeasurements) -> Vec<SafetyViolation> {
        let mut violations = Vec::new();

        // CRITICAL SAFETY CHECK: Detect stale measurements
        // If the Modbus reader thread hangs but leaves old measurements in memory,
        // we could be validating against data from 5 minutes ago
        const MAX_MEASUREMENT_AGE_SECONDS: i64 = 10;
        let now = Utc::now();
        let measurement_age = (now - measurements.timestamp).num_seconds();

        if measurement_age > MAX_MEASUREMENT_AGE_SECONDS {
            violations.push(SafetyViolation::new(
                SafetyViolationType::ControlLoopTimeout,
                measurement_age as f64,
                MAX_MEASUREMENT_AGE_SECONDS as f64,
                format!(
                    "Stale measurements detected - data is {} seconds old (max: {}s)",
                    measurement_age, MAX_MEASUREMENT_AGE_SECONDS
                ),
            ));
            // Return early - don't trust stale data for other checks
            return violations;
        }

        // Check fuse overcurrent
        // CRITICAL FIX: Protect against division by zero/near-zero voltage
        let grid_current_a = if measurements.grid_nominal_voltage_v < 1.0 {
            // Voltage sensor fault or blackout - assume maximum current to trigger safety
            warn!(
                "Grid voltage sensor fault or blackout ({}V < 1V) - assuming max current",
                measurements.grid_nominal_voltage_v
            );
            self.config.fuse_rating_a * 2.0 // Trigger overcurrent fault
        } else {
            measurements.grid_import_kw * 1000.0 / measurements.grid_nominal_voltage_v
        };
        let fuse_trip_threshold = self.config.fuse_rating_a * (1.0 - self.config.fuse_trip_margin);

        if grid_current_a > fuse_trip_threshold {
            violations.push(SafetyViolation::new(
                SafetyViolationType::FuseOvercurrent,
                grid_current_a,
                fuse_trip_threshold,
                format!(
                    "Grid current {:.1}A exceeds fuse trip threshold {:.1}A",
                    grid_current_a, fuse_trip_threshold
                ),
            ));
        }

        // Check grid voltage
        if measurements.grid_voltage_v < self.config.grid_voltage_min_v {
            violations.push(SafetyViolation::new(
                SafetyViolationType::GridVoltageViolation,
                measurements.grid_voltage_v,
                self.config.grid_voltage_min_v,
                format!(
                    "Grid voltage {:.1}V below minimum {:.1}V",
                    measurements.grid_voltage_v, self.config.grid_voltage_min_v
                ),
            ));
        }

        if measurements.grid_voltage_v > self.config.grid_voltage_max_v {
            violations.push(SafetyViolation::new(
                SafetyViolationType::GridVoltageViolation,
                measurements.grid_voltage_v,
                self.config.grid_voltage_max_v,
                format!(
                    "Grid voltage {:.1}V above maximum {:.1}V",
                    measurements.grid_voltage_v, self.config.grid_voltage_max_v
                ),
            ));
        }

        // Check grid frequency
        if measurements.grid_frequency_hz < self.config.grid_frequency_min_hz {
            violations.push(SafetyViolation::new(
                SafetyViolationType::GridFrequencyViolation,
                measurements.grid_frequency_hz,
                self.config.grid_frequency_min_hz,
                format!(
                    "Grid frequency {:.2}Hz below minimum {:.2}Hz",
                    measurements.grid_frequency_hz, self.config.grid_frequency_min_hz
                ),
            ));
        }

        if measurements.grid_frequency_hz > self.config.grid_frequency_max_hz {
            violations.push(SafetyViolation::new(
                SafetyViolationType::GridFrequencyViolation,
                measurements.grid_frequency_hz,
                self.config.grid_frequency_max_hz,
                format!(
                    "Grid frequency {:.2}Hz above maximum {:.2}Hz",
                    measurements.grid_frequency_hz, self.config.grid_frequency_max_hz
                ),
            ));
        }

        // Check battery temperature
        if measurements.battery_temperature_c < self.config.battery_temp_min_c {
            violations.push(SafetyViolation::new(
                SafetyViolationType::BatteryUnderTemperature,
                measurements.battery_temperature_c,
                self.config.battery_temp_min_c,
                format!(
                    "Battery temperature {:.1}°C below minimum {:.1}°C",
                    measurements.battery_temperature_c, self.config.battery_temp_min_c
                ),
            ));
        }

        if measurements.battery_temperature_c > self.config.battery_temp_max_c {
            violations.push(SafetyViolation::new(
                SafetyViolationType::BatteryOverTemperature,
                measurements.battery_temperature_c,
                self.config.battery_temp_max_c,
                format!(
                    "Battery temperature {:.1}°C above maximum {:.1}°C",
                    measurements.battery_temperature_c, self.config.battery_temp_max_c
                ),
            ));
        }

        // Check battery SoC
        if measurements.battery_soc_percent < self.config.battery_soc_min_percent {
            violations.push(SafetyViolation::new(
                SafetyViolationType::BatteryUnderCharge,
                measurements.battery_soc_percent,
                self.config.battery_soc_min_percent,
                format!(
                    "Battery SoC {:.1}% below minimum {:.1}%",
                    measurements.battery_soc_percent, self.config.battery_soc_min_percent
                ),
            ));
        }

        if measurements.battery_soc_percent > self.config.battery_soc_max_percent {
            violations.push(SafetyViolation::new(
                SafetyViolationType::BatteryOverCharge,
                measurements.battery_soc_percent,
                self.config.battery_soc_max_percent,
                format!(
                    "Battery SoC {:.1}% above maximum {:.1}%",
                    measurements.battery_soc_percent, self.config.battery_soc_max_percent
                ),
            ));
        }

        // Check control loop timeout
        let state = self.state.read().await;
        let time_since_heartbeat = Utc::now() - state.last_control_loop_heartbeat;
        drop(state);

        if time_since_heartbeat > Duration::seconds(self.config.control_loop_timeout_s as i64) {
            violations.push(SafetyViolation::new(
                SafetyViolationType::ControlLoopTimeout,
                time_since_heartbeat.num_seconds() as f64,
                self.config.control_loop_timeout_s as f64,
                format!(
                    "Control loop hasn't updated in {} seconds (timeout: {}s)",
                    time_since_heartbeat.num_seconds(),
                    self.config.control_loop_timeout_s
                ),
            ));
        }

        // Update state
        let mut state = self.state.write().await;
        state.last_check = Utc::now();

        if !violations.is_empty() {
            state.total_violations += violations.len() as u64;
            state.last_violation = Some(violations[0].clone());

            const LOG_RATE_LIMIT_SECONDS: i64 = 60;
            let now = Utc::now();
            let mut last_warnings = self.last_warning_time.write().await;

            for violation in &violations {
                let should_log = last_warnings
                    .get(&violation.violation_type)
                    .map(|last_time| (now - *last_time).num_seconds() >= LOG_RATE_LIMIT_SECONDS)
                    .unwrap_or(true);

                if should_log {
                    warn!(
                        "SAFETY VIOLATION: {} - {}",
                        violation.violation_type, violation.message
                    );
                    last_warnings.insert(violation.violation_type, now);
                }
            }
            drop(last_warnings);

            if self.config.enable_emergency_stop {
                let was_already_stopped = state.emergency_stop_active;

                if !was_already_stopped {
                    error!("TRIGGERING EMERGENCY STOP due to safety violation");
                    state.emergency_stop_active = true;
                }

                drop(state);

                if let Some(battery) = &self.battery {
                    if let Err(e) = battery.emergency_shutdown().await {
                        error!("Failed to execute battery emergency shutdown: {}", e);
                    } else if !was_already_stopped {
                        info!("Battery emergency shutdown executed successfully");
                    }
                }

                if let Some(inverter) = &self.inverter {
                    if let Err(e) = inverter.emergency_shutdown().await {
                        error!("Failed to execute inverter emergency shutdown: {}", e);
                    } else if !was_already_stopped {
                        info!("Inverter emergency shutdown executed successfully");
                    }
                }

                if !was_already_stopped {
                    let cmd = SafetyCommand::EmergencyStop(violations[0].clone());
                    if let Err(e) = self.command_tx.send(cmd) {
                        error!("Failed to broadcast emergency stop command: {}", e);
                    }
                }
            }
        }

        violations
    }

    /// Manually trigger emergency stop
    pub async fn trigger_emergency_stop(&self, reason: String) {
        let violation = SafetyViolation::new(
            SafetyViolationType::ControlLoopTimeout, // Generic type
            0.0,
            0.0,
            reason,
        );

        let mut state = self.state.write().await;
        state.emergency_stop_active = true;
        state.last_violation = Some(violation.clone());
        state.total_violations += 1;

        error!("MANUAL EMERGENCY STOP: {}", violation.message);

        let cmd = SafetyCommand::EmergencyStop(violation);
        if let Err(e) = self.command_tx.send(cmd) {
            error!("Failed to broadcast emergency stop command: {}", e);
        }
    }

    /// Clear emergency stop and resume normal operation
    pub async fn resume(&self) -> Result<()> {
        let mut state = self.state.write().await;

        if !state.emergency_stop_active {
            return Ok(());
        }

        info!("Clearing emergency stop, resuming normal operation");
        state.emergency_stop_active = false;

        let cmd = SafetyCommand::Resume;
        if let Err(e) = self.command_tx.send(cmd) {
            error!("Failed to broadcast resume command: {}", e);
        }

        Ok(())
    }

    /// Start the safety monitoring loop
    pub async fn start_monitoring(&self) {
        let mut state = self.state.write().await;
        state.active = true;
        state.last_control_loop_heartbeat = Utc::now();
        info!("Safety monitor started");
    }

    /// Stop the safety monitoring loop
    pub async fn stop_monitoring(&self) {
        let mut state = self.state.write().await;
        state.active = false;
        info!("Safety monitor stopped");
    }

    /// Subscribe to safety commands
    pub fn subscribe(&self) -> broadcast::Receiver<SafetyCommand> {
        self.command_tx.subscribe()
    }

    /// Validate power command against instantaneous safety limits
    /// CRITICAL SAFETY FIX: Added to prevent controllers from bypassing safety checks
    ///
    /// This validates commands against current measurements and limits.
    /// Returns an error if the command would violate safety constraints.
    pub async fn validate_power_command(
        &self,
        command_description: &str,
        power_w: f64,
        measurements: &SafetyMeasurements,
    ) -> Result<()> {
        // Check if emergency stop is active
        let state = self.state.read().await;
        if state.emergency_stop_active {
            anyhow::bail!(
                "Safety validation failed for {}: Emergency stop is active",
                command_description
            );
        }
        drop(state);

        // Validate power is finite
        if !power_w.is_finite() {
            anyhow::bail!(
                "Safety validation failed for {}: Power value is not finite ({})",
                command_description,
                power_w
            );
        }

        // Validate against fuse rating (check if this additional power would overload)
        let voltage_v = if measurements.grid_nominal_voltage_v < 1.0 {
            self.config.grid_voltage_min_v  // Use minimum valid voltage
        } else {
            measurements.grid_nominal_voltage_v
        };

        // Calculate current that would result from this power command
        let additional_current_a = power_w.abs() / voltage_v / 1000.0;
        let current_grid_current_a = measurements.grid_import_kw * 1000.0 / voltage_v;
        let total_current_a = current_grid_current_a + additional_current_a;

        let fuse_trip_threshold =
            self.config.fuse_rating_a * (1.0 - self.config.fuse_trip_margin);

        if total_current_a > fuse_trip_threshold {
            anyhow::bail!(
                "Safety validation failed for {}: Would exceed fuse limit (total current {:.1}A > {:.1}A threshold)",
                command_description,
                total_current_a,
                fuse_trip_threshold
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_safety_monitor_creation() {
        let config = SafetyMonitorConfig::default();
        let (monitor, _rx) = SafetyMonitor::new(config);

        let state = monitor.state().await;
        assert!(!state.active);
        assert!(!state.emergency_stop_active);
        assert_eq!(state.total_violations, 0);
    }

    #[tokio::test]
    async fn test_fuse_overcurrent_detection() {
        let config = SafetyMonitorConfig {
            fuse_rating_a: 25.0,
            fuse_trip_margin: 0.1,
            enable_emergency_stop: false,
            ..Default::default()
        };
        let (monitor, _rx) = SafetyMonitor::new(config);

        monitor.start_monitoring().await;

        // Simulate overcurrent: 30A (exceeds 22.5A trip threshold)
        let measurements = SafetyMeasurements {
            grid_import_kw: 6.9, // 6.9kW / 230V = 30A
            grid_nominal_voltage_v: 230.0,
            ..Default::default()
        };

        let violations = monitor.check_safety(&measurements).await;
        assert!(!violations.is_empty());
        assert_eq!(violations[0].violation_type, SafetyViolationType::FuseOvercurrent);
    }

    #[tokio::test]
    async fn test_grid_voltage_violation() {
        let config = SafetyMonitorConfig {
            grid_voltage_min_v: 207.0,
            grid_voltage_max_v: 253.0,
            enable_emergency_stop: false,
            ..Default::default()
        };
        let (monitor, _rx) = SafetyMonitor::new(config);

        // Test under-voltage
        let measurements = SafetyMeasurements {
            grid_voltage_v: 200.0, // Below 207V minimum
            ..Default::default()
        };

        let violations = monitor.check_safety(&measurements).await;
        assert!(!violations.is_empty());
        assert_eq!(
            violations[0].violation_type,
            SafetyViolationType::GridVoltageViolation
        );

        // Test over-voltage
        let measurements = SafetyMeasurements {
            grid_voltage_v: 260.0, // Above 253V maximum
            ..Default::default()
        };

        let violations = monitor.check_safety(&measurements).await;
        assert!(!violations.is_empty());
        assert_eq!(
            violations[0].violation_type,
            SafetyViolationType::GridVoltageViolation
        );
    }

    #[tokio::test]
    async fn test_battery_temperature_violation() {
        let config = SafetyMonitorConfig {
            battery_temp_max_c: 55.0,
            enable_emergency_stop: false,
            ..Default::default()
        };
        let (monitor, _rx) = SafetyMonitor::new(config);

        let measurements = SafetyMeasurements {
            battery_temperature_c: 60.0, // Above 55°C maximum
            ..Default::default()
        };

        let violations = monitor.check_safety(&measurements).await;
        assert!(!violations.is_empty());
        assert_eq!(
            violations[0].violation_type,
            SafetyViolationType::BatteryOverTemperature
        );
    }

    #[tokio::test]
    async fn test_battery_soc_violation() {
        let config = SafetyMonitorConfig {
            battery_soc_min_percent: 5.0,
            battery_soc_max_percent: 95.0,
            enable_emergency_stop: false,
            ..Default::default()
        };
        let (monitor, _rx) = SafetyMonitor::new(config);

        // Test under-charge
        let measurements = SafetyMeasurements {
            battery_soc_percent: 3.0, // Below 5% minimum
            ..Default::default()
        };

        let violations = monitor.check_safety(&measurements).await;
        assert!(!violations.is_empty());
        assert_eq!(
            violations[0].violation_type,
            SafetyViolationType::BatteryUnderCharge
        );

        // Test over-charge
        let measurements = SafetyMeasurements {
            battery_soc_percent: 97.0, // Above 95% maximum
            ..Default::default()
        };

        let violations = monitor.check_safety(&measurements).await;
        assert!(!violations.is_empty());
        assert_eq!(
            violations[0].violation_type,
            SafetyViolationType::BatteryOverCharge
        );
    }

    #[tokio::test]
    async fn test_control_loop_timeout() {
        let config = SafetyMonitorConfig {
            control_loop_timeout_s: 1, // 1 second timeout for test
            enable_emergency_stop: false,
            ..Default::default()
        };
        let (monitor, _rx) = SafetyMonitor::new(config);

        monitor.start_monitoring().await;

        // Wait for timeout to trigger
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        let measurements = SafetyMeasurements::default();
        let violations = monitor.check_safety(&measurements).await;

        assert!(!violations.is_empty());
        assert_eq!(
            violations[0].violation_type,
            SafetyViolationType::ControlLoopTimeout
        );
    }

    #[tokio::test]
    async fn test_heartbeat() {
        let config = SafetyMonitorConfig {
            control_loop_timeout_s: 1,
            enable_emergency_stop: false,
            ..Default::default()
        };
        let (monitor, _rx) = SafetyMonitor::new(config);

        monitor.start_monitoring().await;

        // Send heartbeat
        monitor.heartbeat().await;

        // Should not timeout immediately
        let measurements = SafetyMeasurements::default();
        let violations = monitor.check_safety(&measurements).await;
        assert!(violations.is_empty());
    }

    #[tokio::test]
    async fn test_emergency_stop_broadcast() {
        let config = SafetyMonitorConfig {
            enable_emergency_stop: true,
            fuse_rating_a: 25.0,
            ..Default::default()
        };
        let (monitor, mut rx) = SafetyMonitor::new(config);

        monitor.start_monitoring().await;

        // Trigger emergency stop via overcurrent
        let measurements = SafetyMeasurements {
            grid_import_kw: 10.0, // Way over fuse limit
            grid_nominal_voltage_v: 230.0,
            ..Default::default()
        };

        monitor.check_safety(&measurements).await;

        // Should receive emergency stop command
        let cmd = rx.try_recv();
        assert!(cmd.is_ok());
        match cmd.unwrap() {
            SafetyCommand::EmergencyStop(violation) => {
                assert_eq!(violation.violation_type, SafetyViolationType::FuseOvercurrent);
            }
            _ => panic!("Expected EmergencyStop command"),
        }
    }

    #[tokio::test]
    async fn test_resume_after_emergency_stop() {
        let config = SafetyMonitorConfig {
            enable_emergency_stop: true,
            ..Default::default()
        };
        let (monitor, mut rx) = SafetyMonitor::new(config);

        // Trigger manual emergency stop
        monitor
            .trigger_emergency_stop("Test emergency stop".to_string())
            .await;

        let state = monitor.state().await;
        assert!(state.emergency_stop_active);

        // Resume
        monitor.resume().await.unwrap();

        let state = monitor.state().await;
        assert!(!state.emergency_stop_active);

        // Should receive resume command
        let cmd = rx.try_recv();
        assert!(cmd.is_ok());
        match cmd.unwrap() {
            SafetyCommand::Resume => (),
            _ => panic!("Expected Resume command"),
        }
    }

    #[tokio::test]
    async fn test_no_violations_with_normal_measurements() {
        let config = SafetyMonitorConfig::default();
        let (monitor, _rx) = SafetyMonitor::new(config);

        monitor.start_monitoring().await;
        monitor.heartbeat().await;

        // Normal measurements within all limits
        let measurements = SafetyMeasurements {
            grid_import_kw: 2.0,          // Well below fuse limit
            grid_voltage_v: 230.0,        // Nominal
            grid_frequency_hz: 50.0,      // Nominal
            battery_soc_percent: 50.0,    // Mid-range
            battery_temperature_c: 25.0,  // Room temperature
            grid_nominal_voltage_v: 230.0,
            timestamp: chrono::Utc::now(),
        };

        let violations = monitor.check_safety(&measurements).await;
        assert!(violations.is_empty());

        let state = monitor.state().await;
        assert_eq!(state.total_violations, 0);
    }
}
