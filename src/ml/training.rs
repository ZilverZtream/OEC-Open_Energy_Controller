#![allow(dead_code)]
//! ML Model Training Pipeline
//!
//! This module provides functionality for training ML models offline.

use super::{FeatureVector, ModelMetadata, ModelType, ValidationMetrics};
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[cfg(all(feature = "ml", feature = "db"))]
use chrono::{Duration, Utc};
#[cfg(all(feature = "ml", feature = "db"))]
use uuid::Uuid;

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

        if self.len() == 0 {
            anyhow::bail!("Cannot split empty dataset");
        }

        // Calculate split index with bounds checking
        let split_idx = (self.len() as f64 * train_ratio).floor() as usize;

        // Ensure split creates non-empty sets
        let split_idx = split_idx.max(1).min(self.len() - 1);

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

/// Consumption Model Training Service
///
/// This module provides the "Nightly Edge Training" pipeline for consumption forecasting.
#[cfg(all(feature = "ml", feature = "db"))]
pub mod consumption_trainer {
    use super::*;
    use crate::forecast::features::{normalize_features_cyclical, FeatureExtractor};
    use crate::ml::smartcore::SmartcoreRandomForest;
    use crate::repo::consumption::ConsumptionRepository;
    use crate::domain::ConsumptionPoint;
    use tracing::{error, info, warn};

    /// Configuration for consumption model training
    #[derive(Debug, Clone)]
    pub struct ConsumptionTrainingConfig {
        /// Number of days of historical data to use
        pub history_days: i64,
        /// Maximum number of samples to use (for memory efficiency)
        pub max_samples: usize,
        /// Validation split ratio
        pub validation_split: f64,
        /// Random forest parameters
        pub n_trees: usize,
        pub max_depth: Option<usize>,
        pub min_samples_split: usize,
    }

    impl Default for ConsumptionTrainingConfig {
        fn default() -> Self {
            Self {
                history_days: 30,        // Use last 30 days
                max_samples: 50_000,     // Limit to prevent OOM
                validation_split: 0.1,   // 10% validation
                n_trees: 50,             // Conservative for Pi
                max_depth: Some(10),     // Prevent depth explosion
                min_samples_split: 5,    // Reduce overfitting
            }
        }
    }

    /// Train a consumption forecasting model
    ///
    /// This function implements the core "Nightly Edge Training" logic:
    /// 1. Fetch historical data from the database
    /// 2. Extract features using cyclical time encoding
    /// 3. Train a RandomForest model
    /// 4. Validate and return the trained model
    pub async fn train_consumption_model(
        repo: &ConsumptionRepository,
        household_id: Uuid,
        latitude: f64,
        longitude: f64,
        config: ConsumptionTrainingConfig,
    ) -> Result<SmartcoreRandomForest> {
        info!("Starting consumption model training for household {}", household_id);

        // Calculate time range
        let end = Utc::now();
        let start = end - Duration::days(config.history_days);

        info!(
            "Fetching consumption data from {} to {}",
            start.format("%Y-%m-%d"),
            end.format("%Y-%m-%d")
        );

        // Fetch historical data
        let data: Vec<ConsumptionPoint> = repo
            .find_range(
                household_id,
                start.into(),
                end.into(),
            )
            .await?;

        if data.is_empty() {
            anyhow::bail!("No historical data available for training");
        }

        info!("Retrieved {} consumption data points", data.len());

        // Downsample if necessary to prevent OOM
        let data: Vec<ConsumptionPoint> = if data.len() > config.max_samples {
            warn!(
                "Downsampling from {} to {} samples to prevent OOM",
                data.len(),
                config.max_samples
            );
            let step = data.len() / config.max_samples;
            data.into_iter()
                .enumerate()
                .filter(|(i, _)| i % step == 0)
                .map(|(_, d)| d)
                .take(config.max_samples)
                .collect()
        } else {
            data
        };

        // Extract features with pre-allocated vectors for better performance
        info!("Extracting features from {} samples", data.len());
        let feature_extractor = FeatureExtractor::new(latitude, longitude);

        let data_len = data.len();
        let mut features_list = Vec::with_capacity(data_len);
        let mut targets = Vec::with_capacity(data_len);

        for point in data {
            let timestamp = point.time_start;
            let temporal_features = feature_extractor.extract_temporal_features(timestamp.into());
            let normalized_features = normalize_features_cyclical(&temporal_features);

            features_list.push(normalized_features);
            targets.push(point.load_kw);
        }

        if features_list.is_empty() {
            anyhow::bail!("Failed to extract features from data");
        }

        info!(
            "Extracted {} features per sample",
            features_list[0].len()
        );

        // Split into training and validation sets
        let split_idx = ((features_list.len() as f64) * (1.0 - config.validation_split)) as usize;
        let split_idx = split_idx.max(1).min(features_list.len() - 1);

        let train_x = features_list[..split_idx].to_vec();
        let train_y = targets[..split_idx].to_vec();
        let val_x = features_list[split_idx..].to_vec();
        let val_y = targets[split_idx..].to_vec();

        info!(
            "Split data: {} training samples, {} validation samples",
            train_x.len(),
            val_x.len()
        );

        // Generate feature names
        let feature_names = vec![
            "hour_sin".to_string(),
            "hour_cos".to_string(),
            "day_of_week_sin".to_string(),
            "day_of_week_cos".to_string(),
            "month_sin".to_string(),
            "month_cos".to_string(),
            "day_of_month_norm".to_string(),
            "is_weekend".to_string(),
            "is_holiday".to_string(),
            "temperature_norm".to_string(),
            "cloud_cover_norm".to_string(),
            "wind_speed_norm".to_string(),
            "season_sin".to_string(),
            "season_cos".to_string(),
            "day_length_norm".to_string(),
        ];

        // Train the model
        info!(
            "Training RandomForest with {} trees, max_depth={:?}",
            config.n_trees, config.max_depth
        );

        let params = SmartcoreRandomForest::custom_parameters(
            config.n_trees,
            config.max_depth,
            config.min_samples_split,
        );

        let model = SmartcoreRandomForest::train(&train_x, &train_y, params, feature_names)?;

        info!(
            "Model trained successfully. Training metrics: MAE={:.3}, RMSE={:.3}, R2={:.3}",
            model.metadata.validation_metrics.mae,
            model.metadata.validation_metrics.rmse,
            model.metadata.validation_metrics.r2
        );

        // Validate on held-out set using batch prediction (more efficient)
        if !val_x.is_empty() {
            use crate::ml::models::MLModel;
            use smartcore::linalg::basic::matrix::DenseMatrix;

            // Convert validation features to DenseMatrix for batch prediction
            let n_val = val_x.len();
            let n_features = val_x[0].len();
            let mut flat_val_data = Vec::with_capacity(n_val * n_features);
            for row in &val_x {
                flat_val_data.extend_from_slice(row);
            }

            let val_matrix = DenseMatrix::new(n_val, n_features, flat_val_data, false);

            // Batch predict on validation set
            let val_predictions = model
                .model
                .as_ref()
                .unwrap()
                .predict(&val_matrix)
                .map_err(|e| anyhow::anyhow!("Validation prediction failed: {:?}", e))?;

            let trainer = ModelTrainer::new(TrainingConfig::default());
            let val_metrics = trainer.calculate_metrics(&val_predictions, &val_y)?;

            info!(
                "Validation metrics: MAE={:.3}, RMSE={:.3}, R2={:.3}",
                val_metrics.mae, val_metrics.rmse, val_metrics.r2
            );

            // Check if model quality is acceptable
            if val_metrics.r2 < 0.3 {
                warn!(
                    "Model R2 score is low ({:.3}), but continuing with training",
                    val_metrics.r2
                );
            }
        }

        Ok(model)
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
