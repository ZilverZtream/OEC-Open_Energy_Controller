use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, FixedOffset, Local, Timelike, TimeZone};
use uuid::Uuid;

use crate::domain::ConsumptionPoint;

#[async_trait]
pub trait ConsumptionForecaster: Send + Sync {
    async fn predict_next_24h(&self, household_id: Uuid) -> Result<Vec<ConsumptionPoint>>;
}

pub struct SimpleConsumptionForecaster;

#[async_trait]
impl ConsumptionForecaster for SimpleConsumptionForecaster {
    async fn predict_next_24h(&self, _household_id: Uuid) -> Result<Vec<ConsumptionPoint>> {
        let now: DateTime<FixedOffset> = Local::now().fixed_offset();
        let tz = *now.offset();
        let start = tz.with_ymd_and_hms(now.year(), now.month(), now.day(), 0, 0, 0).unwrap();

        let mut out = Vec::with_capacity(24);
        for h in 0..24 {
            let t0 = start + chrono::Duration::hours(h);
            let t1 = start + chrono::Duration::hours(h + 1);
            let hh = t0.hour() as f64;

            let base = 0.6;
            let morning = bump(hh, 7.5, 1.5) * 1.0;
            let evening = bump(hh, 18.5, 2.0) * 1.6;

            out.push(ConsumptionPoint { time_start: t0, time_end: t1, load_kw: (base + morning + evening).max(0.2) });
        }
        Ok(out)
    }
}

use chrono::Datelike;
fn bump(x: f64, mu: f64, sigma: f64) -> f64 {
    let z = (x - mu) / sigma.max(0.01);
    (-0.5 * z * z).exp()
}
