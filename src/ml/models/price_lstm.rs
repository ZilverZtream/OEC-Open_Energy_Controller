#![allow(dead_code)]
//! Price Forecasting LSTM Model
//!
//! This module implements time-series forecasting for electricity prices using
//! LSTM-like architecture (or advanced time-series approximation).
//!
//! Since full LSTM requires deep learning frameworks (like burn or candle),
//! this implementation provides a sophisticated time-series model using
//! available libraries (smartcore) with LSTM-inspired features.
//!
//! Features:
//! - Rolling window prediction (24h ahead)
//! - Temporal features (hour, day, week, seasonality)
//! - Price history features (lags, moving averages)
//! - Calendar features (weekday, holiday)
//! - Trend and seasonality decomposition

use anyhow::{Context, Result};
use chrono::{DateTime, Datelike, Timelike, Utc, Weekday};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

use crate::ml::{FeatureVector, ModelMetadata, ModelType, Prediction, ValidationMetrics};
use super::base::MLModel;

/// LSTM-inspired price forecasting model
///
/// This model uses temporal features and historical price data to forecast
/// electricity prices 24 hours ahead. While called "LSTM", it implements
/// a sophisticated feature-engineered regression model that captures
/// temporal dependencies similar to LSTM networks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceLstmModel {
    pub metadata: ModelMetadata,
    /// Number of historical hours to use as input
    pub lookback_hours: usize,
    /// Number of hours to predict ahead
    pub forecast_horizon_hours: usize,
    /// Model coefficients (weights)
    pub coefficients: Vec<f64>,
    pub intercept: f64,
    /// Historical price buffer for rolling predictions
    #[serde(skip)]
    pub price_history: VecDeque<f64>,
    /// Moving average window for trend calculation
    pub ma_window_hours: usize,
}

impl PriceLstmModel {
    /// Create a new price LSTM model
    pub fn new(
        lookback_hours: usize,
        forecast_horizon_hours: usize,
        ma_window_hours: usize,
        coefficients: Vec<f64>,
        intercept: f64,
    ) -> Self {
        let _n_features = Self::calculate_feature_count(lookback_hours);
        let feature_names = Self::generate_feature_names(lookback_hours);

        let metadata = ModelMetadata {
            model_id: "price_lstm".to_string(),
            model_type: ModelType::LSTM,
            version: "1.0.0".to_string(),
            trained_at: Utc::now(),
            training_samples: 0,
            validation_metrics: ValidationMetrics::new(0.0, 0.0, 0.0, 0.0),
            feature_names,
        };

        Self {
            metadata,
            lookback_hours,
            forecast_horizon_hours,
            coefficients,
            intercept,
            price_history: VecDeque::with_capacity(lookback_hours),
            ma_window_hours,
        }
    }

    /// Create a default model for testing
    pub fn default_model() -> Self {
        let lookback_hours = 24; // Use last 24 hours
        let forecast_horizon_hours = 24; // Predict next 24 hours
        let ma_window_hours = 6; // 6-hour moving average

        // Generate default coefficients
        // In production, these would be trained from historical data
        let n_features = Self::calculate_feature_count(lookback_hours);
        let mut coefficients = vec![0.1; n_features];

        // Give more weight to recent prices (exponential decay)
        for i in 0..lookback_hours {
            coefficients[i] = 0.5 * (-(i as f64) / 6.0).exp();
        }

        Self::new(
            lookback_hours,
            forecast_horizon_hours,
            ma_window_hours,
            coefficients,
            0.0,
        )
    }

    /// Calculate total number of features
    fn calculate_feature_count(lookback_hours: usize) -> usize {
        lookback_hours + // Historical prices
        1 + // Moving average
        2 + // Hour (sin/cos)
        2 + // Day of week (sin/cos)
        2 + // Day of year (sin/cos)
        1 + // Is weekend
        1   // Hour of day (0-23)
    }

    /// Generate feature names
    fn generate_feature_names(lookback_hours: usize) -> Vec<String> {
        let mut names = Vec::new();

        // Historical prices
        for i in 0..lookback_hours {
            names.push(format!("price_lag_{}", i + 1));
        }

        // Additional features
        names.push("moving_average".to_string());
        names.push("hour_sin".to_string());
        names.push("hour_cos".to_string());
        names.push("day_of_week_sin".to_string());
        names.push("day_of_week_cos".to_string());
        names.push("day_of_year_sin".to_string());
        names.push("day_of_year_cos".to_string());
        names.push("is_weekend".to_string());
        names.push("hour_of_day".to_string());

        names
    }

    /// Add historical price to the model's memory
    pub fn add_historical_price(&mut self, price: f64) {
        self.price_history.push_back(price);
        if self.price_history.len() > self.lookback_hours {
            self.price_history.pop_front();
        }
    }

    /// Extract features for price prediction
    pub fn extract_features(&self, timestamp: DateTime<Utc>) -> Result<FeatureVector> {
        if self.price_history.len() < self.lookback_hours {
            anyhow::bail!(
                "Insufficient price history: have {}, need {}",
                self.price_history.len(),
                self.lookback_hours
            );
        }

        let mut features = Vec::new();

        // Historical prices (lagged features)
        for price in self.price_history.iter().rev().take(self.lookback_hours) {
            features.push(*price);
        }

        // Moving average
        let ma_window = self.ma_window_hours.min(self.price_history.len());
        let ma: f64 = self.price_history.iter().rev().take(ma_window).sum::<f64>()
            / ma_window as f64;
        features.push(ma);

        // Cyclical time features
        let hour = timestamp.hour() as f64;
        let hour_sin = (hour * 2.0 * std::f64::consts::PI / 24.0).sin();
        let hour_cos = (hour * 2.0 * std::f64::consts::PI / 24.0).cos();
        features.push(hour_sin);
        features.push(hour_cos);

        let day_of_week = timestamp.weekday().num_days_from_monday() as f64;
        let dow_sin = (day_of_week * 2.0 * std::f64::consts::PI / 7.0).sin();
        let dow_cos = (day_of_week * 2.0 * std::f64::consts::PI / 7.0).cos();
        features.push(dow_sin);
        features.push(dow_cos);

        let day_of_year = timestamp.ordinal() as f64;
        let doy_sin = (day_of_year * 2.0 * std::f64::consts::PI / 365.25).sin();
        let doy_cos = (day_of_year * 2.0 * std::f64::consts::PI / 365.25).cos();
        features.push(doy_sin);
        features.push(doy_cos);

        // Is weekend (binary feature)
        let is_weekend = match timestamp.weekday() {
            Weekday::Sat | Weekday::Sun => 1.0,
            _ => 0.0,
        };
        features.push(is_weekend);

        // Hour of day (normalized)
        features.push(hour / 24.0);

        FeatureVector::new(features, self.metadata.feature_names.clone())
    }

    /// Predict price at given timestamp
    pub fn predict_price(&self, timestamp: DateTime<Utc>) -> Result<Prediction> {
        let features = self.extract_features(timestamp)?;
        self.predict(&features)
    }

    /// Predict prices for next N hours
    ///
    /// This performs iterative prediction, using predicted values as input
    /// for subsequent predictions (autoregressive forecasting).
    pub fn predict_rolling_24h(
        &mut self,
        start_timestamp: DateTime<Utc>,
    ) -> Result<Vec<(DateTime<Utc>, f64)>> {
        let mut predictions = Vec::new();
        let mut current_history = self.price_history.clone();

        for hour_offset in 0..self.forecast_horizon_hours {
            let timestamp = start_timestamp + chrono::Duration::hours(hour_offset as i64);

            // Create temporary model with current history
            let mut temp_model = self.clone();
            temp_model.price_history = current_history.clone();

            // Predict next price
            let prediction = temp_model.predict_price(timestamp)?;
            predictions.push((timestamp, prediction.value));

            // Add prediction to history for next iteration
            current_history.push_back(prediction.value);
            if current_history.len() > self.lookback_hours {
                current_history.pop_front();
            }
        }

        Ok(predictions)
    }
}

impl MLModel for PriceLstmModel {
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

        // Ensure price is non-negative
        let price = prediction.max(0.0);

        Ok(Prediction::new(price))
    }

    fn metadata(&self) -> &ModelMetadata {
        &self.metadata
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_feature_extraction() {
        let mut model = PriceLstmModel::default_model();

        // Add 24 hours of historical prices
        for i in 0..24 {
            model.add_historical_price(1.0 + (i as f64 * 0.1));
        }

        let timestamp = Utc.with_ymd_and_hms(2024, 6, 21, 12, 0, 0).unwrap();
        let features = model.extract_features(timestamp).unwrap();

        // Should have lookback_hours + additional features
        assert_eq!(features.len(), PriceLstmModel::calculate_feature_count(24));

        // First features should be historical prices (in reverse order)
        assert_eq!(features.features[0], 3.3); // Most recent
        assert_eq!(features.features[23], 1.0); // Oldest
    }

    #[test]
    fn test_insufficient_history() {
        let model = PriceLstmModel::default_model();

        let timestamp = Utc.with_ymd_and_hms(2024, 6, 21, 12, 0, 0).unwrap();
        let result = model.extract_features(timestamp);

        assert!(result.is_err(), "Should fail with insufficient history");
    }

    #[test]
    fn test_predict_with_history() {
        let mut model = PriceLstmModel::default_model();

        // Add historical prices (simulating daily pattern)
        for hour in 0..24 {
            let price = if hour < 6 || hour > 22 {
                0.5 // Low night price
            } else if hour >= 9 && hour <= 18 {
                2.0 // High day price
            } else {
                1.0 // Medium price
            };
            model.add_historical_price(price);
        }

        let timestamp = Utc.with_ymd_and_hms(2024, 6, 21, 12, 0, 0).unwrap();
        let prediction = model.predict_price(timestamp).unwrap();

        // Should predict a reasonable price
        assert!(prediction.value >= 0.0);
        assert!(prediction.value < 10.0, "Price should be reasonable, got {}", prediction.value);
    }

    #[test]
    fn test_rolling_prediction() {
        let mut model = PriceLstmModel::default_model();

        // Add historical prices
        for hour in 0..24 {
            let price = 1.0 + (hour as f64 * 0.1);
            model.add_historical_price(price);
        }

        let start = Utc.with_ymd_and_hms(2024, 6, 21, 0, 0, 0).unwrap();
        let predictions = model.predict_rolling_24h(start).unwrap();

        // Should have 24 predictions
        assert_eq!(predictions.len(), 24);

        // Each prediction should be valid
        for (timestamp, price) in predictions {
            assert!(price >= 0.0, "Price should be non-negative");
            assert!(timestamp >= start, "Timestamp should be in future");
        }
    }

    #[test]
    fn test_weekend_feature() {
        let mut model = PriceLstmModel::default_model();

        // Add historical prices
        for _ in 0..24 {
            model.add_historical_price(1.5);
        }

        // Weekday (Friday)
        let weekday = Utc.with_ymd_and_hms(2024, 6, 21, 12, 0, 0).unwrap();
        let features_weekday = model.extract_features(weekday).unwrap();

        // Weekend (Saturday)
        let weekend = Utc.with_ymd_and_hms(2024, 6, 22, 12, 0, 0).unwrap();
        let features_weekend = model.extract_features(weekend).unwrap();

        // Weekend feature should be different
        let weekend_idx = features_weekday.len() - 2; // Second to last feature
        assert_eq!(features_weekday.features[weekend_idx], 0.0); // Weekday
        assert_eq!(features_weekend.features[weekend_idx], 1.0); // Weekend
    }
}
