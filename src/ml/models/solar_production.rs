#![allow(dead_code)]
//! Solar Production Forecasting Model
//!
//! This module implements domain-specific solar production forecasting using
//! physics-based features combined with machine learning.
//!
//! Key features:
//! - Clear Sky Radiation calculation (theoretical max solar power)
//! - Solar angle calculations (elevation, azimuth)
//! - Weather features (cloud cover, temperature)
//! - Time-based features (hour, day of year)
//!
//! The model uses Clear Sky Radiation as a baseline and applies ML to learn
//! how weather conditions affect actual production.

use anyhow::{Context, Result};
use chrono::{DateTime, Datelike, Timelike, Utc};
use serde::{Deserialize, Serialize};

use crate::ml::{FeatureVector, ModelMetadata, ModelType, Prediction, ValidationMetrics};
use super::base::MLModel;

/// Solar production model with physics-based features
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolarProductionModel {
    pub metadata: ModelMetadata,
    /// Location latitude (degrees, positive = North)
    pub latitude: f64,
    /// Location longitude (degrees, positive = East)
    pub longitude: f64,
    /// Solar panel capacity in kW (peak power)
    pub panel_capacity_kw: f64,
    /// Panel tilt angle in degrees (0 = horizontal, 90 = vertical)
    pub panel_tilt_deg: f64,
    /// Panel azimuth in degrees (0 = North, 90 = East, 180 = South, 270 = West)
    pub panel_azimuth_deg: f64,
    /// ML model coefficients for weather adjustment
    pub coefficients: Vec<f64>,
    pub intercept: f64,
}

impl SolarProductionModel {
    /// Create a new solar production model
    pub fn new(
        latitude: f64,
        longitude: f64,
        panel_capacity_kw: f64,
        panel_tilt_deg: f64,
        panel_azimuth_deg: f64,
        coefficients: Vec<f64>,
        intercept: f64,
    ) -> Self {
        let metadata = ModelMetadata {
            model_id: "solar_production".to_string(),
            model_type: ModelType::LinearRegression,
            version: "1.0.0".to_string(),
            trained_at: Utc::now(),
            training_samples: 0,
            validation_metrics: ValidationMetrics::new(0.0, 0.0, 0.0, 0.0),
            feature_names: vec![
                "clear_sky_radiation_w_per_m2".to_string(),
                "cloud_cover_percent".to_string(),
                "temperature_c".to_string(),
                "hour_sin".to_string(),
                "hour_cos".to_string(),
                "day_of_year_sin".to_string(),
                "day_of_year_cos".to_string(),
            ],
        };

        Self {
            metadata,
            latitude,
            longitude,
            panel_capacity_kw,
            panel_tilt_deg,
            panel_azimuth_deg,
            coefficients,
            intercept,
        }
    }

    /// Create a default model for testing (simplified physics model)
    pub fn default_model(latitude: f64, longitude: f64, panel_capacity_kw: f64) -> Self {
        // Simplified coefficients that primarily follow clear sky radiation
        // In production, these would be trained from historical data
        let coefficients = vec![
            0.15,   // clear_sky_radiation (scaled by panel efficiency ~15%)
            -0.005, // cloud_cover (negative impact)
            0.001,  // temperature (small positive impact)
            0.05,   // hour_sin (time-of-day variation)
            0.05,   // hour_cos
            0.02,   // day_of_year_sin (seasonal variation)
            0.02,   // day_of_year_cos
        ];

        Self::new(
            latitude,
            longitude,
            panel_capacity_kw,
            30.0,  // Default tilt: 30 degrees (good for mid-latitudes)
            180.0, // Default azimuth: South-facing
            coefficients,
            0.0,
        )
    }

    /// Extract features for solar production prediction
    pub fn extract_features(
        &self,
        timestamp: DateTime<Utc>,
        cloud_cover_percent: f64,
        temperature_c: f64,
    ) -> Result<FeatureVector> {
        // Calculate clear sky radiation
        let clear_sky = self.calculate_clear_sky_radiation(timestamp)?;

        // Time-based features (cyclical encoding)
        let hour = timestamp.hour() as f64;
        let hour_sin = (hour * std::f64::consts::PI / 12.0).sin();
        let hour_cos = (hour * std::f64::consts::PI / 12.0).cos();

        let day_of_year = timestamp.ordinal() as f64;
        let day_sin = (day_of_year * 2.0 * std::f64::consts::PI / 365.25).sin();
        let day_cos = (day_of_year * 2.0 * std::f64::consts::PI / 365.25).cos();

        let features = vec![
            clear_sky,
            cloud_cover_percent,
            temperature_c,
            hour_sin,
            hour_cos,
            day_sin,
            day_cos,
        ];

        FeatureVector::new(features, self.metadata.feature_names.clone())
    }

    /// Calculate clear sky radiation at given time and location
    ///
    /// Uses simplified solar position algorithm and clear sky model.
    /// Returns irradiance in W/m²
    pub fn calculate_clear_sky_radiation(&self, timestamp: DateTime<Utc>) -> Result<f64> {
        // Calculate solar position
        let (elevation, _azimuth) = self.calculate_solar_position(timestamp)?;

        // If sun is below horizon, no radiation
        if elevation <= 0.0 {
            return Ok(0.0);
        }

        // Simplified clear sky model
        // Clear sky irradiance follows: I = I0 * sin(elevation) * atmospheric_transmission
        const SOLAR_CONSTANT: f64 = 1367.0; // W/m² at top of atmosphere
        const ATMOSPHERIC_TRANSMISSION: f64 = 0.7; // Typical clear sky transmission

        // Calculate direct normal irradiance
        let dni = SOLAR_CONSTANT * ATMOSPHERIC_TRANSMISSION
            * elevation.to_radians().sin().powf(0.678);

        // Calculate irradiance on tilted panel
        let panel_irradiance = self.calculate_panel_irradiance(dni, elevation)?;

        Ok(panel_irradiance)
    }

    /// Calculate solar elevation and azimuth angles
    ///
    /// Uses simplified solar position algorithm.
    /// Returns (elevation_deg, azimuth_deg)
    fn calculate_solar_position(&self, timestamp: DateTime<Utc>) -> Result<(f64, f64)> {
        // Get day of year and hour
        let day_of_year = timestamp.ordinal() as f64;
        let hour = timestamp.hour() as f64 + timestamp.minute() as f64 / 60.0;

        // Calculate solar declination (angle of sun relative to equator)
        let declination_rad = (23.45_f64.to_radians())
            * ((360.0 / 365.25) * (day_of_year + 284.0)).to_radians().sin();

        // Calculate hour angle (sun's position in sky relative to solar noon)
        // Solar noon is approximately at 12:00 local time
        let local_hour = hour + self.longitude / 15.0; // Rough longitude correction
        let hour_angle_rad = ((local_hour - 12.0) * 15.0).to_radians();

        // Calculate solar elevation (altitude) angle
        let lat_rad = self.latitude.to_radians();
        let sin_elevation = lat_rad.sin() * declination_rad.sin()
            + lat_rad.cos() * declination_rad.cos() * hour_angle_rad.cos();
        let elevation_rad = sin_elevation.asin();
        let elevation_deg = elevation_rad.to_degrees();

        // Calculate solar azimuth angle
        let cos_azimuth = (declination_rad.sin() - lat_rad.sin() * sin_elevation)
            / (lat_rad.cos() * elevation_rad.cos());
        let azimuth_rad = cos_azimuth.acos();
        let mut azimuth_deg = azimuth_rad.to_degrees();

        // Adjust azimuth based on time of day (morning vs afternoon)
        if hour_angle_rad > 0.0 {
            azimuth_deg = 360.0 - azimuth_deg;
        }

        Ok((elevation_deg, azimuth_deg))
    }

    /// Calculate irradiance on tilted panel surface
    fn calculate_panel_irradiance(&self, dni: f64, solar_elevation: f64) -> Result<f64> {
        // Angle of incidence (angle between sun rays and panel normal)
        // Simplified calculation assuming panel faces due south
        let tilt_rad = self.panel_tilt_deg.to_radians();
        let elevation_rad = solar_elevation.to_radians();

        // For south-facing panel, incidence angle depends on tilt and solar elevation
        // This is simplified; full calculation would include azimuth
        let cos_incidence = (elevation_rad.sin() * tilt_rad.cos()
            + elevation_rad.cos() * tilt_rad.sin())
            .max(0.0);

        // Irradiance on panel surface
        let panel_irradiance = dni * cos_incidence;

        Ok(panel_irradiance.max(0.0))
    }

    /// Predict solar production in kW
    pub fn predict_production(
        &self,
        timestamp: DateTime<Utc>,
        cloud_cover_percent: f64,
        temperature_c: f64,
    ) -> Result<Prediction> {
        let features = self.extract_features(timestamp, cloud_cover_percent, temperature_c)?;

        // Apply linear regression
        let base_prediction: f64 = features
            .features
            .iter()
            .zip(self.coefficients.iter())
            .map(|(f, c)| f * c)
            .sum::<f64>()
            + self.intercept;

        // Scale by panel capacity
        let production_kw = (base_prediction * self.panel_capacity_kw).max(0.0);

        Ok(Prediction::new(production_kw))
    }
}

impl MLModel for SolarProductionModel {
    fn predict(&self, features: &FeatureVector) -> Result<Prediction> {
        if features.len() != self.coefficients.len() {
            anyhow::bail!(
                "Feature count mismatch: expected {}, got {}",
                self.coefficients.len(),
                features.len()
            );
        }

        let prediction: f64 = features
            .features
            .iter()
            .zip(self.coefficients.iter())
            .map(|(f, c)| f * c)
            .sum::<f64>()
            + self.intercept;

        // Scale by panel capacity and ensure non-negative
        let production_kw = (prediction * self.panel_capacity_kw).max(0.0);

        Ok(Prediction::new(production_kw))
    }

    fn metadata(&self) -> &ModelMetadata {
        &self.metadata
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    #[test]
    fn test_solar_position_noon() {
        let model = SolarProductionModel::default_model(59.33, 18.06, 10.0); // Stockholm

        // Summer solstice at noon (approximately highest elevation)
        let summer_noon = Utc.with_ymd_and_hms(2024, 6, 21, 12, 0, 0).unwrap();
        let (elevation, _azimuth) = model.calculate_solar_position(summer_noon).unwrap();

        // At summer solstice in Stockholm, sun elevation at noon should be high
        assert!(elevation > 50.0, "Summer noon elevation should be > 50°, got {}", elevation);

        // Winter solstice at noon (lowest elevation)
        let winter_noon = Utc.with_ymd_and_hms(2024, 12, 21, 12, 0, 0).unwrap();
        let (elevation, _azimuth) = model.calculate_solar_position(winter_noon).unwrap();

        // At winter solstice, sun should be much lower
        assert!(elevation < 15.0, "Winter noon elevation should be < 15°, got {}", elevation);
    }

    #[test]
    fn test_clear_sky_radiation_day_night() {
        let model = SolarProductionModel::default_model(59.33, 18.06, 10.0);

        // Daytime (noon)
        let noon = Utc.with_ymd_and_hms(2024, 6, 21, 12, 0, 0).unwrap();
        let radiation_noon = model.calculate_clear_sky_radiation(noon).unwrap();
        assert!(radiation_noon > 500.0, "Noon radiation should be significant, got {}", radiation_noon);

        // Nighttime
        let midnight = Utc.with_ymd_and_hms(2024, 6, 21, 0, 0, 0).unwrap();
        let radiation_night = model.calculate_clear_sky_radiation(midnight).unwrap();
        assert_eq!(radiation_night, 0.0, "Night radiation should be zero");
    }

    #[test]
    fn test_feature_extraction() {
        let model = SolarProductionModel::default_model(59.33, 18.06, 10.0);

        let timestamp = Utc.with_ymd_and_hms(2024, 6, 21, 12, 0, 0).unwrap();
        let features = model.extract_features(timestamp, 20.0, 25.0).unwrap();

        assert_eq!(features.len(), 7);
        assert!(features.features[0] > 0.0, "Clear sky radiation should be positive");
        assert_eq!(features.features[1], 20.0, "Cloud cover should match input");
        assert_eq!(features.features[2], 25.0, "Temperature should match input");
    }

    #[test]
    fn test_predict_production_clear_day() {
        let model = SolarProductionModel::default_model(59.33, 18.06, 10.0);

        // Clear summer day at noon
        let timestamp = Utc.with_ymd_and_hms(2024, 6, 21, 12, 0, 0).unwrap();
        let prediction = model.predict_production(timestamp, 0.0, 25.0).unwrap();

        // With 10kW panel capacity, clear sky at noon, should get significant production
        assert!(prediction.value > 5.0, "Clear day production should be > 5kW, got {}", prediction.value);
        assert!(prediction.value <= 10.0, "Production can't exceed panel capacity");
    }

    #[test]
    fn test_predict_production_cloudy_day() {
        let model = SolarProductionModel::default_model(59.33, 18.06, 10.0);

        let timestamp = Utc.with_ymd_and_hms(2024, 6, 21, 12, 0, 0).unwrap();

        // Clear day
        let clear_prediction = model.predict_production(timestamp, 0.0, 25.0).unwrap();

        // Cloudy day (80% cloud cover)
        let cloudy_prediction = model.predict_production(timestamp, 80.0, 25.0).unwrap();

        // Cloudy should produce less than clear
        assert!(cloudy_prediction.value < clear_prediction.value,
            "Cloudy day should produce less than clear day");
    }

    #[test]
    fn test_predict_production_night() {
        let model = SolarProductionModel::default_model(59.33, 18.06, 10.0);

        // Nighttime
        let timestamp = Utc.with_ymd_and_hms(2024, 6, 21, 0, 0, 0).unwrap();
        let prediction = model.predict_production(timestamp, 0.0, 15.0).unwrap();

        // Night should produce zero or near-zero
        assert!(prediction.value < 0.1, "Night production should be ~0, got {}", prediction.value);
    }
}
