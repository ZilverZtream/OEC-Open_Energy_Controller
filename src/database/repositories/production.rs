use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use tracing::{debug, info};
use uuid::Uuid;

/// Production history database row
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProductionRow {
    pub id: i64,
    pub household_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub power_w: f64,
    pub energy_kwh: f64,
}

/// Repository for solar/renewable production data with statistics
pub struct ProductionRepository {
    pool: PgPool,
}

impl ProductionRepository {
    /// Create a new ProductionRepository
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Insert a single production record
    pub async fn insert(&self, record: &ProductionRow) -> Result<i64> {
        let id = sqlx::query_scalar!(
            r#"
            INSERT INTO production_history (household_id, timestamp, power_w, energy_kwh)
            VALUES ($1, $2, $3, $4)
            RETURNING id
            "#,
            record.household_id,
            record.timestamp,
            record.power_w,
            record.energy_kwh
        )
        .fetch_one(&self.pool)
        .await
        .context("Failed to insert production record")?;

        debug!(
            "Inserted production: {} W at {} for household {}",
            record.power_w, record.timestamp, record.household_id
        );

        Ok(id)
    }

    /// Insert multiple production records in a batch
    pub async fn insert_batch(&self, records: Vec<ProductionRow>) -> Result<()> {
        if records.is_empty() {
            return Ok(());
        }

        let mut tx = self.pool.begin().await.context("Failed to start transaction")?;

        for record in &records {
            sqlx::query!(
                r#"
                INSERT INTO production_history (household_id, timestamp, power_w, energy_kwh)
                VALUES ($1, $2, $3, $4)
                "#,
                record.household_id,
                record.timestamp,
                record.power_w,
                record.energy_kwh
            )
            .execute(&mut *tx)
            .await
            .context("Failed to insert production record in batch")?;
        }

        tx.commit().await.context("Failed to commit transaction")?;

        info!("Inserted {} production records", records.len());

        Ok(())
    }

    /// Find production records within a time range for a household
    pub async fn find_range(
        &self,
        household_id: Uuid,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<ProductionRow>> {
        let records = sqlx::query_as!(
            ProductionRow,
            r#"
            SELECT id, household_id, timestamp, power_w, energy_kwh
            FROM production_history
            WHERE household_id = $1 AND timestamp >= $2 AND timestamp <= $3
            ORDER BY timestamp ASC
            "#,
            household_id,
            start,
            end
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch production records in range")?;

        debug!(
            "Found {} production records between {} and {} for household {}",
            records.len(),
            start,
            end,
            household_id
        );

        Ok(records)
    }

    /// Find the latest production record for a household
    pub async fn find_latest(&self, household_id: Uuid) -> Result<Option<ProductionRow>> {
        let record = sqlx::query_as!(
            ProductionRow,
            r#"
            SELECT id, household_id, timestamp, power_w, energy_kwh
            FROM production_history
            WHERE household_id = $1
            ORDER BY timestamp DESC
            LIMIT 1
            "#,
            household_id
        )
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch latest production record")?;

        Ok(record)
    }

    /// Get hourly average production for a time period
    pub async fn get_hourly_averages(
        &self,
        household_id: Uuid,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<HourlyAverage>> {
        let averages = sqlx::query_as!(
            HourlyAverage,
            r#"
            SELECT
                date_trunc('hour', timestamp) as "hour!",
                AVG(power_w) as "avg_power_w!",
                SUM(energy_kwh) as "total_energy_kwh!",
                COUNT(*) as "sample_count!"
            FROM production_history
            WHERE household_id = $1 AND timestamp >= $2 AND timestamp <= $3
            GROUP BY date_trunc('hour', timestamp)
            ORDER BY hour ASC
            "#,
            household_id,
            start,
            end
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to calculate hourly production averages")?;

        debug!(
            "Calculated {} hourly averages for household {}",
            averages.len(),
            household_id
        );

        Ok(averages)
    }

    /// Get daily average production for a time period
    pub async fn get_daily_averages(
        &self,
        household_id: Uuid,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<DailyAverage>> {
        let averages = sqlx::query_as!(
            DailyAverage,
            r#"
            SELECT
                date_trunc('day', timestamp) as "day!",
                AVG(power_w) as "avg_power_w!",
                MAX(power_w) as "max_power_w!",
                MIN(power_w) as "min_power_w!",
                SUM(energy_kwh) as "total_energy_kwh!",
                COUNT(*) as "sample_count!"
            FROM production_history
            WHERE household_id = $1 AND timestamp >= $2 AND timestamp <= $3
            GROUP BY date_trunc('day', timestamp)
            ORDER BY day ASC
            "#,
            household_id,
            start,
            end
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to calculate daily production averages")?;

        debug!(
            "Calculated {} daily averages for household {}",
            averages.len(),
            household_id
        );

        Ok(averages)
    }

    /// Get daily total energy production (useful for tracking solar yield)
    pub async fn get_daily_totals(
        &self,
        household_id: Uuid,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<DailyTotal>> {
        let totals = sqlx::query_as!(
            DailyTotal,
            r#"
            SELECT
                date_trunc('day', timestamp) as "day!",
                SUM(energy_kwh) as "total_energy_kwh!",
                MAX(power_w) as "peak_power_w!",
                COUNT(*) as "sample_count!"
            FROM production_history
            WHERE household_id = $1 AND timestamp >= $2 AND timestamp <= $3
            GROUP BY date_trunc('day', timestamp)
            ORDER BY day ASC
            "#,
            household_id,
            start,
            end
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to calculate daily production totals")?;

        debug!(
            "Calculated {} daily totals for household {}",
            totals.len(),
            household_id
        );

        Ok(totals)
    }

    /// Get production statistics for a time period
    pub async fn get_statistics(
        &self,
        household_id: Uuid,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<ProductionStatistics> {
        let stats = sqlx::query!(
            r#"
            SELECT
                MIN(power_w) as "min_power",
                MAX(power_w) as "max_power",
                AVG(power_w) as "avg_power",
                SUM(energy_kwh) as "total_energy",
                COUNT(*) as "count"
            FROM production_history
            WHERE household_id = $1 AND timestamp >= $2 AND timestamp <= $3
            "#,
            household_id,
            start,
            end
        )
        .fetch_one(&self.pool)
        .await
        .context("Failed to calculate production statistics")?;

        Ok(ProductionStatistics {
            min_power_w: stats.min_power.unwrap_or(0.0),
            max_power_w: stats.max_power.unwrap_or(0.0),
            avg_power_w: stats.avg_power.unwrap_or(0.0),
            total_energy_kwh: stats.total_energy.unwrap_or(0.0),
            count: stats.count.unwrap_or(0),
        })
    }

    /// Get total energy production for a time period
    pub async fn get_total_energy(
        &self,
        household_id: Uuid,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<f64> {
        let result = sqlx::query!(
            r#"
            SELECT COALESCE(SUM(energy_kwh), 0.0) as "total"
            FROM production_history
            WHERE household_id = $1 AND timestamp >= $2 AND timestamp <= $3
            "#,
            household_id,
            start,
            end
        )
        .fetch_one(&self.pool)
        .await
        .context("Failed to calculate total energy production")?;

        Ok(result.total.unwrap_or(0.0))
    }

    /// Get peak production power for a time period
    pub async fn get_peak_power(
        &self,
        household_id: Uuid,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<f64> {
        let result = sqlx::query!(
            r#"
            SELECT COALESCE(MAX(power_w), 0.0) as "peak"
            FROM production_history
            WHERE household_id = $1 AND timestamp >= $2 AND timestamp <= $3
            "#,
            household_id,
            start,
            end
        )
        .fetch_one(&self.pool)
        .await
        .context("Failed to find peak production power")?;

        Ok(result.peak.unwrap_or(0.0))
    }

    /// Delete production records older than a specified date
    pub async fn delete_older_than(&self, cutoff: DateTime<Utc>) -> Result<u64> {
        let result = sqlx::query!(
            r#"
            DELETE FROM production_history
            WHERE timestamp < $1
            "#,
            cutoff
        )
        .execute(&self.pool)
        .await
        .context("Failed to delete old production records")?;

        info!("Deleted {} old production records", result.rows_affected());

        Ok(result.rows_affected())
    }

    /// Delete all production records for a household
    pub async fn delete_for_household(&self, household_id: Uuid) -> Result<u64> {
        let result = sqlx::query!(
            r#"
            DELETE FROM production_history
            WHERE household_id = $1
            "#,
            household_id
        )
        .execute(&self.pool)
        .await
        .context("Failed to delete household production records")?;

        info!(
            "Deleted {} production records for household {}",
            result.rows_affected(),
            household_id
        );

        Ok(result.rows_affected())
    }

    /// Get the average production by hour of day (for pattern analysis)
    pub async fn get_hourly_pattern(
        &self,
        household_id: Uuid,
        days_back: i64,
    ) -> Result<Vec<HourlyPattern>> {
        let start = Utc::now() - Duration::days(days_back);
        let end = Utc::now();

        let patterns = sqlx::query_as!(
            HourlyPattern,
            r#"
            SELECT
                EXTRACT(HOUR FROM timestamp)::INTEGER as "hour_of_day!",
                AVG(power_w) as "avg_power_w!",
                MAX(power_w) as "max_power_w!",
                STDDEV(power_w) as "stddev_power_w",
                COUNT(*) as "sample_count!"
            FROM production_history
            WHERE household_id = $1 AND timestamp >= $2 AND timestamp <= $3
            GROUP BY EXTRACT(HOUR FROM timestamp)
            ORDER BY hour_of_day
            "#,
            household_id,
            start,
            end
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to calculate hourly production pattern")?;

        Ok(patterns)
    }

    /// Get monthly production summary for a year
    pub async fn get_monthly_summary(
        &self,
        household_id: Uuid,
        year: i32,
    ) -> Result<Vec<MonthlySummary>> {
        let summaries = sqlx::query_as!(
            MonthlySummary,
            r#"
            SELECT
                EXTRACT(MONTH FROM timestamp)::INTEGER as "month!",
                SUM(energy_kwh) as "total_energy_kwh!",
                MAX(power_w) as "peak_power_w!",
                AVG(power_w) as "avg_power_w!",
                COUNT(*) as "sample_count!"
            FROM production_history
            WHERE household_id = $1 AND EXTRACT(YEAR FROM timestamp) = $2
            GROUP BY EXTRACT(MONTH FROM timestamp)
            ORDER BY month
            "#,
            household_id,
            year as i64
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to calculate monthly production summary")?;

        Ok(summaries)
    }
}

/// Hourly average production
#[derive(Debug, Clone, FromRow)]
pub struct HourlyAverage {
    pub hour: DateTime<Utc>,
    pub avg_power_w: f64,
    pub total_energy_kwh: f64,
    pub sample_count: i64,
}

/// Daily average production with min/max
#[derive(Debug, Clone, FromRow)]
pub struct DailyAverage {
    pub day: DateTime<Utc>,
    pub avg_power_w: f64,
    pub max_power_w: f64,
    pub min_power_w: f64,
    pub total_energy_kwh: f64,
    pub sample_count: i64,
}

/// Daily total production (useful for solar yield tracking)
#[derive(Debug, Clone, FromRow)]
pub struct DailyTotal {
    pub day: DateTime<Utc>,
    pub total_energy_kwh: f64,
    pub peak_power_w: f64,
    pub sample_count: i64,
}

/// Production statistics for a time period
#[derive(Debug, Clone)]
pub struct ProductionStatistics {
    pub min_power_w: f64,
    pub max_power_w: f64,
    pub avg_power_w: f64,
    pub total_energy_kwh: f64,
    pub count: i64,
}

/// Hourly pattern for production (by hour of day)
#[derive(Debug, Clone, FromRow)]
pub struct HourlyPattern {
    pub hour_of_day: i32,
    pub avg_power_w: f64,
    pub max_power_w: f64,
    pub stddev_power_w: Option<f64>,
    pub sample_count: i64,
}

/// Monthly production summary
#[derive(Debug, Clone, FromRow)]
pub struct MonthlySummary {
    pub month: i32,
    pub total_energy_kwh: f64,
    pub peak_power_w: f64,
    pub avg_power_w: f64,
    pub sample_count: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_production_statistics_creation() {
        let stats = ProductionStatistics {
            min_power_w: 0.0,
            max_power_w: 8000.0,
            avg_power_w: 3000.0,
            total_energy_kwh: 100.0,
            count: 200,
        };

        assert_eq!(stats.min_power_w, 0.0);
        assert_eq!(stats.max_power_w, 8000.0);
        assert_eq!(stats.avg_power_w, 3000.0);
        assert_eq!(stats.total_energy_kwh, 100.0);
        assert_eq!(stats.count, 200);
    }

    #[test]
    fn test_monthly_summary() {
        let summary = MonthlySummary {
            month: 6,
            total_energy_kwh: 500.0,
            peak_power_w: 7500.0,
            avg_power_w: 2500.0,
            sample_count: 1000,
        };

        assert_eq!(summary.month, 6); // June
        assert_eq!(summary.total_energy_kwh, 500.0);
    }
}
