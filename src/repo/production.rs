//! Production repository for solar/renewable energy production data

use anyhow::Result;
use chrono::{DateTime, FixedOffset};
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::ProductionPoint;

/// Repository for production history data
pub struct ProductionRepository {
    pool: PgPool,
}

impl ProductionRepository {
    /// Create a new production repository
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Insert a production data point
    pub async fn insert(
        &self,
        household_id: Uuid,
        timestamp: DateTime<FixedOffset>,
        power_w: f64,
        energy_kwh: f64,
    ) -> Result<i64> {
        let row = sqlx::query!(
            r#"
            INSERT INTO production_history (household_id, timestamp, power_w, energy_kwh)
            VALUES ($1, $2, $3, $4)
            RETURNING id
            "#,
            household_id,
            timestamp,
            power_w,
            energy_kwh
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(row.id)
    }

    /// Insert multiple production points
    pub async fn insert_batch(
        &self,
        household_id: Uuid,
        points: Vec<ProductionPoint>,
    ) -> Result<()> {
        for point in points {
            let energy_kwh = point.pv_kw * 1.0; // Assume 1-hour intervals
            self.insert(household_id, point.time_start, point.pv_kw * 1000.0, energy_kwh)
                .await?;
        }

        Ok(())
    }

    /// Find production data in a time range
    pub async fn find_range(
        &self,
        household_id: Uuid,
        start: DateTime<FixedOffset>,
        end: DateTime<FixedOffset>,
    ) -> Result<Vec<ProductionPoint>> {
        let rows = sqlx::query!(
            r#"
            SELECT timestamp, power_w
            FROM production_history
            WHERE household_id = $1 AND timestamp >= $2 AND timestamp < $3
            ORDER BY timestamp ASC
            "#,
            household_id,
            start,
            end
        )
        .fetch_all(&self.pool)
        .await?;

        let points = rows
            .into_iter()
            .map(|row| ProductionPoint {
                time_start: row.timestamp.into(),
                time_end: (row.timestamp + chrono::Duration::hours(1)).into(),
                pv_kw: row.power_w / 1000.0,
            })
            .collect();

        Ok(points)
    }

    /// Get average production for a period
    pub async fn get_average_production(
        &self,
        household_id: Uuid,
        start: DateTime<FixedOffset>,
        end: DateTime<FixedOffset>,
    ) -> Result<f64> {
        let row = sqlx::query!(
            r#"
            SELECT AVG(power_w) as avg_power
            FROM production_history
            WHERE household_id = $1 AND timestamp >= $2 AND timestamp < $3
            "#,
            household_id,
            start,
            end
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(row.avg_power.unwrap_or(0.0))
    }

    /// Get hourly aggregated production
    pub async fn get_hourly_aggregation(
        &self,
        household_id: Uuid,
        start: DateTime<FixedOffset>,
        end: DateTime<FixedOffset>,
    ) -> Result<Vec<ProductionPoint>> {
        let rows = sqlx::query!(
            r#"
            SELECT
                date_trunc('hour', timestamp) as hour,
                AVG(power_w) as avg_power
            FROM production_history
            WHERE household_id = $1 AND timestamp >= $2 AND timestamp < $3
            GROUP BY hour
            ORDER BY hour ASC
            "#,
            household_id,
            start,
            end
        )
        .fetch_all(&self.pool)
        .await?;

        let points = rows
            .into_iter()
            .filter_map(|row| {
                row.hour.map(|h| ProductionPoint {
                    time_start: h.and_utc().into(),
                    time_end: (h + chrono::Duration::hours(1)).and_utc().into(),
                    pv_kw: row.avg_power.unwrap_or(0.0) / 1000.0,
                })
            })
            .collect();

        Ok(points)
    }

    /// Get daily total energy production
    pub async fn get_daily_total(
        &self,
        household_id: Uuid,
        date: DateTime<FixedOffset>,
    ) -> Result<f64> {
        let start = date.date_naive().and_hms_opt(0, 0, 0).unwrap();
        let end = start + chrono::Duration::days(1);

        let row = sqlx::query!(
            r#"
            SELECT SUM(energy_kwh) as total_energy
            FROM production_history
            WHERE household_id = $1
            AND timestamp >= $2
            AND timestamp < $3
            "#,
            household_id,
            start.and_utc(),
            end.and_utc()
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(row.total_energy.unwrap_or(0.0))
    }

    /// Delete old production data (older than specified date)
    pub async fn delete_old_data(&self, before: DateTime<FixedOffset>) -> Result<u64> {
        let result = sqlx::query!(
            r#"
            DELETE FROM production_history
            WHERE timestamp < $1
            "#,
            before
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_production_repository_creation() {
        // Test is a placeholder - actual tests would require a test database
        assert!(true);
    }
}
