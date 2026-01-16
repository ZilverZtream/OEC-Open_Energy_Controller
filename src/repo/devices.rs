#![cfg(feature = "db")]

use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DeviceRow {
    pub id: Uuid,
    pub device_type: String,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub ip: std::net::IpAddr,
    pub port: i32,
    pub modbus_unit_id: Option<i32>,
    pub config: serde_json::Value,
    pub discovered_at: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
}

pub struct DeviceRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> DeviceRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, device: &DeviceRow) -> Result<Uuid> {
        let rec = sqlx::query!(
            r#"
            INSERT INTO devices (id, device_type, manufacturer, model, ip, port, modbus_unit_id, config)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id
            "#,
            device.id,
            device.device_type,
            device.manufacturer,
            device.model,
            device.ip,
            device.port,
            device.modbus_unit_id,
            device.config,
        )
        .fetch_one(self.pool)
        .await?;

        Ok(rec.id)
    }

    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<DeviceRow>> {
        let device = sqlx::query_as!(
            DeviceRow,
            r#"
            SELECT id, device_type, manufacturer, model, ip as "ip: _", port, modbus_unit_id, config, discovered_at, last_seen
            FROM devices
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(self.pool)
        .await?;

        Ok(device)
    }

    pub async fn find_by_ip(&self, ip: std::net::IpAddr) -> Result<Option<DeviceRow>> {
        let device = sqlx::query_as!(
            DeviceRow,
            r#"
            SELECT id, device_type, manufacturer, model, ip as "ip: _", port, modbus_unit_id, config, discovered_at, last_seen
            FROM devices
            WHERE ip = $1
            "#,
            ip
        )
        .fetch_optional(self.pool)
        .await?;

        Ok(device)
    }

    pub async fn find_by_type(&self, device_type: &str) -> Result<Vec<DeviceRow>> {
        let devices = sqlx::query_as!(
            DeviceRow,
            r#"
            SELECT id, device_type, manufacturer, model, ip as "ip: _", port, modbus_unit_id, config, discovered_at, last_seen
            FROM devices
            WHERE device_type = $1
            ORDER BY discovered_at DESC
            "#,
            device_type
        )
        .fetch_all(self.pool)
        .await?;

        Ok(devices)
    }

    pub async fn list_all(&self) -> Result<Vec<DeviceRow>> {
        let devices = sqlx::query_as!(
            DeviceRow,
            r#"
            SELECT id, device_type, manufacturer, model, ip as "ip: _", port, modbus_unit_id, config, discovered_at, last_seen
            FROM devices
            ORDER BY discovered_at DESC
            "#
        )
        .fetch_all(self.pool)
        .await?;

        Ok(devices)
    }

    pub async fn update_last_seen(&self, id: Uuid) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE devices
            SET last_seen = NOW()
            WHERE id = $1
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
            DELETE FROM devices
            WHERE id = $1
            "#,
            id
        )
        .execute(self.pool)
        .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Integration tests would require a test database
    // These are placeholder test structures

    #[tokio::test]
    #[ignore = "requires database"]
    async fn test_device_crud() {
        // This would require database setup
    }
}
