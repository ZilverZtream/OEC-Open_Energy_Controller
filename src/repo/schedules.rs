#![cfg(feature = "db")]

use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ScheduleRow {
    pub id: Uuid,
    pub device_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub valid_from: DateTime<Utc>,
    pub valid_until: DateTime<Utc>,
    pub schedule_json: serde_json::Value,
    pub optimizer_version: String,
    pub cost_savings_estimate: Option<f64>,
}

pub struct ScheduleRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> ScheduleRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    /// Insert a new schedule, automatically invalidating any overlapping schedules
    ///
    /// CRITICAL FIX: Prevents "zombie schedule" overlaps where multiple conflicting
    /// schedules exist for the same device and time window. This ensures the database
    /// query get_current_schedule returns a deterministic result.
    pub async fn insert(&self, schedule: &ScheduleRow) -> Result<Uuid> {
        // Start a transaction to ensure atomicity
        let mut tx = self.pool.begin().await?;

        // CRITICAL: Invalidate all existing schedules that overlap with the new schedule
        // for the same device to prevent conflicting orders in overlapping time windows
        if let Some(device_id) = schedule.device_id {
            sqlx::query!(
                r#"
                UPDATE schedules
                SET valid_until = $1
                WHERE device_id = $2
                  AND valid_until > $1
                  AND (
                    -- New schedule starts during existing schedule
                    ($3 >= valid_from AND $3 < valid_until)
                    -- New schedule ends during existing schedule
                    OR ($4 > valid_from AND $4 <= valid_until)
                    -- New schedule completely contains existing schedule
                    OR ($3 <= valid_from AND $4 >= valid_until)
                  )
                "#,
                schedule.created_at, // Invalidate at creation time of new schedule
                device_id,
                schedule.valid_from,
                schedule.valid_until,
            )
            .execute(&mut *tx)
            .await?;
        }

        // Now insert the new schedule
        let rec = sqlx::query!(
            r#"
            INSERT INTO schedules (id, device_id, created_at, valid_from, valid_until, schedule_json, optimizer_version, cost_savings_estimate)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id
            "#,
            schedule.id,
            schedule.device_id,
            schedule.created_at,
            schedule.valid_from,
            schedule.valid_until,
            schedule.schedule_json,
            schedule.optimizer_version,
            schedule.cost_savings_estimate,
        )
        .fetch_one(&mut *tx)
        .await?;

        // Commit the transaction
        tx.commit().await?;

        Ok(rec.id)
    }

    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<ScheduleRow>> {
        let schedule = sqlx::query_as!(
            ScheduleRow,
            r#"
            SELECT id, device_id, created_at, valid_from, valid_until, schedule_json, optimizer_version, cost_savings_estimate
            FROM schedules
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(self.pool)
        .await?;

        Ok(schedule)
    }

    pub async fn find_active(
        &self,
        device_id: Uuid,
        timestamp: DateTime<Utc>,
    ) -> Result<Option<ScheduleRow>> {
        let schedule = sqlx::query_as!(
            ScheduleRow,
            r#"
            SELECT id, device_id, created_at, valid_from, valid_until, schedule_json, optimizer_version, cost_savings_estimate
            FROM schedules
            WHERE device_id = $1
              AND valid_from <= $2
              AND valid_until >= $2
            ORDER BY created_at DESC
            LIMIT 1
            "#,
            device_id,
            timestamp
        )
        .fetch_optional(self.pool)
        .await?;

        Ok(schedule)
    }

    pub async fn list_for_device(&self, device_id: Uuid, limit: i64) -> Result<Vec<ScheduleRow>> {
        let schedules = sqlx::query_as!(
            ScheduleRow,
            r#"
            SELECT id, device_id, created_at, valid_from, valid_until, schedule_json, optimizer_version, cost_savings_estimate
            FROM schedules
            WHERE device_id = $1
            ORDER BY created_at DESC
            LIMIT $2
            "#,
            device_id,
            limit
        )
        .fetch_all(self.pool)
        .await?;

        Ok(schedules)
    }

    /// Invalidate a schedule by setting its valid_until time to now
    /// This makes the schedule inactive without deleting it
    pub async fn invalidate(&self, id: Uuid) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE schedules
            SET valid_until = NOW()
            WHERE id = $1 AND valid_until > NOW()
            "#,
            id
        )
        .execute(self.pool)
        .await?;

        Ok(())
    }

    pub async fn delete(&self, id: Uuid) -> Result<()> {
        sqlx::query!(
            r#"
            DELETE FROM schedules
            WHERE id = $1
            "#,
            id
        )
        .execute(self.pool)
        .await?;

        Ok(())
    }

    pub async fn cleanup_old_schedules(&self, days: i32) -> Result<u64> {
        let result = sqlx::query!(
            r#"
            DELETE FROM schedules
            WHERE valid_until < NOW() - INTERVAL '1 day' * $1
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
    async fn test_schedule_crud() {
        // This would require database setup
    }
}
