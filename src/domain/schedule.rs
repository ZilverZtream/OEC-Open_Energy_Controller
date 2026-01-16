#![allow(dead_code)]
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[cfg_attr(feature = "swagger", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schedule {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub valid_from: DateTime<Utc>,
    pub valid_until: DateTime<Utc>,
    pub entries: Vec<ScheduleEntry>,
    pub optimizer_version: String,
}

#[cfg_attr(feature = "swagger", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleEntry {
    pub time_start: DateTime<Utc>,
    pub time_end: DateTime<Utc>,
    pub target_power_w: f64,
    pub reason: String,
}

/// A simplified schedule interval with power target only.
#[cfg_attr(feature = "swagger", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleInterval {
    pub time_start: DateTime<Utc>,
    pub time_end: DateTime<Utc>,
    pub target_power_w: f64,
}

impl ScheduleEntry {
    /// Convert this entry into a schedule interval without metadata.
    pub fn interval(&self) -> ScheduleInterval {
        ScheduleInterval {
            time_start: self.time_start,
            time_end: self.time_end,
            target_power_w: self.target_power_w,
        }
    }
}

/// Errors returned when validating a schedule.
#[derive(Debug, Error, PartialEq)]
pub enum ScheduleValidationError {
    #[error("schedule has no entries")]
    EmptySchedule,
    #[error("schedule window is invalid: valid_from must be before valid_until")]
    InvalidWindow,
    #[error("entry {index} has an invalid time range")]
    InvalidEntryRange { index: usize },
    #[error("entry {index} is outside the schedule window")]
    EntryOutOfBounds { index: usize },
    #[error("gap detected between {previous_end} and {next_start}")]
    GapDetected {
        previous_end: DateTime<Utc>,
        next_start: DateTime<Utc>,
    },
    #[error("overlap detected between {previous_end} and {next_start}")]
    OverlapDetected {
        previous_end: DateTime<Utc>,
        next_start: DateTime<Utc>,
    },
    #[error("schedule window does not align with entries")]
    WindowMismatch,
}

impl Schedule {
    /// Get the target power at a specific timestamp
    ///
    /// CRITICAL FIX: Implements "hold previous" strategy for small gaps.
    /// If timestamp falls in a gap, returns the power from the previous entry
    /// (if gap is ≤60 seconds). This prevents controller from idling during
    /// minor schedule alignment issues while still rejecting large gaps.
    pub fn power_at(&self, t: DateTime<Utc>) -> Option<f64> {
        // First, try exact match
        if let Some(entry) = self.entries.iter().find(|e| t >= e.time_start && t < e.time_end) {
            return Some(entry.target_power_w);
        }

        // No exact match - check if we're in a small gap after an entry
        // Find the most recent entry that ended before 't'
        const MAX_GAP_SECONDS: i64 = 60;
        let recent_entry = self
            .entries
            .iter()
            .filter(|e| e.time_end <= t)
            .max_by_key(|e| e.time_end);

        if let Some(entry) = recent_entry {
            let gap_duration = t.signed_duration_since(entry.time_end);
            if gap_duration.num_seconds() <= MAX_GAP_SECONDS {
                // Small gap - hold previous value
                return Some(entry.target_power_w);
            }
        }

        // Large gap or no previous entry - return None
        None
    }

    /// Validate that entries cover the full schedule window without gaps or overlaps.
    pub fn validate(&self) -> Result<(), ScheduleValidationError> {
        if self.entries.is_empty() {
            return Err(ScheduleValidationError::EmptySchedule);
        }
        if self.valid_from >= self.valid_until {
            return Err(ScheduleValidationError::InvalidWindow);
        }

        let mut previous_end: Option<DateTime<Utc>> = None;
        for (index, entry) in self.entries.iter().enumerate() {
            // Validate target_power_w is finite
            if !entry.target_power_w.is_finite() {
                return Err(ScheduleValidationError::InvalidEntryRange { index });
            }

            let interval = entry.interval();
            if interval.time_start >= interval.time_end {
                return Err(ScheduleValidationError::InvalidEntryRange { index });
            }
            if interval.time_start < self.valid_from || interval.time_end > self.valid_until {
                return Err(ScheduleValidationError::EntryOutOfBounds { index });
            }
            match previous_end {
                None => {
                    if interval.time_start != self.valid_from {
                        return Err(ScheduleValidationError::WindowMismatch);
                    }
                }
                Some(end) => {
                    if interval.time_start > end {
                        return Err(ScheduleValidationError::GapDetected {
                            previous_end: end,
                            next_start: interval.time_start,
                        });
                    }
                    if interval.time_start < end {
                        return Err(ScheduleValidationError::OverlapDetected {
                            previous_end: end,
                            next_start: interval.time_start,
                        });
                    }
                }
            }
            previous_end = Some(interval.time_end);
        }

        if previous_end != Some(self.valid_until) {
            return Err(ScheduleValidationError::WindowMismatch);
        }

        Ok(())
    }
    pub fn next_hours(&self, hours: i64) -> Vec<ScheduleEntry> {
        let now = chrono::Utc::now();
        let until = now + chrono::Duration::hours(hours);
        self.entries
            .iter()
            .filter(|e| e.time_end > now && e.time_start < until)
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn make_schedule(entries: Vec<ScheduleEntry>) -> Schedule {
        let now = Utc::now();
        let valid_from = entries.first().map(|e| e.time_start).unwrap_or(now);
        let valid_until = entries.last().map(|e| e.time_end).unwrap_or(now);
        Schedule {
            id: Uuid::new_v4(),
            created_at: valid_from,
            valid_from,
            valid_until,
            entries,
            optimizer_version: "test".to_string(),
        }
    }

    #[test]
    fn validate_accepts_contiguous_entries() {
        let t0 = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        let t1 = t0 + chrono::Duration::hours(1);
        let t2 = t1 + chrono::Duration::hours(1);

        let schedule = make_schedule(vec![
            ScheduleEntry {
                time_start: t0,
                time_end: t1,
                target_power_w: 100.0,
                reason: "slot-1".to_string(),
            },
            ScheduleEntry {
                time_start: t1,
                time_end: t2,
                target_power_w: 200.0,
                reason: "slot-2".to_string(),
            },
        ]);

        assert_eq!(schedule.validate(), Ok(()));
    }

    #[test]
    fn validate_rejects_gap() {
        let t0 = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        let t1 = t0 + chrono::Duration::hours(1);
        let t2 = t1 + chrono::Duration::hours(1);
        let t3 = t2 + chrono::Duration::hours(1);

        let schedule = make_schedule(vec![
            ScheduleEntry {
                time_start: t0,
                time_end: t1,
                target_power_w: 100.0,
                reason: "slot-1".to_string(),
            },
            ScheduleEntry {
                time_start: t2,
                time_end: t3,
                target_power_w: 200.0,
                reason: "slot-2".to_string(),
            },
        ]);

        assert_eq!(
            schedule.validate(),
            Err(ScheduleValidationError::GapDetected {
                previous_end: t1,
                next_start: t2
            })
        );
    }

    #[test]
    fn validate_rejects_overlap() {
        let t0 = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        let t1 = t0 + chrono::Duration::hours(1);
        let t2 = t1 + chrono::Duration::hours(1);

        let schedule = make_schedule(vec![
            ScheduleEntry {
                time_start: t0,
                time_end: t2,
                target_power_w: 100.0,
                reason: "slot-1".to_string(),
            },
            ScheduleEntry {
                time_start: t1,
                time_end: t2,
                target_power_w: 200.0,
                reason: "slot-2".to_string(),
            },
        ]);

        assert_eq!(
            schedule.validate(),
            Err(ScheduleValidationError::OverlapDetected {
                previous_end: t2,
                next_start: t1
            })
        );
    }

    #[test]
    fn validate_rejects_out_of_bounds_entry() {
        let t0 = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        let t1 = t0 + chrono::Duration::hours(1);
        let t2 = t1 + chrono::Duration::hours(1);

        let schedule = Schedule {
            id: Uuid::new_v4(),
            created_at: t0,
            valid_from: t0,
            valid_until: t1,
            entries: vec![ScheduleEntry {
                time_start: t0,
                time_end: t2,
                target_power_w: 100.0,
                reason: "slot-1".to_string(),
            }],
            optimizer_version: "test".to_string(),
        };

        assert_eq!(
            schedule.validate(),
            Err(ScheduleValidationError::EntryOutOfBounds { index: 0 })
        );
    }
}
