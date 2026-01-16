//! Forecast Metrics and Evaluation
//!
//! This module provides comprehensive metrics for evaluating forecast quality,
//! including MAE, RMSE, MAPE, and prediction intervals.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Forecast accuracy metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForecastMetrics {
    /// Mean Absolute Error
    pub mae: f64,
    /// Root Mean Square Error
    pub rmse: f64,
    /// Mean Absolute Percentage Error (%)
    pub mape: f64,
    /// R² (coefficient of determination)
    pub r2: f64,
    /// Number of samples evaluated
    pub sample_count: usize,
    /// Maximum error observed
    pub max_error: f64,
    /// Minimum error observed
    pub min_error: f64,
    /// Standard deviation of errors
    pub std_dev: f64,
}

impl ForecastMetrics {
    /// Calculate metrics from actual and predicted values
    pub fn calculate(actual: &[f64], predicted: &[f64]) -> Result<Self, ForecastMetricsError> {
        if actual.len() != predicted.len() {
            return Err(ForecastMetricsError::DimensionMismatch {
                actual: actual.len(),
                predicted: predicted.len(),
            });
        }

        if actual.is_empty() {
            return Err(ForecastMetricsError::EmptyData);
        }

        let n = actual.len();
        let mut errors = Vec::with_capacity(n);
        let mut squared_errors = Vec::with_capacity(n);
        let mut percentage_errors = Vec::with_capacity(n);

        // Calculate errors
        for (a, p) in actual.iter().zip(predicted.iter()) {
            let error = a - p;
            errors.push(error);
            squared_errors.push(error * error);

            // Percentage error (avoid division by zero)
            if a.abs() > 1e-6 {
                let pct_error = (error.abs() / a.abs()) * 100.0;
                percentage_errors.push(pct_error);
            }
        }

        // MAE: Mean Absolute Error
        let mae = errors.iter().map(|e| e.abs()).sum::<f64>() / n as f64;

        // RMSE: Root Mean Square Error
        let mse = squared_errors.iter().sum::<f64>() / n as f64;
        let rmse = mse.sqrt();

        // MAPE: Mean Absolute Percentage Error
        let mape = if percentage_errors.is_empty() {
            0.0
        } else {
            percentage_errors.iter().sum::<f64>() / percentage_errors.len() as f64
        };

        // R²: Coefficient of determination
        let mean_actual = actual.iter().sum::<f64>() / n as f64;
        let total_variance: f64 = actual.iter().map(|a| (a - mean_actual).powi(2)).sum();
        let residual_variance: f64 = squared_errors.iter().sum();

        let r2 = if total_variance > 1e-10 {
            1.0 - (residual_variance / total_variance)
        } else {
            0.0 // Perfect variance = perfect fit
        };

        // Max and min errors
        let max_error = errors.iter().map(|e| e.abs()).fold(0.0f64, |a, b| a.max(b));
        let min_error = errors
            .iter()
            .map(|e| e.abs())
            .fold(f64::INFINITY, |a, b| a.min(b));

        // Standard deviation of errors
        let mean_error = errors.iter().sum::<f64>() / n as f64;
        let variance = errors.iter().map(|e| (e - mean_error).powi(2)).sum::<f64>() / n as f64;
        let std_dev = variance.sqrt();

        Ok(ForecastMetrics {
            mae,
            rmse,
            mape,
            r2,
            sample_count: n,
            max_error,
            min_error,
            std_dev,
        })
    }

    /// Assess forecast quality based on MAPE
    pub fn quality(&self) -> ForecastQuality {
        if self.mape < 5.0 {
            ForecastQuality::Excellent
        } else if self.mape < 10.0 {
            ForecastQuality::Good
        } else if self.mape < 20.0 {
            ForecastQuality::Fair
        } else if self.mape < 50.0 {
            ForecastQuality::Poor
        } else {
            ForecastQuality::VeryPoor
        }
    }

    /// Check if metrics indicate a reliable forecast
    pub fn is_reliable(&self) -> bool {
        // Criteria for reliability:
        // 1. MAPE < 20%
        // 2. R² > 0.5
        // 3. At least 24 samples (1 day)
        self.mape < 20.0 && self.r2 > 0.5 && self.sample_count >= 24
    }
}

impl fmt::Display for ForecastMetrics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Metrics: MAE={:.3}, RMSE={:.3}, MAPE={:.2}%, R²={:.3}, Quality={:?}",
            self.mae,
            self.rmse,
            self.mape,
            self.r2,
            self.quality()
        )
    }
}

/// Forecast quality classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ForecastQuality {
    Excellent,  // MAPE < 5%
    Good,       // MAPE 5-10%
    Fair,       // MAPE 10-20%
    Poor,       // MAPE 20-50%
    VeryPoor,   // MAPE > 50%
}

/// Forecast metrics calculation errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum ForecastMetricsError {
    #[error("Dimension mismatch: actual={actual}, predicted={predicted}")]
    DimensionMismatch { actual: usize, predicted: usize },

    #[error("Empty data provided")]
    EmptyData,
}

/// Prediction interval calculator
pub struct PredictionInterval {
    /// Confidence level (e.g., 0.95 for 95% confidence)
    confidence: f64,
}

impl PredictionInterval {
    /// Create a new prediction interval calculator
    pub fn new(confidence: f64) -> Self {
        assert!(
            (0.0..=1.0).contains(&confidence),
            "Confidence must be between 0 and 1"
        );
        Self { confidence }
    }

    /// Calculate prediction interval bounds
    ///
    /// Returns (lower_bound, upper_bound) for the prediction interval
    /// based on historical errors.
    pub fn calculate_bounds(
        &self,
        prediction: f64,
        historical_errors: &[f64],
    ) -> Result<(f64, f64), ForecastMetricsError> {
        if historical_errors.is_empty() {
            return Err(ForecastMetricsError::EmptyData);
        }

        // Calculate standard deviation of errors
        let mean_error = historical_errors.iter().sum::<f64>() / historical_errors.len() as f64;
        let variance = historical_errors
            .iter()
            .map(|e| (e - mean_error).powi(2))
            .sum::<f64>()
            / historical_errors.len() as f64;
        let std_dev = variance.sqrt();

        // Z-score for confidence level (simplified - assumes normal distribution)
        // For 95% confidence: z ≈ 1.96
        // For 90% confidence: z ≈ 1.645
        // For 99% confidence: z ≈ 2.576
        let z_score = match self.confidence {
            c if c >= 0.99 => 2.576,
            c if c >= 0.95 => 1.96,
            c if c >= 0.90 => 1.645,
            c if c >= 0.80 => 1.282,
            _ => 1.0, // Default to ~68% confidence
        };

        let margin = z_score * std_dev;
        let lower = prediction - margin;
        let upper = prediction + margin;

        Ok((lower.max(0.0), upper)) // Ensure non-negative for power/energy values
    }
}

/// Time-series cross-validation for forecast evaluation
pub struct TimeSeriesCrossValidation {
    /// Number of folds
    n_folds: usize,
    /// Minimum training size
    min_train_size: usize,
}

impl TimeSeriesCrossValidation {
    /// Create a new time-series cross-validator
    pub fn new(n_folds: usize, min_train_size: usize) -> Self {
        Self {
            n_folds,
            min_train_size,
        }
    }

    /// Split data into train/test folds
    ///
    /// Returns Vec of (train_indices, test_indices) tuples
    pub fn split(&self, data_len: usize) -> Vec<(Vec<usize>, Vec<usize>)> {
        let mut folds = Vec::new();

        if data_len < self.min_train_size + self.n_folds {
            // Not enough data for cross-validation
            return folds;
        }

        let test_size = (data_len - self.min_train_size) / self.n_folds;

        for fold in 0..self.n_folds {
            let train_end = self.min_train_size + (fold * test_size);
            let test_end = (train_end + test_size).min(data_len);

            let train_indices: Vec<usize> = (0..train_end).collect();
            let test_indices: Vec<usize> = (train_end..test_end).collect();

            if !test_indices.is_empty() {
                folds.push((train_indices, test_indices));
            }
        }

        folds
    }
}

/// Forecast performance tracker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForecastPerformanceTracker {
    /// Forecaster name/identifier
    pub forecaster: String,
    /// Metric type (price, consumption, production)
    pub metric_type: String,
    /// Historical metrics (most recent first)
    pub historical_metrics: Vec<(chrono::DateTime<chrono::Utc>, ForecastMetrics)>,
    /// Maximum history to keep
    pub max_history: usize,
}

impl ForecastPerformanceTracker {
    /// Create a new performance tracker
    pub fn new(forecaster: String, metric_type: String) -> Self {
        Self {
            forecaster,
            metric_type,
            historical_metrics: Vec::new(),
            max_history: 30, // Keep 30 days of history
        }
    }

    /// Add new metrics
    pub fn add_metrics(&mut self, timestamp: chrono::DateTime<chrono::Utc>, metrics: ForecastMetrics) {
        self.historical_metrics.push((timestamp, metrics));

        // Keep only most recent entries
        if self.historical_metrics.len() > self.max_history {
            self.historical_metrics.drain(0..1);
        }
    }

    /// Get latest metrics
    pub fn latest(&self) -> Option<&ForecastMetrics> {
        self.historical_metrics.last().map(|(_, m)| m)
    }

    /// Calculate average MAPE over history
    pub fn average_mape(&self) -> f64 {
        if self.historical_metrics.is_empty() {
            return 0.0;
        }

        let sum: f64 = self.historical_metrics.iter().map(|(_, m)| m.mape).sum();
        sum / self.historical_metrics.len() as f64
    }

    /// Calculate average R² over history
    pub fn average_r2(&self) -> f64 {
        if self.historical_metrics.is_empty() {
            return 0.0;
        }

        let sum: f64 = self.historical_metrics.iter().map(|(_, m)| m.r2).sum();
        sum / self.historical_metrics.len() as f64
    }

    /// Check if forecast quality is improving
    pub fn is_improving(&self) -> bool {
        if self.historical_metrics.len() < 2 {
            return false;
        }

        // Compare recent average vs older average
        let mid = self.historical_metrics.len() / 2;
        let older_avg = self.historical_metrics[0..mid]
            .iter()
            .map(|(_, m)| m.mape)
            .sum::<f64>()
            / mid as f64;

        let recent_avg = self.historical_metrics[mid..]
            .iter()
            .map(|(_, m)| m.mape)
            .sum::<f64>()
            / (self.historical_metrics.len() - mid) as f64;

        recent_avg < older_avg // Lower MAPE is better
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perfect_forecast() {
        let actual = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let predicted = vec![1.0, 2.0, 3.0, 4.0, 5.0];

        let metrics = ForecastMetrics::calculate(&actual, &predicted).unwrap();

        assert_eq!(metrics.mae, 0.0);
        assert_eq!(metrics.rmse, 0.0);
        assert_eq!(metrics.mape, 0.0);
        assert_eq!(metrics.r2, 1.0);
        assert_eq!(metrics.quality(), ForecastQuality::Excellent);
    }

    #[test]
    fn test_forecast_with_errors() {
        let actual = vec![100.0, 200.0, 300.0, 400.0, 500.0];
        let predicted = vec![110.0, 190.0, 310.0, 390.0, 510.0];

        let metrics = ForecastMetrics::calculate(&actual, &predicted).unwrap();

        assert!(metrics.mae > 0.0);
        assert!(metrics.mae < 15.0); // Within reasonable bounds
        assert!(metrics.mape < 10.0); // Good forecast
        assert!(metrics.r2 > 0.95); // High R²
        assert_eq!(metrics.quality(), ForecastQuality::Good);
    }

    #[test]
    fn test_dimension_mismatch() {
        let actual = vec![1.0, 2.0, 3.0];
        let predicted = vec![1.0, 2.0];

        let result = ForecastMetrics::calculate(&actual, &predicted);
        assert!(result.is_err());
    }

    #[test]
    fn test_prediction_interval() {
        let interval = PredictionInterval::new(0.95);
        let prediction = 100.0;
        let historical_errors = vec![-5.0, 3.0, -2.0, 8.0, -1.0, 4.0];

        let (lower, upper) = interval.calculate_bounds(prediction, &historical_errors).unwrap();

        assert!(lower < prediction);
        assert!(upper > prediction);
        assert!(lower >= 0.0); // Non-negative bound
    }

    #[test]
    fn test_time_series_cross_validation() {
        let cv = TimeSeriesCrossValidation::new(3, 10);
        let folds = cv.split(40);

        assert_eq!(folds.len(), 3);
        // Verify folds are non-overlapping and sequential
        for (train, test) in &folds {
            assert!(!train.is_empty());
            assert!(!test.is_empty());
            assert!(train.iter().max().unwrap() < test.iter().min().unwrap());
        }
    }

    #[test]
    fn test_forecast_quality_classification() {
        assert_eq!(
            ForecastMetrics {
                mae: 1.0,
                rmse: 1.5,
                mape: 3.0,
                r2: 0.98,
                sample_count: 100,
                max_error: 5.0,
                min_error: 0.1,
                std_dev: 1.2,
            }
            .quality(),
            ForecastQuality::Excellent
        );

        assert_eq!(
            ForecastMetrics {
                mae: 5.0,
                rmse: 7.0,
                mape: 15.0,
                r2: 0.75,
                sample_count: 100,
                max_error: 20.0,
                min_error: 1.0,
                std_dev: 5.0,
            }
            .quality(),
            ForecastQuality::Fair
        );
    }

    #[test]
    fn test_performance_tracker() {
        let mut tracker = ForecastPerformanceTracker::new(
            "TestForecaster".to_string(),
            "consumption".to_string(),
        );

        let metrics1 = ForecastMetrics {
            mae: 10.0,
            rmse: 12.0,
            mape: 15.0,
            r2: 0.8,
            sample_count: 24,
            max_error: 25.0,
            min_error: 2.0,
            std_dev: 8.0,
        };

        let metrics2 = ForecastMetrics {
            mae: 8.0,
            rmse: 10.0,
            mape: 12.0,
            r2: 0.85,
            sample_count: 24,
            max_error: 20.0,
            min_error: 1.5,
            std_dev: 6.0,
        };

        tracker.add_metrics(chrono::Utc::now(), metrics1);
        tracker.add_metrics(chrono::Utc::now() + chrono::Duration::hours(1), metrics2);

        assert!(tracker.average_mape() > 0.0);
        assert!(tracker.average_r2() > 0.0);
    }
}
