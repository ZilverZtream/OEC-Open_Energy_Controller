#![allow(dead_code)]
use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use super::{Action, Constraints, OptimizationStrategy, SystemState};
use crate::domain::{Forecast24h, Schedule, ScheduleEntry};

pub struct DynamicProgrammingOptimizer;

#[async_trait]
impl OptimizationStrategy for DynamicProgrammingOptimizer {
    async fn optimize(
        &self,
        state: &SystemState,
        forecast: &Forecast24h,
        constraints: &Constraints,
    ) -> Result<Schedule> {
        let now = Utc::now();
        let n = forecast.prices.len().min(24);
        if n == 0 {
            anyhow::bail!("no price points available");
        }

        // Validate all prices are finite before entering DP loop
        for (i, price_point) in forecast.prices.iter().take(n).enumerate() {
            if !price_point.price_sek_per_kwh.is_finite() {
                anyhow::bail!(
                    "Price at index {} is not finite: {}",
                    i,
                    price_point.price_sek_per_kwh
                );
            }
        }

        let soc0 = bucket(state.battery.soc_percent);
        // Use 51 states (0-100% in 2% increments) for better granularity
        const NUM_SOC_STATES: usize = 51;
        let mut dp = vec![vec![f64::INFINITY; NUM_SOC_STATES]; n + 1];
        let mut prev = vec![vec![None; NUM_SOC_STATES]; n + 1];
        dp[0][soc0] = 0.0;

        for t in 0..n {
            for soc in 0..NUM_SOC_STATES {
                let cur = dp[t][soc];
                if !cur.is_finite() {
                    continue;
                }
                for action in [Action::Charge, Action::Discharge, Action::Idle] {
                    let (next_soc, cost, target_power_w) =
                        simulate_action(soc, action, forecast, t, constraints)?;
                    let new_cost = cur + cost;
                    if new_cost < dp[t + 1][next_soc] {
                        dp[t + 1][next_soc] = new_cost;
                        prev[t + 1][next_soc] = Some((soc, action, target_power_w));
                    }
                }
            }
        }

        let (mut best_soc, _) = dp[n]
            .iter()
            .enumerate()
            .min_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, v)| (i, *v))
            .unwrap();

        let mut entries = Vec::with_capacity(n);
        for t in (1..=n).rev() {
            let (psoc, action, target_power_w) = prev[t][best_soc]
                .clone()
                .ok_or_else(|| anyhow::anyhow!("backtrack failed"))?;
            let p = &forecast.prices[t - 1];
            entries.push(ScheduleEntry {
                time_start: p.time_start,
                time_end: p.time_end,
                target_power_w,
                price_sek_per_kwh: p.price_sek_per_kwh,
                reason: format!("dp:{:?}", action),
            });
            best_soc = psoc;
        }
        entries.reverse();

        let valid_from = entries.first().map(|e| e.time_start).unwrap_or(now);
        let valid_until = entries.last().map(|e| e.time_end).unwrap_or(now);

        Ok(Schedule {
            id: Uuid::new_v4(),
            created_at: now,
            valid_from,
            valid_until,
            entries,
            optimizer_version: "dp-skeleton-0.1".to_string(),
        })
    }
}

/// Convert SoC percentage to discrete bucket
/// Uses 2% granularity (0-100% -> 0-50 buckets) for better precision
/// than the previous 5% granularity (0-20 buckets)
fn bucket(soc_percent: f64) -> usize {
    let b = (soc_percent.clamp(0.0, 100.0) / 2.0).round() as i64;
    b.clamp(0, 50) as usize
}

fn simulate_action(
    soc_bucket: usize,
    action: Action,
    forecast: &Forecast24h,
    t: usize,
    constraints: &Constraints,
) -> Result<(usize, f64, f64)> {
    let price_point = &forecast.prices[t];
    let import_price = price_point.price_sek_per_kwh;
    let export_price = price_point.export_price();

    // Calculate cycle penalty from battery degradation and replacement cost
    // cycle_penalty = degradation_per_cycle * replacement_cost (SEK)
    // This represents the economic cost of wearing out the battery
    // Example: LiFePO4 with 0.0001 degradation/cycle and 50,000 SEK cost
    // = 0.0001 * 50000 = 5 SEK per full cycle
    let cycle_penalty_per_full_cycle =
        constraints.battery_degradation_per_cycle * constraints.battery_replacement_cost_sek;

    // Use battery physical limits (NOT grid limits!)
    // The constraint is min(grid_limit, battery_limit)
    let battery_max_charge_w = constraints.battery_max_charge_kw.max(0.1) * 1000.0;
    let battery_max_discharge_w = constraints.battery_max_discharge_kw.max(0.1) * 1000.0;
    let grid_max_w = constraints.max_power_grid_kw.max(0.1) * 1000.0;

    let max_charge_w = battery_max_charge_w.min(grid_max_w);
    let max_discharge_w = battery_max_discharge_w.min(grid_max_w);

    // Use configured battery efficiency (NOT hardcoded!)
    let efficiency = constraints.battery_efficiency.clamp(0.5, 1.0);

    // Calculate target power for each action
    let target_power_w: f64 = match action {
        Action::Charge => max_charge_w,
        Action::Discharge => -max_discharge_w,
        Action::Idle => 0.0,
    };

    // CRITICAL FIX: Calculate actual time step duration from forecast (don't assume 1 hour!)
    // Forecast resolution varies: 15min, 30min, or 60min depending on provider
    let dt_hours = (price_point.time_end - price_point.time_start).num_seconds() as f64 / 3600.0;
    // Validate time step is reasonable (between 1 minute and 4 hours)
    if dt_hours <= 0.0 || dt_hours > 4.0 {
        anyhow::bail!(
            "Invalid time step duration: {} hours (must be between 0 and 4)",
            dt_hours
        );
    }

    // CRITICAL POWER CONVENTION:
    // target_power_w is AC-side power (grid perspective), NOT DC battery energy
    // This matches battery.rs convention to prevent efficiency double-dip
    // - Positive = charging (AC from grid -> DC to battery)
    // - Negative = discharging (DC from battery -> AC to grid)
    //
    // Calculate actual SoC change based on AC power and apply efficiency:
    // - Charging: AC * efficiency = DC energy stored in battery
    // - Discharging: DC / efficiency = AC energy delivered to grid
    let energy_kwh = if target_power_w > 0.0 {
        // Charging: AC power from grid * efficiency = DC energy stored
        (target_power_w / 1000.0) * dt_hours * efficiency
    } else if target_power_w < 0.0 {
        // Discharging: DC energy from battery / efficiency = AC power to grid
        (target_power_w / 1000.0) * dt_hours / efficiency
    } else {
        0.0
    };

    // Calculate SoC change in percentage points
    let battery_capacity = constraints.battery_capacity_kwh.max(0.1);
    let soc_change_percent = (energy_kwh / battery_capacity) * 100.0;

    // Calculate number of buckets to move (2% per bucket)
    let buckets_to_move = (soc_change_percent / 2.0).round() as i64;

    // Apply bucket movement
    let mut next = (soc_bucket as i64) + buckets_to_move;

    // Clamp to min/max SoC constraints
    let min_b = bucket(constraints.min_soc_percent);
    let max_b = bucket(constraints.max_soc_percent);
    next = next.clamp(min_b as i64, max_b as i64);

    // Cost calculation: energy cost + cycle degradation penalty (proportional to throughput)
    // CRITICAL FIX: Cycle penalty must be proportional to energy moved
    // A full 0-100% cycle costs cycle_penalty_per_full_cycle
    // A partial cycle costs: (energy_throughput / capacity) * cycle_penalty_per_full_cycle
    //
    // CRITICAL FIX: Use different prices for import (charging) vs export (discharging)
    // When charging (positive energy): we pay import_price
    // When discharging (negative energy): we earn export_price (typically < import_price)
    let mut cost = if energy_kwh > 0.0 {
        // Charging: buying from grid at import price
        energy_kwh * import_price
    } else {
        // Discharging: selling to grid at export price
        // Note: energy_kwh is negative, so this is a negative cost (profit)
        energy_kwh * export_price
    };

    if matches!(action, Action::Charge | Action::Discharge) {
        // Calculate proportional cycle cost based on energy throughput
        let cycle_fraction = energy_kwh.abs() / battery_capacity;
        cost += cycle_fraction * cycle_penalty_per_full_cycle;
    }

    // Validate cost is finite to prevent undefined behavior in min_by comparisons
    if !cost.is_finite() {
        anyhow::bail!(
            "Cost calculation resulted in non-finite value: energy_kwh={}, import_price={}, export_price={}, cost={}",
            energy_kwh,
            import_price,
            export_price,
            cost
        );
    }

    Ok((next as usize, cost, target_power_w))
}
