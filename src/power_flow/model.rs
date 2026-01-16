#![allow(dead_code)]

use super::{AllConstraints, PowerFlowInputs, PowerSnapshot};

// EV Charging Urgency Thresholds
const EV_HIGH_URGENCY_THRESHOLD: f64 = 0.8;   // Above this: use max power even if importing
const EV_MEDIUM_URGENCY_THRESHOLD: f64 = 0.3; // Above this: use PV + some grid
// Below MEDIUM_URGENCY_THRESHOLD: only use excess PV

// Battery SoC Management Thresholds
const BATTERY_LOW_SOC_MULTIPLIER: f64 = 0.7;  // When to charge from cheap grid (70% of max_soc)
const BATTERY_MIN_POWER_THRESHOLD_KW: f64 = 0.1; // Minimum power difference to act on

// Grid Price Arbitrage Thresholds
const CHEAP_GRID_PRICE_MULTIPLIER: f64 = 0.5; // Charge when price < threshold * 0.5

/// Power Flow Model - THE CORE ALGORITHM
///
/// This model computes optimal power flows while respecting the constraint hierarchy:
/// 1. Physical constraints (MUST NEVER violate)
/// 2. Safety constraints (SHOULD respect)
/// 3. Economic objectives (OPTIMIZE for)
pub struct PowerFlowModel {
    constraints: AllConstraints,
}

impl PowerFlowModel {
    /// Create a new power flow model with given constraints
    pub fn new(constraints: AllConstraints) -> Self {
        Self { constraints }
    }

    /// Compute optimal power flows for given inputs
    ///
    /// This is the CORE algorithm that orchestrates all energy flows.
    ///
    /// Algorithm steps:
    /// 1. House load gets priority (always satisfied)
    /// 2. Allocate PV to house first
    /// 3. Calculate EV charging urgency
    /// 4. Allocate power to EV with fuse protection
    /// 5. Charge battery from excess PV
    /// 6. Apply arbitrage logic (charge when cheap, discharge when expensive)
    /// 7. Export excess PV to grid (if beneficial)
    /// 8. Verify power balance and fuse limits
    pub fn compute_flows(&self, inputs: &PowerFlowInputs) -> Result<PowerSnapshot, String> {
        // Step 1: House load always has priority
        let house_kw = inputs.house_load_kw;

        // Step 2: Allocate PV to house first
        let pv_to_house = inputs.pv_production_kw.min(house_kw);
        let remaining_pv = inputs.pv_production_kw - pv_to_house;
        let house_deficit = house_kw - pv_to_house;

        // Step 3: Calculate EV charging urgency and allocate power
        let (ev_kw, remaining_pv_after_ev) = self.allocate_ev_power(
            &inputs,
            remaining_pv,
        )?;

        // Step 4: Battery power decision (charge, discharge, or idle)
        let (battery_kw, _remaining_pv_after_battery) = self.decide_battery_power(
            &inputs,
            remaining_pv_after_ev,
            house_deficit,
        );

        // Step 5: Grid power (import/export)
        let grid_kw = self.calculate_grid_power(
            house_kw,
            inputs.pv_production_kw,
            battery_kw,
            ev_kw,
        );

        // Create power snapshot
        let snapshot = PowerSnapshot {
            pv_kw: inputs.pv_production_kw,
            house_kw,
            battery_kw,
            ev_kw,
            grid_kw,
            timestamp: inputs.timestamp,
        };

        // Step 6: Verify constraints
        self.verify_snapshot(&snapshot)?;

        Ok(snapshot)
    }

    /// Allocate power to EV charging with urgency consideration
    fn allocate_ev_power(
        &self,
        inputs: &PowerFlowInputs,
        available_pv_kw: f64,
    ) -> Result<(f64, f64), String> {
        let ev_state = match &inputs.ev_state {
            Some(ev) if ev.connected && ev.needs_charging() => ev,
            _ => return Ok((0.0, available_pv_kw)), // No EV or doesn't need charging
        };

        // Calculate urgency (0-1)
        let urgency = ev_state.urgency_factor(inputs.timestamp);

        // Determine EV charge power based on urgency and available power
        let desired_ev_kw = if urgency > EV_HIGH_URGENCY_THRESHOLD {
            // High urgency: use max power even if it means importing from grid
            ev_state.max_charge_kw.min(self.constraints.physical.max_grid_import_kw)
        } else if urgency > EV_MEDIUM_URGENCY_THRESHOLD {
            // Medium urgency: use available PV + some grid if needed
            let min_kw = available_pv_kw;
            let max_kw = ev_state.max_charge_kw;
            // Interpolate based on urgency
            let urgency_range = urgency - EV_MEDIUM_URGENCY_THRESHOLD;
            let urgency_span = EV_HIGH_URGENCY_THRESHOLD - EV_MEDIUM_URGENCY_THRESHOLD;
            min_kw + (max_kw - min_kw) * (urgency_range / urgency_span)
        } else {
            // Low urgency: only use excess PV (solar priority)
            available_pv_kw
        };

        // Apply EV charger min/max current limits
        let evse_min_kw = self.evse_min_power_kw();
        let evse_max_kw = self.evse_max_power_kw();

        let ev_kw = if desired_ev_kw >= evse_min_kw {
            desired_ev_kw.min(evse_max_kw).min(ev_state.max_charge_kw)
        } else {
            0.0 // Below minimum, don't charge at all
        };

        // Calculate remaining PV after EV
        let pv_used_by_ev = ev_kw.min(available_pv_kw);
        let remaining_pv = available_pv_kw - pv_used_by_ev;

        Ok((ev_kw, remaining_pv))
    }

    /// Decide battery power (charge/discharge/idle)
    fn decide_battery_power(
        &self,
        inputs: &PowerFlowInputs,
        available_pv_kw: f64,
        house_deficit_kw: f64,
    ) -> (f64, f64) {
        // Check battery SoC constraints
        let soc = inputs.battery_soc_percent;
        let min_soc = self.constraints.safety.battery_min_soc_percent;
        let max_soc = self.constraints.safety.battery_max_soc_percent;

        // Case 1: Excess PV available - charge battery
        if available_pv_kw > BATTERY_MIN_POWER_THRESHOLD_KW && soc < max_soc {
            let charge_kw = available_pv_kw
                .min(self.constraints.physical.max_battery_charge_kw);
            return (charge_kw, available_pv_kw - charge_kw);
        }

        // Case 2: House needs power and price is high - discharge battery
        if house_deficit_kw > 0.0 && soc > min_soc {
            let price = inputs.grid_price_sek_kwh;
            let threshold = self.constraints.economic.arbitrage_threshold_sek_kwh;

            if price > threshold || self.constraints.economic.prefer_self_consumption {
                let discharge_kw = house_deficit_kw
                    .min(self.constraints.physical.max_battery_discharge_kw)
                    .min((soc - min_soc) / 100.0 * 50.0); // Simplified SoC check
                return (-discharge_kw, available_pv_kw);
            }
        }

        // Case 3: Cheap grid price and low SoC - charge from grid
        let cheap_grid_threshold = self.constraints.economic.arbitrage_threshold_sek_kwh * CHEAP_GRID_PRICE_MULTIPLIER;
        let low_soc_threshold = max_soc * BATTERY_LOW_SOC_MULTIPLIER;

        if inputs.grid_price_sek_kwh < cheap_grid_threshold && soc < low_soc_threshold {
            let charge_kw = self.constraints.physical.max_battery_charge_kw * 0.5;
            return (charge_kw, available_pv_kw);
        }

        // Default: idle
        (0.0, available_pv_kw)
    }

    /// Calculate grid power to balance the system
    fn calculate_grid_power(
        &self,
        house_kw: f64,
        pv_kw: f64,
        battery_kw: f64,
        ev_kw: f64,
    ) -> f64 {
        // Grid = House + EV + Battery_charge - PV - Battery_discharge
        // Positive = import, negative = export
        let total_load = house_kw + ev_kw + battery_kw.max(0.0);
        let total_generation = pv_kw - battery_kw.min(0.0);

        total_load - total_generation
    }

    /// Verify power snapshot respects all constraints
    fn verify_snapshot(&self, snapshot: &PowerSnapshot) -> Result<(), String> {
        // Verify power balance
        if !snapshot.verify_power_balance() {
            return Err("Power balance violation".to_string());
        }

        // Verify fuse limit
        if snapshot.exceeds_fuse_limit(self.constraints.physical.max_grid_import_kw) {
            return Err(format!(
                "Fuse limit exceeded: {:.2}kW > {:.2}kW",
                snapshot.grid_import_kw(),
                self.constraints.physical.max_grid_import_kw
            ));
        }

        // Verify export limit
        if snapshot.exceeds_export_limit(self.constraints.physical.max_grid_export_kw) {
            return Err(format!(
                "Export limit exceeded: {:.2}kW > {:.2}kW",
                snapshot.grid_export_kw(),
                self.constraints.physical.max_grid_export_kw
            ));
        }

        Ok(())
    }

    /// Calculate minimum EV charger power (kW)
    fn evse_min_power_kw(&self) -> f64 {
        let phases = self.constraints.physical.phases as f64;
        let voltage = self.constraints.physical.grid_voltage_v;
        let current = self.constraints.physical.evse_min_current_a;

        (phases * voltage * current) / 1000.0
    }

    /// Calculate maximum EV charger power (kW)
    fn evse_max_power_kw(&self) -> f64 {
        let phases = self.constraints.physical.phases as f64;
        let voltage = self.constraints.physical.grid_voltage_v;
        let current = self.constraints.physical.evse_max_current_a;

        (phases * voltage * current) / 1000.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::power_flow::inputs::EvState;

    #[test]
    fn test_simple_pv_to_house() {
        let model = PowerFlowModel::new(AllConstraints::default());
        let inputs = PowerFlowInputs::new(5.0, 3.0, 50.0, 25.0, 1.5);

        let snapshot = model.compute_flows(&inputs).unwrap();

        // PV 5kW, House 3kW -> expect 2kW to battery or export
        assert_eq!(snapshot.pv_kw, 5.0);
        assert_eq!(snapshot.house_kw, 3.0);
        assert!(snapshot.verify_power_balance());
    }

    #[test]
    fn test_excess_pv_charges_battery() {
        let model = PowerFlowModel::new(AllConstraints::default());
        let inputs = PowerFlowInputs::new(10.0, 3.0, 50.0, 25.0, 1.5);

        let snapshot = model.compute_flows(&inputs).unwrap();

        // Should charge battery with excess PV
        assert!(snapshot.battery_kw > 0.0, "Battery should be charging");
        assert!(snapshot.verify_power_balance());
    }

    #[test]
    fn test_high_price_discharges_battery() {
        let model = PowerFlowModel::new(AllConstraints::default());
        let inputs = PowerFlowInputs::new(0.0, 5.0, 80.0, 25.0, 3.0); // High price

        let snapshot = model.compute_flows(&inputs).unwrap();

        // Should discharge battery due to high price
        assert!(snapshot.battery_kw <= 0.0, "Battery should be discharging or idle");
        assert!(snapshot.verify_power_balance());
    }

    #[test]
    fn test_ev_charging_urgent() {
        let model = PowerFlowModel::new(AllConstraints::default());

        let departure = Utc::now() + chrono::Duration::hours(2);
        let ev_state = EvState {
            connected: true,
            soc_percent: 20.0,
            capacity_kwh: 75.0,
            max_charge_kw: 11.0,
            max_discharge_kw: 0.0,
            departure_time: Some(departure),
            target_soc_percent: 80.0,
        };

        let inputs = PowerFlowInputs::new(5.0, 3.0, 50.0, 25.0, 1.5)
            .with_ev_state(ev_state);

        let snapshot = model.compute_flows(&inputs).unwrap();

        // Should charge EV aggressively due to urgency
        assert!(snapshot.ev_kw > 0.0, "EV should be charging");
        assert!(snapshot.verify_power_balance());
    }

    #[test]
    fn test_fuse_limit_protection() {
        let mut constraints = AllConstraints::default();
        constraints.physical.max_grid_import_kw = 10.0;

        let model = PowerFlowModel::new(constraints);
        let inputs = PowerFlowInputs::new(0.0, 15.0, 20.0, 25.0, 1.5);

        let snapshot = model.compute_flows(&inputs).unwrap();

        // Should not exceed fuse limit
        assert!(!snapshot.exceeds_fuse_limit(10.0));
    }
}
