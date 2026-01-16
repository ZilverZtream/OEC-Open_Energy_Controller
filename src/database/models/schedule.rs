use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Schedule database row
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ScheduleRow {
    pub id: Uuid,
    pub device_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub valid_from: DateTime<Utc>,
    pub valid_until: DateTime<Utc>,
    pub schedule_json: serde_json::Value,
    pub optimizer_version: Option<String>,
    pub cost_estimate: Option<f64>,
}

impl ScheduleRow {
    /// Check if the schedule is currently active
    pub fn is_active(&self) -> bool {
        let now = Utc::now();
        now >= self.valid_from && now < self.valid_until
    }

    /// Check if the schedule is expired
    pub fn is_expired(&self) -> bool {
        Utc::now() >= self.valid_until
    }

    /// Get the duration of the schedule in hours
    pub fn duration_hours(&self) -> f64 {
        let duration = self.valid_until.signed_duration_since(self.valid_from);
        duration.num_seconds() as f64 / 3600.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schedule_is_active() {
        let now = Utc::now();
        let schedule = ScheduleRow {
            id: Uuid::new_v4(),
            device_id: Uuid::new_v4(),
            created_at: now - chrono::Duration::hours(2),
            valid_from: now - chrono::Duration::hours(1),
            valid_until: now + chrono::Duration::hours(1),
            schedule_json: serde_json::json!({}),
            optimizer_version: Some("v1.0".to_string()),
            cost_estimate: Some(50.0),
        };

        assert!(schedule.is_active());
        assert!(!schedule.is_expired());
    }

    #[test]
    fn test_schedule_is_expired() {
        let now = Utc::now();
        let schedule = ScheduleRow {
            id: Uuid::new_v4(),
            device_id: Uuid::new_v4(),
            created_at: now - chrono::Duration::hours(3),
            valid_from: now - chrono::Duration::hours(2),
            valid_until: now - chrono::Duration::hours(1),
            schedule_json: serde_json::json!({}),
            optimizer_version: Some("v1.0".to_string()),
            cost_estimate: Some(50.0),
        };

        assert!(!schedule.is_active());
        assert!(schedule.is_expired());
    }

    #[test]
    fn test_schedule_duration_hours() {
        let now = Utc::now();
        let schedule = ScheduleRow {
            id: Uuid::new_v4(),
            device_id: Uuid::new_v4(),
            created_at: now,
            valid_from: now,
            valid_until: now + chrono::Duration::hours(24),
            schedule_json: serde_json::json!({}),
            optimizer_version: Some("v1.0".to_string()),
            cost_estimate: Some(100.0),
        };

        assert!((schedule.duration_hours() - 24.0).abs() < 0.1);
    }
}
