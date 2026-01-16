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

        // Calculate minimum required charge rate
        let required_rate_kw = if time_until_departure > 0.0 {
            energy_needed / time_until_departure
        } else {
            self.max_charge_kw // Urgent if departure is now/past
        };

        // Urgency = required_rate / max_rate
        (required_rate_kw / self.max_charge_kw).min(1.0)
    }
}

impl PowerFlowInputs {
    /// Create new power flow inputs
    pub fn new(
        pv_production_kw: f64,
        house_load_kw: f64,
        battery_soc_percent: f64,
        battery_temp_c: f64,
        grid_price_sek_kwh: f64,
    ) -> Self {
        Self {
            pv_production_kw,
            house_load_kw,
            battery_soc_percent,
            battery_temp_c,
            ev_state: None,
            grid_price_sek_kwh,
            timestamp: Utc::now(),
        }
    }

    /// Set EV state
    pub fn with_ev_state(mut self, ev_state: EvState) -> Self {
        self.ev_state = Some(ev_state);
        self
    }

    /// Validate inputs for sanity
    pub fn validate(&self) -> Result<(), String> {
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
        let inputs = PowerFlowInputs::new(5.0, 3.0, 50.0, 25.0, 1.5);
        assert_eq!(inputs.pv_production_kw, 5.0);
        assert_eq!(inputs.house_load_kw, 3.0);
        assert!(inputs.validate().is_ok());
    }

    #[test]
    fn test_power_flow_inputs_validation() {
        let mut inputs = PowerFlowInputs::new(5.0, 3.0, 50.0, 25.0, 1.5);
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
