use crate::database::models::price::ElectricityPriceRow;
use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use sqlx::PgPool;
use tracing::{debug, info};

/// Repository for electricity price data with full CRUD operations
pub struct PriceRepository {
    pool: PgPool,
}

impl PriceRepository {
    /// Create a new PriceRepository
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Insert a single electricity price
    pub async fn insert_price(&self, price: &ElectricityPriceRow) -> Result<i64> {
        let id = sqlx::query_scalar!(
            r#"
            INSERT INTO electricity_prices (timestamp, price_sek_per_kwh, source, area)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (timestamp, area, source) DO UPDATE
            SET price_sek_per_kwh = EXCLUDED.price_sek_per_kwh
            RETURNING id
            "#,
            price.timestamp,
            price.price_sek_per_kwh,
            price.source,
            price.area
        )
        .fetch_one(&self.pool)
        .await
        .context("Failed to insert electricity price")?;

        debug!(
            "Inserted price: {} SEK/kWh at {} for area {}",
            price.price_sek_per_kwh, price.timestamp, price.area
        );

        Ok(id)
    }

    /// Insert multiple electricity prices in a batch
    pub async fn insert_prices(&self, prices: Vec<ElectricityPriceRow>) -> Result<()> {
        if prices.is_empty() {
            return Ok(());
        }

        let mut tx = self.pool.begin().await.context("Failed to start transaction")?;

        for price in &prices {
            sqlx::query!(
                r#"
                INSERT INTO electricity_prices (timestamp, price_sek_per_kwh, source, area)
                VALUES ($1, $2, $3, $4)
                ON CONFLICT (timestamp, area, source) DO UPDATE
                SET price_sek_per_kwh = EXCLUDED.price_sek_per_kwh
                "#,
                price.timestamp,
                price.price_sek_per_kwh,
                price.source,
                price.area
            )
            .execute(&mut *tx)
            .await
            .context("Failed to insert price in batch")?;
        }

        tx.commit().await.context("Failed to commit transaction")?;

        info!("Inserted {} electricity prices", prices.len());

        Ok(())
    }

    /// Find electricity prices within a time range for a specific area
    pub async fn find_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        area: &str,
    ) -> Result<Vec<ElectricityPriceRow>> {
        let prices = sqlx::query_as!(
            ElectricityPriceRow,
            r#"
            SELECT id, timestamp, price_sek_per_kwh, source, area
            FROM electricity_prices
            WHERE timestamp >= $1 AND timestamp <= $2 AND area = $3
            ORDER BY timestamp ASC
            "#,
            start,
            end,
            area
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch electricity prices in range")?;

        debug!(
            "Found {} prices between {} and {} for area {}",
            prices.len(),
            start,
            end,
            area
        );

        Ok(prices)
    }

    /// Find the latest electricity price for a specific area
    pub async fn find_latest(&self, area: &str) -> Result<Option<ElectricityPriceRow>> {
        let price = sqlx::query_as!(
            ElectricityPriceRow,
            r#"
            SELECT id, timestamp, price_sek_per_kwh, source, area
            FROM electricity_prices
            WHERE area = $1
            ORDER BY timestamp DESC
            LIMIT 1
            "#,
            area
        )
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch latest electricity price")?;

        if let Some(ref p) = price {
            debug!(
                "Latest price for area {}: {} SEK/kWh at {}",
                area, p.price_sek_per_kwh, p.timestamp
            );
        }

        Ok(price)
    }

    /// Find a specific price by ID
    pub async fn find_by_id(&self, id: i64) -> Result<Option<ElectricityPriceRow>> {
        let price = sqlx::query_as!(
            ElectricityPriceRow,
            r#"
            SELECT id, timestamp, price_sek_per_kwh, source, area
            FROM electricity_prices
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch electricity price by ID")?;

        Ok(price)
    }

    /// Get the average electricity price over a specific period
    pub async fn get_average_price(&self, period_hours: i64, area: &str) -> Result<f64> {
        let start = Utc::now() - Duration::hours(period_hours);
        let end = Utc::now();

        let result = sqlx::query!(
            r#"
            SELECT AVG(price_sek_per_kwh) as "avg_price"
            FROM electricity_prices
            WHERE timestamp >= $1 AND timestamp <= $2 AND area = $3
            "#,
            start,
            end,
            area
        )
        .fetch_one(&self.pool)
        .await
        .context("Failed to calculate average price")?;

        let avg = result.avg_price.unwrap_or(0.0);

        debug!(
            "Average price for area {} over last {} hours: {} SEK/kWh",
            area, period_hours, avg
        );

        Ok(avg)
    }

    /// Get price statistics (min, max, average) for a time period
    pub async fn get_statistics(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        area: &str,
    ) -> Result<PriceStatistics> {
        let stats = sqlx::query!(
            r#"
            SELECT
                MIN(price_sek_per_kwh) as "min_price",
                MAX(price_sek_per_kwh) as "max_price",
                AVG(price_sek_per_kwh) as "avg_price",
                COUNT(*) as "count"
            FROM electricity_prices
            WHERE timestamp >= $1 AND timestamp <= $2 AND area = $3
            "#,
            start,
            end,
            area
        )
        .fetch_one(&self.pool)
        .await
        .context("Failed to calculate price statistics")?;

        Ok(PriceStatistics {
            min_price: stats.min_price.unwrap_or(0.0),
            max_price: stats.max_price.unwrap_or(0.0),
            avg_price: stats.avg_price.unwrap_or(0.0),
            count: stats.count.unwrap_or(0),
        })
    }

    /// Delete prices older than a specified date (cleanup old data)
    pub async fn delete_older_than(&self, cutoff: DateTime<Utc>) -> Result<u64> {
        let result = sqlx::query!(
            r#"
            DELETE FROM electricity_prices
            WHERE timestamp < $1
            "#,
            cutoff
        )
        .execute(&self.pool)
        .await
        .context("Failed to delete old electricity prices")?;

        info!("Deleted {} old electricity prices", result.rows_affected());

        Ok(result.rows_affected())
    }

    /// Get all unique areas with price data
    pub async fn get_areas(&self) -> Result<Vec<String>> {
        let areas = sqlx::query_scalar!(
            r#"
            SELECT DISTINCT area
            FROM electricity_prices
            ORDER BY area
            "#
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch unique areas")?;

        Ok(areas)
    }

    /// Get all unique sources for a specific area
    pub async fn get_sources_for_area(&self, area: &str) -> Result<Vec<String>> {
        let sources = sqlx::query_scalar!(
            r#"
            SELECT DISTINCT source
            FROM electricity_prices
            WHERE area = $1
            ORDER BY source
            "#,
            area
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch sources for area")?;

        Ok(sources)
    }

    /// Update a price by ID
    pub async fn update_price(&self, id: i64, new_price: f64) -> Result<bool> {
        let result = sqlx::query!(
            r#"
            UPDATE electricity_prices
            SET price_sek_per_kwh = $1
            WHERE id = $2
            "#,
            new_price,
            id
        )
        .execute(&self.pool)
        .await
        .context("Failed to update electricity price")?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete a price by ID
    pub async fn delete_by_id(&self, id: i64) -> Result<bool> {
        let result = sqlx::query!(
            r#"
            DELETE FROM electricity_prices
            WHERE id = $1
            "#,
            id
        )
        .execute(&self.pool)
        .await
        .context("Failed to delete electricity price")?;

        Ok(result.rows_affected() > 0)
    }
}

/// Price statistics for a time period
#[derive(Debug, Clone)]
pub struct PriceStatistics {
    pub min_price: f64,
    pub max_price: f64,
    pub avg_price: f64,
    pub count: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_price_statistics_creation() {
        let stats = PriceStatistics {
            min_price: 0.5,
            max_price: 2.5,
            avg_price: 1.5,
            count: 100,
        };

        assert_eq!(stats.min_price, 0.5);
        assert_eq!(stats.max_price, 2.5);
        assert_eq!(stats.avg_price, 1.5);
        assert_eq!(stats.count, 100);
    }
}
