//! ML Model Definitions
//!
//! This module contains concrete ML model implementations for various forecasting tasks.

use super::{FeatureVector, ModelMetadata, ModelType, Prediction, ValidationMetrics};
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Trait for ML models
pub trait MLModel: Send + Sync {
    /// Predict a value from features
    fn predict(&self, features: &FeatureVector) -> Result<Prediction>;

    /// Get model metadata
    fn metadata(&self) -> &ModelMetadata;

    /// Get model type
    fn model_type(&self) -> ModelType {
        self.metadata().model_type
    }
}

/// Simple Linear Regression Model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinearRegressionModel {
    pub metadata: ModelMetadata,
    pub coefficients: Vec<f64>,
    pub intercept: f64,
}

impl LinearRegressionModel {
    pub fn new(coefficients: Vec<f64>, intercept: f64, metadata: ModelMetadata) -> Self {
        Self {
            metadata,
            coefficients,
            intercept,
        }
    }

    /// Create a simple model with uniform coefficients (for testing)
    pub fn dummy_model(n_features: usize) -> Self {
        let metadata = ModelMetadata {
            model_id: "dummy_linear".to_string(),
            model_type: ModelType::LinearRegression,
            version: "0.1.0".to_string(),
            trained_at: chrono::Utc::now(),
            training_samples: 1000,
            validation_metrics: ValidationMetrics::new(0.5, 0.7, 5.0, 0.85),
            feature_names: (0..n_features)
                .map(|i| format!("feature_{}", i))
                .collect(),
        };

        Self {
            metadata,
            coefficients: vec![1.0; n_features],
            intercept: 0.0,
        }
    }
}

impl MLModel for LinearRegressionModel {
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

        Ok(Prediction::new(prediction))
    }

    fn metadata(&self) -> &ModelMetadata {
        &self.metadata
    }
}

/// Moving Average Model (baseline forecaster)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovingAverageModel {
    pub metadata: ModelMetadata,
    pub window_size: usize,
    pub historical_values: Vec<f64>,
}

impl MovingAverageModel {
    pub fn new(window_size: usize) -> Self {
        let metadata = ModelMetadata {
            model_id: "moving_average".to_string(),
            model_type: ModelType::LinearRegression, // Simplified
            version: "0.1.0".to_string(),
            trained_at: chrono::Utc::now(),
            training_samples: 0,
            validation_metrics: ValidationMetrics::new(0.0, 0.0, 0.0, 0.0),
            feature_names: vec!["historical_value".to_string()],
        };

        Self {
            metadata,
            window_size,
            historical_values: Vec::new(),
        }
    }

    /// Add a new historical value
    pub fn add_value(&mut self, value: f64) {
        self.historical_values.push(value);
        if self.historical_values.len() > self.window_size {
            self.historical_values.remove(0);
        }
    }

    /// Get the moving average
    pub fn get_average(&self) -> Option<f64> {
        if self.historical_values.is_empty() {
            return None;
        }

        let sum: f64 = self.historical_values.iter().sum();
        Some(sum / self.historical_values.len() as f64)
    }
}

impl MLModel for MovingAverageModel {
    fn predict(&self, _features: &FeatureVector) -> Result<Prediction> {
        let avg = self
            .get_average()
            .ok_or_else(|| anyhow::anyhow!("No historical data available"))?;

        Ok(Prediction::new(avg))
    }

    fn metadata(&self) -> &ModelMetadata {
        &self.metadata
    }
}

/// Exponential Smoothing Model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExponentialSmoothingModel {
    pub metadata: ModelMetadata,
    pub alpha: f64, // Smoothing factor (0 < alpha < 1)
    pub last_value: Option<f64>,
    pub last_smoothed: Option<f64>,
}

impl ExponentialSmoothingModel {
    pub fn new(alpha: f64) -> Self {
        let metadata = ModelMetadata {
            model_id: "exponential_smoothing".to_string(),
            model_type: ModelType::LinearRegression, // Simplified
            version: "0.1.0".to_string(),
            trained_at: chrono::Utc::now(),
            training_samples: 0,
            validation_metrics: ValidationMetrics::new(0.0, 0.0, 0.0, 0.0),
            feature_names: vec!["time_series_value".to_string()],
        };

        Self {
            metadata,
            alpha: alpha.clamp(0.01, 0.99),
            last_value: None,
            last_smoothed: None,
        }
    }

    /// Update with a new observation
    pub fn update(&mut self, value: f64) {
        if let Some(prev_smoothed) = self.last_smoothed {
            self.last_smoothed = Some(self.alpha * value + (1.0 - self.alpha) * prev_smoothed);
        } else {
            self.last_smoothed = Some(value);
        }
        self.last_value = Some(value);
    }

    /// Get the current smoothed value
    pub fn get_smoothed(&self) -> Option<f64> {
        self.last_smoothed
    }
}

impl MLModel for ExponentialSmoothingModel {
    fn predict(&self, _features: &FeatureVector) -> Result<Prediction> {
        let smoothed = self
            .get_smoothed()
            .ok_or_else(|| anyhow::anyhow!("No smoothed value available"))?;

        Ok(Prediction::new(smoothed))
    }

    fn metadata(&self) -> &ModelMetadata {
        &self.metadata
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linear_regression_predict() {
        let model = LinearRegressionModel::new(
            vec![2.0, 3.0, 1.0],
            5.0,
            ModelMetadata {
                model_id: "test".to_string(),
                model_type: ModelType::LinearRegression,
                version: "0.1.0".to_string(),
                trained_at: chrono::Utc::now(),
                training_samples: 100,
                validation_metrics: ValidationMetrics::new(0.5, 0.7, 5.0, 0.85),
                feature_names: vec!["f1".to_string(), "f2".to_string(), "f3".to_string()],
            },
        );

        let features = FeatureVector::new(
            vec![1.0, 2.0, 3.0],
            vec!["f1".to_string(), "f2".to_string(), "f3".to_string()],
        )
        .unwrap();

        let prediction = model.predict(&features).unwrap();
        // 2*1 + 3*2 + 1*3 + 5 = 2 + 6 + 3 + 5 = 16
        assert_eq!(prediction.value, 16.0);
    }

    #[test]
    fn test_moving_average() {
        let mut model = MovingAverageModel::new(3);

        model.add_value(10.0);
        model.add_value(20.0);
        model.add_value(30.0);

        let avg = model.get_average().unwrap();
        assert_eq!(avg, 20.0);

        // Add more values, oldest should be dropped
        model.add_value(40.0);
        let avg = model.get_average().unwrap();
        assert_eq!(avg, 30.0); // (20 + 30 + 40) / 3
    }

    #[test]
    fn test_exponential_smoothing() {
        let mut model = ExponentialSmoothingModel::new(0.5);

        model.update(10.0);
        assert_eq!(model.get_smoothed().unwrap(), 10.0);

        model.update(20.0);
        let smoothed = model.get_smoothed().unwrap();
        assert_eq!(smoothed, 15.0); // 0.5*20 + 0.5*10 = 15

        model.update(30.0);
        let smoothed = model.get_smoothed().unwrap();
        assert_eq!(smoothed, 22.5); // 0.5*30 + 0.5*15 = 22.5
    }
}
