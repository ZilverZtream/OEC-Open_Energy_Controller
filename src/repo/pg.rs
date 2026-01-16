#![cfg(feature = "db")]

use anyhow::Result;
use sqlx::{postgres::PgPoolOptions, PgPool};

pub mod battery_states;
pub mod devices;
pub mod schedules;

pub use battery_states::BatteryStateRepository;
pub use devices::DeviceRepository;
pub use schedules::ScheduleRepository;

pub struct PgRepo {
    pub pool: PgPool,
}

impl PgRepo {
    pub async fn connect(url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(10)
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
}
