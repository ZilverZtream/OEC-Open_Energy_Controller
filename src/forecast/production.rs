#![allow(dead_code)]
use anyhow::Result;
use async_trait::async_trait;
use chrono::{Datelike, Utc, TimeZone, Timelike};
use uuid::Uuid;

use crate::domain::ProductionPoint;

#[async_trait]
pub trait ProductionForecaster: Send + Sync {
    async fn predict_next_24h(&self, household_id: Uuid) -> Result<Vec<ProductionPoint>>;
}

pub struct SimpleProductionForecaster {
    pub peak_kw: f64,
    pub sunrise: f64,
    pub sunset: f64,
    pub cloud_factor: f64,
}

impl Default for SimpleProductionForecaster {
    fn default() -> Self {
        Self {
            peak_kw: 3.5,
            sunrise: 8.0,
            sunset: 16.0,
            cloud_factor: 0.75,
        }
    }
}

#[async_trait]
impl ProductionForecaster for SimpleProductionForecaster {
    async fn predict_next_24h(&self, _household_id: Uuid) -> Result<Vec<ProductionPoint>> {
        let now = Utc::now();
        let start = Utc
            .with_ymd_and_hms(now.year(), now.month(), now.day(), 0, 0, 0)
            .unwrap();

        let mut out = Vec::with_capacity(24);
        for h in 0..24 {
            let t0 = start + chrono::Duration::hours(h);
            let t1 = start + chrono::Duration::hours(h + 1);
            let hh = t0.hour() as f64;

            let pv = if hh < self.sunrise || hh > self.sunset {
                0.0
            } else {
                let day_len = (self.sunset - self.sunrise).max(0.01);
                let x = (hh - self.sunrise) / day_len;
                (std::f64::consts::PI * x).sin().max(0.0) * self.peak_kw * self.cloud_factor
            };

            out.push(ProductionPoint {
                time_start: t0,
                time_end: t1,
                pv_kw: pv,
            });
        }
        Ok(out)
    }
}
