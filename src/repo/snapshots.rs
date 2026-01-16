#![cfg(feature = "db")]

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

/// Power flow snapshot data structure
#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct PowerFlowSnapshotRow {
    pub id: i64,
    pub timestamp: DateTime<Utc>,

    // Power flow values (kW)
    pub pv_production_kw: f64,
    pub house_load_kw: f64,
    pub battery_power_kw: f64,
    pub ev_charger_power_kw: f64,
    pub grid_import_kw: f64,
    pub grid_export_kw: f64,

    // Battery state
    pub battery_soc_percent: Option<f64>,
    pub battery_temperature_c: Option<f64>,

    // Grid state
    pub grid_frequency_hz: Option<f64>,
    pub grid_voltage_v: Option<f64>,
    pub grid_available: bool,

    // Constraints
    pub constraints_version: Option<String>,
    pub fuse_limit_a: Option<f64>,

    // Decision metadata
    pub control_mode: Option<String>,
    pub decision_reason: Option<String>,

    // Economic metrics
    pub spot_price_sek_per_kwh: Option<f64>,
    pub estimated_cost_sek: Option<f64>,

    // Optimization metrics
    pub schedule_id: Option<Uuid>,
    pub deviation_from_schedule_kw: Option<f64>,
}

/// Input data for creating a new power flow snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerFlowSnapshotInput {
    // Power flow values (kW)
    pub pv_production_kw: f64,
    pub house_load_kw: f64,
    pub battery_power_kw: f64,
    pub ev_charger_power_kw: f64,
    pub grid_import_kw: f64,
    pub grid_export_kw: f64,

    // Battery state
    pub battery_soc_percent: Option<f64>,
    pub battery_temperature_c: Option<f64>,

    // Grid state
    pub grid_frequency_hz: Option<f64>,
    pub grid_voltage_v: Option<f64>,
    pub grid_available: bool,

    // Constraints
    pub constraints_version: Option<String>,
    pub fuse_limit_a: Option<f64>,

    // Decision metadata
    pub control_mode: Option<String>,
    pub decision_reason: Option<String>,

    // Economic metrics
    pub spot_price_sek_per_kwh: Option<f64>,
    pub estimated_cost_sek: Option<f64>,

    // Optimization metrics
    pub schedule_id: Option<Uuid>,
    pub deviation_from_schedule_kw: Option<f64>,
}

/// Hourly aggregated statistics
#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct HourlyPowerFlowStats {
    pub hour: DateTime<Utc>,
    pub snapshot_count: Option<i64>,
    pub avg_pv_kw: Option<f64>,
    pub avg_house_load_kw: Option<f64>,
    pub avg_battery_power_kw: Option<f64>,
    pub avg_grid_import_kw: Option<f64>,
    pub avg_grid_export_kw: Option<f64>,
    pub total_grid_import_kwh: Option<f64>,
    pub total_grid_export_kwh: Option<f64>,
    pub avg_spot_price: Option<f64>,
    pub total_cost_sek: Option<f64>,
    pub grid_outage_count: Option<i64>,
}

pub struct PowerFlowSnapshotRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> PowerFlowSnapshotRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    /// Insert a single power flow snapshot
    pub async fn insert(&self, snapshot: &PowerFlowSnapshotInput) -> Result<i64> {
        let rec = sqlx::query!(
            r#"
            INSERT INTO power_flow_snapshots (
                pv_production_kw, house_load_kw, battery_power_kw, ev_charger_power_kw,
                grid_import_kw, grid_export_kw,
                battery_soc_percent, battery_temperature_c,
                grid_frequency_hz, grid_voltage_v, grid_available,
                constraints_version, fuse_limit_a,
                control_mode, decision_reason,
                spot_price_sek_per_kwh, estimated_cost_sek,
                schedule_id, deviation_from_schedule_kw
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19)
            RETURNING id
            "#,
            snapshot.pv_production_kw,
            snapshot.house_load_kw,
            snapshot.battery_power_kw,
            snapshot.ev_charger_power_kw,
            snapshot.grid_import_kw,
            snapshot.grid_export_kw,
            snapshot.battery_soc_percent,
            snapshot.battery_temperature_c,
            snapshot.grid_frequency_hz,
            snapshot.grid_voltage_v,
            snapshot.grid_available,
            snapshot.constraints_version,
            snapshot.fuse_limit_a,
            snapshot.control_mode,
            snapshot.decision_reason,
            snapshot.spot_price_sek_per_kwh,
            snapshot.estimated_cost_sek,
            snapshot.schedule_id,
            snapshot.deviation_from_schedule_kw,
        )
        .fetch_one(self.pool)
        .await?;

        Ok(rec.id)
    }

    /// Insert a batch of snapshots (for backfilling or bulk import)
    pub async fn insert_batch(&self, snapshots: &[PowerFlowSnapshotInput]) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        for snapshot in snapshots {
            sqlx::query!(
                r#"
                INSERT INTO power_flow_snapshots (
                    pv_production_kw, house_load_kw, battery_power_kw, ev_charger_power_kw,
                    grid_import_kw, grid_export_kw,
                    battery_soc_percent, battery_temperature_c,
                    grid_frequency_hz, grid_voltage_v, grid_available,
                    constraints_version, fuse_limit_a,
                    control_mode, decision_reason,
                    spot_price_sek_per_kwh, estimated_cost_sek,
                    schedule_id, deviation_from_schedule_kw
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19)
                "#,
                snapshot.pv_production_kw,
                snapshot.house_load_kw,
                snapshot.battery_power_kw,
                snapshot.ev_charger_power_kw,
                snapshot.grid_import_kw,
                snapshot.grid_export_kw,
                snapshot.battery_soc_percent,
                snapshot.battery_temperature_c,
                snapshot.grid_frequency_hz,
                snapshot.grid_voltage_v,
                snapshot.grid_available,
                snapshot.constraints_version,
                snapshot.fuse_limit_a,
                snapshot.control_mode,
                snapshot.decision_reason,
                snapshot.spot_price_sek_per_kwh,
                snapshot.estimated_cost_sek,
                snapshot.schedule_id,
                snapshot.deviation_from_schedule_kw,
            )
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    /// Get the latest snapshot
    pub async fn find_latest(&self) -> Result<Option<PowerFlowSnapshotRow>> {
        let snapshot = sqlx::query_as!(
            PowerFlowSnapshotRow,
            r#"
            SELECT *
            FROM power_flow_snapshots
            ORDER BY timestamp DESC
            LIMIT 1
            "#
        )
        .fetch_optional(self.pool)
        .await?;

        Ok(snapshot)
    }

    /// Get snapshots for a time range
    pub async fn find_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<PowerFlowSnapshotRow>> {
        let snapshots = sqlx::query_as!(
            PowerFlowSnapshotRow,
            r#"
            SELECT *
            FROM power_flow_snapshots
            WHERE timestamp >= $1 AND timestamp <= $2
            ORDER BY timestamp ASC
            "#,
            start,
            end
        )
        .fetch_all(self.pool)
        .await?;

        Ok(snapshots)
    }

    /// Get snapshots for a specific schedule
    pub async fn find_by_schedule(
        &self,
        schedule_id: Uuid,
    ) -> Result<Vec<PowerFlowSnapshotRow>> {
        let snapshots = sqlx::query_as!(
            PowerFlowSnapshotRow,
            r#"
            SELECT *
            FROM power_flow_snapshots
            WHERE schedule_id = $1
            ORDER BY timestamp ASC
            "#,
            schedule_id
        )
        .fetch_all(self.pool)
        .await?;

        Ok(snapshots)
    }

    /// Get snapshots by control mode
    pub async fn find_by_control_mode(
        &self,
        control_mode: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<PowerFlowSnapshotRow>> {
        let snapshots = sqlx::query_as!(
            PowerFlowSnapshotRow,
            r#"
            SELECT *
            FROM power_flow_snapshots
            WHERE control_mode = $1
              AND timestamp >= $2
              AND timestamp <= $3
            ORDER BY timestamp ASC
            "#,
            control_mode,
            start,
            end
        )
        .fetch_all(self.pool)
        .await?;

        Ok(snapshots)
    }

    /// Get hourly aggregated statistics for a time range
    pub async fn get_hourly_stats(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<HourlyPowerFlowStats>> {
        let stats = sqlx::query_as!(
            HourlyPowerFlowStats,
            r#"
            SELECT *
            FROM power_flow_hourly_stats
            WHERE hour >= $1 AND hour <= $2
            ORDER BY hour ASC
            "#,
            start,
            end
        )
        .fetch_all(self.pool)
        .await?;

        Ok(stats)
    }

    /// Calculate average deviation from schedule for a time period
    pub async fn get_average_schedule_deviation(
        &self,
        schedule_id: Uuid,
    ) -> Result<Option<f64>> {
        let rec = sqlx::query!(
            r#"
            SELECT AVG(ABS(deviation_from_schedule_kw)) as avg_deviation
            FROM power_flow_snapshots
            WHERE schedule_id = $1
              AND deviation_from_schedule_kw IS NOT NULL
            "#,
            schedule_id
        )
        .fetch_one(self.pool)
        .await?;

        Ok(rec.avg_deviation)
    }

    /// Get total grid import/export energy for a time period
    pub async fn get_grid_energy_totals(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        sample_interval_seconds: i32,
    ) -> Result<(f64, f64)> {
        let rec = sqlx::query!(
            r#"
            SELECT
                SUM(grid_import_kw * $3 / 3600.0) as total_import_kwh,
                SUM(grid_export_kw * $3 / 3600.0) as total_export_kwh
            FROM power_flow_snapshots
            WHERE timestamp >= $1 AND timestamp <= $2
            "#,
            start,
            end,
            sample_interval_seconds
        )
        .fetch_one(self.pool)
        .await?;

        Ok((
            rec.total_import_kwh.unwrap_or(0.0),
            rec.total_export_kwh.unwrap_or(0.0),
        ))
    }

    /// Refresh the hourly statistics materialized view
    pub async fn refresh_hourly_stats(&self) -> Result<()> {
        sqlx::query!("SELECT refresh_power_flow_hourly_stats()")
            .execute(self.pool)
            .await?;
        Ok(())
    }

    /// Clean up old snapshots (keep only last N days)
    pub async fn cleanup_old_data(&self, days: i32) -> Result<u64> {
        let result = sqlx::query!(
            r#"
            DELETE FROM power_flow_snapshots
            WHERE timestamp < NOW() - INTERVAL '1 day' * $1
            "#,
            days
        )
        .execute(self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    /// Count snapshots in a time range
    pub async fn count_snapshots(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<i64> {
        let rec = sqlx::query!(
            r#"
            SELECT COUNT(*) as count
            FROM power_flow_snapshots
            WHERE timestamp >= $1 AND timestamp <= $2
            "#,
            start,
            end
        )
        .fetch_one(self.pool)
        .await?;

        Ok(rec.count.unwrap_or(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "requires database"]
    async fn test_power_flow_snapshot_crud() {
        // This would require database setup
    }

    #[test]
    fn test_power_flow_snapshot_input_serialization() {
        let input = PowerFlowSnapshotInput {
            pv_production_kw: 5.0,
            house_load_kw: 2.0,
            battery_power_kw: -3.0,
            ev_charger_power_kw: 0.0,
            grid_import_kw: 0.0,
            grid_export_kw: 0.0,
            battery_soc_percent: Some(75.0),
            battery_temperature_c: Some(25.0),
            grid_frequency_hz: Some(50.0),
            grid_voltage_v: Some(230.0),
            grid_available: true,
            constraints_version: Some("v1.0".to_string()),
            fuse_limit_a: Some(25.0),
            control_mode: Some("schedule".to_string()),
            decision_reason: Some("Following optimizer schedule".to_string()),
            spot_price_sek_per_kwh: Some(0.85),
            estimated_cost_sek: Some(0.02),
            schedule_id: None,
            deviation_from_schedule_kw: Some(0.1),
        };

        let json = serde_json::to_string(&input).unwrap();
        assert!(json.contains("pv_production_kw"));
        assert!(json.contains("5.0"));
    }
}
