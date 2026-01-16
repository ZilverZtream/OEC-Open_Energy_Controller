#![allow(dead_code)]
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, FixedOffset, Local};
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
        let now: DateTime<FixedOffset> = Local::now().fixed_offset();
        let n = forecast.prices.len().min(24);
        if n == 0 {
            anyhow::bail!("no price points available");
        }

        let soc0 = bucket(state.battery.soc_percent);
        let mut dp = vec![vec![f64::INFINITY; 21]; n + 1];
        let mut prev = vec![vec![None; 21]; n + 1];
        dp[0][soc0] = 0.0;

        for t in 0..n {
            for soc in 0..21 {
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

fn bucket(soc_percent: f64) -> usize {
    let b = (soc_percent.clamp(0.0, 100.0) / 5.0).round() as i64;
    b.clamp(0, 20) as usize
}

fn simulate_action(
    soc_bucket: usize,
    action: Action,
    forecast: &Forecast24h,
    t: usize,
    constraints: &Constraints,
) -> Result<(usize, f64, f64)> {
    let price = forecast.prices[t].price_sek_per_kwh;

    let mut next = soc_bucket as i64;
    let cycle_penalty = 0.001;

    let mut target_power_w: f64 = match action {
        Action::Charge => {
            next += 1;
            2000.0
        }
        Action::Discharge => {
            next -= 1;
            -2000.0
        }
        Action::Idle => {
            0.0
        }
    };

    let min_b = bucket(constraints.min_soc_percent);
    let max_b = bucket(constraints.max_soc_percent);
    next = next.clamp(min_b as i64, max_b as i64);

    let max_grid_w = constraints.max_power_grid_kw.max(0.1) * 1000.0;
    target_power_w = target_power_w.clamp(-max_grid_w, max_grid_w);

    let energy_kwh = (target_power_w / 1000.0) * 1.0; // 1h step placeholder
    let mut cost = energy_kwh * price;
    if matches!(action, Action::Charge | Action::Discharge) {
        cost += cycle_penalty;
    }

    Ok((next as usize, cost, target_power_w))
}
