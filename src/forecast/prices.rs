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

/// Nordpool API client for day-ahead electricity prices
#[derive(Clone)]
pub struct NordpoolPriceForecaster {
    base_url: String,
    client: reqwest::Client,
    cache: Arc<RwLock<Option<(DateTime<FixedOffset>, PriceArea, Vec<PricePoint>)>>>,
    ttl: Duration,
    eur_to_sek_rate: f64, // Exchange rate for EUR to SEK conversion
}

impl NordpoolPriceForecaster {
    /// Create a new Nordpool price forecaster
    pub fn new(eur_to_sek_rate: f64) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static("open-energy-controller/0.2"),
        );
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .default_headers(headers)
            .build()?;

        Ok(Self {
            base_url: "https://www.nordpoolgroup.com/api/marketdata/page/10".to_string(),
            client,
            cache: Arc::new(RwLock::new(None)),
            ttl: Duration::from_secs(3600), // 1 hour cache
            eur_to_sek_rate,
        })
    }

    /// Fetch day-ahead prices from Nordpool
    async fn fetch_day_ahead_prices(&self, area: PriceArea) -> Result<Vec<PricePoint>> {
        let now = Local::now().fixed_offset();
        let date = now.date_naive();

        // Nordpool API URL for day-ahead prices
        let url = format!(
            "{}?currency=EUR,SEK&endDate={:04}-{:02}-{:02}",
            self.base_url,
            date.year(),
            date.month(),
            date.day()
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to fetch Nordpool prices")?;

        if !resp.status().is_success() {
            anyhow::bail!("Nordpool API error: HTTP {}", resp.status());
        }

        let nordpool_response: NordpoolResponse = resp
            .json()
            .await
            .context("Failed to parse Nordpool response")?;

        self.parse_nordpool_response(area, nordpool_response)
    }

    /// Parse Nordpool response into price points
    fn parse_nordpool_response(
        &self,
        area: PriceArea,
        response: NordpoolResponse,
    ) -> Result<Vec<PricePoint>> {
        let mut points = Vec::new();

        if let Some(rows) = response.data.rows {
            for row in rows {
                if let Some(columns) = row.columns {
                    // Find the column for the specified area
                    for col in columns {
                        if col.name == area.to_string() {
                            if let Some(value_str) = col.value {
                                // Parse price value (remove spaces and convert comma to dot)
                                let price_eur: f64 = value_str
                                    .replace(' ', "")
                                    .replace(',', ".")
                                    .parse()
                                    .unwrap_or(0.0);

                                // Convert EUR/MWh to SEK/kWh
                                let price_sek_per_kwh = (price_eur / 1000.0) * self.eur_to_sek_rate;

                                points.push(PricePoint {
                                    time_start: row.start_time,
                                    time_end: row.end_time,
                                    price_sek_per_kwh,
                                });
                            }
                        }
                    }
                }
            }
        }

        Ok(points)
    }
}

#[async_trait]
impl PriceForecaster for NordpoolPriceForecaster {
    async fn predict_next_24h(&self, area: PriceArea) -> Result<Vec<PricePoint>> {
        // Check cache first
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

        // Fetch from API
        let points = self.fetch_day_ahead_prices(area).await?;

        // Update cache
        let mut c = self.cache.write().await;
        *c = Some((Local::now().fixed_offset(), area, points.clone()));

        Ok(points)
    }
}

// Nordpool API response structures
#[derive(Debug, Deserialize)]
struct NordpoolResponse {
    data: NordpoolData,
}

#[derive(Debug, Deserialize)]
struct NordpoolData {
    #[serde(rename = "Rows")]
    rows: Option<Vec<NordpoolRow>>,
}

#[derive(Debug, Deserialize)]
struct NordpoolRow {
    #[serde(rename = "StartTime")]
    start_time: DateTime<FixedOffset>,
    #[serde(rename = "EndTime")]
    end_time: DateTime<FixedOffset>,
    #[serde(rename = "Columns")]
    columns: Option<Vec<NordpoolColumn>>,
}

#[derive(Debug, Deserialize)]
struct NordpoolColumn {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Value")]
    value: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_elpriset_just_nu_forecaster() {
        let forecaster = ElprisetJustNuPriceForecaster::new(
            "https://www.elprisetjustnu.se".to_string(),
            Duration::from_secs(3600),
        )
        .unwrap();

        // This test requires network access
        // Uncomment to test with real API
        // let result = forecaster.predict_next_24h(PriceArea::SE3).await;
        // assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_nordpool_forecaster() {
        let forecaster = NordpoolPriceForecaster::new(11.0).unwrap();

        // This test requires network access
        // Uncomment to test with real API
        // let result = forecaster.predict_next_24h(PriceArea::SE3).await;
        // assert!(result.is_ok());
    }
}
