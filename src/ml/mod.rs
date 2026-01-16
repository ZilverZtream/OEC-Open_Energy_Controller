#![allow(dead_code)]
//! Machine Learning Module
//!
//! This module provides ML capabilities for forecasting and optimization:
//! - Price forecasting
//! - Consumption forecasting
//! - Solar production forecasting
//! - Battery degradation prediction
//!
//! # Architecture
//! - Training pipeline for offline model training
//! - Inference engine for production predictions
//! - Model versioning and deployment
//! - Feature engineering and normalization

use anyhow::Result;
use serde::{Deserialize, Serialize};

pub mod models;
pub mod training;
pub mod inference;

#[cfg(feature = "ml")]
pub mod smartcore;

/// ML Model Type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ModelType {
    LinearRegression,
    RandomForest,
    GradientBoosting,
    LSTM,
    Transformer,
}

/// ML Model Metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMetadata {
    pub model_id: String,
    pub model_type: ModelType,
    pub version: String,
    pub trained_at: chrono::DateTime<chrono::Utc>,
    pub training_samples: usize,
    pub validation_metrics: ValidationMetrics,
    pub feature_names: Vec<String>,
}

/// Validation Metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationMetrics {
    pub mae: f64,  // Mean Absolute Error
    pub rmse: f64, // Root Mean Square Error
    pub mape: f64, // Mean Absolute Percentage Error
    pub r2: f64,   // R-squared
}

impl ValidationMetrics {
    pub fn new(mae: f64, rmse: f64, mape: f64, r2: f64) -> Self {
        Self {
            mae,
            rmse,
            mape,
            r2,
        }
    }

    /// Check if metrics meet quality thresholds
    pub fn meets_quality_threshold(&self, max_mape: f64, min_r2: f64) -> bool {
        self.mape <= max_mape && self.r2 >= min_r2
    }
}

/// Feature Vector for ML models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureVector {
    pub features: Vec<f64>,
    pub feature_names: Vec<String>,
}

impl FeatureVector {
    pub fn new(features: Vec<f64>, feature_names: Vec<String>) -> Result<Self> {
        if features.len() != feature_names.len() {
            anyhow::bail!(
                "Feature count mismatch: {} features, {} names",
                features.len(),
                feature_names.len()
            );
        }
        Ok(Self {
            features,
            feature_names,
        })
    }

    pub fn len(&self) -> usize {
        self.features.len()
    }

    pub fn is_empty(&self) -> bool {
        self.features.is_empty()
    }

    /// Normalize features using min-max scaling
    pub fn normalize(&self, min_vals: &[f64], max_vals: &[f64]) -> Result<Self> {
        if min_vals.len() != self.features.len() || max_vals.len() != self.features.len() {
            anyhow::bail!("Normalization parameter count mismatch");
        }

        let normalized = self
            .features
            .iter()
            .zip(min_vals.iter().zip(max_vals.iter()))
            .map(|(f, (min, max))| {
                if (max - min).abs() < 1e-10 {
                    0.5 // Avoid division by zero
                } else {
                    (f - min) / (max - min)
                }
            })
            .collect();

        Ok(Self {
            features: normalized,
            feature_names: self.feature_names.clone(),
        })
    }

    /// Standardize features using z-score normalization
    pub fn standardize(&self, means: &[f64], stds: &[f64]) -> Result<Self> {
        if means.len() != self.features.len() || stds.len() != self.features.len() {
            anyhow::bail!("Standardization parameter count mismatch");
        }

        let standardized = self
            .features
            .iter()
            .zip(means.iter().zip(stds.iter()))
            .map(|(f, (mean, std))| {
                if std.abs() < 1e-10 {
                    0.0 // Avoid division by zero
                } else {
                    (f - mean) / std
                }
            })
            .collect();

        Ok(Self {
            features: standardized,
            feature_names: self.feature_names.clone(),
        })
    }
}

/// ML Prediction Result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prediction {
    pub value: f64,
    pub confidence: f64,
    pub lower_bound: Option<f64>,
    pub upper_bound: Option<f64>,
}

impl Prediction {
    pub fn new(value: f64) -> Self {
        Self {
            value,
            confidence: 1.0,
            lower_bound: None,
            upper_bound: None,
        }
    }

    pub fn with_confidence(value: f64, confidence: f64) -> Self {
        Self {
            value,
            confidence,
            lower_bound: None,
            upper_bound: None,
        }
    }

    pub fn with_bounds(value: f64, lower: f64, upper: f64) -> Self {
        Self {
            value,
            confidence: 1.0,
            lower_bound: Some(lower),
            upper_bound: Some(upper),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_vector_creation() {
        let features = vec![1.0, 2.0, 3.0];
        let names = vec!["f1".to_string(), "f2".to_string(), "f3".to_string()];

        let fv = FeatureVector::new(features, names).unwrap();
        assert_eq!(fv.len(), 3);
        assert!(!fv.is_empty());
    }

    #[test]
    fn test_feature_vector_normalize() {
        let features = vec![10.0, 20.0, 30.0];
        let names = vec!["f1".to_string(), "f2".to_string(), "f3".to_string()];
        let fv = FeatureVector::new(features, names).unwrap();

        let min_vals = vec![0.0, 10.0, 20.0];
        let max_vals = vec![100.0, 30.0, 40.0];

        let normalized = fv.normalize(&min_vals, &max_vals).unwrap();
        assert_eq!(normalized.features[0], 0.1); // (10-0)/(100-0)
        assert_eq!(normalized.features[1], 0.5); // (20-10)/(30-10)
        assert_eq!(normalized.features[2], 0.5); // (30-20)/(40-20)
    }

    #[test]
    fn test_feature_vector_standardize() {
        let features = vec![10.0, 20.0, 30.0];
        let names = vec!["f1".to_string(), "f2".to_string(), "f3".to_string()];
        let fv = FeatureVector::new(features, names).unwrap();

        let means = vec![10.0, 20.0, 30.0];
        let stds = vec![2.0, 5.0, 10.0];

        let standardized = fv.standardize(&means, &stds).unwrap();
        assert_eq!(standardized.features[0], 0.0); // (10-10)/2
        assert_eq!(standardized.features[1], 0.0); // (20-20)/5
        assert_eq!(standardized.features[2], 0.0); // (30-30)/10
    }

    #[test]
    fn test_validation_metrics() {
        let metrics = ValidationMetrics::new(0.5, 0.7, 5.0, 0.95);

        assert!(metrics.meets_quality_threshold(10.0, 0.9));
        assert!(!metrics.meets_quality_threshold(3.0, 0.9));
        assert!(!metrics.meets_quality_threshold(10.0, 0.97));
    }

    #[test]
    fn test_prediction_creation() {
        let pred = Prediction::new(42.0);
        assert_eq!(pred.value, 42.0);
        assert_eq!(pred.confidence, 1.0);

        let pred_conf = Prediction::with_confidence(42.0, 0.85);
        assert_eq!(pred_conf.confidence, 0.85);

        let pred_bounds = Prediction::with_bounds(42.0, 40.0, 44.0);
        assert_eq!(pred_bounds.lower_bound, Some(40.0));
        assert_eq!(pred_bounds.upper_bound, Some(44.0));
    }
}
