//! # Power Transition and Ramping
//!
//! Provides smooth power transitions to prevent:
//! - Fuse trips due to sudden current spikes
//! - Battery damage from rapid charge/discharge changes
//! - Grid instability from abrupt power changes
//! - Inverter stress from instant load changes
//!
//! Implements rate-limiting (dP/dt constraints) to gradually transition
//! from current power to target power.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Power ramp configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerRampConfig {
    /// Maximum rate of power change in W/s (watts per second)
    pub max_ramp_rate_w_per_s: f64,

    /// Minimum power change that triggers ramping (W)
    /// Changes smaller than this are applied instantly
    pub min_ramp_threshold_w: f64,

    /// Emergency mode: allow instant changes (disables ramping)
    pub emergency_mode: bool,
}

impl Default for PowerRampConfig {
    fn default() -> Self {
        Self {
            max_ramp_rate_w_per_s: 500.0, // 500W/s = 3kW in 6 seconds
            min_ramp_threshold_w: 100.0,   // Don't ramp for changes < 100W
            emergency_mode: false,
        }
    }
}

impl PowerRampConfig {
    /// Conservative profile for long battery life
    pub fn conservative() -> Self {
        Self {
            max_ramp_rate_w_per_s: 200.0, // Slower ramp
            min_ramp_threshold_w: 50.0,
            emergency_mode: false,
        }
    }

    /// Aggressive profile for fast response
    pub fn aggressive() -> Self {
        Self {
            max_ramp_rate_w_per_s: 1000.0, // Faster ramp
            min_ramp_threshold_w: 200.0,
            emergency_mode: false,
        }
    }

    /// Emergency profile (no ramping, instant changes)
    pub fn emergency() -> Self {
        Self {
            max_ramp_rate_w_per_s: f64::INFINITY,
            min_ramp_threshold_w: 0.0,
            emergency_mode: true,
        }
    }
}

/// Power ramp state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerRampState {
    /// Current power (W)
    pub current_power_w: f64,

    /// Target power (W)
    pub target_power_w: f64,

    /// Timestamp of last update
    pub last_update: DateTime<Utc>,

    /// Whether ramping is currently active
    pub is_ramping: bool,

    /// Estimated time to reach target (seconds)
    pub estimated_completion_s: f64,
}

impl Default for PowerRampState {
    fn default() -> Self {
        Self {
            current_power_w: 0.0,
            target_power_w: 0.0,
            last_update: Utc::now(),
            is_ramping: false,
            estimated_completion_s: 0.0,
        }
    }
}

/// Power ramper - manages smooth power transitions
pub struct PowerRamp {
    config: PowerRampConfig,
    state: PowerRampState,
}

impl PowerRamp {
    /// Create a new power ramper
    pub fn new(config: PowerRampConfig) -> Self {
        Self {
            config,
            state: PowerRampState::default(),
        }
    }

    /// Create with default configuration
    pub fn default() -> Self {
        Self::new(PowerRampConfig::default())
    }

    /// Create with a specific initial power
    pub fn with_initial_power(config: PowerRampConfig, initial_power_w: f64) -> Self {
        let mut state = PowerRampState::default();
        state.current_power_w = initial_power_w;
        state.target_power_w = initial_power_w;

        Self { config, state }
    }

    /// Get current configuration
    pub fn config(&self) -> &PowerRampConfig {
        &self.config
    }

    /// Update configuration
    pub fn set_config(&mut self, config: PowerRampConfig) {
        self.config = config;
    }

    /// Get current state
    pub fn state(&self) -> &PowerRampState {
        &self.state
    }

    /// Get current power
    pub fn current_power_w(&self) -> f64 {
        self.state.current_power_w
    }

    /// Get target power
    pub fn target_power_w(&self) -> f64 {
        self.state.target_power_w
    }

    /// Check if ramping is active
    pub fn is_ramping(&self) -> bool {
        self.state.is_ramping
    }

    /// Set a new target power
    ///
    /// The ramper will gradually transition from current power to this target.
    pub fn set_target(&mut self, target_power_w: f64) {
        self.state.target_power_w = target_power_w;

        let power_delta = (target_power_w - self.state.current_power_w).abs();

        // Determine if ramping is needed
        if self.config.emergency_mode
            || power_delta < self.config.min_ramp_threshold_w {
            // Instant change
            self.state.current_power_w = target_power_w;
            self.state.is_ramping = false;
            self.state.estimated_completion_s = 0.0;
        } else {
            // Ramping needed
            self.state.is_ramping = true;
            self.state.estimated_completion_s =
                power_delta / self.config.max_ramp_rate_w_per_s;
        }

        self.state.last_update = Utc::now();
    }

    /// Update the ramper and get the current power setpoint
    ///
    /// Call this periodically (e.g., every control loop iteration) to advance
    /// the ramp toward the target.
    ///
    /// Returns the current power setpoint to apply to the device.
    pub fn update(&mut self) -> f64 {
        if !self.state.is_ramping {
            return self.state.current_power_w;
        }

        let now = Utc::now();
        let elapsed = now - self.state.last_update;
        let elapsed_s = elapsed.num_milliseconds() as f64 / 1000.0;

        if elapsed_s <= 0.0 {
            return self.state.current_power_w;
        }

        // Calculate maximum power change allowed
        let max_delta = self.config.max_ramp_rate_w_per_s * elapsed_s;

        // Calculate actual power change
        let target_delta = self.state.target_power_w - self.state.current_power_w;
        let actual_delta = if target_delta.abs() <= max_delta {
            // Reached target
            target_delta
        } else {
            // Still ramping
            max_delta * target_delta.signum()
        };

        // Update current power
        self.state.current_power_w += actual_delta;

        // Check if we've reached the target
        let remaining_delta = (self.state.target_power_w - self.state.current_power_w).abs();
        if remaining_delta < 1.0 {
            // Close enough - snap to target
            self.state.current_power_w = self.state.target_power_w;
            self.state.is_ramping = false;
            self.state.estimated_completion_s = 0.0;
        } else {
            self.state.estimated_completion_s =
                remaining_delta / self.config.max_ramp_rate_w_per_s;
        }

        self.state.last_update = now;
        self.state.current_power_w
    }

    /// Force instant change to target (override ramping)
    pub fn force_instant(&mut self) {
        self.state.current_power_w = self.state.target_power_w;
        self.state.is_ramping = false;
        self.state.estimated_completion_s = 0.0;
        self.state.last_update = Utc::now();
    }

    /// Reset to zero power
    pub fn reset(&mut self) {
        self.state.current_power_w = 0.0;
        self.state.target_power_w = 0.0;
        self.state.is_ramping = false;
        self.state.estimated_completion_s = 0.0;
        self.state.last_update = Utc::now();
    }

    /// Get estimated time to completion in seconds
    pub fn estimated_completion_time(&self) -> f64 {
        self.state.estimated_completion_s
    }

    /// Calculate the power that will be reached after a given duration
    pub fn predict_power_at(&self, duration: Duration) -> f64 {
        if !self.state.is_ramping {
            return self.state.current_power_w;
        }

        let duration_s = duration.num_milliseconds() as f64 / 1000.0;
        let max_delta = self.config.max_ramp_rate_w_per_s * duration_s;
        let target_delta = self.state.target_power_w - self.state.current_power_w;

        if target_delta.abs() <= max_delta {
            // Will reach target
            self.state.target_power_w
        } else {
            // Still ramping
            self.state.current_power_w + (max_delta * target_delta.signum())
        }
    }
}

impl fmt::Display for PowerRamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "PowerRamp(current={:.0}W, target={:.0}W, ramping={}, eta={:.1}s)",
            self.state.current_power_w,
            self.state.target_power_w,
            self.state.is_ramping,
            self.state.estimated_completion_s
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;
    use std::time::Duration as StdDuration;

    #[test]
    fn test_instant_change_below_threshold() {
        let mut ramp = PowerRamp::default();

        // Small change (< 100W threshold) should be instant
        ramp.set_target(50.0);
        assert!(!ramp.is_ramping());
        assert_eq!(ramp.current_power_w(), 50.0);
    }

    #[test]
    fn test_ramping_large_change() {
        let config = PowerRampConfig {
            max_ramp_rate_w_per_s: 500.0,
            min_ramp_threshold_w: 100.0,
            emergency_mode: false,
        };
        let mut ramp = PowerRamp::new(config);

        // Large change (1000W) should trigger ramping
        ramp.set_target(1000.0);
        assert!(ramp.is_ramping());
        assert_eq!(ramp.current_power_w(), 0.0); // Not moved yet

        // Estimated completion: 1000W / 500W/s = 2 seconds
        assert!((ramp.estimated_completion_time() - 2.0).abs() < 0.1);
    }

    #[test]
    fn test_ramping_progression() {
        let config = PowerRampConfig {
            max_ramp_rate_w_per_s: 1000.0, // 1000W/s for faster test
            min_ramp_threshold_w: 100.0,
            emergency_mode: false,
        };
        let mut ramp = PowerRamp::new(config);

        ramp.set_target(2000.0);
        assert!(ramp.is_ramping());

        // Simulate 0.5 second delay
        sleep(StdDuration::from_millis(500));
        let power = ramp.update();

        // Should have ramped ~500W (1000W/s * 0.5s)
        // Allow some tolerance for timing variations
        assert!(power > 400.0 && power < 600.0);
        assert!(ramp.is_ramping()); // Still ramping to 2000W
    }

    #[test]
    fn test_emergency_mode() {
        let mut config = PowerRampConfig::default();
        config.emergency_mode = true;

        let mut ramp = PowerRamp::new(config);

        // Even large change should be instant in emergency mode
        ramp.set_target(5000.0);
        assert!(!ramp.is_ramping());
        assert_eq!(ramp.current_power_w(), 5000.0);
    }

    #[test]
    fn test_force_instant() {
        let mut ramp = PowerRamp::default();

        ramp.set_target(2000.0);
        assert!(ramp.is_ramping());

        // Force instant change
        ramp.force_instant();
        assert!(!ramp.is_ramping());
        assert_eq!(ramp.current_power_w(), 2000.0);
    }

    #[test]
    fn test_negative_power_ramping() {
        let config = PowerRampConfig {
            max_ramp_rate_w_per_s: 500.0,
            min_ramp_threshold_w: 100.0,
            emergency_mode: false,
        };
        let mut ramp = PowerRamp::with_initial_power(config, 1000.0);

        // Ramp down to negative (discharge)
        ramp.set_target(-1000.0);
        assert!(ramp.is_ramping());

        // Should ramp down gradually
        sleep(StdDuration::from_millis(500));
        let power = ramp.update();

        // Should have decreased by ~250W (500W/s * 0.5s)
        assert!(power < 1000.0 && power > 500.0);
    }

    #[test]
    fn test_reset() {
        let mut ramp = PowerRamp::with_initial_power(PowerRampConfig::default(), 1000.0);

        ramp.set_target(2000.0);
        assert!(ramp.is_ramping());

        ramp.reset();
        assert_eq!(ramp.current_power_w(), 0.0);
        assert_eq!(ramp.target_power_w(), 0.0);
        assert!(!ramp.is_ramping());
    }

    #[test]
    fn test_predict_power_at() {
        let config = PowerRampConfig {
            max_ramp_rate_w_per_s: 1000.0,
            min_ramp_threshold_w: 100.0,
            emergency_mode: false,
        };
        let mut ramp = PowerRamp::new(config);

        ramp.set_target(3000.0);

        // Predict power after 1 second
        let predicted = ramp.predict_power_at(Duration::seconds(1));
        assert!((predicted - 1000.0).abs() < 10.0); // Should be ~1000W

        // Predict power after 5 seconds (should reach target)
        let predicted = ramp.predict_power_at(Duration::seconds(5));
        assert_eq!(predicted, 3000.0);
    }

    #[test]
    fn test_conservative_profile() {
        let config = PowerRampConfig::conservative();
        let mut ramp = PowerRamp::new(config);

        ramp.set_target(1000.0);

        // Should take longer due to slower ramp rate (200W/s)
        let eta = ramp.estimated_completion_time();
        assert!((eta - 5.0).abs() < 0.1); // 1000W / 200W/s = 5s
    }

    #[test]
    fn test_aggressive_profile() {
        let config = PowerRampConfig::aggressive();
        let mut ramp = PowerRamp::new(config);

        ramp.set_target(1000.0);

        // Should be faster due to higher ramp rate (1000W/s)
        let eta = ramp.estimated_completion_time();
        assert!((eta - 1.0).abs() < 0.1); // 1000W / 1000W/s = 1s
    }

    #[test]
    fn test_multiple_target_changes() {
        let config = PowerRampConfig {
            max_ramp_rate_w_per_s: 500.0,
            min_ramp_threshold_w: 100.0,
            emergency_mode: false,
        };
        let mut ramp = PowerRamp::new(config);

        // First target
        ramp.set_target(1000.0);
        sleep(StdDuration::from_millis(500));
        ramp.update();

        // Change target mid-ramp
        ramp.set_target(2000.0);
        assert!(ramp.is_ramping());

        // Should now ramp toward new target
        sleep(StdDuration::from_millis(500));
        let power = ramp.update();
        assert!(power > 500.0); // Should be progressing
    }
}
