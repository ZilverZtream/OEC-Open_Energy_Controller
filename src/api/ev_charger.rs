use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::{
    auth::AuthBearer,
    controller::AppState,
    domain::{ChargerState, ChargerStatus},
};

#[cfg_attr(feature = "swagger", derive(utoipa::ToSchema))]
#[derive(Debug, Serialize)]
pub struct EvChargerResponse {
    pub state: ChargerState,
    pub available: bool,
    pub session_active: bool,
}

#[cfg_attr(feature = "swagger", derive(utoipa::ToSchema))]
#[derive(Debug, Deserialize)]
pub struct SetCurrentRequest {
    pub current_amps: f64,
}

#[cfg_attr(feature = "swagger", derive(utoipa::ToSchema))]
#[derive(Debug, Deserialize)]
pub struct StartChargingRequest {
    pub max_current_amps: Option<f64>,
}

/// Get EV charger current state
pub async fn get_charger_state(
    State(_st): State<AppState>,
    AuthBearer(_): AuthBearer,
) -> impl IntoResponse {
    // TODO: Get from actual EV charger device when implemented
    let state = ChargerState {
        status: ChargerStatus::Available,
        connected: false,
        charging: false,
        current_amps: 0.0,
        power_w: 0.0,
        energy_delivered_kwh: 0.0,
        session_duration_seconds: 0,
        vehicle_soc_percent: None,
    };

    let response = EvChargerResponse {
        available: state.status == ChargerStatus::Available,
        session_active: state.charging,
        state,
    };

    (StatusCode::OK, Json(response)).into_response()
}

/// Set charging current
pub async fn set_charging_current(
    State(_st): State<AppState>,
    AuthBearer(_): AuthBearer,
    Json(req): Json<SetCurrentRequest>,
) -> impl IntoResponse {
    // Validate current range
    if !(6.0..=32.0).contains(&req.current_amps) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "Current must be between 6A and 32A"
            })),
        )
            .into_response();
    }

    // TODO: Actually set current on real charger
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "success": true,
            "current_amps": req.current_amps
        })),
    )
        .into_response()
}

/// Start charging session
pub async fn start_charging(
    State(_st): State<AppState>,
    AuthBearer(_): AuthBearer,
    Json(_req): Json<StartChargingRequest>,
) -> impl IntoResponse {
    // TODO: Start actual charging session
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "success": true,
            "message": "Charging started"
        })),
    )
        .into_response()
}

/// Stop charging session
pub async fn stop_charging(
    State(_st): State<AppState>,
    AuthBearer(_): AuthBearer,
) -> impl IntoResponse {
    // TODO: Stop actual charging session
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "success": true,
            "message": "Charging stopped"
        })),
    )
        .into_response()
}

/// Get charging session history
pub async fn get_charging_sessions(
    State(_st): State<AppState>,
    AuthBearer(_): AuthBearer,
) -> impl IntoResponse {
    // TODO: Fetch from database
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "sessions": []
        })),
    )
        .into_response()
}
