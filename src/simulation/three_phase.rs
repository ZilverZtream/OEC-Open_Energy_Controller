//! # 3-Phase AC Power Simulation
//!
//! Models 3-phase electrical systems common in European households.
//! Handles per-phase power flow, phase unbalance, and realistic grid interactions.
//!
//! ## 3-Phase Systems
//!
//! In Europe, many households have 3-phase connections (L1, L2, L3 + N):
//! - Nominal voltage: 230V phase-to-neutral, 400V phase-to-phase
//! - Power is distributed across 3 phases
//! - **Critical**: Phase unbalance can cause issues even when net power is balanced
//!
//! ## Example Scenario
//!
//! **Balanced Net, Unbalanced Phases:**
//! - House load: 6 kW on L1, 0 kW on L2/L3
//! - Inverter output: 6 kW (2 kW per phase)
//! - **Net result**: 0 kW (looks balanced)
//! - **Reality**: Import 4 kW on L1, export 2 kW on L2, export 2 kW on L3
//!
//! The controller must handle this phase-level complexity.

use serde::{Deserialize, Serialize};

/// 3-phase power measurement
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct ThreePhasePower {
    /// Phase L1 power (W, positive = import/consumption)
    pub l1_w: f64,
    /// Phase L2 power (W, positive = import/consumption)
    pub l2_w: f64,
    /// Phase L3 power (W, positive = import/consumption)
    pub l3_w: f64,
}

impl ThreePhasePower {
    /// Create new 3-phase power measurement
    pub fn new(l1_w: f64, l2_w: f64, l3_w: f64) -> Self {
        Self { l1_w, l2_w, l3_w }
    }

    /// Create from single-phase power (distribute equally)
    pub fn from_single_phase(total_w: f64) -> Self {
        let per_phase = total_w / 3.0;
        Self {
            l1_w: per_phase,
            l2_w: per_phase,
            l3_w: per_phase,
        }
    }

    /// Total power across all phases
    pub fn total(&self) -> f64 {
        self.l1_w + self.l2_w + self.l3_w
    }

    /// Maximum single-phase power
    pub fn max_phase(&self) -> f64 {
        self.l1_w.max(self.l2_w).max(self.l3_w)
    }

    /// Minimum single-phase power
    pub fn min_phase(&self) -> f64 {
        self.l1_w.min(self.l2_w).min(self.l3_w)
    }

    /// Phase unbalance factor (0.0 = perfectly balanced, 1.0 = completely unbalanced)
    ///
    /// Calculated as: (max - min) / (max + min + 1e-9)
    pub fn unbalance_factor(&self) -> f64 {
        let max = self.max_phase().abs();
        let min = self.min_phase().abs();
        let sum = max + min + 1e-9; // Avoid division by zero
        (max - min) / sum
    }

    /// Calculate current per phase (A)
    pub fn currents(&self, voltage_v: f64) -> ThreePhaseCurrent {
        ThreePhaseCurrent {
            l1_a: self.l1_w / voltage_v,
            l2_a: self.l2_w / voltage_v,
            l3_a: self.l3_w / voltage_v,
        }
    }

    /// Add another 3-phase power
    pub fn add(&self, other: &ThreePhasePower) -> Self {
        Self {
            l1_w: self.l1_w + other.l1_w,
            l2_w: self.l2_w + other.l2_w,
            l3_w: self.l3_w + other.l3_w,
        }
    }

    /// Subtract another 3-phase power
    pub fn sub(&self, other: &ThreePhasePower) -> Self {
        Self {
            l1_w: self.l1_w - other.l1_w,
            l2_w: self.l2_w - other.l2_w,
            l3_w: self.l3_w - other.l3_w,
        }
    }
}

/// 3-phase current measurement
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ThreePhaseCurrent {
    /// Phase L1 current (A)
    pub l1_a: f64,
    /// Phase L2 current (A)
    pub l2_a: f64,
    /// Phase L3 current (A)
    pub l3_a: f64,
}

impl ThreePhaseCurrent {
    /// Maximum single-phase current
    pub fn max_phase(&self) -> f64 {
        self.l1_a.abs().max(self.l2_a.abs()).max(self.l3_a.abs())
    }

    /// Check if any phase exceeds limit
    pub fn exceeds_limit(&self, limit_a: f64) -> bool {
        self.max_phase() > limit_a
    }

    /// Which phase exceeds the limit
    pub fn overloaded_phase(&self, limit_a: f64) -> Option<usize> {
        if self.l1_a.abs() > limit_a {
            Some(1)
        } else if self.l2_a.abs() > limit_a {
            Some(2)
        } else if self.l3_a.abs() > limit_a {
            Some(3)
        } else {
            None
        }
    }
}

/// 3-phase voltage measurement
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ThreePhaseVoltage {
    /// Phase L1-N voltage (V)
    pub l1_v: f64,
    /// Phase L2-N voltage (V)
    pub l2_v: f64,
    /// Phase L3-N voltage (V)
    pub l3_v: f64,
}

impl ThreePhaseVoltage {
    /// Create balanced voltages
    pub fn balanced(voltage_v: f64) -> Self {
        Self {
            l1_v: voltage_v,
            l2_v: voltage_v,
            l3_v: voltage_v,
        }
    }

    /// Calculate voltage unbalance
    pub fn unbalance_percent(&self) -> f64 {
        let avg = (self.l1_v + self.l2_v + self.l3_v) / 3.0;
        let max_deviation = (self.l1_v - avg)
            .abs()
            .max((self.l2_v - avg).abs())
            .max((self.l3_v - avg).abs());
        (max_deviation / avg) * 100.0
    }
}

/// House load distribution pattern
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoadDistribution {
    /// All load on single phase (worst case)
    SinglePhase(usize), // 1, 2, or 3
    /// Load split 80/10/10 across phases (unbalanced)
    Unbalanced,
    /// Load split 33/33/34 across phases (balanced)
    Balanced,
    /// Custom distribution percentages (must sum to 100)
    Custom(u8, u8, u8),
}

impl LoadDistribution {
    /// Distribute total power according to pattern
    pub fn distribute(&self, total_w: f64) -> ThreePhasePower {
        match self {
            LoadDistribution::SinglePhase(phase) => match phase {
                1 => ThreePhasePower::new(total_w, 0.0, 0.0),
                2 => ThreePhasePower::new(0.0, total_w, 0.0),
                3 => ThreePhasePower::new(0.0, 0.0, total_w),
                _ => ThreePhasePower::from_single_phase(total_w), // Fallback to balanced
            },
            LoadDistribution::Unbalanced => ThreePhasePower::new(
                total_w * 0.8,
                total_w * 0.1,
                total_w * 0.1,
            ),
            LoadDistribution::Balanced => ThreePhasePower::from_single_phase(total_w),
            LoadDistribution::Custom(l1_pct, l2_pct, l3_pct) => {
                let sum = (*l1_pct as f64 + *l2_pct as f64 + *l3_pct as f64).max(1.0);
                ThreePhasePower::new(
                    total_w * (*l1_pct as f64 / sum),
                    total_w * (*l2_pct as f64 / sum),
                    total_w * (*l3_pct as f64 / sum),
                )
            }
        }
    }
}

/// 3-phase grid state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreePhaseGridState {
    /// Per-phase voltages
    pub voltage: ThreePhaseVoltage,

    /// House load per phase
    pub house_load: ThreePhasePower,

    /// Solar/inverter output per phase
    pub solar_output: ThreePhasePower,

    /// Battery charge/discharge per phase
    pub battery_power: ThreePhasePower,

    /// Net grid import/export per phase
    pub grid_power: ThreePhasePower,

    /// Phase currents
    pub grid_current: ThreePhaseCurrent,

    /// Load distribution pattern
    pub load_distribution: LoadDistribution,
}

impl ThreePhaseGridState {
    /// Calculate net grid power from house, solar, and battery
    pub fn calculate_grid_power(&mut self, nominal_voltage_v: f64) {
        // Grid power = House load - Solar - Battery
        // Positive = import from grid
        // Negative = export to grid
        self.grid_power = self
            .house_load
            .sub(&self.solar_output)
            .sub(&self.battery_power);

        // Calculate currents
        self.grid_current = self.grid_power.currents(nominal_voltage_v);
    }

    /// Check if any phase is overloaded
    pub fn is_phase_overloaded(&self, phase_limit_a: f64) -> bool {
        self.grid_current.exceeds_limit(phase_limit_a)
    }

    /// Get overloaded phase number (1, 2, or 3)
    pub fn overloaded_phase(&self, phase_limit_a: f64) -> Option<usize> {
        self.grid_current.overloaded_phase(phase_limit_a)
    }

    /// Calculate total unbalance severity (0.0-1.0)
    pub fn unbalance_severity(&self) -> f64 {
        self.grid_power.unbalance_factor()
    }
}

/// 3-phase power simulator
pub struct ThreePhaseSimulator {
    nominal_voltage_v: f64,
    phase_fuse_rating_a: f64,
    load_distribution: LoadDistribution,
}

impl ThreePhaseSimulator {
    /// Create new 3-phase simulator
    pub fn new(nominal_voltage_v: f64, phase_fuse_rating_a: f64) -> Self {
        Self {
            nominal_voltage_v,
            phase_fuse_rating_a,
            load_distribution: LoadDistribution::Balanced,
        }
    }

    /// Set load distribution pattern
    pub fn set_load_distribution(&mut self, distribution: LoadDistribution) {
        self.load_distribution = distribution;
    }

    /// Create grid state from single-phase measurements
    pub fn create_state(
        &self,
        house_load_w: f64,
        solar_output_w: f64,
        battery_power_w: f64,
    ) -> ThreePhaseGridState {
        let house_load = self.load_distribution.distribute(house_load_w);
        let solar_output = LoadDistribution::Balanced.distribute(solar_output_w);
        let battery_power = LoadDistribution::Balanced.distribute(battery_power_w);

        let mut state = ThreePhaseGridState {
            voltage: ThreePhaseVoltage::balanced(self.nominal_voltage_v),
            house_load,
            solar_output,
            battery_power,
            grid_power: ThreePhasePower::default(),
            grid_current: ThreePhaseCurrent {
                l1_a: 0.0,
                l2_a: 0.0,
                l3_a: 0.0,
            },
            load_distribution: self.load_distribution,
        };

        state.calculate_grid_power(self.nominal_voltage_v);
        state
    }

    /// Check for phase overload and calculate curtailment needed
    ///
    /// Returns (is_overloaded, curtailment_needed_w)
    pub fn check_phase_overload(&self, state: &ThreePhaseGridState) -> (bool, f64) {
        if let Some(phase) = state.overloaded_phase(self.phase_fuse_rating_a) {
            // Calculate how much we need to reduce to stay within limit
            let phase_current = match phase {
                1 => state.grid_current.l1_a,
                2 => state.grid_current.l2_a,
                3 => state.grid_current.l3_a,
                _ => 0.0,
            };

            let excess_current = phase_current.abs() - self.phase_fuse_rating_a;

            // CRITICAL PHYSICS FIX: Account for balanced 3-phase inverter output
            // Most residential inverters output balanced power across all 3 phases.
            // To reduce current on L1 by 10A, the inverter must reduce TOTAL power by 3×(10A×230V).
            // Example: To fix a 10A overload on L1:
            //   - Single-phase calculation: 10A × 230V = 2.3kW (WRONG)
            //   - Balanced 3-phase:        3 × (10A × 230V) = 6.9kW (CORRECT)
            // Without this fix, the controller under-curtails and the fuse still blows.
            let curtailment_per_phase = excess_current * self.nominal_voltage_v;
            let curtailment_w = curtailment_per_phase * 3.0;

            (true, curtailment_w)
        } else {
            (false, 0.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_three_phase_power_total() {
        let power = ThreePhasePower::new(1000.0, 2000.0, 3000.0);
        assert!((power.total() - 6000.0).abs() < 0.1);
    }

    #[test]
    fn test_balanced_distribution() {
        let power = ThreePhasePower::from_single_phase(6000.0);
        assert!((power.l1_w - 2000.0).abs() < 0.1);
        assert!((power.l2_w - 2000.0).abs() < 0.1);
        assert!((power.l3_w - 2000.0).abs() < 0.1);
        assert!(power.unbalance_factor() < 0.01);
    }

    #[test]
    fn test_single_phase_distribution() {
        let dist = LoadDistribution::SinglePhase(1);
        let power = dist.distribute(6000.0);
        assert!((power.l1_w - 6000.0).abs() < 0.1);
        assert!((power.l2_w).abs() < 0.1);
        assert!((power.l3_w).abs() < 0.1);
    }

    #[test]
    fn test_unbalance_factor() {
        // Perfectly balanced
        let balanced = ThreePhasePower::new(2000.0, 2000.0, 2000.0);
        assert!(balanced.unbalance_factor() < 0.01);

        // Completely unbalanced
        let unbalanced = ThreePhasePower::new(6000.0, 0.0, 0.0);
        assert!(unbalanced.unbalance_factor() > 0.9);
    }

    #[test]
    fn test_phase_current_calculation() {
        let power = ThreePhasePower::new(2300.0, 2300.0, 2300.0);
        let current = power.currents(230.0);

        assert!((current.l1_a - 10.0).abs() < 0.1);
        assert!((current.l2_a - 10.0).abs() < 0.1);
        assert!((current.l3_a - 10.0).abs() < 0.1);
    }

    #[test]
    fn test_phase_overload_detection() {
        let current = ThreePhaseCurrent {
            l1_a: 30.0, // Overloaded
            l2_a: 10.0,
            l3_a: 10.0,
        };

        assert!(current.exceeds_limit(25.0));
        assert_eq!(current.overloaded_phase(25.0), Some(1));
    }

    #[test]
    fn test_grid_state_calculation() {
        let sim = ThreePhaseSimulator::new(230.0, 25.0);

        // Scenario: 6kW house load on L1, 6kW solar balanced
        let state = sim.create_state(6000.0, 6000.0, 0.0);

        // Net should be 0, but phases are unbalanced
        assert!((state.grid_power.total()).abs() < 100.0); // Close to zero

        // But L1 should be importing, L2/L3 exporting
        if state.load_distribution == LoadDistribution::SinglePhase(1) {
            assert!(state.grid_power.l1_w > 0.0); // Importing on L1
            assert!(state.grid_power.l2_w < 0.0); // Exporting on L2
            assert!(state.grid_power.l3_w < 0.0); // Exporting on L3
        }
    }

    #[test]
    fn test_phase_overload_scenario() {
        let mut sim = ThreePhaseSimulator::new(230.0, 25.0);
        sim.set_load_distribution(LoadDistribution::SinglePhase(1));

        // 10kW house load all on L1, no solar
        let state = sim.create_state(10000.0, 0.0, 0.0);

        // L1 current should be ~43A (10000W / 230V), exceeding 25A limit
        let (overloaded, _curtailment) = sim.check_phase_overload(&state);
        assert!(overloaded);
    }

    #[test]
    fn test_unbalanced_vs_balanced_load() {
        let balanced_dist = LoadDistribution::Balanced;
        let unbalanced_dist = LoadDistribution::Unbalanced;

        let balanced_power = balanced_dist.distribute(6000.0);
        let unbalanced_power = unbalanced_dist.distribute(6000.0);

        // Both should sum to same total
        assert!((balanced_power.total() - unbalanced_power.total()).abs() < 1.0);

        // But unbalanced should have higher unbalance factor
        assert!(unbalanced_power.unbalance_factor() > balanced_power.unbalance_factor());
    }

    #[test]
    fn test_voltage_unbalance() {
        let balanced = ThreePhaseVoltage::balanced(230.0);
        assert!(balanced.unbalance_percent() < 0.1);

        let unbalanced = ThreePhaseVoltage {
            l1_v: 235.0,
            l2_v: 230.0,
            l3_v: 225.0,
        };
        assert!(unbalanced.unbalance_percent() > 1.0);
    }
}
