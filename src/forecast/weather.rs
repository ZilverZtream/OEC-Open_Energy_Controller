#![allow(dead_code)]
//! Weather forecast integration (SMHI - Swedish Meteorological Institute)
//!
//! This module provides weather data for forecasting solar production and
//! energy consumption patterns.

use anyhow::Result;
use chrono::{DateTime, FixedOffset};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, error, info};

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
    ///
    /// # Robustness: Persistence Forecast Fallback
    ///
    /// CRITICAL FIX #6: A "Smart" home must degrade gracefully when offline.
    /// If the SMHI/Met.no API is unreachable (network glitch, API down),
    /// this function will generate a simple persistence forecast:
    /// - Assumes current weather conditions persist for the next 24h
    /// - Uses typical Swedish winter values as fallback
    ///
    /// This allows heating and charging control loops to continue operating
    /// instead of crashing/halting when the API is unavailable.
    ///
    /// The system should still attempt to fetch real forecasts, but can
    /// fall back to "dumb thermostat" behavior when necessary.
    pub async fn fetch_forecast(&self, location: &GeoLocation) -> Result<WeatherForecast> {
        let url = format!(
            "{}/category/pmp3g/version/2/geotype/point/lon/{:.6}/lat/{:.6}/data.json",
            self.base_url, location.longitude, location.latitude
        );

        debug!("Fetching weather forecast from SMHI: {}", url);

        // Attempt to fetch from API
        let response_result = self
            .client
            .get(&url)
            .send()
            .await;

        match response_result {
            Ok(response) if response.status().is_success() => {
                // CRITICAL FIX #12: Panic on "200 OK" Bad JSON
                // If the API returns HTTP 200 but the schema changed (e.g., field renamed),
                // serde will return an error. We must NOT crash the heating system!
                // Instead, fall back to persistence forecast.

                // Try to parse JSON
                let json_parse_result = response
                    .json::<SmhiResponse>()
                    .await;

                match json_parse_result {
                    Ok(smhi_response) => {
                        // Success: parse real forecast
                        info!(
                            "Successfully fetched weather forecast from SMHI for location ({}, {})",
                            location.latitude, location.longitude
                        );
                        self.parse_forecast(location.clone(), smhi_response)
                    }
                    Err(e) => {
                        // JSON parse error: API schema changed or malformed response
                        error!(
                            "SMHI API returned 200 OK but JSON parsing failed ({}). \
                             This usually means the API schema changed. \
                             Falling back to persistence forecast. \
                             Heating/charging control will continue with conservative assumptions.",
                            e
                        );
                        Ok(self.generate_persistence_forecast(location.clone()))
                    }
                }
            }
            Ok(response) => {
                // API returned error status
                error!("SMHI API returned error status: {}. Falling back to persistence forecast.", response.status());
                Ok(self.generate_persistence_forecast(location.clone()))
            }
            Err(e) => {
                // Network error or timeout
                error!(
                    "Failed to fetch weather from SMHI ({}). Using persistence forecast fallback. \
                     Heating/charging control will continue with conservative assumptions.",
                    e
                );
                Ok(self.generate_persistence_forecast(location.clone()))
            }
        }
    }

    /// Generate a simple persistence forecast when API is unavailable
    ///
    /// Assumes current conditions persist for 24 hours.
    /// Uses typical Swedish winter values as conservative baseline.
    fn generate_persistence_forecast(&self, location: GeoLocation) -> WeatherForecast {
        use chrono::{Utc, Duration as ChronoDuration};

        info!("Generating persistence forecast (API unavailable) for location ({}, {})",
              location.latitude, location.longitude);

        // Conservative Swedish winter assumptions
        // These values are pessimistic to avoid under-heating
        const FALLBACK_TEMP_C: f64 = -5.0;      // Cold but not extreme
        const FALLBACK_CLOUD_COVER: f64 = 75.0;  // Mostly cloudy (low solar)
        const FALLBACK_WIND_MS: f64 = 5.0;       // Moderate wind
        const FALLBACK_HUMIDITY: f64 = 80.0;     // High humidity (Swedish winter)

        let now = Utc::now();
        let mut points = Vec::new();

        // Generate 24 hourly points
        for hour in 0..24 {
            let timestamp = now + ChronoDuration::hours(hour);
            points.push(WeatherPoint {
                timestamp: timestamp.into(),
                temperature_c: FALLBACK_TEMP_C,
                cloud_cover_percent: FALLBACK_CLOUD_COVER,
                wind_speed_ms: FALLBACK_WIND_MS,
                precipitation_mm: 0.0,  // Assume no precipitation
                humidity_percent: FALLBACK_HUMIDITY,
            });
        }

        WeatherForecast {
            location,
            generated_at: now.into(),
            points,
        }
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
