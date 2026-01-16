#![allow(dead_code)]
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, FixedOffset, Local};
use uuid::Uuid;

use super::{Constraints, OptimizationStrategy, SystemState};
use crate::domain::{Forecast24h, Schedule, ScheduleEntry};

/// Simple greedy optimizer that follows basic rules:
/// - Charge when prices are below average
/// - Discharge when prices are above average
/// - Stay idle otherwise
/// This serves as a baseline to compare against more sophisticated strategies
pub struct GreedyOptimizer {
    /// Threshold multiplier for charge decision (e.g., 0.9 means charge when price < 0.9 * avg)
    pub charge_threshold: f64,
    /// Threshold multiplier for discharge decision (e.g., 1.1 means discharge when price > 1.1 * avg)
    pub discharge_threshold: f64,
}

impl Default for GreedyOptimizer {
    fn default() -> Self {
        Self {
            charge_threshold: 0.9,
            discharge_threshold: 1.1,
        }
    }
}

impl GreedyOptimizer {
    pub fn new(charge_threshold: f64, discharge_threshold: f64) -> Self {
        Self {
            charge_threshold,
            discharge_threshold,
        }
    }

    /// Calculate average price from forecast
    fn average_price(forecast: &Forecast24h) -> f64 {
        if forecast.prices.is_empty() {
            return 0.0;
        }

        let sum: f64 = forecast.prices.iter().map(|p| p.price_sek_per_kwh).sum();
        sum / forecast.prices.len() as f64
    }

    /// Determine target power based on price and SoC
    fn determine_power(
        &self,
        price: f64,
        avg_price: f64,
        current_soc: f64,
        constraints: &Constraints,
    ) -> (f64, &str) {
        let max_charge_w = constraints.max_power_grid_kw * 1000.0;
        let max_discharge_w = constraints.max_power_grid_kw * 1000.0;

        // Don't charge if already at max SoC
        if current_soc >= constraints.max_soc_percent {
            return (0.0, "at_max_soc");
        }

        // Don't discharge if at min SoC
        if current_soc <= constraints.min_soc_percent {
            return (0.0, "at_min_soc");
        }

        // Charge decision
        if price < avg_price * self.charge_threshold {
            let power = if current_soc < constraints.min_soc_percent + 10.0 {
                // Charge faster if SoC is low
                max_charge_w
            } else {
                // Normal charging
                max_charge_w * 0.7
            };
            return (power, "cheap_price_charge");
        }

        // Discharge decision
        if price > avg_price * self.discharge_threshold && current_soc > constraints.min_soc_percent + 20.0 {
            let power = if current_soc > constraints.max_soc_percent - 10.0 {
                // Discharge faster if SoC is high
                -max_discharge_w
            } else {
                // Normal discharging
                -max_discharge_w * 0.7
            };
            return (power, "high_price_discharge");
        }

        // Stay idle for prices near average
        (0.0, "idle")
    }

    /// Simulate SoC change based on power command
    fn simulate_soc_change(current_soc: f64, power_w: f64, duration_h: f64, capacity_kwh: f64) -> f64 {
        let efficiency = 0.95; // 95% round-trip efficiency
        let energy_kwh = (power_w / 1000.0) * duration_h;

        let soc_delta = if power_w >= 0.0 {
            // Charging
            (energy_kwh / capacity_kwh) * efficiency * 100.0
        } else {
            // Discharging
            (energy_kwh / capacity_kwh) / efficiency * 100.0
        };

        (current_soc + soc_delta).clamp(0.0, 100.0)
    }
}

#[async_trait]
impl OptimizationStrategy for GreedyOptimizer {
    async fn optimize(
        &self,
        state: &SystemState,
        forecast: &Forecast24h,
        constraints: &Constraints,
    ) -> Result<Schedule> {
        let now: DateTime<FixedOffset> = Local::now().fixed_offset();

        if forecast.prices.is_empty() {
            anyhow::bail!("No price points available for optimization");
        }

        let avg_price = Self::average_price(forecast);
        let mut current_soc = state.battery.soc_percent;

        // Assume 10 kWh battery capacity (could be read from battery capabilities)
        let capacity_kwh = 10.0;

        let mut entries = Vec::new();

        for price_point in &forecast.prices {
            let (target_power_w, reason) = self.determine_power(
                price_point.price_sek_per_kwh,
                avg_price,
                current_soc,
                constraints,
            );

            // Calculate duration for this interval (typically 1 hour)
            let duration = price_point.time_end.signed_duration_since(price_point.time_start);
            let duration_h = duration.num_minutes() as f64 / 60.0;

            entries.push(ScheduleEntry {
                time_start: price_point.time_start,
                time_end: price_point.time_end,
                target_power_w,
                reason: format!("greedy:{}", reason),
            });

            // Update simulated SoC for next iteration
            current_soc = Self::simulate_soc_change(current_soc, target_power_w, duration_h, capacity_kwh);
        }

        let valid_from = entries.first().map(|e| e.time_start).unwrap_or(now);
        let valid_until = entries.last().map(|e| e.time_end).unwrap_or(now);

        Ok(Schedule {
            id: Uuid::new_v4(),
            created_at: now,
            valid_from,
            valid_until,
            entries,
            optimizer_version: "greedy-v1.0".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_average_price() {
        let forecast = Forecast24h {
            area: crate::domain::types::PriceArea::SE3,
            generated_at: Local::now().fixed_offset(),
            prices: vec![
                crate::domain::types::PricePoint {
                    time_start: Local::now().fixed_offset(),
                    time_end: Local::now().fixed_offset() + Duration::hours(1),
                    price_sek_per_kwh: 1.0,
                },
                crate::domain::types::PricePoint {
                    time_start: Local::now().fixed_offset() + Duration::hours(1),
                    time_end: Local::now().fixed_offset() + Duration::hours(2),
                    price_sek_per_kwh: 2.0,
                },
                crate::domain::types::PricePoint {
                    time_start: Local::now().fixed_offset() + Duration::hours(2),
                    time_end: Local::now().fixed_offset() + Duration::hours(3),
                    price_sek_per_kwh: 3.0,
                },
            ],
            consumption: vec![],
            production: vec![],
        };

        assert_eq!(GreedyOptimizer::average_price(&forecast), 2.0);
    }

    #[test]
    fn test_soc_simulation() {
        // Charging 2kW for 1 hour with 10kWh capacity
        let new_soc = GreedyOptimizer::simulate_soc_change(50.0, 2000.0, 1.0, 10.0);
        // Should increase by approximately 2kWh * 0.95 efficiency / 10kWh * 100 = 19%
        assert!((new_soc - 69.0).abs() < 1.0); // Allow small tolerance

        // Discharging 2kW for 1 hour
        let new_soc = GreedyOptimizer::simulate_soc_change(50.0, -2000.0, 1.0, 10.0);
        // Should decrease by approximately 2kWh / 0.95 efficiency / 10kWh * 100 = 21.05%
        assert!((new_soc - 29.0).abs() < 1.0);
    }

    #[test]
    fn test_greedy_threshold_logic() {
        let optimizer = GreedyOptimizer::default();
        let constraints = Constraints::default();

        // Low price should trigger charging
        let (power, reason) = optimizer.determine_power(0.5, 1.0, 50.0, &constraints);
        assert!(power > 0.0);
        assert_eq!(reason, "cheap_price_charge");

        // High price should trigger discharging
        let (power, reason) = optimizer.determine_power(2.0, 1.0, 50.0, &constraints);
        assert!(power < 0.0);
        assert_eq!(reason, "high_price_discharge");

        // Average price should stay idle
        let (power, reason) = optimizer.determine_power(1.0, 1.0, 50.0, &constraints);
        assert_eq!(power, 0.0);
        assert_eq!(reason, "idle");
    }
}
