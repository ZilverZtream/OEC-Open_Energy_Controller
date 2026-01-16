//! Database models
//!
//! This module contains the database schema models that map directly to database tables.
//! These are separate from domain models to maintain a clean separation between
//! database representation and business logic.

pub mod device;
pub mod battery_state;
pub mod schedule;
pub mod price;

pub use device::{Device, DeviceType};
pub use battery_state::BatteryStateRow;
pub use schedule::ScheduleRow;
pub use price::ElectricityPriceRow;
