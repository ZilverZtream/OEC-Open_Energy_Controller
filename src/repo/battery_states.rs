#![cfg(feature = "db")]
//! # Battery State Repository
//!
//! ## CRITICAL: Flash Storage Wear Prevention (Issue #7)
//!
//! **Problem**: High-frequency database writes (1Hz or 1-minute resolution) will destroy
//! SD cards on Raspberry Pi within 6-12 months due to write endurance limits.
//!
//! **Solution**:
//! 1. **Batch writes**: Use `insert_batch()` to write multiple states at once (every 15 minutes)
//! 2. **Coalesce updates**: Don't write every state change - aggregate in memory
//! 3. **Flush on shutdown**: Use SIGTERM handler to ensure pending data is written
//! 4. **Consider WAL mode**: PostgreSQL WAL can reduce writes, but still batch inserts
//!
//! ## Recommended Write Frequency
//!
//! | Resolution | IOPS/day | SD Card Lifespan | Recommended? |
//! |-----------|----------|------------------|--------------|
//! | 1 second  | 86,400   | 3-6 months       | ❌ NO        |
//! | 1 minute  | 1,440    | 6-12 months      | ❌ NO        |
//! | 5 minutes | 288      | 3-5 years        | ✅ OK        |
//! | 15 minutes| 96       | 10+ years        | ✅ BEST      |
//!
//! ## Example: Batched Writes
//!
//! ```ignore
//! use std::collections::VecDeque;
//! use std::time::{Duration, Instant};
//!
//! struct BatteryStateBuffer {
//!     states: VecDeque<BatteryStateRow>,
//!     last_flush: Instant,
//!     flush_interval: Duration,
//! }
//!
//! impl BatteryStateBuffer {
//!     fn new() -> Self {
//!         Self {
//!             states: VecDeque::new(),
//!             last_flush: Instant::now(),
//!             flush_interval: Duration::from_secs(15 * 60),  // 15 minutes
//!         }
//!     }
//!
//!     fn push(&mut self, state: BatteryStateRow) {
//!         self.states.push_back(state);
//!     }
//!
//!     async fn maybe_flush(&mut self, repo: &BatteryStateRepository<'_>) -> Result<()> {
//!         if self.last_flush.elapsed() >= self.flush_interval && !self.states.is_empty() {
//!             let states: Vec<_> = self.states.drain(..).collect();
//!             repo.insert_batch(&states).await?;
//!             self.last_flush = Instant::now();
//!         }
//!         Ok(())
//!     }
//!
//!     async fn force_flush(&mut self, repo: &BatteryStateRepository<'_>) -> Result<()> {
//!         if !self.states.is_empty() {
//!             let states: Vec<_> = self.states.drain(..).collect();
//!             repo.insert_batch(&states).await?;
//!             self.last_flush = Instant::now();
//!         }
//!         Ok(())
//!     }
//! }
//!
//! // In your main control loop:
//! let mut buffer = BatteryStateBuffer::new();
//!
//! loop {
//!     let state = read_battery_state();
//!     buffer.push(state);
//!     buffer.maybe_flush(&repo).await?;
//!
//!     tokio::time::sleep(Duration::from_secs(60)).await;
//! }
//!
//! // On shutdown (SIGTERM):
//! buffer.force_flush(&repo).await?;
//! ```

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
