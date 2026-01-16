#![allow(dead_code)]
use anyhow::Result;
use chrono::Utc;
use uuid::Uuid;

use super::{ConsumptionForecaster, PriceForecaster, ProductionForecaster};
use crate::domain::{Forecast24h, PriceArea};

pub struct ForecastEngine {
    pub price_forecaster: Box<dyn PriceForecaster>,
    pub consumption_forecaster: Box<dyn ConsumptionForecaster>,
    pub production_forecaster: Box<dyn ProductionForecaster>,
}

impl ForecastEngine {
    pub fn new(
        price: Box<dyn PriceForecaster>,
        cons: Box<dyn ConsumptionForecaster>,
        prod: Box<dyn ProductionForecaster>,
    ) -> Self {
        Self {
            price_forecaster: price,
            consumption_forecaster: cons,
            production_forecaster: prod,
        }
    }

    pub async fn get_forecast_24h(
        &self,
        area: PriceArea,
        household_id: Uuid,
    ) -> Result<Forecast24h> {
        let generated_at = Utc::now();

        // CRITICAL FIX: Use join! instead of try_join! to handle failures individually
        // If consumption/production forecasts fail, we can use fallback values
        // But if price forecast fails, the entire system should fail
        // This prevents unnecessary safety fallback when only non-critical forecasts fail
        let (price_result, consumption_result, production_result) = tokio::join!(
            self.price_forecaster.predict_next_24h(area),
            self.consumption_forecaster.predict_next_24h(household_id),
            self.production_forecaster.predict_next_24h(household_id)
        );

        // Price forecast is critical - fail if unavailable
        let prices = price_result?;

        // Consumption forecast is non-critical - use fallback if unavailable
        let consumption = match consumption_result {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(
                    error=%e,
                    "Consumption forecast failed, using fallback (flat 2kW load)"
                );
                // Fallback: assume constant 2kW load for next 24 hours
                let mut fallback = Vec::new();
                let now = Utc::now();
                for i in 0..24 {
                    fallback.push(crate::domain::ConsumptionPoint {
                        time_start: now + chrono::Duration::hours(i),
                        time_end: now + chrono::Duration::hours(i + 1),
                        load_kw: 2.0,
                    });
                }
                fallback
            }
        };

        // Production forecast is non-critical - use fallback if unavailable
        let production = match production_result {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!(
                    error=%e,
                    "Production forecast failed, using fallback (zero production)"
                );
                // Fallback: assume no solar production (conservative)
                let mut fallback = Vec::new();
                let now = Utc::now();
                for i in 0..24 {
                    fallback.push(crate::domain::ProductionPoint {
                        time_start: now + chrono::Duration::hours(i),
                        time_end: now + chrono::Duration::hours(i + 1),
                        pv_kw: 0.0,
                    });
                }
                fallback
            }
        };

        Ok(Forecast24h {
            area,
            generated_at,
            prices,
            consumption,
            production,
        })
    }
}
