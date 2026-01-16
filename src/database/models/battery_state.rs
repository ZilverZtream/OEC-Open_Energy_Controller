use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Battery state database row
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BatteryStateRow {
    pub id: i64,
    pub device_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub soc_percent: f64,
    pub power_w: f64,
    pub voltage_v: f64,
    pub temperature_c: f64,
    pub health_percent: Option<f64>,
    pub status: Option<String>,
}

impl BatteryStateRow {
    /// Convert to domain BatteryState
    pub fn to_domain(&self) -> crate::domain::battery::BatteryState {
        use crate::domain::battery::{BatteryState, BatteryStatus};

        let status = self.status.as_ref()
            .and_then(|s| s.parse::<BatteryStatus>().ok())
            .unwrap_or(BatteryStatus::Idle);

        BatteryState {
            soc_percent: self.soc_percent,
            power_w: self.power_w,
            voltage_v: self.voltage_v,
            temperature_c: self.temperature_c,
            health_percent: self.health_percent.unwrap_or(100.0),
            status,
        }
    }

    /// Create from domain BatteryState
    pub fn from_domain(device_id: Uuid, state: &crate::domain::battery::BatteryState) -> Self {
        Self {
            id: 0, // Will be set by database
            device_id,
            timestamp: Utc::now(),
            soc_percent: state.soc_percent,
            power_w: state.power_w,
            voltage_v: state.voltage_v,
            temperature_c: state.temperature_c,
            health_percent: Some(state.health_percent),
            status: Some(format!("{:?}", state.status)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::battery::BatteryStatus;

    #[test]
    fn test_battery_state_row_creation() {
        let row = BatteryStateRow {
            id: 1,
            device_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            soc_percent: 75.0,
            power_w: 2000.0,
            voltage_v: 400.0,
            temperature_c: 25.0,
            health_percent: Some(98.5),
            status: Some("Charging".to_string()),
        };

        assert_eq!(row.soc_percent, 75.0);
        assert_eq!(row.power_w, 2000.0);
    }

    #[test]
    fn test_battery_state_row_to_domain() {
        let row = BatteryStateRow {
            id: 1,
            device_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            soc_percent: 80.0,
            power_w: -1500.0,
            voltage_v: 400.0,
            temperature_c: 22.0,
            health_percent: Some(95.0),
            status: Some("Discharging".to_string()),
        };

        let domain_state = row.to_domain();
        assert_eq!(domain_state.soc_percent, 80.0);
        assert_eq!(domain_state.power_w, -1500.0);
        assert_eq!(domain_state.health_percent, 95.0);
    }
}
