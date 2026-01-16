#![allow(dead_code)]
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Physical constraints (hard limits that CANNOT be violated)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicalConstraints {
    /// Maximum grid import power (fuse limit)
    pub max_grid_import_kw: f64,

    /// Maximum grid export power
    pub max_grid_export_kw: f64,

    /// Maximum battery charge power
    pub max_battery_charge_kw: f64,

    /// Maximum battery discharge power
    pub max_battery_discharge_kw: f64,

    /// Minimum EV charger current (A)
    pub evse_min_current_a: f64,

    /// Maximum EV charger current (A)
    pub evse_max_current_a: f64,

    /// Number of phases (1 or 3)
    pub phases: u8,

    /// Maximum current per phase (optional, for 3-phase systems)
    pub max_current_per_phase_a: Option<f64>,

    /// Grid voltage (V, typically 230V single-phase or 400V three-phase)
    pub grid_voltage_v: f64,
}

impl Default for PhysicalConstraints {
    fn default() -> Self {
        // CRITICAL SAFETY FIX: Default to 0.0 (Safe Mode) for all power values
        // If configuration file is missing, corrupt, or fails to load,
        // the system MUST refuse to operate until explicit positive limits are loaded.
        // This prevents the controller from ramping power beyond actual fuse ratings.
        Self {
            max_grid_import_kw: 0.0,       // SAFE MODE: No power until explicitly configured
            max_grid_export_kw: 0.0,       // SAFE MODE: No power until explicitly configured
            max_battery_charge_kw: 0.0,    // SAFE MODE: No power until explicitly configured
            max_battery_discharge_kw: 0.0, // SAFE MODE: No power until explicitly configured
            evse_min_current_a: 6.0,       // IEC 61851 minimum (informational only)
            evse_max_current_a: 0.0,       // SAFE MODE: No EV charging until configured
            phases: 1,
            max_current_per_phase_a: Some(0.0), // SAFE MODE
            grid_voltage_v: 230.0,         // Standard European voltage (informational)
        }
    }
}

/// Safety constraints (important limits that should be respected)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyConstraints {
    /// Minimum battery SoC (%)
    pub battery_min_soc_percent: f64,

    /// Maximum battery SoC (%)
    pub battery_max_soc_percent: f64,

    /// House load always has priority (cannot be curtailed)
    pub house_priority: bool,

    /// Maximum battery cycles per day (for longevity)
    pub max_battery_cycles_per_day: f64,

    /// Maximum battery temperature (°C)
    pub max_battery_temp_c: f64,
}

impl Default for SafetyConstraints {
    fn default() -> Self {
        Self {
            battery_min_soc_percent: 20.0,
            battery_max_soc_percent: 95.0,
            house_priority: true,
            max_battery_cycles_per_day: 1.5,
            max_battery_temp_c: 45.0,
        }
    }
}

/// Economic objectives (optimization goals)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomicObjectives {
    /// Current grid electricity price (SEK/kWh)
    pub grid_price_sek_kwh: f64,

    /// Grid export price (SEK/kWh, usually lower than import price)
    pub export_price_sek_kwh: f64,

    /// Prefer self-consumption over grid export
    pub prefer_self_consumption: bool,

    /// Price threshold for arbitrage (only discharge battery if price > threshold)
    pub arbitrage_threshold_sek_kwh: f64,

    /// EV departure time (if EV is connected)
    pub ev_departure_time: Option<DateTime<Utc>>,

    /// EV target SoC (%) at departure time
    pub ev_target_soc_percent: Option<f64>,

    /// Low price charge rate (fraction of max charge power, 0.1-1.0)
    /// Used when grid price is cheap but battery isn't full
    /// Default: 0.5 (50% of max charge rate)
    pub low_price_charge_rate: f64,
}

impl Default for EconomicObjectives {
    fn default() -> Self {
        Self {
            grid_price_sek_kwh: 1.5,
            export_price_sek_kwh: 0.8,
            prefer_self_consumption: true,
            arbitrage_threshold_sek_kwh: 2.0,
            ev_departure_time: None,
            ev_target_soc_percent: None,
            low_price_charge_rate: 0.5,
        }
    }
}

/// All constraints bundled together
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllConstraints {
    pub physical: PhysicalConstraints,
    pub safety: SafetyConstraints,
    pub economic: EconomicObjectives,
}

impl Default for AllConstraints {
    fn default() -> Self {
        Self {
            physical: PhysicalConstraints::default(),
            safety: SafetyConstraints::default(),
            economic: EconomicObjectives::default(),
        }
    }
}

impl AllConstraints {
    /// Create a new set of constraints
    pub fn new(
        physical: PhysicalConstraints,
        safety: SafetyConstraints,
        economic: EconomicObjectives,
    ) -> Self {
        Self {
            physical,
            safety,
            economic,
        }
    }

    /// Validate constraints for consistency
    pub fn validate(&self) -> Result<(), String> {
        // Check all constraint values are finite (not NaN or Inf)

        // Physical constraints finite checks
        if !self.physical.max_grid_import_kw.is_finite() {
            return Err(format!("max_grid_import_kw is not finite: {}", self.physical.max_grid_import_kw));
        }
        if !self.physical.max_grid_export_kw.is_finite() {
            return Err(format!("max_grid_export_kw is not finite: {}", self.physical.max_grid_export_kw));
        }
        if !self.physical.max_battery_charge_kw.is_finite() {
            return Err(format!("max_battery_charge_kw is not finite: {}", self.physical.max_battery_charge_kw));
        }
        if !self.physical.max_battery_discharge_kw.is_finite() {
            return Err(format!("max_battery_discharge_kw is not finite: {}", self.physical.max_battery_discharge_kw));
        }
        if !self.physical.evse_min_current_a.is_finite() {
            return Err(format!("evse_min_current_a is not finite: {}", self.physical.evse_min_current_a));
        }
        if !self.physical.evse_max_current_a.is_finite() {
            return Err(format!("evse_max_current_a is not finite: {}", self.physical.evse_max_current_a));
        }
        if !self.physical.grid_voltage_v.is_finite() {
            return Err(format!("grid_voltage_v is not finite: {}", self.physical.grid_voltage_v));
        }

        // Safety constraints finite checks
        if !self.safety.battery_min_soc_percent.is_finite() {
            return Err(format!("battery_min_soc_percent is not finite: {}", self.safety.battery_min_soc_percent));
        }
        if !self.safety.battery_max_soc_percent.is_finite() {
            return Err(format!("battery_max_soc_percent is not finite: {}", self.safety.battery_max_soc_percent));
        }
        if !self.safety.max_battery_cycles_per_day.is_finite() {
            return Err(format!("max_battery_cycles_per_day is not finite: {}", self.safety.max_battery_cycles_per_day));
        }
        if !self.safety.max_battery_temp_c.is_finite() {
            return Err(format!("max_battery_temp_c is not finite: {}", self.safety.max_battery_temp_c));
        }

        // Economic constraints finite checks
        if !self.economic.grid_price_sek_kwh.is_finite() {
            return Err(format!("grid_price_sek_kwh is not finite: {}", self.economic.grid_price_sek_kwh));
        }
        if !self.economic.export_price_sek_kwh.is_finite() {
            return Err(format!("export_price_sek_kwh is not finite: {}", self.economic.export_price_sek_kwh));
        }
        if !self.economic.arbitrage_threshold_sek_kwh.is_finite() {
            return Err(format!("arbitrage_threshold_sek_kwh is not finite: {}", self.economic.arbitrage_threshold_sek_kwh));
        }
        if !self.economic.low_price_charge_rate.is_finite() {
            return Err(format!("low_price_charge_rate is not finite: {}", self.economic.low_price_charge_rate));
        }

        // Now check physical constraints ranges
        // CRITICAL: Enforce explicit configuration (reject default 0.0 values)
        if self.physical.max_grid_import_kw <= 0.0 {
            return Err("max_grid_import_kw must be positive (0.0 = safe mode, explicit config required)".to_string());
        }

        if self.physical.max_battery_charge_kw <= 0.0 {
            return Err("max_battery_charge_kw must be positive (0.0 = safe mode, explicit config required)".to_string());
        }

        if self.physical.max_battery_discharge_kw <= 0.0 {
            return Err("max_battery_discharge_kw must be positive (0.0 = safe mode, explicit config required)".to_string());
        }

        if self.physical.evse_max_current_a <= 0.0 {
            return Err("evse_max_current_a must be positive (0.0 = safe mode, explicit config required)".to_string());
        }

        if self.physical.evse_min_current_a < 6.0 {
            return Err("evse_min_current_a must be at least 6A (IEC 61851)".to_string());
        }

        if self.physical.evse_max_current_a < self.physical.evse_min_current_a {
            return Err("evse_max_current_a must be >= evse_min_current_a".to_string());
        }

        // Check safety constraints ranges
        if self.safety.battery_min_soc_percent < 0.0 || self.safety.battery_min_soc_percent > 100.0 {
            return Err("battery_min_soc_percent must be between 0 and 100".to_string());
        }

        if self.safety.battery_max_soc_percent < self.safety.battery_min_soc_percent {
            return Err("battery_max_soc_percent must be >= battery_min_soc_percent".to_string());
        }

        // Check economic objectives ranges
        if self.economic.grid_price_sek_kwh < 0.0 {
            return Err("grid_price_sek_kwh cannot be negative".to_string());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_constraints_safe_mode() {
        // Default constraints should be in SAFE MODE (all 0.0 power limits)
        // and validation should FAIL to prevent operation without explicit config
        let constraints = AllConstraints::default();
        assert!(constraints.validate().is_err(), "Default constraints should fail validation (safe mode)");

        // Verify all power limits are 0.0 (safe mode)
        assert_eq!(constraints.physical.max_grid_import_kw, 0.0);
        assert_eq!(constraints.physical.max_grid_export_kw, 0.0);
        assert_eq!(constraints.physical.max_battery_charge_kw, 0.0);
        assert_eq!(constraints.physical.max_battery_discharge_kw, 0.0);
        assert_eq!(constraints.physical.evse_max_current_a, 0.0);
    }

    #[test]
    fn test_physical_constraints_default_safe_mode() {
        let physical = PhysicalConstraints::default();
        assert_eq!(physical.phases, 1);
        // CRITICAL: Default should be 0.0 (safe mode), NOT positive
        assert_eq!(physical.max_grid_import_kw, 0.0, "Default must be safe mode (0.0)");
        assert_eq!(physical.max_battery_charge_kw, 0.0, "Default must be safe mode (0.0)");
        assert_eq!(physical.evse_max_current_a, 0.0, "Default must be safe mode (0.0)");
    }

    #[test]
    fn test_safety_constraints_default() {
        let safety = SafetyConstraints::default();
        assert_eq!(safety.battery_min_soc_percent, 20.0);
        assert_eq!(safety.battery_max_soc_percent, 95.0);
        assert!(safety.house_priority);
    }

    #[test]
    fn test_validation_invalid_soc() {
        let mut constraints = AllConstraints::default();
        constraints.safety.battery_min_soc_percent = 110.0; // Invalid
        assert!(constraints.validate().is_err());
    }

    #[test]
    fn test_validation_invalid_current() {
        let mut constraints = AllConstraints::default();
        constraints.physical.evse_min_current_a = 3.0; // Too low
        assert!(constraints.validate().is_err());
    }

    #[test]
    fn test_validation_invalid_price() {
        let mut constraints = AllConstraints::default();
        constraints.economic.grid_price_sek_kwh = -1.0; // Negative
        assert!(constraints.validate().is_err());
    }
}
