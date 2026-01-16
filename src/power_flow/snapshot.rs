#![allow(dead_code)]
use serde::{Deserialize, Serialize};
use std::fmt;


/// Power snapshot representing all energy flows at a single point in time
///
/// Power balance equation: PV + Battery + Grid = House + EV
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerSnapshot {
    /// Solar PV production (always positive)
    pub pv_kw: f64,

    /// Household consumption (always positive)
    pub house_kw: f64,

    /// Battery power (positive = charging, negative = discharging)
    pub battery_kw: f64,

    /// EV charger power (always positive when charging)
    pub ev_kw: f64,

    /// Grid power (positive = import, negative = export)
    pub grid_kw: f64,

    /// Timestamp of this snapshot
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl PowerSnapshot {
    /// Create a new power snapshot with explicit timestamp
    ///
    /// CRITICAL FIX: Timestamp must be captured BEFORE data collection begins,
    /// not after. Modbus polling over 9600 baud serial can take 2-3 seconds.
    /// If timestamp is captured after polling, the PID controller's derivative
    /// term will calculate dt incorrectly, leading to control instability.
    pub fn new(
        pv_kw: f64,
        house_kw: f64,
        battery_kw: f64,
        ev_kw: f64,
        grid_kw: f64,
        timestamp: chrono::DateTime<chrono::Utc>,
    ) -> Self {
        Self {
            pv_kw,
            house_kw,
            battery_kw,
            ev_kw,
            grid_kw,
            timestamp,
        }
    }

    /// Create a new power snapshot with current timestamp
    ///
    /// WARNING: Only use this for testing or when data collection is instantaneous.
    /// For production code with real sensors, capture timestamp before polling
    /// and use new() with explicit timestamp.
    pub fn new_now(
        pv_kw: f64,
        house_kw: f64,
        battery_kw: f64,
        ev_kw: f64,
        grid_kw: f64,
    ) -> Self {
        Self::new(pv_kw, house_kw, battery_kw, ev_kw, grid_kw, chrono::Utc::now())
    }

    /// Verify power balance holds (sources = sinks)
    ///
    /// Sources: PV, Battery (if discharging), Grid (if importing)
    /// Sinks: House, EV, Battery (if charging), Grid (if exporting)
    pub fn verify_power_balance(&self) -> bool {
        let sources = self.pv_kw + self.grid_kw.max(0.0) - self.battery_kw.min(0.0);
        let sinks = self.house_kw + self.ev_kw + self.battery_kw.max(0.0) - self.grid_kw.min(0.0);

        // Allow small floating point error (100W = 0.1kW)
        (sources - sinks).abs() < 0.1
    }

    /// Calculate net grid flow (import is positive, export is negative)
    pub fn net_grid_kw(&self) -> f64 {
        self.grid_kw
    }

    /// Calculate net grid import (0 if exporting)
    pub fn grid_import_kw(&self) -> f64 {
        self.grid_kw.max(0.0)
    }

    /// Calculate net grid export (0 if importing)
    pub fn grid_export_kw(&self) -> f64 {
        (-self.grid_kw).max(0.0)
    }

    /// Check if grid import exceeds fuse limit
    pub fn exceeds_fuse_limit(&self, fuse_limit_kw: f64) -> bool {
        self.grid_import_kw() > fuse_limit_kw
    }

    /// Check if grid export exceeds limit
    pub fn exceeds_export_limit(&self, export_limit_kw: f64) -> bool {
        self.grid_export_kw() > export_limit_kw
    }

    /// Calculate self-consumption (PV used locally vs exported)
    pub fn self_consumption_kw(&self) -> f64 {
        let pv_consumed = self.pv_kw - self.grid_export_kw();
        pv_consumed.max(0.0).min(self.pv_kw)
    }

    /// Calculate self-sufficiency ratio (0-1)
    pub fn self_sufficiency_ratio(&self) -> f64 {
        let total_load = self.house_kw + self.ev_kw + self.battery_kw.max(0.0);
        // CRITICAL SAFETY FIX D10: Use a reasonable threshold to avoid division edge cases
        // Very small loads (< 10W = 0.01kW) should be treated as zero load
        const MIN_LOAD_THRESHOLD_KW: f64 = 0.01;
        if total_load < MIN_LOAD_THRESHOLD_KW {
            // No meaningful load, consider 0% self-sufficient (can't be self-sufficient with no load)
            return 0.0;
        }
        // Calculate ratio and clamp to [0, 1]
        (self.pv_kw / total_load).clamp(0.0, 1.0)
    }
}

impl fmt::Display for PowerSnapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "PowerSnapshot {{ PV: {:.2}kW, House: {:.2}kW, Battery: {:.2}kW, EV: {:.2}kW, Grid: {:.2}kW, Balanced: {} }}",
            self.pv_kw,
            self.house_kw,
            self.battery_kw,
            self.ev_kw,
            self.grid_kw,
            if self.verify_power_balance() { "✓" } else { "✗" }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_power_balance_simple() {
        // PV = House (perfect balance, no battery, no EV, no grid)
        let snapshot = PowerSnapshot::new_now(5.0, 5.0, 0.0, 0.0, 0.0);
        assert!(snapshot.verify_power_balance());
    }

    #[test]
    fn test_power_balance_with_battery_charging() {
        // PV 5kW = House 3kW + Battery 2kW (charging)
        let snapshot = PowerSnapshot::new_now(5.0, 3.0, 2.0, 0.0, 0.0);
        assert!(snapshot.verify_power_balance());
    }

    #[test]
    fn test_power_balance_with_battery_discharging() {
        // PV 2kW + Battery 3kW (discharging) = House 5kW
        let snapshot = PowerSnapshot::new_now(2.0, 5.0, -3.0, 0.0, 0.0);
        assert!(snapshot.verify_power_balance());
    }

    #[test]
    fn test_power_balance_with_grid_import() {
        // PV 2kW + Grid 3kW = House 5kW
        let snapshot = PowerSnapshot::new_now(2.0, 5.0, 0.0, 0.0, 3.0);
        assert!(snapshot.verify_power_balance());
    }

    #[test]
    fn test_power_balance_with_grid_export() {
        // PV 10kW = House 3kW + Grid -7kW (export)
        let snapshot = PowerSnapshot::new_now(10.0, 3.0, 0.0, 0.0, -7.0);
        assert!(snapshot.verify_power_balance());
    }

    #[test]
    fn test_power_balance_complex() {
        // PV 8kW + Battery -2kW (discharge) = House 4kW + EV 3kW + Grid -3kW (export)
        let snapshot = PowerSnapshot::new_now(8.0, 4.0, -2.0, 3.0, -3.0);
        assert!(snapshot.verify_power_balance());
    }

    #[test]
    fn test_exceeds_fuse_limit() {
        let snapshot = PowerSnapshot::new_now(0.0, 15.0, 0.0, 0.0, 15.0);
        assert!(snapshot.exceeds_fuse_limit(10.0));
        assert!(!snapshot.exceeds_fuse_limit(20.0));
    }

    #[test]
    fn test_self_consumption() {
        // PV 10kW, House 3kW, export 7kW -> self-consumption = 3kW
        let snapshot = PowerSnapshot::new_now(10.0, 3.0, 0.0, 0.0, -7.0);
        assert_eq!(snapshot.self_consumption_kw(), 3.0);
    }

    #[test]
    fn test_self_sufficiency_ratio() {
        // PV 5kW, House 10kW, Grid 5kW -> 50% self-sufficient
        let snapshot = PowerSnapshot::new_now(5.0, 10.0, 0.0, 0.0, 5.0);
        assert!((snapshot.self_sufficiency_ratio() - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_display() {
        let snapshot = PowerSnapshot::new_now(5.0, 3.0, 1.0, 1.0, 0.0);
        let display = format!("{}", snapshot);
        assert!(display.contains("PV: 5.00kW"));
        assert!(display.contains("House: 3.00kW"));
    }
}
