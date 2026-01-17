//! # OCPP EV Charger Implementation
//!
//! Implements the EvCharger domain trait using OCPP 1.6 protocol for real EV chargers.

use crate::domain::ev_charger::{
    ChargerCapabilities, ChargerError, ChargerState, ChargerStatus, ConnectorType, EvCharger,
};
use crate::ocpp::messages::{
    ChargePointStatus, ChargingProfile, ChargingProfileKind, ChargingProfilePurpose,
    ChargingRateUnit, ChargingSchedule, ChargingSchedulePeriod, RemoteStartStopStatus,
    RemoteStartTransactionRequest, RemoteStopTransactionRequest,
};
use crate::ocpp::{ConnectionState, OcppClient};
use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Configuration for OCPP EV Charger
#[derive(Debug, Clone)]
pub struct OcppEvChargerConfig {
    /// OCPP central system URL (WebSocket endpoint)
    pub central_system_url: String,
    /// Charge point identifier
    pub charge_point_id: String,
    /// Connector ID (typically 1 for single-connector chargers)
    pub connector_id: i32,
    /// Charger capabilities
    pub capabilities: ChargerCapabilities,
    /// ID tag for remote start (user identifier)
    pub default_id_tag: String,
}

impl Default for OcppEvChargerConfig {
    fn default() -> Self {
        Self {
            central_system_url: "ws://localhost:8080/ocpp".to_string(),
            charge_point_id: "CP001".to_string(),
            connector_id: 1,
            capabilities: ChargerCapabilities {
                max_current_amps: 32.0,
                min_current_amps: 6.0,
                phases: 3,
                voltage_v: 230.0,
                connector_type: ConnectorType::Type2,
                power_max_kw: 22.0,
                supports_v2g: false,
            },
            default_id_tag: "OEC-USER-001".to_string(),
        }
    }
}

/// Internal state tracking for OCPP charger
#[derive(Debug, Clone)]
struct OcppChargerInternalState {
    /// Current charger state
    pub charger_state: ChargerState,
    /// Current transaction ID (if charging session active)
    pub transaction_id: Option<i32>,
    /// Last known OCPP status
    pub last_ocpp_status: ChargePointStatus,
    /// Last status update timestamp
    pub last_update: chrono::DateTime<Utc>,
}

impl Default for OcppChargerInternalState {
    fn default() -> Self {
        Self {
            charger_state: ChargerState {
                status: ChargerStatus::Available,
                connected: false,
                charging: false,
                current_amps: 0.0,
                power_w: 0.0,
                energy_delivered_kwh: 0.0,
                session_duration_seconds: 0,
                vehicle_soc_percent: None,
                discharging: false,
                energy_discharged_kwh: 0.0,
            },
            transaction_id: None,
            last_ocpp_status: ChargePointStatus::Available,
            last_update: Utc::now(),
        }
    }
}

/// OCPP-based EV Charger implementation
pub struct OcppEvCharger {
    config: OcppEvChargerConfig,
    ocpp_client: Arc<OcppClient>,
    internal_state: Arc<RwLock<OcppChargerInternalState>>,
}

impl OcppEvCharger {
    /// Create a new OCPP EV Charger
    pub fn new(config: OcppEvChargerConfig) -> Self {
        let ocpp_client = Arc::new(OcppClient::new(
            config.central_system_url.clone(),
            config.charge_point_id.clone(),
        ));

        Self {
            config,
            ocpp_client,
            internal_state: Arc::new(RwLock::new(OcppChargerInternalState::default())),
        }
    }

    /// Connect to OCPP central system
    pub async fn connect(&self) -> Result<()> {
        info!("Connecting OCPP charger {}", self.config.charge_point_id);
        self.ocpp_client.connect().await?;
        Ok(())
    }

    /// Disconnect from OCPP central system
    pub async fn disconnect(&self) -> Result<()> {
        info!(
            "Disconnecting OCPP charger {}",
            self.config.charge_point_id
        );
        self.ocpp_client.disconnect().await?;
        Ok(())
    }

    /// Check connection status
    pub async fn is_connected(&self) -> bool {
        self.ocpp_client.get_state().await == ConnectionState::Connected
    }

    /// Update charger state from OCPP status notification
    ///
    /// This should be called when receiving StatusNotification messages from the charger
    pub async fn handle_status_notification(&self, status: ChargePointStatus) -> Result<()> {
        let mut state = self.internal_state.write().await;

        debug!(
            "Handling OCPP status notification: {:?} for connector {}",
            status, self.config.connector_id
        );

        state.last_ocpp_status = status;
        state.last_update = Utc::now();

        // Map OCPP status to domain status
        state.charger_state.status = match status {
            ChargePointStatus::Available => ChargerStatus::Available,
            ChargePointStatus::Preparing => ChargerStatus::Preparing,
            ChargePointStatus::Charging => ChargerStatus::Charging,
            ChargePointStatus::SuspendedEVSE => ChargerStatus::SuspendedEVSE,
            ChargePointStatus::SuspendedEV => ChargerStatus::SuspendedEV,
            ChargePointStatus::Finishing => ChargerStatus::Finishing,
            ChargePointStatus::Reserved => ChargerStatus::Reserved,
            ChargePointStatus::Unavailable => ChargerStatus::Unavailable,
            ChargePointStatus::Faulted => ChargerStatus::Faulted,
        };

        // Update connection and charging flags based on status
        state.charger_state.connected = matches!(
            status,
            ChargePointStatus::Preparing
                | ChargePointStatus::Charging
                | ChargePointStatus::SuspendedEVSE
                | ChargePointStatus::SuspendedEV
                | ChargePointStatus::Finishing
        );

        state.charger_state.charging = status == ChargePointStatus::Charging;

        // If not charging, reset current and power
        if !state.charger_state.charging {
            state.charger_state.current_amps = 0.0;
            state.charger_state.power_w = 0.0;
        }

        Ok(())
    }

    /// Update charger state from meter values
    ///
    /// This should be called when receiving MeterValues messages from the charger
    pub async fn handle_meter_values(
        &self,
        energy_kwh: Option<f64>,
        power_w: Option<f64>,
        current_a: Option<f64>,
        soc_percent: Option<f64>,
    ) -> Result<()> {
        let mut state = self.internal_state.write().await;

        debug!("Handling OCPP meter values: energy={:?} kWh, power={:?} W, current={:?} A, soc={:?}%",
            energy_kwh, power_w, current_a, soc_percent);

        if let Some(energy) = energy_kwh {
            state.charger_state.energy_delivered_kwh = energy;
        }

        if let Some(power) = power_w {
            state.charger_state.power_w = power;
        }

        if let Some(current) = current_a {
            state.charger_state.current_amps = current;
        }

        if let Some(soc) = soc_percent {
            state.charger_state.vehicle_soc_percent = Some(soc);
        }

        state.last_update = Utc::now();

        Ok(())
    }

    /// Send a charging profile to the charger using SetChargingProfile
    ///
    /// This is the primary way to control charging current in OCPP
    async fn send_charging_profile(&self, max_current_amps: f64) -> Result<()> {
        debug!(
            "Sending charging profile with max current {} A",
            max_current_amps
        );

        // Create a charging profile with a single period
        let _profile = ChargingProfile {
            charging_profile_id: 1,
            stack_level: 0,
            charging_profile_purpose: ChargingProfilePurpose::TxProfile,
            charging_profile_kind: ChargingProfileKind::Absolute,
            recurrency_kind: None,
            valid_from: Some(Utc::now()),
            valid_to: None,
            charging_schedule: ChargingSchedule {
                charging_rate_unit: ChargingRateUnit::A,
                charging_schedule_period: vec![ChargingSchedulePeriod {
                    start_period: 0,
                    limit: max_current_amps,
                    number_phases: Some(self.config.capabilities.phases as i32),
                }],
                duration: None,
                start_schedule: Some(Utc::now()),
                min_charging_rate: Some(self.config.capabilities.min_current_amps),
            },
        };

        // TODO: Actually send SetChargingProfile OCPP message via WebSocket
        // For now, just log it
        info!(
            "Would send SetChargingProfile to connector {} with limit {} A",
            self.config.connector_id, max_current_amps
        );

        // Update internal state to reflect the new current limit
        let mut state = self.internal_state.write().await;
        if state.charger_state.charging {
            state.charger_state.current_amps = max_current_amps;
            state.charger_state.power_w = max_current_amps
                * self.config.capabilities.voltage_v
                * self.config.capabilities.phases as f64;
        }

        Ok(())
    }

    /// Send RemoteStartTransaction command
    async fn send_remote_start(&self) -> Result<()> {
        debug!("Sending RemoteStartTransaction");

        let _request = RemoteStartTransactionRequest {
            id_tag: self.config.default_id_tag.clone(),
            connector_id: Some(self.config.connector_id),
            charging_profile: None, // Can set initial profile here if needed
        };

        // TODO: Actually send OCPP message via WebSocket
        info!(
            "Would send RemoteStartTransaction to connector {}",
            self.config.connector_id
        );

        // For now, simulate success
        let status = RemoteStartStopStatus::Accepted;

        if status == RemoteStartStopStatus::Accepted {
            let mut state = self.internal_state.write().await;
            state.charger_state.charging = true;
            state.charger_state.status = ChargerStatus::Charging;
            state.transaction_id = Some(1); // Simulated transaction ID
            Ok(())
        } else {
            Err(ChargerError::SessionError(
                "Remote start rejected by charger".to_string(),
            )
            .into())
        }
    }

    /// Send RemoteStopTransaction command
    async fn send_remote_stop(&self) -> Result<()> {
        debug!("Sending RemoteStopTransaction");

        let state = self.internal_state.read().await;
        let transaction_id = state
            .transaction_id
            .ok_or_else(|| ChargerError::SessionError("No active transaction".to_string()))?;

        drop(state); // Release read lock

        let _request = RemoteStopTransactionRequest { transaction_id };

        // TODO: Actually send OCPP message via WebSocket
        info!("Would send RemoteStopTransaction for transaction {}", transaction_id);

        // For now, simulate success
        let status = RemoteStartStopStatus::Accepted;

        if status == RemoteStartStopStatus::Accepted {
            let mut state = self.internal_state.write().await;
            state.charger_state.charging = false;
            state.charger_state.status = ChargerStatus::SuspendedEVSE;
            state.charger_state.current_amps = 0.0;
            state.charger_state.power_w = 0.0;
            state.transaction_id = None;
            Ok(())
        } else {
            Err(ChargerError::SessionError("Remote stop rejected by charger".to_string()).into())
        }
    }
}

#[async_trait]
impl EvCharger for OcppEvCharger {
    async fn read_state(&self) -> Result<ChargerState> {
        // Check connection status first
        if !self.is_connected().await {
            warn!(
                "OCPP charger {} is not connected to central system",
                self.config.charge_point_id
            );
            // Return offline state
            return Ok(ChargerState {
                status: ChargerStatus::Unavailable,
                connected: false,
                charging: false,
                current_amps: 0.0,
                power_w: 0.0,
                energy_delivered_kwh: 0.0,
                session_duration_seconds: 0,
                vehicle_soc_percent: None,
                discharging: false,
                energy_discharged_kwh: 0.0,
            });
        }

        let state = self.internal_state.read().await;
        Ok(state.charger_state.clone())
    }

    async fn set_current(&self, amps: f64) -> Result<()> {
        // Validate connection
        if !self.is_connected().await {
            return Err(ChargerError::Offline.into());
        }

        // Validate current range
        if amps < 0.0 || amps > self.config.capabilities.max_current_amps {
            return Err(ChargerError::InvalidCurrent(amps).into());
        }

        // Clamp to minimum if non-zero
        let clamped = if amps > 0.0 {
            amps.max(self.config.capabilities.min_current_amps)
        } else {
            0.0
        };

        debug!(
            "Setting OCPP charger current to {} A (requested: {} A)",
            clamped, amps
        );

        // Send charging profile with new current limit
        self.send_charging_profile(clamped).await?;

        Ok(())
    }

    async fn start_charging(&self) -> Result<()> {
        // Validate connection
        if !self.is_connected().await {
            return Err(ChargerError::Offline.into());
        }

        let state = self.internal_state.read().await;

        // Check if vehicle is connected
        if !state.charger_state.connected {
            return Err(ChargerError::VehicleNotConnected.into());
        }

        // Check if already charging
        if state.charger_state.charging {
            debug!("Charger is already charging, ignoring start command");
            return Ok(());
        }

        // Check for fault state
        if state.charger_state.status == ChargerStatus::Faulted {
            return Err(ChargerError::Fault("Charger is in fault state".to_string()).into());
        }

        drop(state); // Release read lock

        // Send remote start transaction
        self.send_remote_start().await?;

        info!("Started charging on OCPP charger {}", self.config.charge_point_id);
        Ok(())
    }

    async fn stop_charging(&self) -> Result<()> {
        // Validate connection
        if !self.is_connected().await {
            return Err(ChargerError::Offline.into());
        }

        let state = self.internal_state.read().await;

        // Check if charging
        if !state.charger_state.charging {
            debug!("Charger is not charging, ignoring stop command");
            return Ok(());
        }

        drop(state); // Release read lock

        // Send remote stop transaction
        self.send_remote_stop().await?;

        info!("Stopped charging on OCPP charger {}", self.config.charge_point_id);
        Ok(())
    }

    fn capabilities(&self) -> ChargerCapabilities {
        self.config.capabilities.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ocpp_charger_creation() {
        let config = OcppEvChargerConfig::default();
        let charger = OcppEvCharger::new(config);

        let caps = charger.capabilities();
        assert_eq!(caps.max_current_amps, 32.0);
        assert_eq!(caps.phases, 3);
    }

    #[tokio::test]
    async fn test_ocpp_status_notification_mapping() {
        let config = OcppEvChargerConfig::default();
        let charger = OcppEvCharger::new(config);

        // Test various status mappings
        charger
            .handle_status_notification(ChargePointStatus::Available)
            .await
            .unwrap();
        let state = charger.read_state().await.unwrap();
        assert_eq!(state.status, ChargerStatus::Available);
        assert!(!state.connected);

        charger
            .handle_status_notification(ChargePointStatus::Charging)
            .await
            .unwrap();
        let state = charger.read_state().await.unwrap();
        assert_eq!(state.status, ChargerStatus::Charging);
        assert!(state.connected);
        assert!(state.charging);

        charger
            .handle_status_notification(ChargePointStatus::Faulted)
            .await
            .unwrap();
        let state = charger.read_state().await.unwrap();
        assert_eq!(state.status, ChargerStatus::Faulted);
    }

    #[tokio::test]
    async fn test_ocpp_meter_values() {
        let config = OcppEvChargerConfig::default();
        let charger = OcppEvCharger::new(config);

        // Simulate meter values update
        charger
            .handle_meter_values(Some(5.5), Some(11000.0), Some(16.0), Some(75.0))
            .await
            .unwrap();

        let state = charger.read_state().await.unwrap();
        assert_eq!(state.energy_delivered_kwh, 5.5);
        assert_eq!(state.power_w, 11000.0);
        assert_eq!(state.current_amps, 16.0);
        assert_eq!(state.vehicle_soc_percent, Some(75.0));
    }

    #[tokio::test]
    async fn test_ocpp_set_current_validation() {
        let config = OcppEvChargerConfig::default();
        let charger = OcppEvCharger::new(config);

        // Connect first
        charger.connect().await.unwrap();

        // Valid current should succeed (in simulation mode)
        let result = charger.set_current(16.0).await;
        // Will succeed in simulation
        assert!(result.is_ok());

        // Invalid current (too high) should fail
        let result = charger.set_current(100.0).await;
        assert!(result.is_err());

        // Negative current should fail
        let result = charger.set_current(-5.0).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_ocpp_start_stop_charging() {
        let config = OcppEvChargerConfig::default();
        let charger = OcppEvCharger::new(config);

        // Connect first
        charger.connect().await.unwrap();

        // Can't start without vehicle connected
        let result = charger.start_charging().await;
        assert!(result.is_err());

        // Simulate vehicle connection via status notification
        charger
            .handle_status_notification(ChargePointStatus::Preparing)
            .await
            .unwrap();

        // Now can start (in simulation mode)
        let result = charger.start_charging().await;
        assert!(result.is_ok());

        let state = charger.read_state().await.unwrap();
        assert!(state.charging);

        // Stop charging
        let result = charger.stop_charging().await;
        assert!(result.is_ok());

        let state = charger.read_state().await.unwrap();
        assert!(!state.charging);
    }

    #[tokio::test]
    async fn test_ocpp_offline_handling() {
        let config = OcppEvChargerConfig::default();
        let charger = OcppEvCharger::new(config);

        // Don't connect - charger is offline

        // Read state should return unavailable but not error
        let state = charger.read_state().await.unwrap();
        assert_eq!(state.status, ChargerStatus::Unavailable);

        // Operations should fail with offline error
        let result = charger.set_current(16.0).await;
        assert!(result.is_err());

        let result = charger.start_charging().await;
        assert!(result.is_err());
    }
}
