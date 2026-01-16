#![allow(dead_code)]
#![allow(dead_code)]
//! OCPP 1.6 Message Definitions
//!
//! This module defines the message payloads for various OCPP operations.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Boot Notification Request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BootNotificationRequest {
    pub charge_point_vendor: String,
    pub charge_point_model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub charge_point_serial_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub charge_box_serial_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub firmware_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iccid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub imsi: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meter_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meter_serial_number: Option<String>,
}

/// Boot Notification Response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BootNotificationResponse {
    pub status: RegistrationStatus,
    pub current_time: DateTime<Utc>,
    pub interval: i32, // Heartbeat interval in seconds
}

/// Registration Status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RegistrationStatus {
    Accepted,
    Pending,
    Rejected,
}

/// Heartbeat Request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatRequest {}

/// Heartbeat Response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeartbeatResponse {
    pub current_time: DateTime<Utc>,
}

/// Status Notification Request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusNotificationRequest {
    pub connector_id: i32,
    pub error_code: ChargePointErrorCode,
    pub status: ChargePointStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vendor_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vendor_error_code: Option<String>,
}

/// Status Notification Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusNotificationResponse {}

/// Charge Point Error Code
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChargePointErrorCode {
    NoError,
    ConnectorLockFailure,
    EVCommunicationError,
    GroundFailure,
    HighTemperature,
    InternalError,
    LocalListConflict,
    OtherError,
    OverCurrentFailure,
    PowerMeterFailure,
    PowerSwitchFailure,
    ReaderFailure,
    ResetFailure,
    UnderVoltage,
    OverVoltage,
    WeakSignal,
}

/// Charge Point Status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChargePointStatus {
    Available,
    Preparing,
    Charging,
    SuspendedEVSE,
    SuspendedEV,
    Finishing,
    Reserved,
    Unavailable,
    Faulted,
}

/// Remote Start Transaction Request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteStartTransactionRequest {
    pub id_tag: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connector_id: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub charging_profile: Option<ChargingProfile>,
}

/// Remote Start Transaction Response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteStartTransactionResponse {
    pub status: RemoteStartStopStatus,
}

/// Remote Stop Transaction Request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteStopTransactionRequest {
    pub transaction_id: i32,
}

/// Remote Stop Transaction Response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteStopTransactionResponse {
    pub status: RemoteStartStopStatus,
}

/// Remote Start/Stop Status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RemoteStartStopStatus {
    Accepted,
    Rejected,
}

/// Charging Profile
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChargingProfile {
    pub charging_profile_id: i32,
    pub stack_level: i32,
    pub charging_profile_purpose: ChargingProfilePurpose,
    pub charging_profile_kind: ChargingProfileKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recurrency_kind: Option<RecurrencyKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valid_from: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valid_to: Option<DateTime<Utc>>,
    pub charging_schedule: ChargingSchedule,
}

/// Charging Profile Purpose
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChargingProfilePurpose {
    ChargePointMaxProfile,
    TxDefaultProfile,
    TxProfile,
}

/// Charging Profile Kind
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChargingProfileKind {
    Absolute,
    Recurring,
    Relative,
}

/// Recurrency Kind
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RecurrencyKind {
    Daily,
    Weekly,
}

/// Charging Schedule
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChargingSchedule {
    pub charging_rate_unit: ChargingRateUnit,
    pub charging_schedule_period: Vec<ChargingSchedulePeriod>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_schedule: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_charging_rate: Option<f64>,
}

/// Charging Rate Unit
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChargingRateUnit {
    W, // Watts
    A, // Amperes
}

/// Charging Schedule Period
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChargingSchedulePeriod {
    pub start_period: i32, // Seconds from start of schedule
    pub limit: f64,        // Max current or power
    #[serde(skip_serializing_if = "Option::is_none")]
    pub number_phases: Option<i32>,
}

/// Change Configuration Request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeConfigurationRequest {
    pub key: String,
    pub value: String,
}

/// Change Configuration Response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeConfigurationResponse {
    pub status: ConfigurationStatus,
}

/// Configuration Status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ConfigurationStatus {
    Accepted,
    Rejected,
    RebootRequired,
    NotSupported,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_boot_notification_serialization() {
        let request = BootNotificationRequest {
            charge_point_vendor: "OpenEnergyController".to_string(),
            charge_point_model: "OEC-CP-001".to_string(),
            charge_point_serial_number: Some("SN123456".to_string()),
            charge_box_serial_number: None,
            firmware_version: Some("1.0.0".to_string()),
            iccid: None,
            imsi: None,
            meter_type: None,
            meter_serial_number: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("OpenEnergyController"));
        assert!(json.contains("OEC-CP-001"));
    }

    #[test]
    fn test_charging_profile_serialization() {
        let profile = ChargingProfile {
            charging_profile_id: 1,
            stack_level: 0,
            charging_profile_purpose: ChargingProfilePurpose::TxProfile,
            charging_profile_kind: ChargingProfileKind::Absolute,
            recurrency_kind: None,
            valid_from: None,
            valid_to: None,
            charging_schedule: ChargingSchedule {
                charging_rate_unit: ChargingRateUnit::A,
                charging_schedule_period: vec![ChargingSchedulePeriod {
                    start_period: 0,
                    limit: 32.0,
                    number_phases: Some(3),
                }],
                duration: None,
                start_schedule: None,
                min_charging_rate: Some(6.0),
            },
        };

        let json = serde_json::to_string(&profile).unwrap();
        assert!(json.contains("\"limit\":32"));
    }
}
