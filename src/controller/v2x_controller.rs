#![allow(dead_code)]
//! V2X (Vehicle-to-Grid/Home) Controller
//!
//! This module manages bidirectional power flow between electric vehicles
//! and the grid/home through V2X-capable EV chargers.
//!
//! Key responsibilities:
//! - Coordinate EV discharge to support home loads or grid export
//! - Preserve minimum driving range (SoC threshold)
//! - Optimize discharge timing based on electricity prices
//! - Respect battery degradation constraints
//! - Integrate with battery optimizer for holistic energy management

use anyhow::{Context, Result};
use chrono::{DateTime, Timelike, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::controller::safety_monitor::SafetyMonitor;
use crate::domain::{EvCharger, V2XCapabilities};

/// V2X control mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum V2XMode {
    /// V2X disabled - vehicle only charges, never discharges
    Disabled,
    /// V2H (Vehicle-to-Home) - discharge to support home loads
    VehicleToHome,
    /// V2G (Vehicle-to-Grid) - discharge for grid export
    VehicleToGrid,
    /// Smart mode - automatically choose based on prices and conditions
    Smart,
}

/// V2X controller configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct V2XConfig {
    /// Control mode
    pub mode: V2XMode,
    /// Minimum vehicle SoC to preserve for driving (%)
    pub min_driving_range_soc: f64,
    /// Maximum discharge power (W) - can be less than charger capability
    pub max_discharge_power_w: f64,
    /// Minimum price differential to enable V2G (SEK/kWh)
    /// Only discharge if price exceeds average by this amount
    pub min_price_differential: f64,
    /// Enable during peak hours only
    pub peak_hours_only: bool,
    /// Peak hours definition (start hour, end hour in 24h format)
    pub peak_hours: (u8, u8),
}

impl Default for V2XConfig {
    fn default() -> Self {
        Self {
            mode: V2XMode::Disabled,
            min_driving_range_soc: 50.0, // Keep at least 50% for driving
            max_discharge_power_w: 11000.0, // 11kW default
            min_price_differential: 0.5, // 0.5 SEK/kWh above average
            peak_hours_only: true,
            peak_hours: (17, 21), // 5 PM - 9 PM
        }
    }
}

/// V2X discharge decision
#[derive(Debug, Clone)]
pub struct V2XDecision {
    /// Should discharge?
    pub should_discharge: bool,
    /// Target discharge power (W)
    pub target_power_w: f64,
    /// Reason for decision
    pub reason: String,
    /// Current vehicle SoC (%)
    pub vehicle_soc: f64,
}

/// V2X Controller
pub struct V2XController {
    /// Configuration
    config: Arc<RwLock<V2XConfig>>,
    /// Reference to the EV charger
    charger: Arc<dyn EvCharger>,
    /// V2X capabilities cache
    capabilities: Option<V2XCapabilities>,
    /// Safety monitor reference - CRITICAL for emergency stop integration
    safety_monitor: Option<Arc<SafetyMonitor>>,
}

impl V2XController {
    /// Create a new V2X controller
    pub fn new(charger: Arc<dyn EvCharger>, config: V2XConfig) -> Self {
        let capabilities = charger.v2x_capabilities();

        Self {
            config: Arc::new(RwLock::new(config)),
            charger,
            capabilities,
            safety_monitor: None,
        }
    }

    /// Set safety monitor reference for emergency stop integration
    /// CRITICAL: Should be called during initialization for proper safety integration
    pub fn set_safety_monitor(&mut self, safety_monitor: Arc<SafetyMonitor>) {
        self.safety_monitor = Some(safety_monitor);
        info!("V2X controller safety monitor integration configured");
    }

    /// Check if V2X is available and enabled
    pub fn is_v2x_available(&self) -> bool {
        self.capabilities.is_some()
    }

    /// Get current configuration
    pub async fn get_config(&self) -> V2XConfig {
        self.config.read().await.clone()
    }

    /// Update configuration
    pub async fn update_config(&self, config: V2XConfig) {
        *self.config.write().await = config;
    }

    /// Determine if vehicle should discharge at current conditions
    pub async fn evaluate_discharge_decision(
        &self,
        current_price: f64,
        average_price: f64,
        current_time: DateTime<Utc>,
    ) -> Result<V2XDecision> {
        let config = self.config.read().await.clone();

        // Check if V2X is available
        if !self.is_v2x_available() {
            return Ok(V2XDecision {
                should_discharge: false,
                target_power_w: 0.0,
                reason: "v2x_not_supported".to_string(),
                vehicle_soc: 0.0,
            });
        }

        // Check if V2X is enabled
        if config.mode == V2XMode::Disabled {
            return Ok(V2XDecision {
                should_discharge: false,
                target_power_w: 0.0,
                reason: "v2x_disabled".to_string(),
                vehicle_soc: 0.0,
            });
        }

        // Get charger state
        let state = self
            .charger
            .read_state()
            .await
            .context("Failed to read charger state")?;

        // Check if vehicle is connected
        if !state.connected {
            return Ok(V2XDecision {
                should_discharge: false,
                target_power_w: 0.0,
                reason: "vehicle_not_connected".to_string(),
                vehicle_soc: 0.0,
            });
        }

        // Get vehicle SoC
        let vehicle_soc = state.vehicle_soc_percent.unwrap_or(0.0);

        // Check minimum driving range
        if vehicle_soc < config.min_driving_range_soc {
            return Ok(V2XDecision {
                should_discharge: false,
                target_power_w: 0.0,
                reason: format!("soc_below_minimum_{}%", config.min_driving_range_soc),
                vehicle_soc,
            });
        }

        // Check peak hours if required
        if config.peak_hours_only {
            let hour = current_time.hour() as u8;
            let (start, end) = config.peak_hours;
            if hour < start || hour >= end {
                return Ok(V2XDecision {
                    should_discharge: false,
                    target_power_w: 0.0,
                    reason: "outside_peak_hours".to_string(),
                    vehicle_soc,
                });
            }
        }

        // Check price differential for V2G
        if matches!(config.mode, V2XMode::VehicleToGrid | V2XMode::Smart) {
            let price_diff = current_price - average_price;
            if price_diff < config.min_price_differential {
                return Ok(V2XDecision {
                    should_discharge: false,
                    target_power_w: 0.0,
                    reason: format!("price_too_low_diff_{:.2}", price_diff),
                    vehicle_soc,
                });
            }
        }

        // All conditions met - discharge
        let v2x_caps = self.capabilities.as_ref().unwrap();

        // Calculate target discharge power
        // Use available capacity above minimum driving range
        let available_soc = vehicle_soc - config.min_driving_range_soc;
        let discharge_factor = (available_soc / 50.0).min(1.0); // Scale from 0-1

        let target_power = config
            .max_discharge_power_w
            .min(v2x_caps.max_discharge_power_w)
            * discharge_factor;

        Ok(V2XDecision {
            should_discharge: true,
            target_power_w: target_power,
            reason: match config.mode {
                V2XMode::VehicleToHome => "v2h_active".to_string(),
                V2XMode::VehicleToGrid => "v2g_active".to_string(),
                V2XMode::Smart => "smart_discharge".to_string(),
                V2XMode::Disabled => unreachable!(),
            },
            vehicle_soc,
        })
    }

    /// Main control loop step
    ///
    /// This should be called periodically (e.g., every 10 seconds)
    pub async fn control_step(
        &self,
        current_price: f64,
        average_price: f64,
        current_time: DateTime<Utc>,
    ) -> Result<V2XDecision> {
        // CRITICAL SAFETY FIX: Check for emergency stop before any V2X operations
        if let Some(safety_monitor) = &self.safety_monitor {
            let safety_state = safety_monitor.state().await;
            if safety_state.emergency_stop_active {
                warn!("V2X control step aborted - emergency stop active");

                let state = self
                    .charger
                    .read_state()
                    .await
                    .context("Failed to read charger state during emergency stop")?;

                return Ok(V2XDecision {
                    should_discharge: false,
                    target_power_w: 0.0,
                    reason: "emergency_stop_active".to_string(),
                    vehicle_soc: state.vehicle_soc_percent.unwrap_or(0.0),
                });
            }
        }

        let decision = self
            .evaluate_discharge_decision(current_price, average_price, current_time)
            .await?;

        Ok(decision)
    }

    /// Get current V2X statistics
    pub async fn get_statistics(&self) -> Result<V2XStatistics> {
        let state = self.charger.read_state().await?;
        let config = self.config.read().await.clone();

        Ok(V2XStatistics {
            is_available: self.is_v2x_available(),
            is_enabled: config.mode != V2XMode::Disabled,
            is_discharging: state.discharging,
            current_discharge_power_w: if state.discharging {
                -state.power_w
            } else {
                0.0
            },
            total_energy_discharged_kwh: state.energy_discharged_kwh,
            vehicle_soc_percent: state.vehicle_soc_percent,
            min_driving_range_soc: config.min_driving_range_soc,
        })
    }
}

/// V2X statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct V2XStatistics {
    pub is_available: bool,
    pub is_enabled: bool,
    pub is_discharging: bool,
    pub current_discharge_power_w: f64,
    pub total_energy_discharged_kwh: f64,
    pub vehicle_soc_percent: Option<f64>,
    pub min_driving_range_soc: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{ChargerStatus, SimulatedEvCharger};
    use chrono::TimeZone;

    #[tokio::test]
    async fn test_v2x_controller_disabled() {
        let charger = Arc::new(SimulatedEvCharger::v2x_charger());
        let config = V2XConfig {
            mode: V2XMode::Disabled,
            ..Default::default()
        };

        let controller = V2XController::new(charger, config);

        let now = Utc::now();
        let decision = controller
            .evaluate_discharge_decision(2.0, 1.0, now)
            .await
            .unwrap();

        assert!(!decision.should_discharge);
        assert_eq!(decision.reason, "v2x_disabled");
    }

    #[tokio::test]
    async fn test_v2x_controller_no_vehicle() {
        let charger = Arc::new(SimulatedEvCharger::v2x_charger());
        let config = V2XConfig {
            mode: V2XMode::VehicleToHome,
            ..Default::default()
        };

        let controller = V2XController::new(charger, config);

        let now = Utc::now();
        let decision = controller
            .evaluate_discharge_decision(2.0, 1.0, now)
            .await
            .unwrap();

        assert!(!decision.should_discharge);
        assert_eq!(decision.reason, "vehicle_not_connected");
    }

    #[tokio::test]
    async fn test_v2x_controller_low_soc() {
        let charger = Arc::new(SimulatedEvCharger::v2x_charger());

        // Simulate connected vehicle with low SoC
        charger.simulate_connect().await;
        {
            let mut state = charger.state.write().await;
            state.vehicle_soc_percent = Some(30.0); // Below default 50% minimum
        }

        let config = V2XConfig {
            mode: V2XMode::VehicleToHome,
            min_driving_range_soc: 50.0,
            ..Default::default()
        };

        let controller = V2XController::new(charger, config);

        let now = Utc::now();
        let decision = controller
            .evaluate_discharge_decision(2.0, 1.0, now)
            .await
            .unwrap();

        assert!(!decision.should_discharge);
        assert!(decision.reason.contains("soc_below_minimum"));
    }

    #[tokio::test]
    async fn test_v2x_controller_peak_hours() {
        let charger = Arc::new(SimulatedEvCharger::v2x_charger());

        // Simulate connected vehicle with good SoC
        charger.simulate_connect().await;
        {
            let mut state = charger.state.write().await;
            state.vehicle_soc_percent = Some(80.0);
        }

        let config = V2XConfig {
            mode: V2XMode::VehicleToHome,
            min_driving_range_soc: 50.0,
            peak_hours_only: true,
            peak_hours: (17, 21), // 5 PM - 9 PM
            ..Default::default()
        };

        let controller = V2XController::new(charger, config);

        // Test during peak hours (6 PM)
        let peak_time = Utc.with_ymd_and_hms(2024, 6, 21, 18, 0, 0).unwrap();
        let decision = controller
            .evaluate_discharge_decision(2.0, 1.0, peak_time)
            .await
            .unwrap();

        assert!(
            decision.should_discharge,
            "Should discharge during peak hours"
        );

        // Test outside peak hours (10 AM)
        let off_peak = Utc.with_ymd_and_hms(2024, 6, 21, 10, 0, 0).unwrap();
        let decision = controller
            .evaluate_discharge_decision(2.0, 1.0, off_peak)
            .await
            .unwrap();

        assert!(
            !decision.should_discharge,
            "Should not discharge outside peak hours"
        );
        assert_eq!(decision.reason, "outside_peak_hours");
    }

    #[tokio::test]
    async fn test_v2x_controller_price_check() {
        let charger = Arc::new(SimulatedEvCharger::v2x_charger());

        // Simulate connected vehicle
        charger.simulate_connect().await;
        {
            let mut state = charger.state.write().await;
            state.vehicle_soc_percent = Some(80.0);
        }

        let config = V2XConfig {
            mode: V2XMode::VehicleToGrid,
            min_driving_range_soc: 50.0,
            peak_hours_only: false,
            min_price_differential: 0.5, // Need 0.5 SEK/kWh above average
            ..Default::default()
        };

        let controller = V2XController::new(charger, config);

        let now = Utc::now();

        // High price (above threshold)
        let decision = controller
            .evaluate_discharge_decision(2.0, 1.0, now)
            .await
            .unwrap();
        assert!(
            decision.should_discharge,
            "Should discharge when price is high"
        );

        // Low price (below threshold)
        let decision = controller
            .evaluate_discharge_decision(1.2, 1.0, now)
            .await
            .unwrap();
        assert!(
            !decision.should_discharge,
            "Should not discharge when price is low"
        );
        assert!(decision.reason.contains("price_too_low"));
    }
}
