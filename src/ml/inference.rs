#![allow(dead_code)]
//! ML Model Inference Engine
//!
//! This module provides functionality for running trained models in production.

use super::{models::MLModel, FeatureVector, Prediction};
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Model Registry for managing multiple models
///
/// CRITICAL FIX: Changed from Box<dyn MLModel> to Arc<dyn MLModel>
/// to enable model sharing across threads without ownership transfer.
/// Arc allows multiple references to the same model, fixing the "placebo ML engine"
/// issue where get() always returned None.
pub struct ModelRegistry {
    models: Arc<RwLock<std::collections::HashMap<String, Arc<dyn MLModel>>>>,
}

impl ModelRegistry {
    pub fn new() -> Self {
        Self {
            models: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Register a new model (takes ownership via Arc)
    pub async fn register(&self, model_id: String, model: Arc<dyn MLModel>) {
        let mut models = self.models.write().await;
        models.insert(model_id, model);
    }

    /// Register a new model from a boxed instance (converts to Arc internally)
    pub async fn register_boxed(&self, model_id: String, model: Box<dyn MLModel>) {
        let arc_model: Arc<dyn MLModel> = model.into();
        self.register(model_id, arc_model).await;
    }

    /// Get a model by ID
    ///
    /// Returns an Arc clone of the model, allowing multiple consumers
    /// to use the same model instance concurrently.
    pub async fn get(&self, model_id: &str) -> Option<Arc<dyn MLModel>> {
        let models = self.models.read().await;
        models.get(model_id).map(Arc::clone)
    }

    /// List all registered model IDs
    pub async fn list_model_ids(&self) -> Vec<String> {
        let models = self.models.read().await;
        models.keys().cloned().collect()
    }

    /// Remove a model
    pub async fn unregister(&self, model_id: &str) -> bool {
        let mut models = self.models.write().await;
        models.remove(model_id).is_some()
    }

    /// Get model count
    pub async fn count(&self) -> usize {
        let models = self.models.read().await;
        models.len()
    }
}

impl Default for ModelRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Inference Engine for running predictions
pub struct InferenceEngine {
    registry: ModelRegistry,
}

impl InferenceEngine {
    pub fn new() -> Self {
        Self {
            registry: ModelRegistry::new(),
        }
    }

    /// Register a model with the engine (Box version for backwards compatibility)
    pub async fn register_model(&self, model_id: String, model: Box<dyn MLModel>) {
        self.registry.register_boxed(model_id, model).await;
    }

    /// Register a model with the engine (Arc version for efficiency)
    pub async fn register_model_arc(&self, model_id: String, model: Arc<dyn MLModel>) {
        self.registry.register(model_id, model).await;
    }

    /// Run inference with a specific model
    ///
    /// CRITICAL FIX: Now properly retrieves and uses registered models
    /// instead of always returning an error.
    pub async fn predict(
        &self,
        model_id: &str,
        features: &FeatureVector,
    ) -> Result<Prediction> {
        let model = self
            .registry
            .get(model_id)
            .await
            .ok_or_else(|| anyhow::anyhow!("Model '{}' not found", model_id))?;

        model.predict(features)
    }

    /// List available models
    pub async fn list_models(&self) -> Vec<String> {
        self.registry.list_model_ids().await
    }

    /// Remove a model from the registry
    pub async fn unregister_model(&self, model_id: &str) -> bool {
        self.registry.unregister(model_id).await
    }
}

impl Default for InferenceEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Batch Prediction
pub struct BatchPredictor<M: MLModel + 'static> {
    model: Arc<M>,
}

impl<M: MLModel + 'static> BatchPredictor<M> {
    pub fn new(model: M) -> Self {
        Self {
            model: Arc::new(model),
        }
    }

    /// Run batch predictions
    pub fn predict_batch(&self, features: Vec<FeatureVector>) -> Result<Vec<Prediction>> {
        features
            .iter()
            .map(|f| self.model.predict(f))
            .collect()
    }

    /// Run parallel batch predictions
    pub async fn predict_batch_parallel(
        &self,
        features: Vec<FeatureVector>,
    ) -> Result<Vec<Prediction>> {
        let mut handles = Vec::new();

        for feature_vec in features {
            let model = Arc::clone(&self.model);
            let handle = tokio::spawn(async move { model.predict(&feature_vec) });
            handles.push(handle);
        }

        let mut results = Vec::new();
        for handle in handles {
            results.push(handle.await??);
        }

        Ok(results)
    }
}

/// Ensemble Predictor - combines multiple models
pub struct EnsemblePredictor {
    models: Vec<Box<dyn MLModel>>,
    weights: Vec<f64>,
}

impl EnsemblePredictor {
    pub fn new(models: Vec<Box<dyn MLModel>>, weights: Option<Vec<f64>>) -> Result<Self> {
        let weights = weights.unwrap_or_else(|| vec![1.0; models.len()]);

        if weights.len() != models.len() {
            anyhow::bail!("Number of weights must match number of models");
        }

        Ok(Self { models, weights })
    }

    /// Predict using weighted average of all models
    pub fn predict(&self, features: &FeatureVector) -> Result<Prediction> {
        if self.models.is_empty() {
            anyhow::bail!("No models in ensemble");
        }

        let mut weighted_sum = 0.0;
        let weight_sum: f64 = self.weights.iter().sum();

        for (model, weight) in self.models.iter().zip(self.weights.iter()) {
            let pred = model.predict(features)?;
            weighted_sum += pred.value * weight;
        }

        let final_value = weighted_sum / weight_sum;

        Ok(Prediction::new(final_value))
    }
}

/// Model Persistence Module
///
/// Provides functionality to save and load trained models to/from disk.
pub mod persistence {
    use anyhow::Result;
    use std::path::Path;
    use tokio::fs;
    use tracing::{info, warn};

    #[cfg(feature = "ml")]
    use crate::ml::smartcore::SmartcoreRandomForest;

    /// Default model storage directory
    pub const DEFAULT_MODEL_DIR: &str = "/var/lib/oec/models";

    /// Model file naming convention
    pub const CONSUMPTION_MODEL_NAME: &str = "consumption_v1.bin";

    /// Get the full path for the consumption model
    pub fn get_consumption_model_path() -> std::path::PathBuf {
        Path::new(DEFAULT_MODEL_DIR).join(CONSUMPTION_MODEL_NAME)
    }

    /// Ensure model directory exists
    pub async fn ensure_model_directory() -> Result<()> {
        fs::create_dir_all(DEFAULT_MODEL_DIR).await?;
        Ok(())
    }

    /// Save a SmartCore model to disk
    #[cfg(feature = "ml")]
    pub async fn save_model_to_disk(
        mut model: SmartcoreRandomForest,
        path: &Path,
    ) -> Result<()> {
        info!("Saving model to {}", path.display());

        ensure_model_directory().await?;

        let model_dir = Path::new(DEFAULT_MODEL_DIR).canonicalize()
            .unwrap_or_else(|_| Path::new(DEFAULT_MODEL_DIR).to_path_buf());

        if let Some(parent) = path.parent() {
            let canonical_parent = parent.canonicalize()
                .or_else(|_| {
                    std::fs::create_dir_all(parent)?;
                    parent.canonicalize()
                })?;

            if !canonical_parent.starts_with(&model_dir) {
                anyhow::bail!("Security: Model path must be within {}", DEFAULT_MODEL_DIR);
            }
        }

        model.prepare_for_serialization()?;

        let json = serde_json::to_string_pretty(&model)
            .map_err(|e| anyhow::anyhow!("Failed to serialize model to JSON: {}", e))?;

        let json_len = json.len();

        fs::write(path, json).await?;

        info!(
            "Model saved successfully: {} bytes",
            json_len
        );

        Ok(())
    }

    /// Load a SmartCore model from disk
    #[cfg(feature = "ml")]
    pub async fn load_model_from_disk(path: &Path) -> Result<SmartcoreRandomForest> {
        info!("Loading model from {}", path.display());

        let canonical_path = path.canonicalize()
            .map_err(|e| anyhow::anyhow!("Invalid model path: {}", e))?;

        let model_dir = Path::new(DEFAULT_MODEL_DIR).canonicalize()
            .unwrap_or_else(|_| Path::new(DEFAULT_MODEL_DIR).to_path_buf());

        if !canonical_path.starts_with(&model_dir) {
            anyhow::bail!("Security: Model path must be within {}", DEFAULT_MODEL_DIR);
        }

        if !canonical_path.exists() {
            anyhow::bail!("Model file does not exist: {}", canonical_path.display());
        }

        let metadata = fs::metadata(&canonical_path).await?;
        const MAX_MODEL_SIZE: u64 = 100 * 1024 * 1024;
        if metadata.len() > MAX_MODEL_SIZE {
            anyhow::bail!("Model file too large: {} bytes (max {} bytes)", metadata.len(), MAX_MODEL_SIZE);
        }

        let json = fs::read_to_string(&canonical_path).await?;

        let mut model: SmartcoreRandomForest = serde_json::from_str(&json)
            .map_err(|e| anyhow::anyhow!("Failed to deserialize model: {}", e))?;

        model.restore_from_serialization()?;

        info!(
            "Model loaded successfully: trained at {}, {} samples",
            model.metadata.trained_at, model.metadata.training_samples
        );

        Ok(model)
    }

    /// Check if a model file exists
    pub async fn model_exists(path: &Path) -> bool {
        path.exists()
    }

    /// Delete a model file
    pub async fn delete_model(path: &Path) -> Result<()> {
        if path.exists() {
            fs::remove_file(path).await?;
            info!("Deleted model at {}", path.display());
        } else {
            warn!("Model file does not exist: {}", path.display());
        }
        Ok(())
    }

    /// Get model metadata without loading the full model
    #[cfg(feature = "ml")]
    pub async fn get_model_metadata(path: &Path) -> Result<crate::ml::ModelMetadata> {
        let json = fs::read_to_string(path).await?;

        // We only need to deserialize the metadata field
        let model: SmartcoreRandomForest = serde_json::from_str(&json)
            .map_err(|e| anyhow::anyhow!("Failed to deserialize model metadata: {}", e))?;

        Ok(model.metadata)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ml::models::LinearRegressionModel;

    #[tokio::test]
    async fn test_model_registry() {
        let registry = ModelRegistry::new();

        // Initially empty
        assert_eq!(registry.count().await, 0);

        // Register a model
        let model = LinearRegressionModel::dummy_model(3);
        registry
            .register_boxed("test_model".to_string(), Box::new(model))
            .await;

        assert_eq!(registry.count().await, 1);

        let model_ids = registry.list_model_ids().await;
        assert!(model_ids.contains(&"test_model".to_string()));

        // Test get() - should return the model (not None)
        let retrieved_model = registry.get("test_model").await;
        assert!(retrieved_model.is_some(), "Model should be retrievable after registration");

        // Unregister
        let removed = registry.unregister("test_model").await;
        assert!(removed);
        assert_eq!(registry.count().await, 0);

        // After unregister, get() should return None
        let retrieved_after_unregister = registry.get("test_model").await;
        assert!(retrieved_after_unregister.is_none(), "Model should not be retrievable after unregistration");
    }

    #[tokio::test]
    async fn test_inference_engine() {
        let engine = InferenceEngine::new();

        let model = LinearRegressionModel::dummy_model(3);
        engine
            .register_model("model1".to_string(), Box::new(model))
            .await;

        let models = engine.list_models().await;
        assert_eq!(models.len(), 1);
    }

    #[test]
    fn test_batch_predictor() {
        let model = LinearRegressionModel::dummy_model(2);
        let predictor = BatchPredictor::new(model);

        let features = vec![
            FeatureVector::new(vec![1.0, 2.0], vec!["f1".to_string(), "f2".to_string()]).unwrap(),
            FeatureVector::new(vec![3.0, 4.0], vec!["f1".to_string(), "f2".to_string()]).unwrap(),
        ];

        let predictions = predictor.predict_batch(features).unwrap();
        assert_eq!(predictions.len(), 2);
    }
}
