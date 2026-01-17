#![allow(dead_code)]
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Input state for power flow computation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerFlowInputs {
    /// Solar PV production (kW)
    pub pv_production_kw: f64,

    /// Household load (kW)
    pub house_load_kw: f64,

    /// Battery state of charge (%)
    pub battery_soc_percent: f64,

    /// Battery temperature (°C)
    pub battery_temp_c: f64,

    /// EV state (if connected)
    pub ev_state: Option<EvState>,

    /// Current grid electricity price (SEK/kWh)
    pub grid_price_sek_kwh: f64,

    /// Target battery power from optimizer schedule (W)
    /// Positive = charge, negative = discharge, None = no schedule
    pub target_power_w: Option<f64>,

    /// AUDIT FIX #6: Target EV discharge power from V2X controller (W)
    /// Separate from battery target to prevent V2X from controlling home battery
    /// Positive = EV charge, negative = EV discharge (V2G/V2H), None = no V2X command
    pub ev_target_power_w: Option<f64>,

    /// Timestamp of this input state
    pub timestamp: DateTime<Utc>,
}

/// EV state information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvState {
    /// Is EV connected to charger
    pub connected: bool,

    /// Current EV battery SoC (%)
    pub soc_percent: f64,

    /// EV battery capacity (kWh)
    pub capacity_kwh: f64,

    /// Maximum charge power (kW)
    pub max_charge_kw: f64,

    /// Maximum discharge power (kW, for V2G)
    pub max_discharge_kw: f64,

    /// Departure time (when user needs the car)
    pub departure_time: Option<DateTime<Utc>>,

    /// Target SoC at departure (%)
    pub target_soc_percent: f64,
}

impl EvState {
    /// Calculate how much energy is needed to reach target SoC
    pub fn energy_needed_kwh(&self) -> f64 {
        let current_kwh = self.capacity_kwh * (self.soc_percent / 100.0);
        let target_kwh = self.capacity_kwh * (self.target_soc_percent / 100.0);
        (target_kwh - current_kwh).max(0.0)
    }

    /// Calculate time until departure (in hours)
    pub fn time_until_departure(&self, now: DateTime<Utc>) -> Option<f64> {
        self.departure_time.map(|departure| {
            let duration = departure.signed_duration_since(now);
            duration.num_seconds() as f64 / 3600.0
        })
    }

    /// Check if EV needs charging (SoC below target and connected)
    pub fn needs_charging(&self) -> bool {
        self.connected && self.soc_percent < self.target_soc_percent
    }

    /// Calculate urgency factor (0-1, higher = more urgent)
    ///
    /// Considers both energy needed and time available
    pub fn urgency_factor(&self, now: DateTime<Utc>) -> f64 {
        if !self.needs_charging() {
            return 0.0;
        }

        let energy_needed = self.energy_needed_kwh();
        let time_until_departure = self.time_until_departure(now).unwrap_or(24.0);

        // Handle edge cases: if departure is imminent (<36 seconds), use maximum urgency
        const MIN_TIME_HOURS: f64 = 0.01;
        let required_rate_kw = if time_until_departure > MIN_TIME_HOURS {
            energy_needed / time_until_departure
        } else {
            // Departure is imminent or past, use maximum urgency
            self.max_charge_kw
        };

        // Calculate urgency = required_rate / max_rate
        if self.max_charge_kw > 0.01 {
            (required_rate_kw / self.max_charge_kw).min(1.0)
        } else {
            // If max charge rate is essentially zero, can't charge anyway
            0.0
        }
    }
}

impl PowerFlowInputs {
    /// Create new power flow inputs with explicit timestamp
    ///
    /// CRITICAL FIX: Timestamp should be captured BEFORE sensor polling begins,
    /// not after. This ensures accurate time-based calculations in PID controllers.
    pub fn new(
        pv_production_kw: f64,
        house_load_kw: f64,
        battery_soc_percent: f64,
        battery_temp_c: f64,
        grid_price_sek_kwh: f64,
        timestamp: DateTime<Utc>,
    ) -> Self {
        Self {
            pv_production_kw,
            house_load_kw,
            battery_soc_percent,
            battery_temp_c,
            ev_state: None,
            grid_price_sek_kwh,
            target_power_w: None,
            ev_target_power_w: None,
            timestamp,
        }
    }

    /// Create new power flow inputs with current timestamp
    ///
    /// WARNING: Only use for testing. In production, capture timestamp before
    /// sensor polling and use new() with explicit timestamp.
    pub fn new_now(
        pv_production_kw: f64,
        house_load_kw: f64,
        battery_soc_percent: f64,
        battery_temp_c: f64,
        grid_price_sek_kwh: f64,
    ) -> Self {
        Self::new(
            pv_production_kw,
            house_load_kw,
            battery_soc_percent,
            battery_temp_c,
            grid_price_sek_kwh,
            Utc::now(),
        )
    }

    /// Set target battery power from optimizer schedule
    pub fn with_target_power_w(mut self, target_power_w: f64) -> Self {
        self.target_power_w = Some(target_power_w);
        self
    }

    /// AUDIT FIX #6: Set target EV power from V2X controller
    pub fn with_ev_target_power_w(mut self, ev_target_power_w: f64) -> Self {
        self.ev_target_power_w = Some(ev_target_power_w);
        self
    }

    /// Set EV state
    pub fn with_ev_state(mut self, ev_state: EvState) -> Self {
        self.ev_state = Some(ev_state);
        self
    }

    /// Validate inputs for sanity
    pub fn validate(&self) -> Result<(), String> {
        // Check all values are finite (not NaN or Inf)
        if !self.pv_production_kw.is_finite() {
            return Err(format!("pv_production_kw is not finite: {}", self.pv_production_kw));
        }

        if !self.house_load_kw.is_finite() {
            return Err(format!("house_load_kw is not finite: {}", self.house_load_kw));
        }

        if !self.battery_soc_percent.is_finite() {
            return Err(format!("battery_soc_percent is not finite: {}", self.battery_soc_percent));
        }

        if !self.battery_temp_c.is_finite() {
            return Err(format!("battery_temp_c is not finite: {}", self.battery_temp_c));
        }

        if !self.grid_price_sek_kwh.is_finite() {
            return Err(format!("grid_price_sek_kwh is not finite: {}", self.grid_price_sek_kwh));
        }

        if let Some(target) = self.target_power_w {
            if !target.is_finite() {
                return Err(format!("target_power_w is not finite: {}", target));
            }
        }

        // Now check ranges
        if self.pv_production_kw < 0.0 {
            return Err("pv_production_kw cannot be negative".to_string());
        }

        if self.house_load_kw < 0.0 {
            return Err("house_load_kw cannot be negative".to_string());
        }

        if self.battery_soc_percent < 0.0 || self.battery_soc_percent > 100.0 {
            return Err("battery_soc_percent must be between 0 and 100".to_string());
        }

        if let Some(ref ev) = self.ev_state {
            // Check EV values are finite
            if !ev.soc_percent.is_finite() {
                return Err(format!("EV soc_percent is not finite: {}", ev.soc_percent));
            }

            if !ev.capacity_kwh.is_finite() {
                return Err(format!("EV capacity_kwh is not finite: {}", ev.capacity_kwh));
            }

            if !ev.max_charge_kw.is_finite() {
                return Err(format!("EV max_charge_kw is not finite: {}", ev.max_charge_kw));
            }

            if !ev.max_discharge_kw.is_finite() {
                return Err(format!("EV max_discharge_kw is not finite: {}", ev.max_discharge_kw));
            }

            // Now check ranges
            if ev.soc_percent < 0.0 || ev.soc_percent > 100.0 {
                return Err("EV soc_percent must be between 0 and 100".to_string());
            }

            if ev.capacity_kwh <= 0.0 {
                return Err("EV capacity_kwh must be positive".to_string());
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_power_flow_inputs_creation() {
        let inputs = PowerFlowInputs::new(5.0, 3.0, 50.0, 25.0, 1.5, chrono::Utc::now());
        assert_eq!(inputs.pv_production_kw, 5.0);
        assert_eq!(inputs.house_load_kw, 3.0);
        assert!(inputs.validate().is_ok());
    }

    #[test]
    fn test_power_flow_inputs_validation() {
        let mut inputs = PowerFlowInputs::new(5.0, 3.0, 50.0, 25.0, 1.5, chrono::Utc::now());
        inputs.battery_soc_percent = 110.0; // Invalid
        assert!(inputs.validate().is_err());
    }

    #[test]
    fn test_ev_state_energy_needed() {
        let ev = EvState {
            connected: true,
            soc_percent: 50.0,
            capacity_kwh: 75.0,
            max_charge_kw: 11.0,
            max_discharge_kw: 0.0,
            departure_time: None,
            target_soc_percent: 80.0,
        };

        // 30% of 75kWh = 22.5kWh needed
        assert!((ev.energy_needed_kwh() - 22.5).abs() < 0.01);
    }

    #[test]
    fn test_ev_state_needs_charging() {
        let ev = EvState {
            connected: true,
            soc_percent: 50.0,
            capacity_kwh: 75.0,
            max_charge_kw: 11.0,
            max_discharge_kw: 0.0,
            departure_time: None,
            target_soc_percent: 80.0,
        };

        assert!(ev.needs_charging());

        let mut ev_full = ev.clone();
        ev_full.soc_percent = 80.0;
        assert!(!ev_full.needs_charging());
    }

    #[test]
    fn test_ev_urgency_factor() {
        let now = Utc::now();
        let departure = now + chrono::Duration::hours(4);

        let ev = EvState {
            connected: true,
            soc_percent: 20.0,
            capacity_kwh: 75.0,
            max_charge_kw: 11.0,
            max_discharge_kw: 0.0,
            departure_time: Some(departure),
            target_soc_percent: 80.0,
        };

        let urgency = ev.urgency_factor(now);
        // Need 45kWh in 4 hours = 11.25kW required, max is 11kW
        // So urgency should be high (> 1.0, but clamped to 1.0)
        assert!(urgency > 0.9);
    }
}
