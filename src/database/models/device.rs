use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Device type enumeration
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "text")]
#[serde(rename_all = "lowercase")]
pub enum DeviceType {
    Battery,
    Inverter,
    EvCharger,
    GridMeter,
    SolarPanel,
    Unknown,
}

impl std::fmt::Display for DeviceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Battery => "battery",
            Self::Inverter => "inverter",
            Self::EvCharger => "ev_charger",
            Self::GridMeter => "grid_meter",
            Self::SolarPanel => "solar_panel",
            Self::Unknown => "unknown",
        };
        write!(f, "{}", s)
    }
}

impl std::str::FromStr for DeviceType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "battery" => Ok(Self::Battery),
            "inverter" => Ok(Self::Inverter),
            "ev_charger" | "evcharger" => Ok(Self::EvCharger),
            "grid_meter" | "gridmeter" => Ok(Self::GridMeter),
            "solar_panel" | "solarpanel" => Ok(Self::SolarPanel),
            "unknown" => Ok(Self::Unknown),
            _ => Err(format!("Invalid device type: {}", s)),
        }
    }
}

/// Device database model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Device {
    pub id: Uuid,
    pub device_type: String,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub ip_address: Option<String>,
    pub port: Option<i32>,
    pub config: Option<serde_json::Value>,
    pub discovered_at: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub enabled: bool,
}

impl Device {
    /// Get the device type as an enum
    pub fn get_device_type(&self) -> DeviceType {
        self.device_type.parse().unwrap_or(DeviceType::Unknown)
    }

    /// Check if the device is online (seen in the last 5 minutes)
    pub fn is_online(&self) -> bool {
        let now = Utc::now();
        let threshold = chrono::Duration::minutes(5);
        now.signed_duration_since(self.last_seen) < threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_type_display() {
        assert_eq!(DeviceType::Battery.to_string(), "battery");
        assert_eq!(DeviceType::EvCharger.to_string(), "ev_charger");
    }

    #[test]
    fn test_device_type_from_str() {
        assert_eq!("battery".parse::<DeviceType>().unwrap(), DeviceType::Battery);
        assert_eq!("INVERTER".parse::<DeviceType>().unwrap(), DeviceType::Inverter);
        assert_eq!("ev_charger".parse::<DeviceType>().unwrap(), DeviceType::EvCharger);
        assert_eq!("evcharger".parse::<DeviceType>().unwrap(), DeviceType::EvCharger);
        assert!("invalid".parse::<DeviceType>().is_err());
    }

    #[test]
    fn test_device_is_online() {
        let mut device = Device {
            id: Uuid::new_v4(),
            device_type: "battery".to_string(),
            manufacturer: Some("TestCo".to_string()),
            model: Some("Model1".to_string()),
            ip_address: Some("192.168.1.100".to_string()),
            port: Some(502),
            config: None,
            discovered_at: Utc::now(),
            last_seen: Utc::now(),
            enabled: true,
        };

        assert!(device.is_online());

        // Set last_seen to 10 minutes ago
        device.last_seen = Utc::now() - chrono::Duration::minutes(10);
        assert!(!device.is_online());
    }
}
