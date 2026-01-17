#![allow(dead_code)]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraints {
    pub min_soc_percent: f64,
    pub max_soc_percent: f64,
    pub max_cycles_per_day: f64,
    pub max_power_grid_kw: f64,

    /// CRITICAL FIX #8: V2G (Vehicle-to-Grid) Enable Flag
    ///
    /// **Default: false** (99% of EV chargers are unidirectional)
    ///
    /// 99% of EV chargers (Wallbox, Zaptec, Easee, etc.) are **UNIDIRECTIONAL**:
    /// - They can ONLY charge (grid → vehicle)
    /// - They CANNOT discharge (vehicle → grid/home)
    /// - Hardware has diode bridges preventing reverse power flow
    ///
    /// Set to `true` ONLY if you have:
    /// - Bidirectional charger hardware (CHAdeMO, CCS bidirectional)
    /// - Grid operator approval for V2G
    /// - Vehicle with V2G capability (Nissan Leaf, Ford F-150 Lightning, etc.)
    ///
    /// If `false`, optimizer MUST NOT allow negative EV charging power.
    /// See ev_driver.rs for more details on V2G vs unidirectional charging.
    pub v2g_enabled: bool,
    // Battery physical constraints (CRITICAL for DP math)
    pub battery_capacity_kwh: f64,
    pub battery_max_charge_kw: f64,
    pub battery_max_discharge_kw: f64,
    pub battery_efficiency: f64,
    pub battery_degradation_per_cycle: f64,
    pub battery_replacement_cost_sek: f64,

    /// CRITICAL: Swedish "Effekttariff" (Peak Power Tariff)
    /// Grid operators (Ellevio, Vattenfall, E.ON) charge based on monthly peak hourly average power
    /// Typical: 50-120 SEK/kW/month
    /// This is SEPARATE from energy cost (kWh) and can dominate the bill!
    ///
    /// If disabled (0.0), optimizer ignores peak power and may create expensive spikes.
    /// If enabled (typical 100 SEK/kW), optimizer will flatten load profile to avoid peaks.
    pub peak_power_tariff_sek_per_kw: f64,
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
            battery_degradation_per_cycle: 0.0001,
            battery_replacement_cost_sek: 50000.0,
            // Enable peak power tariff by default (100 SEK/kW is typical for Swedish grid)
            // Set to 0.0 to disable if not applicable
            peak_power_tariff_sek_per_kw: 100.0,
        }
    }
}
