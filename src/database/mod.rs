#![cfg(feature = "db")]

pub mod models;
pub mod repositories;

use anyhow::{Context, Result};
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::time::Duration;
use tracing::{info, warn};

/// Database configuration
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub acquire_timeout_secs: u64,
    pub idle_timeout_secs: u64,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "postgres://localhost/energy_controller".to_string(),
            max_connections: 10,
            min_connections: 2,
            acquire_timeout_secs: 30,
            idle_timeout_secs: 600,
        }
    }
}

impl DatabaseConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self> {
        let url = std::env::var("DATABASE_URL")
            .context("DATABASE_URL environment variable not set")?;

        let max_connections = std::env::var("DB_MAX_CONNECTIONS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(10);

        let min_connections = std::env::var("DB_MIN_CONNECTIONS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(2);

        let acquire_timeout_secs = std::env::var("DB_ACQUIRE_TIMEOUT_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(30);

        let idle_timeout_secs = std::env::var("DB_IDLE_TIMEOUT_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(600);

        Ok(Self {
            url,
            max_connections,
            min_connections,
            acquire_timeout_secs,
            idle_timeout_secs,
        })
    }
}

/// Database connection pool with retry logic and health checks
pub struct Database {
    pool: PgPool,
}

impl Database {
    /// Create a new database connection pool
    pub async fn new(config: &DatabaseConfig) -> Result<Self> {
        info!("Initializing database connection pool");

        let pool = Self::connect_with_retry(config, 5).await?;

        // Perform health check
        Self::health_check(&pool).await?;

        info!("Database connection pool initialized successfully");
        Ok(Self { pool })
    }

    /// Connect with exponential backoff retry logic
    async fn connect_with_retry(config: &DatabaseConfig, max_attempts: usize) -> Result<PgPool> {
        let mut attempt = 0;
        let mut delay = Duration::from_secs(1);

        loop {
            attempt += 1;
            match Self::try_connect(config).await {
                Ok(pool) => return Ok(pool),
                Err(e) if attempt >= max_attempts => {
                    return Err(e).context(format!(
                        "Failed to connect to database after {} attempts",
                        max_attempts
                    ));
                }
                Err(e) => {
                    warn!(
                        "Database connection attempt {}/{} failed: {}. Retrying in {:?}",
                        attempt, max_attempts, e, delay
                    );
                    tokio::time::sleep(delay).await;
                    delay *= 2; // Exponential backoff
                }
            }
        }
    }

    /// Try to establish a database connection
    async fn try_connect(config: &DatabaseConfig) -> Result<PgPool> {
        let pool = PgPoolOptions::new()
            .max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .acquire_timeout(Duration::from_secs(config.acquire_timeout_secs))
            .idle_timeout(Duration::from_secs(config.idle_timeout_secs))
            .connect(&config.url)
            .await
            .context("Failed to create database pool")?;

        Ok(pool)
    }

    /// Perform a health check query
    pub async fn health_check(&self) -> Result<()> {
        Self::health_check(&self.pool).await
    }

    /// Static health check that can be used during initialization
    async fn health_check(pool: &PgPool) -> Result<()> {
        sqlx::query("SELECT 1")
            .fetch_one(pool)
            .await
            .context("Database health check failed")?;
        Ok(())
    }

    /// Get the underlying connection pool
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Gracefully close the database connection pool
    pub async fn close(self) {
        info!("Closing database connection pool");
        self.pool.close().await;
        info!("Database connection pool closed");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_config_default() {
        let config = DatabaseConfig::default();
        assert_eq!(config.max_connections, 10);
        assert_eq!(config.min_connections, 2);
        assert_eq!(config.acquire_timeout_secs, 30);
        assert_eq!(config.idle_timeout_secs, 600);
    }

    #[test]
    fn test_database_config_from_env_missing_url() {
        // This should fail when DATABASE_URL is not set
        std::env::remove_var("DATABASE_URL");
        let result = DatabaseConfig::from_env();
        assert!(result.is_err());
    }

    #[test]
    fn test_database_config_from_env_with_url() {
        std::env::set_var("DATABASE_URL", "postgres://localhost/test");
        std::env::set_var("DB_MAX_CONNECTIONS", "20");

        let config = DatabaseConfig::from_env().unwrap();
        assert_eq!(config.url, "postgres://localhost/test");
        assert_eq!(config.max_connections, 20);

        // Cleanup
        std::env::remove_var("DATABASE_URL");
        std::env::remove_var("DB_MAX_CONNECTIONS");
    }
}
