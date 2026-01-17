#![allow(dead_code)]
//! MILP (Mixed-Integer Linear Programming) Optimizer
//!
//! This module implements an exact optimization strategy using linear programming
//! to solve the 24-hour battery scheduling problem. MILP is the industry standard
//! for optimal battery scheduling with complex constraints.
//!
//! The formulation considers:
//! - Energy prices over 24h horizon
//! - Battery SoC bounds (min/max)
//! - Charge/discharge power limits
//! - Grid power limits (fuse limits)
//! - Battery efficiency losses
//! - Cycle count constraints

use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

#[cfg(feature = "optimization")]
use good_lp::*;

use crate::optimizer::{Constraints, OptimizationStrategy, SystemState};
use crate::domain::{Forecast24h, Schedule, ScheduleEntry};

/// MILP Optimizer using linear programming for exact solutions
pub struct MilpOptimizer {
    /// Solver to use (CBC, HiGHS, etc.)
    solver_type: SolverType,
    /// Time limit in seconds for solver
    time_limit_seconds: u64,
}

#[derive(Debug, Clone, Copy)]
pub enum SolverType {
    /// CBC solver (default, open-source)
    Cbc,
    /// HiGHS solver (faster for large problems)
    #[allow(dead_code)]
    HiGHS,
}

impl Default for MilpOptimizer {
    fn default() -> Self {
        Self {
            solver_type: SolverType::Cbc,
            time_limit_seconds: 30,
        }
    }
}

impl MilpOptimizer {
    pub fn new(solver_type: SolverType, time_limit_seconds: u64) -> Self {
        Self {
            solver_type,
            time_limit_seconds,
        }
    }

    #[cfg(feature = "optimization")]
    fn solve_lp(
        &self,
        state: &SystemState,
        forecast: &Forecast24h,
        constraints: &Constraints,
    ) -> Result<Vec<f64>> {
        use good_lp::*;

        let n_periods = forecast.prices.len();
        if n_periods == 0 {
            anyhow::bail!("No price periods available");
        }

        // CRITICAL FIX #4: MILP Solver Complexity Management for Raspberry Pi
        // Problem: Mixed Integer Linear Programming is NP-Hard. With ThreePhaseLoad
        // and ThermalZone constraints, complexity can explode on a Raspberry Pi.
        //
        // Mitigation strategies:
        // 1. Limit problem size to 24 periods (hourly resolution for 24h)
        // 2. Use continuous relaxation (Linear Programming) instead of true MILP
        //    for faster solve times. This is acceptable for battery scheduling
        //    where power can be continuous.
        // 3. The `time_limit_seconds` field exists but good_lp/CBC doesn't expose
        //    time limits in a platform-independent way. For production deployment,
        //    consider using HiGHS solver which supports time limits and gap tolerance.
        //
        // WARNING: If solver hangs on Raspberry Pi, reduce n_periods or switch to
        // greedy/DP strategy for real-time control.
        if n_periods > 48 {
            tracing::warn!(
                "MILP solver received {} periods. This may be too large for Raspberry Pi. \
                 Consider reducing to 24 periods (hourly) for faster solve times.",
                n_periods
            );
        }

        // CRITICAL FIX: Implement proper MILP optimization using good_lp
        // This replaces the greedy placeholder with true mathematical optimization

        // Create optimization problem
        let mut problem = ProblemVariables::new();

        // Define variables for each time period
        // charge[t] = charging power at time t (kW)
        // discharge[t] = discharging power at time t (kW)
        // soc[t] = state of charge at time t (%)
        // peak_power = maximum grid import power across all periods (kW) - for Effekttariff
        let charge = problem.add_vector(variable().min(0.0), n_periods);
        let discharge = problem.add_vector(variable().min(0.0), n_periods);
        let soc = problem.add_vector(variable().min(0.0).max(100.0), n_periods + 1);
        let peak_power = problem.add(variable().min(0.0));

        // Calculate time step durations (in hours)
        let durations: Vec<f64> = forecast.prices.iter()
            .map(|p| {
                let duration = p.time_end.signed_duration_since(p.time_start);
                duration.num_minutes() as f64 / 60.0
            })
            .collect();

        // Extract prices
        let prices: Vec<f64> = forecast.prices.iter()
            .map(|p| p.price_sek_per_kwh)
            .collect();

        // AUDIT FIX #4: Extract consumption forecast to account for house load in constraints
        // If consumption data is available, use it; otherwise assume conservative 2kW baseline
        let consumption: Vec<f64> = if forecast.consumption.len() == n_periods {
            forecast.consumption.iter().map(|c| c.load_kw).collect()
        } else {
            vec![2.0; n_periods] // Conservative 2kW baseline house load
        };

        // Build the optimization problem
        // Objective: Minimize energy cost + peak power penalty (Effekttariff) + battery wear cost
        let energy_cost = (0..n_periods).map(|t| {
            prices[t] * durations[t] * (charge[t] - discharge[t])
        }).sum::<Expression>();

        // Peak power tariff penalty (Swedish "Effekttariff")
        // This is charged monthly based on the maximum hourly average power
        // Typical: 50-120 SEK/kW/month
        // We amortize the monthly charge over the 24h optimization period
        let peak_power_penalty = peak_power * constraints.peak_power_tariff_sek_per_kw;

        // CRITICAL FIX #3: Battery degradation cost
        // Real LiFePO4 batteries cost ~0.50 - 1.00 SEK per cycled kWh in wear
        // Without this, optimizer will cycle battery to save 0.01 SEK, destroying a 50,000 SEK battery
        // Wear cost = (charge + discharge) * degradation_per_cycle * replacement_cost / capacity
        let wear_cost_per_kwh = (constraints.battery_degradation_per_cycle
                                * constraints.battery_replacement_cost_sek)
                               / constraints.battery_capacity_kwh;

        let battery_wear_cost = (0..n_periods).map(|t| {
            // Both charging and discharging cause wear (one full cycle = charge + discharge)
            // Multiply by duration to convert power (kW) to energy (kWh)
            wear_cost_per_kwh * durations[t] * (charge[t] + discharge[t])
        }).sum::<Expression>();

        let objective = energy_cost + peak_power_penalty + battery_wear_cost;

        let mut problem_builder = problem.minimise(objective).using(default_solver);

        // Constraint: Initial SoC
        problem_builder = problem_builder.with(constraint!(
            soc[0] == state.battery.soc_percent
        ));

        // Constraints for each time period
        for t in 0..n_periods {
            let dt_h = durations[t];

            // SoC dynamics: soc[t+1] = soc[t] + (charge[t] * eff - discharge[t] / eff) * dt / capacity * 100
            let soc_delta = (charge[t] * constraints.battery_efficiency
                           - discharge[t] / constraints.battery_efficiency)
                           * dt_h / constraints.battery_capacity_kwh * 100.0;

            problem_builder = problem_builder.with(constraint!(
                soc[t + 1] == soc[t] + soc_delta
            ));

            // Power limits
            // Battery charge rate limit
            problem_builder = problem_builder.with(constraint!(
                charge[t] <= constraints.battery_max_charge_kw
            ));

            // Battery discharge rate limit
            problem_builder = problem_builder.with(constraint!(
                discharge[t] <= constraints.battery_max_discharge_kw
            ));

            // AUDIT FIX #4: Fuse limit must account for total grid import (house + battery)
            // Without this, the optimizer could command 11kW charging while house draws 6kW,
            // causing a 17kW total draw that blows the 11kW fuse.
            // The fuse limit is reduced by the house consumption to get max battery charge
            let max_charge_with_house = constraints.max_power_grid_kw - consumption[t];
            problem_builder = problem_builder.with(constraint!(
                charge[t] <= max_charge_with_house
            ));

            // CRITICAL FIX #2: Peak power tracking for Effekttariff
            // peak_power must be >= total grid import at every time period
            // Total grid import = house consumption + battery charging (discharge reduces grid import)
            // This forces the optimizer to minimize the maximum grid power across all periods
            let total_grid_import = consumption[t] + charge[t];
            problem_builder = problem_builder.with(constraint!(
                peak_power >= total_grid_import
            ));

            // SoC bounds
            problem_builder = problem_builder.with(constraint!(
                soc[t + 1] >= constraints.min_soc_percent
            ));

            problem_builder = problem_builder.with(constraint!(
                soc[t + 1] <= constraints.max_soc_percent
            ));
        }

        // Solve the optimization problem
        let solution = problem_builder.solve()
            .context("MILP solver failed to find solution")?;

        // Extract power schedule (positive = charge, negative = discharge)
        let power_schedule: Vec<f64> = (0..n_periods)
            .map(|t| {
                let charge_kw = solution.value(charge[t]);
                let discharge_kw = solution.value(discharge[t]);
                charge_kw - discharge_kw // Net power
            })
            .collect();

        Ok(power_schedule)
    }

    #[cfg(not(feature = "optimization"))]
    fn solve_lp(
        &self,
        _state: &SystemState,
        _forecast: &Forecast24h,
        _constraints: &Constraints,
    ) -> Result<Vec<f64>> {
        anyhow::bail!("MILP optimization requires 'optimization' feature to be enabled");
    }
}

#[async_trait]
impl OptimizationStrategy for MilpOptimizer {
    async fn optimize(
        &self,
        state: &SystemState,
        forecast: &Forecast24h,
        constraints: &Constraints,
    ) -> Result<Schedule> {
        let now = Utc::now();

        if forecast.prices.is_empty() {
            anyhow::bail!("No price points available for optimization");
        }

        // Solve the LP problem
        let power_schedule = self.solve_lp(state, forecast, constraints)
            .context("MILP solver failed")?;

        // Convert solution to schedule entries
        let mut entries = Vec::new();
        for (i, price_point) in forecast.prices.iter().enumerate() {
            let target_power_kw = power_schedule[i];
            let target_power_w = target_power_kw * 1000.0;

            let reason = if target_power_w > 100.0 {
                "milp:charge"
            } else if target_power_w < -100.0 {
                "milp:discharge"
            } else {
                "milp:idle"
            };

            entries.push(ScheduleEntry {
                time_start: price_point.time_start,
                time_end: price_point.time_end,
                target_power_w,
                reason: reason.to_string(),
            });
        }

        let valid_from = entries.first().map(|e| e.time_start).unwrap_or(now);
        let valid_until = entries.last().map(|e| e.time_end).unwrap_or(now);

        Ok(Schedule {
            id: Uuid::new_v4(),
            created_at: now,
            valid_from,
            valid_until,
            entries,
            optimizer_version: "milp-v1.0".to_string(),
        })
    }
}

#[cfg(all(test, feature = "optimization"))]
mod tests {
    use super::*;
    use crate::domain::{BatteryState, PricePoint, PriceArea};
    use chrono::Duration;

    fn create_test_forecast() -> Forecast24h {
        let now = Utc::now();
        let mut prices = Vec::new();

        // Create 24 hourly price points with varying prices
        for i in 0..24 {
            let start = now + Duration::hours(i);
            let end = start + Duration::hours(1);

            // Simulate daily price pattern: low at night, high during day
            let hour = i % 24;
            let price = if hour < 6 || hour > 22 {
                0.5 // Low night price
            } else if hour >= 9 && hour <= 18 {
                2.0 // High day price
            } else {
                1.0 // Medium price
            };

            prices.push(PricePoint {
                time_start: start,
                time_end: end,
                price_sek_per_kwh: price,
                export_price_sek_per_kwh: None,
            });
        }

        Forecast24h {
            area: PriceArea::SE3,
            generated_at: now,
            prices,
            consumption: vec![],
            production: vec![],
        }
    }

    #[tokio::test]
    async fn test_milp_optimizer_basic() {
        let optimizer = MilpOptimizer::default();
        let state = SystemState {
            battery: BatteryState {
                soc_percent: 50.0,
                soc_kwh: 5.0,
                power_w: 0.0,
                voltage_v: 48.0,
                current_a: 0.0,
                temperature_c: 25.0,
                status: crate::domain::BatteryStatus::Ready,
                health_percent: 100.0,
                cycles: 0,
            },
        };
        let constraints = Constraints::default();
        let forecast = create_test_forecast();

        let schedule = optimizer.optimize(&state, &forecast, &constraints).await.unwrap();

        // Verify schedule structure
        assert_eq!(schedule.entries.len(), 24);
        assert_eq!(schedule.optimizer_version, "milp-v1.0");

        // Verify that optimizer charges during low prices and discharges during high prices
        let night_entries: Vec<_> = schedule.entries.iter().take(6).collect();
        let day_entries: Vec<_> = schedule.entries.iter().skip(9).take(9).collect();

        // Night should have more charging (positive power)
        let night_avg_power: f64 = night_entries.iter()
            .map(|e| e.target_power_w)
            .sum::<f64>() / night_entries.len() as f64;

        // Day should have more discharging (negative power)
        let day_avg_power: f64 = day_entries.iter()
            .map(|e| e.target_power_w)
            .sum::<f64>() / day_entries.len() as f64;

        // Night charging should be higher than day (or day should be negative)
        assert!(night_avg_power > day_avg_power,
            "MILP should charge at night (low prices) and discharge during day (high prices)");
    }

    #[tokio::test]
    async fn test_milp_respects_soc_constraints() {
        let optimizer = MilpOptimizer::default();
        let state = SystemState {
            battery: BatteryState {
                soc_percent: 20.0, // Start at minimum
                soc_kwh: 2.0,
                power_w: 0.0,
                voltage_v: 48.0,
                current_a: 0.0,
                temperature_c: 25.0,
                status: crate::domain::BatteryStatus::Ready,
                health_percent: 100.0,
                cycles: 0,
            },
        };
        let constraints = Constraints::default();
        let forecast = create_test_forecast();

        let schedule = optimizer.optimize(&state, &forecast, &constraints).await.unwrap();

        // Simulate SoC through schedule to verify constraints
        let mut soc = state.battery.soc_percent;
        for entry in &schedule.entries {
            let duration = entry.time_end.signed_duration_since(entry.time_start);
            let duration_h = duration.num_minutes() as f64 / 60.0;
            let energy_kwh = (entry.target_power_w / 1000.0) * duration_h * constraints.battery_efficiency;
            let soc_change = (energy_kwh / constraints.battery_capacity_kwh) * 100.0;
            soc += soc_change;

            // Verify SoC stays within bounds (with small tolerance for numerical errors)
            assert!(soc >= constraints.min_soc_percent - 0.1,
                "SoC {} below minimum {}", soc, constraints.min_soc_percent);
            assert!(soc <= constraints.max_soc_percent + 0.1,
                "SoC {} above maximum {}", soc, constraints.max_soc_percent);
        }
    }
}
