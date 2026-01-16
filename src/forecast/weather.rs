//! Weather forecast integration (SMHI - Swedish Meteorological Institute)
//!
//! This module provides weather data for forecasting solar production and
//! energy consumption patterns.

use anyhow::{Context, Result};
use chrono::{DateTime, FixedOffset};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, error, info, warn};

/// Weather forecast point
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "swagger", derive(utoipa::ToSchema))]
pub struct WeatherPoint {
    pub timestamp: DateTime<FixedOffset>,
    pub temperature_c: f64,
    pub cloud_cover_percent: f64,
    pub wind_speed_ms: f64,
    pub precipitation_mm: f64,
    pub humidity_percent: f64,
}

/// Weather forecast for a location
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "swagger", derive(utoipa::ToSchema))]
pub struct WeatherForecast {
    pub location: GeoLocation,
    pub generated_at: DateTime<FixedOffset>,
    pub points: Vec<WeatherPoint>,
}

/// Geographic location
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "swagger", derive(utoipa::ToSchema))]
pub struct GeoLocation {
    pub latitude: f64,
    pub longitude: f64,
    pub name: Option<String>,
}

/// SMHI API client for Swedish weather forecasts
pub struct SmhiClient {
    client: Client,
    base_url: String,
}

impl SmhiClient {
    /// Create a new SMHI client
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
            base_url: "https://opendata-download-metfcst.smhi.se/api".to_string(),
        }
    }

    /// Fetch weather forecast for a location
    pub async fn fetch_forecast(&self, location: &GeoLocation) -> Result<WeatherForecast> {
        let url = format!(
            "{}/category/pmp3g/version/2/geotype/point/lon/{:.6}/lat/{:.6}/data.json",
            self.base_url, location.longitude, location.latitude
        );

        debug!("Fetching weather forecast from SMHI: {}", url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to send request to SMHI API")?;

        if !response.status().is_success() {
            error!("SMHI API returned error status: {}", response.status());
            anyhow::bail!("SMHI API error: {}", response.status());
        }

        let smhi_response: SmhiResponse = response
            .json()
            .await
            .context("Failed to parse SMHI response")?;

        info!(
            "Successfully fetched weather forecast from SMHI for location ({}, {})",
            location.latitude, location.longitude
        );

        self.parse_forecast(location.clone(), smhi_response)
    }

    /// Parse SMHI response into our weather forecast format
    fn parse_forecast(
        &self,
        location: GeoLocation,
        response: SmhiResponse,
    ) -> Result<WeatherForecast> {
        let mut points = Vec::new();

        for time_series in response.time_series {
            let timestamp = time_series.valid_time;

            let mut temperature_c = None;
            let mut cloud_cover_percent = None;
            let mut wind_speed_ms = None;
            let mut precipitation_mm = None;
            let mut humidity_percent = None;

            for param in time_series.parameters {
                match param.name.as_str() {
                    "t" => temperature_c = Some(param.values.first().copied().unwrap_or(0.0)),
                    "tcc_mean" => {
                        cloud_cover_percent = Some(param.values.first().copied().unwrap_or(0.0))
                    }
                    "ws" => wind_speed_ms = Some(param.values.first().copied().unwrap_or(0.0)),
                    "pmean" => precipitation_mm = Some(param.values.first().copied().unwrap_or(0.0)),
                    "r" => humidity_percent = Some(param.values.first().copied().unwrap_or(0.0)),
                    _ => {}
                }
            }

            points.push(WeatherPoint {
                timestamp,
                temperature_c: temperature_c.unwrap_or(15.0),
                cloud_cover_percent: cloud_cover_percent.unwrap_or(50.0) * 12.5, // Convert from oktas (0-8) to percent
                wind_speed_ms: wind_speed_ms.unwrap_or(3.0),
                precipitation_mm: precipitation_mm.unwrap_or(0.0),
                humidity_percent: humidity_percent.unwrap_or(70.0),
            });
        }

        Ok(WeatherForecast {
            location,
            generated_at: chrono::Utc::now().into(),
            points,
        })
    }
}

impl Default for SmhiClient {
    fn default() -> Self {
        Self::new()
    }
}

// SMHI API response structures
#[derive(Debug, Deserialize)]
struct SmhiResponse {
    #[serde(rename = "timeSeries")]
    time_series: Vec<SmhiTimeSeries>,
}

#[derive(Debug, Deserialize)]
struct SmhiTimeSeries {
    #[serde(rename = "validTime")]
    valid_time: DateTime<FixedOffset>,
    parameters: Vec<SmhiParameter>,
}

#[derive(Debug, Deserialize)]
struct SmhiParameter {
    name: String,
    values: Vec<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_geo_location() {
        let location = GeoLocation {
            latitude: 59.3293,
            longitude: 18.0686,
            name: Some("Stockholm".to_string()),
        };

        assert_eq!(location.latitude, 59.3293);
        assert_eq!(location.longitude, 18.0686);
        assert_eq!(location.name, Some("Stockholm".to_string()));
    }

    #[test]
    fn test_weather_point() {
        let point = WeatherPoint {
            timestamp: chrono::Utc::now().into(),
            temperature_c: 20.0,
            cloud_cover_percent: 50.0,
            wind_speed_ms: 5.0,
            precipitation_mm: 0.0,
            humidity_percent: 70.0,
        };

        assert_eq!(point.temperature_c, 20.0);
        assert_eq!(point.cloud_cover_percent, 50.0);
    }

    #[tokio::test]
    #[ignore] // Ignore by default as it requires network access
    async fn test_fetch_weather_forecast() {
        let client = SmhiClient::new();
        let location = GeoLocation {
            latitude: 59.3293,
            longitude: 18.0686,
            name: Some("Stockholm".to_string()),
        };

        let result = client.fetch_forecast(&location).await;
        assert!(result.is_ok(), "Failed to fetch weather forecast");

        let forecast = result.unwrap();
        assert!(!forecast.points.is_empty());
    }
}
