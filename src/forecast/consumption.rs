#![allow(dead_code)]
use anyhow::Result;
use async_trait::async_trait;
use chrono::{Utc, TimeZone, Timelike};
use uuid::Uuid;

use crate::domain::ConsumptionPoint;

#[cfg(feature = "ml")]
use crate::forecast::features::{normalize_features_cyclical, FeatureExtractor};
#[cfg(feature = "ml")]
use crate::ml::inference::persistence::{get_consumption_model_path, load_model_from_disk};
#[cfg(feature = "ml")]
use crate::ml::models::MLModel;
#[cfg(feature = "ml")]
use crate::ml::smartcore::SmartcoreRandomForest;
#[cfg(feature = "ml")]
use crate::ml::FeatureVector;
#[cfg(feature = "ml")]
use std::sync::Arc;
#[cfg(feature = "ml")]
use tokio::sync::RwLock;
#[cfg(feature = "ml")]
use tracing::{error, info, warn};

#[async_trait]
pub trait ConsumptionForecaster: Send + Sync {
    async fn predict_next_24h(&self, household_id: Uuid) -> Result<Vec<ConsumptionPoint>>;
}

pub struct SimpleConsumptionForecaster;

#[async_trait]
impl ConsumptionForecaster for SimpleConsumptionForecaster {
    async fn predict_next_24h(&self, _household_id: Uuid) -> Result<Vec<ConsumptionPoint>> {
        let now = Utc::now();
        let start = Utc
            .with_ymd_and_hms(now.year(), now.month(), now.day(), 0, 0, 0)
            .unwrap();

        let mut out = Vec::with_capacity(24);
        for h in 0..24 {
            let t0 = start + chrono::Duration::hours(h);
            let t1 = start + chrono::Duration::hours(h + 1);
            let hh = t0.hour() as f64;

            let base = 0.6;
            let morning = bump(hh, 7.5, 1.5) * 1.0;
            let evening = bump(hh, 18.5, 2.0) * 1.6;

            out.push(ConsumptionPoint {
                time_start: t0,
                time_end: t1,
                load_kw: (base + morning + evening).max(0.2),
            });
        }
        Ok(out)
    }
}

/// ML-Enhanced Consumption Forecaster
///
/// Uses a trained RandomForest model for predictions, with automatic fallback
/// to the simple baseline model if the ML model is unavailable or produces invalid results.
#[cfg(feature = "ml")]
pub struct MLConsumptionForecaster {
    /// Trained ML model (loaded from disk on startup)
    model: Arc<RwLock<Option<SmartcoreRandomForest>>>,
    /// Fallback forecaster
    fallback: SimpleConsumptionForecaster,
    /// Feature extractor
    feature_extractor: FeatureExtractor,
}

#[cfg(feature = "ml")]
impl MLConsumptionForecaster {
    /// Create a new ML-enhanced forecaster
    ///
    /// Will attempt to load the trained model from disk on creation.
    pub async fn new(latitude: f64, longitude: f64) -> Self {
        let model_path = get_consumption_model_path();

        let model = if model_path.exists() {
            info!("Loading consumption model from {}", model_path.display());
            match load_model_from_disk(&model_path).await {
                Ok(m) => {
                    info!(
                        "Successfully loaded ML model: trained at {}, R2={:.3}",
                        m.metadata.trained_at,
                        m.metadata.validation_metrics.r2
                    );
                    Some(m)
                }
                Err(e) => {
                    error!("Failed to load ML model: {}. Using fallback.", e);
                    None
                }
            }
        } else {
            info!(
                "No trained model found at {}. Using fallback forecaster.",
                model_path.display()
            );
            None
        };

        Self {
            model: Arc::new(RwLock::new(model)),
            fallback: SimpleConsumptionForecaster,
            feature_extractor: FeatureExtractor::new(latitude, longitude),
        }
    }

    /// Reload the model from disk
    ///
    /// This should be called after a new model is trained and saved.
    pub async fn reload_model(&self) -> Result<()> {
        let model_path = get_consumption_model_path();

        if !model_path.exists() {
            anyhow::bail!("Model file does not exist: {}", model_path.display());
        }

        info!("Reloading consumption model from {}", model_path.display());

        let new_model = load_model_from_disk(&model_path).await?;

        info!(
            "Successfully reloaded ML model: trained at {}, R2={:.3}",
            new_model.metadata.trained_at,
            new_model.metadata.validation_metrics.r2
        );

        let mut model = self.model.write().await;
        *model = Some(new_model);

        Ok(())
    }

    /// Predict using ML model
    async fn predict_with_ml(&self, timestamp: chrono::DateTime<Utc>) -> Option<f64> {
        let model_guard = self.model.read().await;
        let model = model_guard.as_ref()?;

        // Extract features
        let temporal_features = self
            .feature_extractor
            .extract_temporal_features(timestamp.into());
        let normalized = normalize_features_cyclical(&temporal_features);

        // Create feature vector
        let feature_vector = match FeatureVector::new(
            normalized,
            model.metadata.feature_names.clone(),
        ) {
            Ok(fv) => fv,
            Err(e) => {
                error!("Failed to create feature vector: {}", e);
                return None;
            }
        };

        // Predict
        match model.predict(&feature_vector) {
            Ok(prediction) => {
                let value = prediction.value;

                // Sanity checks
                if value < 0.0 || value > 100.0 {
                    warn!(
                        "ML model predicted unreasonable value: {:.2} kW. Using fallback.",
                        value
                    );
                    return None;
                }

                Some(value)
            }
            Err(e) => {
                error!("ML prediction failed: {}. Using fallback.", e);
                None
            }
        }
    }
}

#[cfg(feature = "ml")]
#[async_trait]
impl ConsumptionForecaster for MLConsumptionForecaster {
    async fn predict_next_24h(&self, _household_id: Uuid) -> Result<Vec<ConsumptionPoint>> {
        let now = Utc::now();
        let start = Utc
            .with_ymd_and_hms(now.year(), now.month(), now.day(), 0, 0, 0)
            .unwrap();

        let has_model = self.model.read().await.is_some();

        let mut out = Vec::with_capacity(24);

        for h in 0..24 {
            let t0 = start + chrono::Duration::hours(h);
            let t1 = start + chrono::Duration::hours(h + 1);

            // Try ML prediction first
            let load_kw = if has_model {
                match self.predict_with_ml(t0).await {
                    Some(value) => value,
                    None => {
                        // Fallback to simple model
                        let hh = t0.hour() as f64;
                        let base = 0.6;
                        let morning = bump(hh, 7.5, 1.5) * 1.0;
                        let evening = bump(hh, 18.5, 2.0) * 1.6;
                        (base + morning + evening).max(0.2)
                    }
                }
            } else {
                // No model available, use simple baseline
                let hh = t0.hour() as f64;
                let base = 0.6;
                let morning = bump(hh, 7.5, 1.5) * 1.0;
                let evening = bump(hh, 18.5, 2.0) * 1.6;
                (base + morning + evening).max(0.2)
            };

            out.push(ConsumptionPoint {
                time_start: t0,
                time_end: t1,
                load_kw,
            });
        }

        Ok(out)
    }
}

use chrono::Datelike;
fn bump(x: f64, mu: f64, sigma: f64) -> f64 {
    let z = (x - mu) / sigma.max(0.01);
    (-0.5 * z * z).exp()
}
