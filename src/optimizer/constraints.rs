#![allow(dead_code)]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraints {
    pub min_soc_percent: f64,
    pub max_soc_percent: f64,
    pub max_cycles_per_day: f64,
    pub max_power_grid_kw: f64,
    pub v2g_enabled: bool,
    // Battery physical constraints (CRITICAL for DP math)
    pub battery_capacity_kwh: f64,
    pub battery_max_charge_kw: f64,
    pub battery_max_discharge_kw: f64,
    pub battery_efficiency: f64,
}

impl Default for Constraints {
    fn default() -> Self {
        Self {
            min_soc_percent: 20.0,
            max_soc_percent: 90.0,
            max_cycles_per_day: 1.0,
            max_power_grid_kw: 11.0,
            v2g_enabled: false,
            battery_capacity_kwh: 10.0,
            battery_max_charge_kw: 5.0,
            battery_max_discharge_kw: 5.0,
            battery_efficiency: 0.95,
        }
    }
}
