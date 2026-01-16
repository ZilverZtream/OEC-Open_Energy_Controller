//! Feature engineering for forecasting models
//!
//! This module extracts features from time series data for use in ML models

use chrono::{DateTime, Datelike, FixedOffset, Timelike};
use serde::{Deserialize, Serialize};

/// Feature vector for consumption/production forecasting
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "swagger", derive(utoipa::ToSchema))]
pub struct TimeSeriesFeatures {
    /// Hour of day (0-23)
    pub hour_of_day: u32,
    /// Day of week (0=Monday, 6=Sunday)
    pub day_of_week: u32,
    /// Day of month (1-31)
    pub day_of_month: u32,
    /// Month (1-12)
    pub month: u32,
    /// Is weekend (Saturday or Sunday)
    pub is_weekend: bool,
    /// Is holiday (Swedish public holidays)
    pub is_holiday: bool,
    /// Temperature (Celsius)
    pub temperature_c: Option<f64>,
    /// Cloud cover (0-100%)
    pub cloud_cover_percent: Option<f64>,
    /// Wind speed (m/s)
    pub wind_speed_ms: Option<f64>,
    /// Season (0=winter, 1=spring, 2=summer, 3=autumn)
    pub season: u32,
    /// Day length (hours)
    pub day_length_hours: f64,
}

/// Feature extractor for time series data
pub struct FeatureExtractor {
    latitude: f64,
    longitude: f64,
}

impl FeatureExtractor {
    /// Create a new feature extractor for a location
    pub fn new(latitude: f64, longitude: f64) -> Self {
        Self {
            latitude,
            longitude,
        }
    }

    /// Extract features from a timestamp
    pub fn extract_temporal_features(
        &self,
        timestamp: DateTime<FixedOffset>,
    ) -> TimeSeriesFeatures {
        let hour = timestamp.hour();
        let day_of_week = timestamp.weekday().num_days_from_monday();
        let day_of_month = timestamp.day();
        let month = timestamp.month();
        let is_weekend = day_of_week >= 5; // Saturday (5) or Sunday (6)
        let is_holiday = self.is_swedish_holiday(&timestamp);
        let season = self.get_season(month);
        let day_length = self.calculate_day_length(timestamp.ordinal0(), self.latitude);

        TimeSeriesFeatures {
            hour_of_day: hour,
            day_of_week,
            day_of_month,
            month,
            is_weekend,
            is_holiday,
            temperature_c: None,
            cloud_cover_percent: None,
            wind_speed_ms: None,
            season,
            day_length_hours: day_length,
        }
    }

    /// Add weather features to existing feature vector
    pub fn add_weather_features(
        &self,
        mut features: TimeSeriesFeatures,
        temperature_c: f64,
        cloud_cover_percent: f64,
        wind_speed_ms: f64,
    ) -> TimeSeriesFeatures {
        features.temperature_c = Some(temperature_c);
        features.cloud_cover_percent = Some(cloud_cover_percent);
        features.wind_speed_ms = Some(wind_speed_ms);
        features
    }

    /// Check if a date is a Swedish public holiday
    fn is_swedish_holiday(&self, timestamp: &DateTime<FixedOffset>) -> bool {
        let month = timestamp.month();
        let day = timestamp.day();

        // Fixed holidays
        matches!(
            (month, day),
            (1, 1)   // New Year's Day
            | (1, 6)   // Epiphany
            | (5, 1)   // Labour Day
            | (6, 6)   // National Day
            | (12, 24) // Christmas Eve
            | (12, 25) // Christmas Day
            | (12, 26) // Boxing Day
            | (12, 31) // New Year's Eve
        )
        // Note: Easter, Midsummer, etc. are movable and would require more complex logic
    }

    /// Get season from month (0=winter, 1=spring, 2=summer, 3=autumn)
    fn get_season(&self, month: u32) -> u32 {
        match month {
            12 | 1 | 2 => 0,  // Winter
            3 | 4 | 5 => 1,   // Spring
            6 | 7 | 8 => 2,   // Summer
            9 | 10 | 11 => 3, // Autumn
            _ => 0,
        }
    }

    /// Calculate day length in hours for a given day of year and latitude
    fn calculate_day_length(&self, day_of_year: u32, latitude: f64) -> f64 {
        let lat_rad = latitude.to_radians();
        let axis_tilt = 23.44_f64.to_radians();
        let day_angle = 2.0 * std::f64::consts::PI * (day_of_year as f64 - 81.0) / 365.0;
        let declination = axis_tilt * day_angle.sin();

        let hour_angle = (-lat_rad.tan() * declination.tan()).acos();
        let day_length = 2.0 * hour_angle.to_degrees() / 15.0;

        day_length.clamp(0.0, 24.0)
    }
}

/// Normalize features for ML models
pub fn normalize_features(features: &TimeSeriesFeatures) -> Vec<f64> {
    vec![
        features.hour_of_day as f64 / 24.0,
        features.day_of_week as f64 / 7.0,
        features.day_of_month as f64 / 31.0,
        features.month as f64 / 12.0,
        if features.is_weekend { 1.0 } else { 0.0 },
        if features.is_holiday { 1.0 } else { 0.0 },
        features.temperature_c.unwrap_or(15.0) / 40.0, // Normalize to -20 to +40
        features.cloud_cover_percent.unwrap_or(50.0) / 100.0,
        features.wind_speed_ms.unwrap_or(5.0) / 30.0,
        features.season as f64 / 4.0,
        features.day_length_hours / 24.0,
    ]
}

/// Create lag features for time series (previous values)
pub fn create_lag_features(
    values: &[f64],
    num_lags: usize,
) -> Vec<Vec<f64>> {
    let mut lag_features = Vec::new();

    for i in num_lags..values.len() {
        let mut lags = Vec::new();
        for lag in 1..=num_lags {
            lags.push(values[i - lag]);
        }
        lag_features.push(lags);
    }

    lag_features
}

/// Calculate rolling statistics (mean, std, min, max)
pub fn rolling_statistics(
    values: &[f64],
    window_size: usize,
) -> Vec<(f64, f64, f64, f64)> {
    let mut stats = Vec::new();

    for i in window_size..=values.len() {
        let window = &values[i - window_size..i];
        let mean = window.iter().sum::<f64>() / window.len() as f64;
        let variance = window.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / window.len() as f64;
        let std = variance.sqrt();
        let min = window.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max = window.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));

        stats.push((mean, std, min, max));
    }

    stats
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_extract_temporal_features() {
        let extractor = FeatureExtractor::new(59.3293, 18.0686); // Stockholm
        let timestamp = Utc::now().into();
        let features = extractor.extract_temporal_features(timestamp);

        assert!(features.hour_of_day < 24);
        assert!(features.day_of_week < 7);
        assert!(features.month >= 1 && features.month <= 12);
        assert!(features.season < 4);
    }

    #[test]
    fn test_season_calculation() {
        let extractor = FeatureExtractor::new(59.3293, 18.0686);
        assert_eq!(extractor.get_season(1), 0); // January = Winter
        assert_eq!(extractor.get_season(4), 1); // April = Spring
        assert_eq!(extractor.get_season(7), 2); // July = Summer
        assert_eq!(extractor.get_season(10), 3); // October = Autumn
    }

    #[test]
    fn test_normalize_features() {
        let features = TimeSeriesFeatures {
            hour_of_day: 12,
            day_of_week: 3,
            day_of_month: 15,
            month: 6,
            is_weekend: false,
            is_holiday: false,
            temperature_c: Some(20.0),
            cloud_cover_percent: Some(50.0),
            wind_speed_ms: Some(5.0),
            season: 2,
            day_length_hours: 16.0,
        };

        let normalized = normalize_features(&features);
        assert_eq!(normalized.len(), 11);
        // All values should be between 0 and 1
        for value in normalized {
            assert!(value >= 0.0 && value <= 1.0);
        }
    }

    #[test]
    fn test_create_lag_features() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let lag_features = create_lag_features(&values, 2);

        assert_eq!(lag_features.len(), 3);
        assert_eq!(lag_features[0], vec![2.0, 1.0]); // For value 3.0
        assert_eq!(lag_features[1], vec![3.0, 2.0]); // For value 4.0
        assert_eq!(lag_features[2], vec![4.0, 3.0]); // For value 5.0
    }

    #[test]
    fn test_rolling_statistics() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let stats = rolling_statistics(&values, 3);

        assert_eq!(stats.len(), 3);
        let (mean, _, min, max) = stats[0];
        assert_eq!(mean, 2.0); // Mean of [1.0, 2.0, 3.0]
        assert_eq!(min, 1.0);
        assert_eq!(max, 3.0);
    }

    #[test]
    fn test_day_length_calculation() {
        let extractor = FeatureExtractor::new(59.3293, 18.0686); // Stockholm

        // Summer solstice (around day 172)
        let summer_day_length = extractor.calculate_day_length(172, 59.3293);
        assert!(summer_day_length > 18.0); // Long days in summer

        // Winter solstice (around day 355)
        let winter_day_length = extractor.calculate_day_length(355, 59.3293);
        assert!(winter_day_length < 7.0); // Short days in winter
    }
}
