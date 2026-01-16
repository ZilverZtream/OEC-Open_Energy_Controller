#![allow(dead_code)]
//! OCPP 1.6 Protocol Implementation for EV Charger Communication
//!
//! This module implements the Open Charge Point Protocol (OCPP) version 1.6,
//! which is the standard protocol for communication with EV chargers.
//!
//! # Architecture
//! - WebSocket-based communication with EV chargers
//! - JSON message format (OCPP 1.6J)
//! - Call-Result-Error message pattern
//! - Heartbeat and status notification support

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

pub mod messages;
pub mod client;

/// OCPP Message Type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    Call = 2,
    CallResult = 3,
    CallError = 4,
}

/// OCPP Call message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Call {
    pub message_type_id: u8,
    pub message_id: String,
    pub action: String,
    pub payload: serde_json::Value,
}

/// OCPP CallResult message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallResult {
    pub message_type_id: u8,
    pub message_id: String,
    pub payload: serde_json::Value,
}

/// OCPP CallError message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallError {
    pub message_type_id: u8,
    pub message_id: String,
    pub error_code: String,
    pub error_description: String,
    pub error_details: serde_json::Value,
}

/// OCPP Error Codes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ErrorCode {
    NotImplemented,
    NotSupported,
    InternalError,
    ProtocolError,
    SecurityError,
    FormationViolation,
    PropertyConstraintViolation,
    OccurrenceConstraintViolation,
    TypeConstraintViolation,
    GenericError,
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::NotImplemented => "NotImplemented",
            Self::NotSupported => "NotSupported",
            Self::InternalError => "InternalError",
            Self::ProtocolError => "ProtocolError",
            Self::SecurityError => "SecurityError",
            Self::FormationViolation => "FormationViolation",
            Self::PropertyConstraintViolation => "PropertyConstraintViolation",
            Self::OccurrenceConstraintViolation => "OccurrenceConstraintViolation",
            Self::TypeConstraintViolation => "TypeConstraintViolation",
            Self::GenericError => "GenericError",
        };
        write!(f, "{}", s)
    }
}

/// OCPP Connection State
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Error,
}

/// OCPP Client State
pub struct OcppState {
    pub connection_state: ConnectionState,
    pub last_heartbeat: Option<chrono::DateTime<chrono::Utc>>,
    pub charge_point_id: String,
}

impl OcppState {
    pub fn new(charge_point_id: String) -> Self {
        Self {
            connection_state: ConnectionState::Disconnected,
            last_heartbeat: None,
            charge_point_id,
        }
    }
}

/// OCPP Client
pub struct OcppClient {
    state: Arc<RwLock<OcppState>>,
    endpoint_url: String,
}

impl OcppClient {
    /// Create a new OCPP client
    pub fn new(endpoint_url: String, charge_point_id: String) -> Self {
        Self {
            state: Arc::new(RwLock::new(OcppState::new(charge_point_id))),
            endpoint_url,
        }
    }

    /// Connect to the OCPP central system
    pub async fn connect(&self) -> Result<()> {
        let mut state = self.state.write().await;
        state.connection_state = ConnectionState::Connecting;

        // TODO: Implement WebSocket connection
        // For now, just mark as connected for simulation
        tracing::info!("OCPP client connecting to {}", self.endpoint_url);
        state.connection_state = ConnectionState::Connected;
        state.last_heartbeat = Some(chrono::Utc::now());

        Ok(())
    }

    /// Send a heartbeat to the central system
    pub async fn send_heartbeat(&self) -> Result<()> {
        let mut state = self.state.write().await;

        if state.connection_state != ConnectionState::Connected {
            return Err(anyhow!("Not connected to OCPP central system"));
        }

        // TODO: Send actual heartbeat message
        tracing::debug!("Sending OCPP heartbeat");
        state.last_heartbeat = Some(chrono::Utc::now());

        Ok(())
    }

    /// Get current connection state
    pub async fn get_state(&self) -> ConnectionState {
        self.state.read().await.connection_state
    }

    /// Disconnect from the central system
    pub async fn disconnect(&self) -> Result<()> {
        let mut state = self.state.write().await;
        state.connection_state = ConnectionState::Disconnected;
        tracing::info!("OCPP client disconnected");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ocpp_client_creation() {
        let client = OcppClient::new(
            "ws://localhost:8080/ocpp".to_string(),
            "CP001".to_string(),
        );

        let state = client.get_state().await;
        assert_eq!(state, ConnectionState::Disconnected);
    }

    #[tokio::test]
    async fn test_ocpp_connect() {
        let client = OcppClient::new(
            "ws://localhost:8080/ocpp".to_string(),
            "CP001".to_string(),
        );

        client.connect().await.unwrap();
        let state = client.get_state().await;
        assert_eq!(state, ConnectionState::Connected);
    }

    #[tokio::test]
    async fn test_ocpp_heartbeat() {
        let client = OcppClient::new(
            "ws://localhost:8080/ocpp".to_string(),
            "CP001".to_string(),
        );

        client.connect().await.unwrap();
        client.send_heartbeat().await.unwrap();

        let state = client.state.read().await;
        assert!(state.last_heartbeat.is_some());
    }
}
