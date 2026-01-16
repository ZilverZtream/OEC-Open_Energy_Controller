#![allow(dead_code)]
//! ML Model Training Pipeline
//!
//! This module provides functionality for training ML models offline.

use super::{FeatureVector, ModelMetadata, ModelType, ValidationMetrics};
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Training Dataset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingDataset {
    pub features: Vec<FeatureVector>,
    pub targets: Vec<f64>,
}

impl TrainingDataset {
    pub fn new(features: Vec<FeatureVector>, targets: Vec<f64>) -> Result<Self> {
        if features.len() != targets.len() {
            anyhow::bail!(
                "Feature and target count mismatch: {} features, {} targets",
                features.len(),
                targets.len()
            );
        }
        Ok(Self { features, targets })
    }

    pub fn len(&self) -> usize {
        self.features.len()
    }

    pub fn is_empty(&self) -> bool {
        self.features.is_empty()
    }

    /// Split dataset into training and validation sets
    pub fn split(&self, train_ratio: f64) -> Result<(TrainingDataset, TrainingDataset)> {
        if train_ratio <= 0.0 || train_ratio >= 1.0 {
            anyhow::bail!("Train ratio must be between 0 and 1");
        }

        let split_idx = (self.len() as f64 * train_ratio).floor() as usize;

        let train = TrainingDataset {
            features: self.features[..split_idx].to_vec(),
            targets: self.targets[..split_idx].to_vec(),
        };

        let val = TrainingDataset {
            features: self.features[split_idx..].to_vec(),
            targets: self.targets[split_idx..].to_vec(),
        };

        Ok((train, val))
    }
}

/// Training Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingConfig {
    pub model_type: ModelType,
    pub learning_rate: f64,
    pub max_iterations: usize,
    pub early_stopping_patience: usize,
    pub validation_split: f64,
}

impl Default for TrainingConfig {
    fn default() -> Self {
        Self {
            model_type: ModelType::LinearRegression,
            learning_rate: 0.01,
            max_iterations: 1000,
            early_stopping_patience: 10,
            validation_split: 0.2,
        }
    }
}

/// Model Trainer
pub struct ModelTrainer {
    config: TrainingConfig,
}

impl ModelTrainer {
    pub fn new(config: TrainingConfig) -> Self {
        Self { config }
    }

    /// Calculate validation metrics
    pub fn calculate_metrics(&self, predictions: &[f64], targets: &[f64]) -> Result<ValidationMetrics> {
        if predictions.len() != targets.len() {
            anyhow::bail!("Prediction and target count mismatch");
        }

        if predictions.is_empty() {
            anyhow::bail!("No predictions to evaluate");
        }

        let n = predictions.len() as f64;

        // Mean Absolute Error
        let mae: f64 = predictions
            .iter()
            .zip(targets.iter())
            .map(|(p, t)| (p - t).abs())
            .sum::<f64>()
            / n;

        // Root Mean Square Error
        let mse: f64 = predictions
            .iter()
            .zip(targets.iter())
            .map(|(p, t)| (p - t).powi(2))
            .sum::<f64>()
            / n;
        let rmse = mse.sqrt();

        // Mean Absolute Percentage Error
        let mape: f64 = predictions
            .iter()
            .zip(targets.iter())
            .filter(|(_, t)| t.abs() > 1e-10) // Avoid division by zero
            .map(|(p, t)| ((p - t) / t).abs() * 100.0)
            .sum::<f64>()
            / n;

        // R-squared
        let mean_target: f64 = targets.iter().sum::<f64>() / n;
        let ss_tot: f64 = targets.iter().map(|t| (t - mean_target).powi(2)).sum();
        let ss_res: f64 = predictions
            .iter()
            .zip(targets.iter())
            .map(|(p, t)| (t - p).powi(2))
            .sum();

        let r2 = if ss_tot.abs() < 1e-10 {
            0.0
        } else {
            1.0 - (ss_res / ss_tot)
        };

        Ok(ValidationMetrics::new(mae, rmse, mape, r2))
    }

    /// Train a simple linear regression model
    pub fn train_linear_regression(
        &self,
        dataset: &TrainingDataset,
    ) -> Result<super::models::LinearRegressionModel> {
        if dataset.is_empty() {
            anyhow::bail!("Cannot train on empty dataset");
        }

        let n_features = dataset.features[0].len();

        // Simple gradient descent (simplified implementation)
        let mut coefficients = vec![0.0; n_features];
        let mut intercept = 0.0;

        for _iter in 0..self.config.max_iterations {
            let mut coef_gradients = vec![0.0; n_features];
            let mut intercept_gradient = 0.0;
            let n = dataset.len() as f64;

            // Calculate gradients
            for (features, target) in dataset.features.iter().zip(dataset.targets.iter()) {
                let prediction: f64 = features
                    .features
                    .iter()
                    .zip(coefficients.iter())
                    .map(|(f, c)| f * c)
                    .sum::<f64>()
                    + intercept;

                let error = prediction - target;

                for (i, feature_val) in features.features.iter().enumerate() {
                    coef_gradients[i] += error * feature_val / n;
                }
                intercept_gradient += error / n;
            }

            // Update parameters
            for i in 0..n_features {
                coefficients[i] -= self.config.learning_rate * coef_gradients[i];
            }
            intercept -= self.config.learning_rate * intercept_gradient;
        }

        // Calculate validation metrics
        let predictions: Vec<f64> = dataset
            .features
            .iter()
            .map(|f| {
                f.features
                    .iter()
                    .zip(coefficients.iter())
                    .map(|(feat, coef)| feat * coef)
                    .sum::<f64>()
                    + intercept
            })
            .collect();

        let metrics = self.calculate_metrics(&predictions, &dataset.targets)?;

        let metadata = ModelMetadata {
            model_id: format!("linear_regression_{}", uuid::Uuid::new_v4()),
            model_type: ModelType::LinearRegression,
            version: "0.1.0".to_string(),
            trained_at: chrono::Utc::now(),
            training_samples: dataset.len(),
            validation_metrics: metrics,
            feature_names: dataset.features[0].feature_names.clone(),
        };

        Ok(super::models::LinearRegressionModel::new(
            coefficients,
            intercept,
            metadata,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dataset_split() {
        let features = vec![
            FeatureVector::new(vec![1.0, 2.0], vec!["f1".to_string(), "f2".to_string()]).unwrap(),
            FeatureVector::new(vec![2.0, 3.0], vec!["f1".to_string(), "f2".to_string()]).unwrap(),
            FeatureVector::new(vec![3.0, 4.0], vec!["f1".to_string(), "f2".to_string()]).unwrap(),
            FeatureVector::new(vec![4.0, 5.0], vec!["f1".to_string(), "f2".to_string()]).unwrap(),
        ];
        let targets = vec![3.0, 5.0, 7.0, 9.0];

        let dataset = TrainingDataset::new(features, targets).unwrap();
        let (train, val) = dataset.split(0.75).unwrap();

        assert_eq!(train.len(), 3);
        assert_eq!(val.len(), 1);
    }

    #[test]
    fn test_calculate_metrics() {
        let trainer = ModelTrainer::new(TrainingConfig::default());

        let predictions = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let targets = vec![1.1, 2.1, 2.9, 4.2, 4.8];

        let metrics = trainer.calculate_metrics(&predictions, &targets).unwrap();

        assert!(metrics.mae < 0.3);
        assert!(metrics.rmse < 0.4);
        assert!(metrics.r2 > 0.9);
    }

    #[test]
    fn test_train_linear_regression() {
        let features = vec![
            FeatureVector::new(vec![1.0], vec!["x".to_string()]).unwrap(),
            FeatureVector::new(vec![2.0], vec!["x".to_string()]).unwrap(),
            FeatureVector::new(vec![3.0], vec!["x".to_string()]).unwrap(),
            FeatureVector::new(vec![4.0], vec!["x".to_string()]).unwrap(),
        ];
        // y = 2x + 1
        let targets = vec![3.0, 5.0, 7.0, 9.0];

        let dataset = TrainingDataset::new(features, targets).unwrap();

        let mut config = TrainingConfig::default();
        config.max_iterations = 5000;
        config.learning_rate = 0.1;

        let trainer = ModelTrainer::new(config);
        let model = trainer.train_linear_regression(&dataset).unwrap();

        // Check if coefficients are close to 2.0 and intercept close to 1.0
        assert!((model.coefficients[0] - 2.0).abs() < 0.5);
        assert!((model.intercept - 1.0).abs() < 0.5);
        assert!(model.metadata.validation_metrics.r2 > 0.8);
    }
}
