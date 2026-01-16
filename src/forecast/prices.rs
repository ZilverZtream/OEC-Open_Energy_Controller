use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Datelike, FixedOffset, Local};
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use serde::Deserialize;
use std::{sync::Arc, time::Duration};
use tokio::sync::RwLock;

use crate::domain::{PriceArea, PricePoint};

#[async_trait]
pub trait PriceForecaster: Send + Sync {
    async fn predict_next_24h(&self, area: PriceArea) -> Result<Vec<PricePoint>>;
}

#[derive(Clone)]
pub struct ElprisetJustNuPriceForecaster {
    base_url: String,
    client: reqwest::Client,
    cache: Arc<RwLock<Option<(DateTime<FixedOffset>, PriceArea, Vec<PricePoint>)>>>,
    ttl: Duration,
}

impl ElprisetJustNuPriceForecaster {
    pub fn new(base_url: String, ttl: Duration) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static("open-energy-controller/0.2"),
        );
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .default_headers(headers)
            .build()?;
        Ok(Self {
            base_url,
            client,
            cache: Arc::new(RwLock::new(None)),
            ttl,
        })
    }

    fn url_for_today(&self, area: PriceArea) -> String {
        let now = Local::now().fixed_offset();
        let date = now.date_naive();
        format!(
            "{}/api/v1/prices/{:04}/{:02}-{:02}_{}.json",
            self.base_url.trim_end_matches('/'),
            date.year(),
            date.month(),
            date.day(),
            area
        )
    }
}

#[async_trait]
impl PriceForecaster for ElprisetJustNuPriceForecaster {
    async fn predict_next_24h(&self, area: PriceArea) -> Result<Vec<PricePoint>> {
        {
            let c = self.cache.read().await;
            if let Some((ts, a, v)) = &*c {
                if *a == area
                    && (Local::now().fixed_offset() - *ts).num_seconds() < self.ttl.as_secs() as i64
                {
                    return Ok(v.clone());
                }
            }
        }

        let url = self.url_for_today(area);
        let resp = self
            .client
            .get(url)
            .send()
            .await
            .context("price GET failed")?;
        let status = resp.status();
        let body = resp.text().await.context("price read failed")?;
        if !status.is_success() {
            anyhow::bail!("price API error: HTTP {status}: {body}");
        }

        let raw: Vec<RawPrice> = serde_json::from_str(&body).context("price JSON parse failed")?;
        let points = raw
            .into_iter()
            .map(|r| PricePoint {
                time_start: r.time_start,
                time_end: r.time_end,
                price_sek_per_kwh: r.sek_per_kwh,
            })
            .collect::<Vec<_>>();

        let mut c = self.cache.write().await;
        *c = Some((Local::now().fixed_offset(), area, points.clone()));
        Ok(points)
    }
}

#[derive(Debug, Deserialize)]
struct RawPrice {
    #[serde(rename = "SEK_per_kWh")]
    sek_per_kwh: f64,
    time_start: DateTime<FixedOffset>,
    time_end: DateTime<FixedOffset>,
}
