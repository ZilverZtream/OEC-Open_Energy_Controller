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
    let price = forecast.prices[t].price_sek_per_kwh;

    let cycle_penalty = 0.001;

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

    // CRITICAL FIX: Calculate actual SoC change based on energy and capacity
    // This is the "Physics Hallucination" fix
    let energy_kwh = if target_power_w > 0.0 {
        // Charging: power delivered over 1 hour with efficiency loss
        (target_power_w / 1000.0) * 1.0 * efficiency
    } else if target_power_w < 0.0 {
        // Discharging: power delivered over 1 hour with efficiency loss
        (target_power_w / 1000.0) * 1.0 / efficiency
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

    // Cost calculation: energy cost + cycle degradation penalty
    let mut cost = energy_kwh * price;
    if matches!(action, Action::Charge | Action::Discharge) {
        cost += cycle_penalty;
    }

    Ok((next as usize, cost, target_power_w))
}
