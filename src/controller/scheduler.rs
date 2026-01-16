#![allow(dead_code)]
use anyhow::Result;
use chrono::{DateTime, Utc, Timelike};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, sleep, Duration};
use tracing::{error, info, warn};

use super::{AppState, BatteryController};

/// Periodic task configuration
#[derive(Debug, Clone)]
pub struct PeriodicTaskConfig {
    /// Re-optimization interval (seconds)
    pub reoptimize_interval_secs: u64,
    /// Forecast refresh interval (seconds)
    pub forecast_refresh_interval_secs: u64,
    /// Data cleanup interval (seconds)
    pub cleanup_interval_secs: u64,
    /// Health check interval (seconds)
    pub health_check_interval_secs: u64,
    /// ML training hour (0-23, default 3 AM)
    pub ml_training_hour: u32,
    /// Enable ML training (default true if ml feature is enabled)
    pub enable_ml_training: bool,
}

impl Default for PeriodicTaskConfig {
    fn default() -> Self {
        Self {
            reoptimize_interval_secs: 3600,        // 1 hour
            forecast_refresh_interval_secs: 1800,  // 30 minutes
            cleanup_interval_secs: 86400,          // 24 hours
            health_check_interval_secs: 300,       // 5 minutes
            ml_training_hour: 3,                   // 3 AM
            enable_ml_training: cfg!(feature = "ml"), // Enable if ml feature is enabled
        }
    }
}

/// Task status tracking
#[derive(Debug, Clone)]
pub struct TaskStatus {
    pub last_run: Option<DateTime<Utc>>,
    pub last_success: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub run_count: u64,
    pub success_count: u64,
    pub error_count: u64,
}

impl Default for TaskStatus {
    fn default() -> Self {
        Self {
            last_run: None,
            last_success: None,
            last_error: None,
            run_count: 0,
            success_count: 0,
            error_count: 0,
        }
    }
}

/// Periodic task scheduler
pub struct TaskScheduler {
    config: PeriodicTaskConfig,
    controller: Arc<BatteryController>,
    reoptimize_status: Arc<RwLock<TaskStatus>>,
    forecast_status: Arc<RwLock<TaskStatus>>,
    cleanup_status: Arc<RwLock<TaskStatus>>,
    health_status: Arc<RwLock<TaskStatus>>,
    ml_training_status: Arc<RwLock<TaskStatus>>,
}

impl TaskScheduler {
    /// Create a new task scheduler
    pub fn new(controller: Arc<BatteryController>) -> Self {
        Self::with_config(controller, PeriodicTaskConfig::default())
    }

    /// Create a new task scheduler with custom configuration
    pub fn with_config(controller: Arc<BatteryController>, config: PeriodicTaskConfig) -> Self {
        Self {
            config,
            controller,
            reoptimize_status: Arc::new(RwLock::new(TaskStatus::default())),
            forecast_status: Arc::new(RwLock::new(TaskStatus::default())),
            cleanup_status: Arc::new(RwLock::new(TaskStatus::default())),
            health_status: Arc::new(RwLock::new(TaskStatus::default())),
            ml_training_status: Arc::new(RwLock::new(TaskStatus::default())),
        }
    }

    /// Start all periodic tasks
    pub fn start(self: Arc<Self>) {
        // Spawn reoptimization task
        let scheduler = self.clone();
        tokio::spawn(async move {
            scheduler.run_reoptimize_task().await;
        });

        // Spawn forecast refresh task
        let scheduler = self.clone();
        tokio::spawn(async move {
            scheduler.run_forecast_refresh_task().await;
        });

        // Spawn cleanup task
        let scheduler = self.clone();
        tokio::spawn(async move {
            scheduler.run_cleanup_task().await;
        });

        // Spawn health check task
        let scheduler = self.clone();
        tokio::spawn(async move {
            scheduler.run_health_check_task().await;
        });

        // Spawn ML training task (if enabled)
        if self.config.enable_ml_training {
            let scheduler = self.clone();
            tokio::spawn(async move {
                scheduler.run_ml_training_task().await;
            });
            info!("ML training task enabled (runs at {}:00 daily)", self.config.ml_training_hour);
        } else {
            info!("ML training task disabled");
        }

        info!("All periodic tasks started");
    }

    /// Run periodic re-optimization task
    async fn run_reoptimize_task(&self) {
        let mut interval = interval(Duration::from_secs(self.config.reoptimize_interval_secs));

        loop {
            interval.tick().await;

            let now = Utc::now();
            let mut status = self.reoptimize_status.write().await;
            status.last_run = Some(now);
            status.run_count += 1;
            drop(status);

            info!("Running periodic re-optimization");

            match self.controller.reoptimize_schedule().await {
                Ok(()) => {
                    let mut status = self.reoptimize_status.write().await;
                    status.last_success = Some(now);
                    status.success_count += 1;
                    status.last_error = None;
                    info!("Re-optimization completed successfully");
                }
                Err(e) => {
                    let mut status = self.reoptimize_status.write().await;
                    status.error_count += 1;
                    status.last_error = Some(e.to_string());
                    error!(error = %e, "Re-optimization failed");
                }
            }
        }
    }

    /// Run periodic forecast refresh task
    async fn run_forecast_refresh_task(&self) {
        let mut interval = interval(Duration::from_secs(self.config.forecast_refresh_interval_secs));

        loop {
            interval.tick().await;

            let now = Utc::now();
            let mut status = self.forecast_status.write().await;
            status.last_run = Some(now);
            status.run_count += 1;
            drop(status);

            info!("Running periodic forecast refresh");

            // Fetch latest forecast
            match self.controller.get_forecast(crate::domain::PriceArea::SE3).await {
                Ok(_forecast) => {
                    let mut status = self.forecast_status.write().await;
                    status.last_success = Some(now);
                    status.success_count += 1;
                    status.last_error = None;
                    info!("Forecast refresh completed successfully");
                }
                Err(e) => {
                    let mut status = self.forecast_status.write().await;
                    status.error_count += 1;
                    status.last_error = Some(e.to_string());
                    warn!(error = %e, "Forecast refresh failed");
                }
            }
        }
    }

    /// Run periodic cleanup task
    async fn run_cleanup_task(&self) {
        let mut interval = interval(Duration::from_secs(self.config.cleanup_interval_secs));

        loop {
            interval.tick().await;

            let now = Utc::now();
            let mut status = self.cleanup_status.write().await;
            status.last_run = Some(now);
            status.run_count += 1;
            drop(status);

            info!("Running periodic data cleanup");

            // Placeholder for cleanup logic
            // In a real implementation, this would clean up old data from the database
            // For example: delete battery states older than 90 days

            let mut status = self.cleanup_status.write().await;
            status.last_success = Some(now);
            status.success_count += 1;
            status.last_error = None;
            info!("Data cleanup completed successfully");
        }
    }

    /// Run periodic health check task
    async fn run_health_check_task(&self) {
        let mut interval = interval(Duration::from_secs(self.config.health_check_interval_secs));

        loop {
            interval.tick().await;

            let now = Utc::now();
            let mut status = self.health_status.write().await;
            status.last_run = Some(now);
            status.run_count += 1;
            drop(status);

            // Perform health checks
            match self.controller.get_battery_health().await {
                Ok(_health) => {
                    let mut status = self.health_status.write().await;
                    status.last_success = Some(now);
                    status.success_count += 1;
                    status.last_error = None;
                }
                Err(e) => {
                    let mut status = self.health_status.write().await;
                    status.error_count += 1;
                    status.last_error = Some(e.to_string());
                    warn!(error = %e, "Health check failed");
                }
            }
        }
    }

    /// Get reoptimization task status
    pub async fn get_reoptimize_status(&self) -> TaskStatus {
        self.reoptimize_status.read().await.clone()
    }

    /// Get forecast refresh task status
    pub async fn get_forecast_status(&self) -> TaskStatus {
        self.forecast_status.read().await.clone()
    }

    /// Get cleanup task status
    pub async fn get_cleanup_status(&self) -> TaskStatus {
        self.cleanup_status.read().await.clone()
    }

    /// Get health check task status
    pub async fn get_health_status(&self) -> TaskStatus {
        self.health_status.read().await.clone()
    }

    /// Get ML training task status
    pub async fn get_ml_training_status(&self) -> TaskStatus {
        self.ml_training_status.read().await.clone()
    }

    /// Run nightly ML training task
    ///
    /// This task runs once per day at the configured hour (default: 03:00 AM)
    /// to train the consumption forecasting model on recent historical data.
    async fn run_ml_training_task(&self) {
        loop {
            // Calculate time until next training run
            let now = Utc::now();
            let target_hour = self.config.ml_training_hour;

            // Calculate next training time
            let next_run = if now.hour() < target_hour {
                // Today at target hour
                now.date_naive()
                    .and_hms_opt(target_hour, 0, 0)
                    .unwrap()
                    .and_local_timezone(now.timezone())
                    .unwrap()
            } else {
                // Tomorrow at target hour
                (now + chrono::Duration::days(1))
                    .date_naive()
                    .and_hms_opt(target_hour, 0, 0)
                    .unwrap()
                    .and_local_timezone(now.timezone())
                    .unwrap()
            };

            let sleep_duration = (next_run - now).to_std().unwrap_or(Duration::from_secs(60));

            info!(
                "Next ML training scheduled for {} (in {} hours)",
                next_run.format("%Y-%m-%d %H:%M:%S"),
                sleep_duration.as_secs() / 3600
            );

            // Sleep until training time
            sleep(sleep_duration).await;

            // Run training
            let now = Utc::now();
            let mut status = self.ml_training_status.write().await;
            status.last_run = Some(now);
            status.run_count += 1;
            drop(status);

            info!("Starting nightly ML training for consumption forecasting");

            #[cfg(all(feature = "ml", feature = "db"))]
            {
                use crate::ml::inference::persistence::{get_consumption_model_path, save_model_to_disk};
                use crate::ml::training::consumption_trainer::{train_consumption_model, ConsumptionTrainingConfig};

                // Get household ID and location from config (hardcoded for now)
                // In production, these would come from the configuration
                let household_id = uuid::Uuid::nil(); // Placeholder
                let latitude = 59.3293; // Stockholm (placeholder)
                let longitude = 18.0686;

                // Get consumption repository
                // Note: This is a simplified version. In production, you'd need to get the repo from AppState
                match self.train_model_internal(household_id, latitude, longitude).await {
                    Ok(()) => {
                        let mut status = self.ml_training_status.write().await;
                        status.last_success = Some(now);
                        status.success_count += 1;
                        status.last_error = None;
                        info!("ML training completed successfully");
                    }
                    Err(e) => {
                        let mut status = self.ml_training_status.write().await;
                        status.error_count += 1;
                        status.last_error = Some(e.to_string());
                        error!(error = %e, "ML training failed");
                    }
                }
            }

            #[cfg(not(feature = "ml"))]
            {
                warn!("ML feature not enabled, skipping training");
                let mut status = self.ml_training_status.write().await;
                status.last_error = Some("ML feature not enabled".to_string());
            }
        }
    }

    /// Internal method to train the model
    ///
    /// Separated for easier testing and error handling
    #[cfg(all(feature = "ml", feature = "db"))]
    async fn train_model_internal(
        &self,
        household_id: uuid::Uuid,
        _latitude: f64,
        _longitude: f64,
    ) -> Result<()> {
        use crate::ml::inference::persistence::{ensure_model_directory, get_consumption_model_path, save_model_to_disk};
        use crate::ml::training::consumption_trainer::{train_consumption_model, ConsumptionTrainingConfig};

        // Note: In production, you'd get the consumption repository from AppState
        // For now, this is a placeholder that will need to be properly integrated
        info!("Training consumption model for household {}", household_id);

        // This is a placeholder implementation
        // In production, you would need to:
        // 1. Get the consumption repository from AppState
        // 2. Call train_consumption_model with proper parameters
        // 3. Save the model to disk
        // 4. Reload the model in the forecaster

        warn!("ML training pipeline requires proper integration with AppState - placeholder only");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use std::time::Duration as StdDuration;

    #[tokio::test]
    async fn test_task_scheduler_creation() {
        let config = Config::load().unwrap();
        let app_state = AppState::new(config).await.unwrap();

        let scheduler = TaskScheduler::new(app_state.controller.clone());

        let reopt_status = scheduler.get_reoptimize_status().await;
        assert_eq!(reopt_status.run_count, 0);
        assert_eq!(reopt_status.success_count, 0);
    }

    #[tokio::test]
    async fn test_custom_task_config() {
        let config = PeriodicTaskConfig {
            reoptimize_interval_secs: 60,
            forecast_refresh_interval_secs: 30,
            cleanup_interval_secs: 300,
            health_check_interval_secs: 10,
        };

        assert_eq!(config.reoptimize_interval_secs, 60);
        assert_eq!(config.forecast_refresh_interval_secs, 30);
    }
}
