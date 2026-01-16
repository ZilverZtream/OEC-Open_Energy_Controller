use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg_attr(feature = "swagger", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schedule {
    pub id: Uuid,
    pub created_at: DateTime<FixedOffset>,
    pub valid_from: DateTime<FixedOffset>,
    pub valid_until: DateTime<FixedOffset>,
    pub entries: Vec<ScheduleEntry>,
    pub optimizer_version: String,
}

#[cfg_attr(feature = "swagger", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleEntry {
    pub time_start: DateTime<FixedOffset>,
    pub time_end: DateTime<FixedOffset>,
    pub target_power_w: f64,
    pub reason: String,
}

impl Schedule {
    pub fn power_at(&self, t: DateTime<FixedOffset>) -> Option<f64> {
        self.entries.iter().find(|e| t >= e.time_start && t < e.time_end).map(|e| e.target_power_w)
    }
    pub fn next_hours(&self, hours: i64) -> Vec<ScheduleEntry> {
        let now = chrono::Local::now().fixed_offset();
        let until = now + chrono::Duration::hours(hours);
        self.entries.iter().filter(|e| e.time_end > now && e.time_start < until).cloned().collect()
    }
}
