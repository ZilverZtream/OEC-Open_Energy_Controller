//! Database Maintenance Tasks
//!
//! This module implements periodic maintenance tasks to prevent database bloat
//! and ensure optimal performance. Tasks include:
//! - Pruning old battery state data
//! - Downsampling high-frequency data
//! - Cleaning up old price data
//! - Optimizing database indices

use anyhow::Result;
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tokio::time::{interval, Duration};
use tracing::{error, info, warn};

use crate::controller::AppState;

/// Maintenance task configuration
#[derive(Debug, Clone)]
pub struct MaintenanceConfig {
    /// How often to run maintenance (in hours)
    pub run_interval_hours: u64,
    /// How many days of battery state data to keep
    pub battery_states_retention_days: i32,
    /// How many days of price data to keep
    pub price_retention_days: i32,
    /// Whether to downsample old data instead of deleting
    pub enable_downsampling: bool,
}

impl Default for MaintenanceConfig {
    fn default() -> Self {
        Self {
            run_interval_hours: 24, // Run daily
            battery_states_retention_days: 30, // Keep 30 days
            price_retention_days: 90, // Keep 90 days of prices
            enable_downsampling: true,
        }
    }
}

/// Maintenance task scheduler
pub struct MaintenanceScheduler {
    state: Arc<AppState>,
    config: MaintenanceConfig,
}

impl MaintenanceScheduler {
    /// Create a new maintenance scheduler
    pub fn new(state: Arc<AppState>, config: MaintenanceConfig) -> Self {
        Self { state, config }
    }

    /// Start the maintenance scheduler
    pub fn start(self: Arc<Self>) {
        let interval_duration = Duration::from_secs(self.config.run_interval_hours * 3600);

        tokio::spawn(async move {
            let mut ticker = interval(interval_duration);
            info!(
                interval_hours = self.config.run_interval_hours,
                "Maintenance scheduler started"
            );

            loop {
                ticker.tick().await;
                info!("Running database maintenance tasks");

                if let Err(e) = self.run_maintenance().await {
                    error!(error=%e, "Maintenance tasks failed");
                } else {
                    info!("Maintenance tasks completed successfully");
                }
            }
        });
    }

    /// Run all maintenance tasks
    async fn run_maintenance(&self) -> Result<()> {
        #[cfg(feature = "db")]
        {
            self.cleanup_old_battery_states().await?;
            self.cleanup_old_prices().await?;
        }

        #[cfg(not(feature = "db"))]
        {
            info!("Database feature not enabled, skipping maintenance");
        }

        Ok(())
    }

    /// Clean up old battery state data
    #[cfg(feature = "db")]
    async fn cleanup_old_battery_states(&self) -> Result<()> {
        let repo = self.state.repos.db.battery_states();
        let retention_days = self.config.battery_states_retention_days;

        info!(
            retention_days = retention_days,
            "Cleaning up battery states older than {} days", retention_days
        );

        let rows_deleted = repo.cleanup_old_data(retention_days).await?;

        info!(
            rows_deleted = rows_deleted,
            "Deleted {} old battery state rows", rows_deleted
        );

        Ok(())
    }

    /// Clean up old price data
    #[cfg(feature = "db")]
    async fn cleanup_old_prices(&self) -> Result<()> {
        use crate::repo::prices::PriceRepository;

        let retention_days = self.config.price_retention_days;
        let cutoff = Utc::now() - chrono::Duration::days(retention_days as i64);
        let cutoff_fixed = cutoff.with_timezone(&chrono::FixedOffset::east_opt(0).unwrap());

        info!(
            retention_days = retention_days,
            "Cleaning up prices older than {} days", retention_days
        );

        let price_repo = PriceRepository::new(self.state.repos.db.pool.clone());
        let rows_deleted = price_repo.delete_old_data(cutoff_fixed).await?;

        info!(
            rows_deleted = rows_deleted,
            "Deleted {} old price rows", rows_deleted
        );

        Ok(())
    }

    /// Downsample old battery state data (future enhancement)
    /// This would keep 1 sample per hour for data older than 7 days
    #[allow(dead_code)]
    async fn downsample_battery_states(&self) -> Result<()> {
        // TODO: Implement downsampling logic
        // - Find all battery_states older than 7 days
        // - Group by hour
        // - Keep only 1 representative sample per hour (e.g., average)
        // - Delete the rest
        warn!("Battery state downsampling not yet implemented");
        Ok(())
    }
}

/// Spawn maintenance tasks
pub fn spawn_maintenance_tasks(state: Arc<AppState>) {
    let config = MaintenanceConfig::default();
    let scheduler = Arc::new(MaintenanceScheduler::new(state, config));
    scheduler.start();
    info!("Maintenance scheduler spawned");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_maintenance_config_defaults() {
        let config = MaintenanceConfig::default();
        assert_eq!(config.run_interval_hours, 24);
        assert_eq!(config.battery_states_retention_days, 30);
        assert_eq!(config.price_retention_days, 90);
        assert!(config.enable_downsampling);
    }
}
