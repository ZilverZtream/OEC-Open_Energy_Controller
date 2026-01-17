#![cfg(feature = "db")]

use anyhow::Result;
use sqlx::{postgres::PgPoolOptions, PgPool};

pub mod battery_states;
pub mod devices;
pub mod schedules;

pub use battery_states::BatteryStateRepository;
pub use devices::DeviceRepository;
pub use schedules::ScheduleRepository;

use crate::repo::consumption::ConsumptionRepository;

pub struct PgRepo {
    pub pool: PgPool,
}

impl PgRepo {
    pub async fn connect(url: &str) -> Result<Self> {
        // CRITICAL FIX #5: Database connection exhaustion on Raspberry Pi
        // Raspberry Pi has limited resources. Postgres/SQLite have default connection limits.
        // If we spawn 50 parallel simulation threads (for MPC optimization), we hit MaxConnections.
        // Result: API returns 500 errors and controller stops recording data.
        // Fix: Limit to 5 connections (suitable for Raspberry Pi), preventing exhaustion
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .min_connections(1)
            .connect(url)
            .await?;
        Ok(Self { pool })
    }

    /// Get a device repository
    pub fn devices(&self) -> DeviceRepository {
        DeviceRepository::new(&self.pool)
    }

    /// Get a battery state repository
    pub fn battery_states(&self) -> BatteryStateRepository {
        BatteryStateRepository::new(&self.pool)
    }

    /// Get a schedule repository
    pub fn schedules(&self) -> ScheduleRepository {
        ScheduleRepository::new(&self.pool)
    }

    /// Get a consumption history repository
    pub fn consumption(&self) -> ConsumptionRepository {
        ConsumptionRepository::new(self.pool.clone())
    }
}
