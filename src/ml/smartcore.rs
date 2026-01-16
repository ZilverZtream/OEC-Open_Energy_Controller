#![allow(dead_code)]
//! SmartCore ML Model Wrapper
//!
//! This module provides a wrapper for SmartCore's RandomForestRegressor
//! optimized for Raspberry Pi deployment.

use super::{FeatureVector, ModelMetadata, ModelType, Prediction};
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[cfg(feature = "ml")]
use smartcore::ensemble::random_forest_regressor::{RandomForestRegressor, RandomForestRegressorParameters};
#[cfg(feature = "ml")]
use smartcore::linalg::basic::matrix::DenseMatrix;

/// SmartCore RandomForest Model Wrapper
///
/// This wrapper is optimized for Raspberry Pi with conservative parameters:
/// - Limited tree depth to prevent OOM
/// - Moderate number of trees for balance between accuracy and speed
/// - Efficient memory usage with proper cleanup
#[derive(Debug, Serialize, Deserialize)]
pub struct SmartcoreRandomForest {
    pub metadata: ModelMetadata,
    #[serde(skip)]
    #[cfg(feature = "ml")]
    model: Option<RandomForestRegressor<f64, f64, DenseMatrix<f64>, Vec<f64>>>,
    /// Serialized model bytes (for persistence)
    #[cfg(feature = "ml")]
    model_bytes: Option<Vec<u8>>,
    /// Training parameters for reproducibility
    pub n_trees: usize,
    pub max_depth: Option<usize>,
    pub min_samples_split: usize,
    pub min_samples_leaf: usize,
}

#[cfg(feature = "ml")]
impl SmartcoreRandomForest {
    /// Create a new RandomForest model with trained instance
    pub fn new(
        model: RandomForestRegressor<f64, f64, DenseMatrix<f64>, Vec<f64>>,
        metadata: ModelMetadata,
        n_trees: usize,
        max_depth: Option<usize>,
    ) -> Self {
        Self {
            metadata,
            model: Some(model),
            model_bytes: None,
            n_trees,
            max_depth,
            min_samples_split: 2,
            min_samples_leaf: 1,
        }
    }

    /// Get training parameters optimized for Raspberry Pi
    ///
    /// Conservative settings to prevent OOM and ensure < 2 min training time:
    /// - 50 trees (balance between accuracy and speed)
    /// - Max depth 10 (prevents memory explosion)
    /// - Min samples split 5 (reduces overfitting)
    pub fn default_parameters() -> RandomForestRegressorParameters {
        RandomForestRegressorParameters {
            max_depth: Some(10),
            min_samples_leaf: 2,
            min_samples_split: 5,
            n_trees: 50,
            m: None, // Use sqrt(n_features) by default
            keep_samples: false, // Don't store training samples (saves memory)
            seed: 42,
        }
    }

    /// Get optimized parameters with custom settings
    pub fn custom_parameters(
        n_trees: usize,
        max_depth: Option<usize>,
        min_samples_split: usize,
    ) -> RandomForestRegressorParameters {
        RandomForestRegressorParameters {
            max_depth: max_depth.map(|d| d as u16),
            min_samples_leaf: 2,
            min_samples_split,
            n_trees,
            m: None,
            keep_samples: false,
            seed: 42,
        }
    }

    /// Train a new RandomForest model
    pub fn train(
        x: &[Vec<f64>],
        y: &[f64],
        params: RandomForestRegressorParameters,
        feature_names: Vec<String>,
    ) -> Result<Self> {
        if x.is_empty() || y.is_empty() {
            anyhow::bail!("Cannot train on empty dataset");
        }

        if x.len() != y.len() {
            anyhow::bail!(
                "Feature and target count mismatch: {} features, {} targets",
                x.len(),
                y.len()
            );
        }

        // Store params values before moving
        let n_trees = params.n_trees;
        let max_depth = params.max_depth.map(|d| d as usize);

        // Convert to DenseMatrix
        let n_samples = x.len();
        let n_features = x[0].len();

        let mut flat_data = Vec::with_capacity(n_samples * n_features);
        for row in x {
            if row.len() != n_features {
                anyhow::bail!("All feature vectors must have the same length");
            }
            flat_data.extend_from_slice(row);
        }

        let x_matrix = DenseMatrix::new(n_samples, n_features, flat_data, false);
        let y_vec = y.to_vec();

        // Train the model
        let model = RandomForestRegressor::fit(&x_matrix, &y_vec, params)
            .map_err(|e| anyhow::anyhow!("RandomForest training failed: {:?}", e))?;

        // Calculate training metrics
        let predictions = model
            .predict(&x_matrix)
            .map_err(|e| anyhow::anyhow!("Prediction failed during validation: {:?}", e))?;

        let metrics = crate::ml::training::ModelTrainer::new(
            crate::ml::training::TrainingConfig::default()
        )
        .calculate_metrics(&predictions, y)?;

        let metadata = ModelMetadata {
            model_id: format!("smartcore_rf_{}", uuid::Uuid::new_v4()),
            model_type: ModelType::RandomForest,
            version: "1.0.0".to_string(),
            trained_at: chrono::Utc::now(),
            training_samples: n_samples,
            validation_metrics: metrics,
            feature_names,
        };

        Ok(Self::new(model, metadata, n_trees, max_depth))
    }

    /// Prepare model for serialization
    pub fn prepare_for_serialization(&mut self) -> Result<()> {
        if let Some(model) = &self.model {
            // Serialize the model to bytes using bincode
            let bytes = bincode::serialize(model)
                .map_err(|e| anyhow::anyhow!("Failed to serialize model: {}", e))?;
            self.model_bytes = Some(bytes);
        }
        Ok(())
    }

    /// Restore model from serialized bytes
    pub fn restore_from_serialization(&mut self) -> Result<()> {
        if let Some(bytes) = &self.model_bytes {
            let model: RandomForestRegressor<f64, f64, DenseMatrix<f64>, Vec<f64>> =
                bincode::deserialize(bytes)
                    .map_err(|e| anyhow::anyhow!("Failed to deserialize model: {}", e))?;
            self.model = Some(model);
        }
        Ok(())
    }
}

#[cfg(feature = "ml")]
impl super::models::MLModel for SmartcoreRandomForest {
    fn predict(&self, features: &FeatureVector) -> Result<Prediction> {
        let model = self
            .model
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Model not loaded"))?;

        // Convert feature vector to DenseMatrix (1 row, n features)
        let n_features = features.len();
        let x = DenseMatrix::new(1, n_features, features.features.clone(), false);

        // Predict
        let predictions = model
            .predict(&x)
            .map_err(|e| anyhow::anyhow!("Prediction failed: {:?}", e))?;

        if predictions.is_empty() {
            anyhow::bail!("Model returned empty predictions");
        }

        let value = predictions[0];

        // Sanity check: consumption should be positive
        if value < 0.0 {
            anyhow::bail!(
                "Invalid prediction: negative consumption ({:.2} kW)",
                value
            );
        }

        // Sanity check: consumption should be reasonable (< 100 kW for household)
        if value > 100.0 {
            anyhow::bail!(
                "Invalid prediction: unreasonably high consumption ({:.2} kW)",
                value
            );
        }

        Ok(Prediction::new(value))
    }

    fn metadata(&self) -> &ModelMetadata {
        &self.metadata
    }
}

#[cfg(not(feature = "ml"))]
impl SmartcoreRandomForest {
    pub fn default_parameters() -> () {
        ()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "ml")]
    #[test]
    fn test_random_forest_parameters() {
        let params = SmartcoreRandomForest::default_parameters();
        assert_eq!(params.n_trees, 50);
        assert_eq!(params.max_depth, Some(10));
        assert_eq!(params.min_samples_split, 5);
        assert!(!params.keep_samples);
    }

    #[cfg(feature = "ml")]
    #[test]
    fn test_custom_parameters() {
        let params = SmartcoreRandomForest::custom_parameters(100, Some(15), 10);
        assert_eq!(params.n_trees, 100);
        assert_eq!(params.max_depth, Some(15));
        assert_eq!(params.min_samples_split, 10);
    }

    #[cfg(feature = "ml")]
    #[test]
    fn test_train_random_forest() {
        // Create synthetic training data: y = 2x1 + 3x2 + noise
        let x: Vec<Vec<f64>> = vec![
            vec![1.0, 1.0],
            vec![2.0, 1.0],
            vec![1.0, 2.0],
            vec![2.0, 2.0],
            vec![3.0, 3.0],
            vec![4.0, 2.0],
            vec![2.0, 4.0],
            vec![3.0, 1.0],
            vec![1.0, 3.0],
            vec![4.0, 4.0],
        ];

        let y: Vec<f64> = vec![
            5.0,  // 2*1 + 3*1
            7.0,  // 2*2 + 3*1
            8.0,  // 2*1 + 3*2
            10.0, // 2*2 + 3*2
            15.0, // 2*3 + 3*3
            14.0, // 2*4 + 3*2
            14.0, // 2*2 + 3*4
            9.0,  // 2*3 + 3*1
            11.0, // 2*1 + 3*3
            20.0, // 2*4 + 3*4
        ];

        let params = SmartcoreRandomForest::custom_parameters(10, Some(5), 2);
        let feature_names = vec!["x1".to_string(), "x2".to_string()];

        let model = SmartcoreRandomForest::train(&x, &y, params, feature_names);
        assert!(model.is_ok());

        let model = model.unwrap();
        assert_eq!(model.metadata.training_samples, 10);
        assert_eq!(model.n_trees, 10);
    }

    #[cfg(feature = "ml")]
    #[test]
    fn test_predict() {
        // Create and train a simple model
        let x: Vec<Vec<f64>> = vec![
            vec![1.0, 2.0],
            vec![2.0, 3.0],
            vec![3.0, 4.0],
            vec![4.0, 5.0],
            vec![5.0, 6.0],
        ];

        let y: Vec<f64> = vec![3.0, 5.0, 7.0, 9.0, 11.0];

        let params = SmartcoreRandomForest::custom_parameters(5, Some(3), 2);
        let feature_names = vec!["f1".to_string(), "f2".to_string()];

        let model = SmartcoreRandomForest::train(&x, &y, params, feature_names.clone()).unwrap();

        // Test prediction
        let test_features = FeatureVector::new(vec![3.0, 4.0], feature_names).unwrap();
        let prediction = model.predict(&test_features);

        assert!(prediction.is_ok());
        let pred = prediction.unwrap();

        // Should predict something reasonable (around 7.0)
        assert!(pred.value > 5.0 && pred.value < 9.0);
    }
}
