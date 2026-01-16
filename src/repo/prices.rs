//! Price repository for electricity price data

use anyhow::Result;
use chrono::{DateTime, FixedOffset};
use sqlx::PgPool;

use crate::domain::{PriceArea, PricePoint};

/// Repository for electricity price data
pub struct PriceRepository {
    pool: PgPool,
}

impl PriceRepository {
    /// Create a new price repository
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Insert multiple price points
    pub async fn insert_prices(&self, prices: Vec<PricePoint>, area: PriceArea) -> Result<()> {
        for price in prices {
            sqlx::query!(
                r#"
                INSERT INTO electricity_prices (timestamp, price_sek_per_kwh, source, area)
                VALUES ($1, $2, 'nordpool', $3)
                ON CONFLICT (timestamp, area, source) DO UPDATE
                SET price_sek_per_kwh = EXCLUDED.price_sek_per_kwh
                "#,
                price.time_start,
                price.price_sek_per_kwh,
                area.to_string()
            )
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    /// Find prices in a time range for a specific area
    pub async fn find_range(
        &self,
        start: DateTime<FixedOffset>,
        end: DateTime<FixedOffset>,
        area: PriceArea,
    ) -> Result<Vec<PricePoint>> {
        let rows = sqlx::query!(
            r#"
            SELECT timestamp, price_sek_per_kwh
            FROM electricity_prices
            WHERE timestamp >= $1 AND timestamp < $2 AND area = $3
            ORDER BY timestamp ASC
            "#,
            start,
            end,
            area.to_string()
        )
        .fetch_all(&self.pool)
        .await?;

        let points = rows
            .into_iter()
            .map(|row| PricePoint {
                time_start: row.timestamp.into(),
                time_end: (row.timestamp + chrono::Duration::hours(1)).into(),
                price_sek_per_kwh: row.price_sek_per_kwh,
                export_price_sek_per_kwh: None, // Use default (40% of import price)
            })
            .collect();

        Ok(points)
    }

    /// Find the latest price for an area
    pub async fn find_latest(&self, area: PriceArea) -> Result<Option<PricePoint>> {
        let row = sqlx::query!(
            r#"
            SELECT timestamp, price_sek_per_kwh
            FROM electricity_prices
            WHERE area = $1
            ORDER BY timestamp DESC
            LIMIT 1
            "#,
            area.to_string()
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| PricePoint {
            time_start: r.timestamp.into(),
            time_end: (r.timestamp + chrono::Duration::hours(1)).into(),
            price_sek_per_kwh: r.price_sek_per_kwh,
            export_price_sek_per_kwh: None, // Use default (40% of import price)
        }))
    }

    /// Get average price for a period
    pub async fn get_average_price(
        &self,
        start: DateTime<FixedOffset>,
        end: DateTime<FixedOffset>,
        area: PriceArea,
    ) -> Result<f64> {
        let row = sqlx::query!(
            r#"
            SELECT AVG(price_sek_per_kwh) as avg_price
            FROM electricity_prices
            WHERE timestamp >= $1 AND timestamp < $2 AND area = $3
            "#,
            start,
            end,
            area.to_string()
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(row.avg_price.unwrap_or(0.0))
    }

    /// Delete old price data (older than specified date)
    pub async fn delete_old_data(&self, before: DateTime<FixedOffset>) -> Result<u64> {
        let result = sqlx::query!(
            r#"
            DELETE FROM electricity_prices
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
    use chrono::Utc;

    #[test]
    fn test_price_repository_creation() {
        // Test is a placeholder - actual tests would require a test database
        assert!(true);
    }
}
