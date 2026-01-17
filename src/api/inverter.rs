#![allow(dead_code)]
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
    domain::{InverterMode, InverterState, InverterStatus},
};

#[cfg_attr(feature = "swagger", derive(utoipa::ToSchema))]
#[derive(Debug, Serialize)]
pub struct InverterResponse {
    pub state: InverterState,
    pub production_today_kwh: f64,
    pub status_ok: bool,
}

#[cfg_attr(feature = "swagger", derive(utoipa::ToSchema))]
#[derive(Debug, Deserialize)]
pub struct SetModeRequest {
    pub mode: InverterMode,
}

#[cfg_attr(feature = "swagger", derive(utoipa::ToSchema))]
#[derive(Debug, Deserialize)]
pub struct SetExportLimitRequest {
    pub limit_watts: f64,
}

/// Get inverter current state
pub async fn get_inverter_state(
    State(_st): State<AppState>,
    AuthBearer: AuthBearer,
) -> impl IntoResponse {
    // TODO: Get from actual inverter device when implemented
    let state = InverterState {
        mode: InverterMode::GridTied,
        pv_power_w: 0.0,
        ac_output_power_w: 0.0,
        dc_input_power_w: 0.0,
        grid_frequency_hz: 50.0,
        ac_voltage_v: 230.0,
        dc_voltage_v: 400.0,
        temperature_c: 35.0,
        efficiency_percent: 97.0,
        status: InverterStatus::Normal,
        daily_energy_kwh: 0.0,
        total_energy_kwh: 0.0,
    };

    let response = InverterResponse {
        status_ok: state.status == InverterStatus::Normal,
        production_today_kwh: state.daily_energy_kwh,
        state,
    };

    (StatusCode::OK, Json(response)).into_response()
}

/// Set inverter operating mode
pub async fn set_inverter_mode(
    State(_st): State<AppState>,
    AuthBearer: AuthBearer,
    Json(req): Json<SetModeRequest>,
) -> impl IntoResponse {
    // TODO: Actually set mode on real inverter
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "success": true,
            "mode": req.mode
        })),
    )
        .into_response()
}

/// Set export power limit
pub async fn set_export_limit(
    State(_st): State<AppState>,
    AuthBearer: AuthBearer,
    Json(req): Json<SetExportLimitRequest>,
) -> impl IntoResponse {
    // Validate limit
    if req.limit_watts < 0.0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "Export limit must be non-negative"
            })),
        )
            .into_response();
    }

    // TODO: Actually set limit on real inverter
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "success": true,
            "limit_watts": req.limit_watts
        })),
    )
        .into_response()
}

/// Get inverter production history
pub async fn get_production_history(
    State(_st): State<AppState>,
    AuthBearer: AuthBearer,
) -> impl IntoResponse {
    // TODO: Fetch from database
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "history": [],
            "total_kwh": 0.0,
            "today_kwh": 0.0
        })),
    )
        .into_response()
}

/// Get inverter efficiency statistics
pub async fn get_efficiency_stats(
    State(_st): State<AppState>,
    AuthBearer: AuthBearer,
) -> impl IntoResponse {
    // TODO: Calculate from historical data
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "current_efficiency": 97.0,
            "avg_efficiency_30d": 96.5,
            "max_efficiency": 97.8
        })),
    )
        .into_response()
}
