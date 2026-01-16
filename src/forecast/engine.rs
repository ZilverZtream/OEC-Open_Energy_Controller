use anyhow::Result;
use chrono::{DateTime, FixedOffset, Local};
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
        let generated_at: DateTime<FixedOffset> = Local::now().fixed_offset();
        let prices = self.price_forecaster.predict_next_24h(area).await?;
        let consumption = self
            .consumption_forecaster
            .predict_next_24h(household_id)
            .await?;
        let production = self
            .production_forecaster
            .predict_next_24h(household_id)
            .await?;
        Ok(Forecast24h {
            area,
            generated_at,
            prices,
            consumption,
            production,
        })
    }
}
