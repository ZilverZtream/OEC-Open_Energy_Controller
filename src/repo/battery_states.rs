#![cfg(feature = "db")]

use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BatteryStateRow {
    pub id: i64,
    pub device_id: Option<Uuid>,
    pub timestamp: DateTime<Utc>,
    pub soc_percent: f64,
    pub power_w: f64,
    pub voltage_v: Option<f64>,
    pub temperature_c: Option<f64>,
}

pub struct BatteryStateRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> BatteryStateRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, state: &BatteryStateRow) -> Result<i64> {
        let rec = sqlx::query!(
            r#"
            INSERT INTO battery_states (device_id, timestamp, soc_percent, power_w, voltage_v, temperature_c)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id
            "#,
            state.device_id,
            state.timestamp,
            state.soc_percent,
            state.power_w,
            state.voltage_v,
            state.temperature_c,
        )
        .fetch_one(self.pool)
        .await?;

        Ok(rec.id)
    }

    pub async fn insert_batch(&self, states: &[BatteryStateRow]) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        for state in states {
            sqlx::query!(
                r#"
                INSERT INTO battery_states (device_id, timestamp, soc_percent, power_w, voltage_v, temperature_c)
                VALUES ($1, $2, $3, $4, $5, $6)
                "#,
                state.device_id,
                state.timestamp,
                state.soc_percent,
                state.power_w,
                state.voltage_v,
                state.temperature_c,
            )
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    pub async fn find_latest(&self, device_id: Uuid) -> Result<Option<BatteryStateRow>> {
        let state = sqlx::query_as!(
            BatteryStateRow,
            r#"
            SELECT id, device_id, timestamp, soc_percent, power_w, voltage_v, temperature_c
            FROM battery_states
            WHERE device_id = $1
            ORDER BY timestamp DESC
            LIMIT 1
            "#,
            device_id
        )
        .fetch_optional(self.pool)
        .await?;

        Ok(state)
    }

    pub async fn find_range(
        &self,
        device_id: Uuid,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<BatteryStateRow>> {
        let states = sqlx::query_as!(
            BatteryStateRow,
            r#"
            SELECT id, device_id, timestamp, soc_percent, power_w, voltage_v, temperature_c
            FROM battery_states
            WHERE device_id = $1
              AND timestamp >= $2
              AND timestamp <= $3
            ORDER BY timestamp ASC
            "#,
            device_id,
            start,
            end
        )
        .fetch_all(self.pool)
        .await?;

        Ok(states)
    }

    pub async fn get_average_soc(&self, device_id: Uuid, hours: i32) -> Result<Option<f64>> {
        let rec = sqlx::query!(
            r#"
            SELECT AVG(soc_percent) as avg_soc
            FROM battery_states
            WHERE device_id = $1
              AND timestamp >= NOW() - INTERVAL '1 hour' * $2
            "#,
            device_id,
            hours
        )
        .fetch_one(self.pool)
        .await?;

        Ok(rec.avg_soc)
    }

    pub async fn cleanup_old_data(&self, days: i32) -> Result<u64> {
        let result = sqlx::query!(
            r#"
            DELETE FROM battery_states
            WHERE timestamp < NOW() - INTERVAL '1 day' * $1
            "#,
            days
        )
        .execute(self.pool)
        .await?;

        Ok(result.rows_affected())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "requires database"]
    async fn test_battery_state_crud() {
        // This would require database setup
    }
}
