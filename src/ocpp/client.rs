#![allow(dead_code)]
#![allow(dead_code)]
//! OCPP Client Implementation
//!
//! This module provides a WebSocket-based client for communicating with
//! EV chargers using the OCPP 1.6 protocol.

use super::messages::*;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

/// OCPP WebSocket Client
pub struct OcppWebSocketClient {
    endpoint_url: String,
    charge_point_id: String,
    heartbeat_interval: Arc<RwLock<Option<i32>>>,
}

impl OcppWebSocketClient {
    /// Create a new OCPP WebSocket client
    pub fn new(endpoint_url: String, charge_point_id: String) -> Self {
        Self {
            endpoint_url,
            charge_point_id,
            heartbeat_interval: Arc::new(RwLock::new(None)),
        }
    }

    /// Send a boot notification
    pub async fn send_boot_notification(
        &self,
        _request: BootNotificationRequest,
    ) -> Result<BootNotificationResponse> {
        // TODO: Implement actual WebSocket communication
        // For now, return a mock response
        tracing::debug!("Sending boot notification for charge point {}", self.charge_point_id);

        Ok(BootNotificationResponse {
            status: RegistrationStatus::Accepted,
            current_time: chrono::Utc::now(),
            interval: 300, // 5 minutes
        })
    }

    /// Send a heartbeat
    pub async fn send_heartbeat(&self) -> Result<HeartbeatResponse> {
        // TODO: Implement actual WebSocket communication
        tracing::debug!("Sending heartbeat");

        Ok(HeartbeatResponse {
            current_time: chrono::Utc::now(),
        })
    }

    /// Send a status notification
    pub async fn send_status_notification(
        &self,
        request: StatusNotificationRequest,
    ) -> Result<StatusNotificationResponse> {
        // TODO: Implement actual WebSocket communication
        tracing::debug!(
            "Sending status notification for connector {}: {:?}",
            request.connector_id,
            request.status
        );

        Ok(StatusNotificationResponse {})
    }

    /// Handle remote start transaction
    pub async fn handle_remote_start(
        &self,
        request: RemoteStartTransactionRequest,
    ) -> Result<RemoteStartTransactionResponse> {
        // TODO: Implement actual transaction start logic
        tracing::info!(
            "Handling remote start for ID tag: {}",
            request.id_tag
        );

        Ok(RemoteStartTransactionResponse {
            status: RemoteStartStopStatus::Accepted,
        })
    }

    /// Handle remote stop transaction
    pub async fn handle_remote_stop(
        &self,
        request: RemoteStopTransactionRequest,
    ) -> Result<RemoteStopTransactionResponse> {
        // TODO: Implement actual transaction stop logic
        tracing::info!(
            "Handling remote stop for transaction ID: {}",
            request.transaction_id
        );

        Ok(RemoteStopTransactionResponse {
            status: RemoteStartStopStatus::Accepted,
        })
    }

    /// Get the configured heartbeat interval
    pub async fn get_heartbeat_interval(&self) -> Option<i32> {
        *self.heartbeat_interval.read().await
    }

    /// Set the heartbeat interval
    pub async fn set_heartbeat_interval(&self, interval: i32) {
        *self.heartbeat_interval.write().await = Some(interval);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_boot_notification() {
        let client = OcppWebSocketClient::new(
            "ws://localhost:8080/ocpp".to_string(),
            "CP001".to_string(),
        );

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

        let response = client.send_boot_notification(request).await.unwrap();
        assert_eq!(response.status, RegistrationStatus::Accepted);
    }

    #[tokio::test]
    async fn test_heartbeat() {
        let client = OcppWebSocketClient::new(
            "ws://localhost:8080/ocpp".to_string(),
            "CP001".to_string(),
        );

        let response = client.send_heartbeat().await.unwrap();
        assert!(response.current_time.timestamp() > 0);
    }

    #[tokio::test]
    async fn test_remote_start() {
        let client = OcppWebSocketClient::new(
            "ws://localhost:8080/ocpp".to_string(),
            "CP001".to_string(),
        );

        let request = RemoteStartTransactionRequest {
            id_tag: "USER001".to_string(),
            connector_id: Some(1),
            charging_profile: None,
        };

        let response = client.handle_remote_start(request).await.unwrap();
        assert_eq!(response.status, RemoteStartStopStatus::Accepted);
    }
}
