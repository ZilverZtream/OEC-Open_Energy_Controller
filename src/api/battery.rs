//! Battery API endpoints
//! TODO: Re-enable once BatteryController implements missing methods

#![allow(dead_code, unused_variables)]

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::{
    auth::AuthBearer,
    controller::AppState,
    domain::{BatteryCapabilities, BatteryState, HealthStatus},
};

/// Get current battery state
pub async fn get_battery_state(
    State(st): State<AppState>,
    AuthBearer(_): AuthBearer,
) -> impl IntoResponse {
    match st.controller.get_current_state().await {
        Ok(state) => (StatusCode::OK, Json(state)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to get battery state: {}", e),
            }),
        )
            .into_response(),
    }
}

/// Get battery capabilities
pub async fn get_battery_capabilities(
    State(_st): State<AppState>,
    AuthBearer(_): AuthBearer,
) -> impl IntoResponse {
    // TODO: Implement once BatteryController has get_battery_capabilities method
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(ErrorResponse {
            error: "Method not yet implemented".to_string(),
        }),
    )
        .into_response()
}

/// Get battery health status
pub async fn get_battery_health(
    State(_st): State<AppState>,
    AuthBearer(_): AuthBearer,
) -> impl IntoResponse {
    // TODO: Implement once BatteryController has get_battery_health method
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(ErrorResponse {
            error: "Method not yet implemented".to_string(),
        }),
    )
        .into_response()
}

/// Set battery power command
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(utoipa::ToSchema))]
pub struct SetPowerRequest {
    pub power_w: f64,
}

pub async fn set_battery_power(
    State(_st): State<AppState>,
    AuthBearer(_): AuthBearer,
    Json(_req): Json<SetPowerRequest>,
) -> impl IntoResponse {
    // TODO: Implement once BatteryController has set_battery_power method
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(ErrorResponse {
            error: "Method not yet implemented".to_string(),
        }),
    )
        .into_response()
}

/// Get battery history for a time range
#[derive(Debug, Deserialize)]
pub struct BatteryHistoryQuery {
    pub hours: Option<u32>,
}

pub async fn get_battery_history(
    State(_st): State<AppState>,
    AuthBearer(_): AuthBearer,
    Query(_q): Query<BatteryHistoryQuery>,
) -> impl IntoResponse {
    // TODO: Implement once BatteryController has get_battery_history method
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(ErrorResponse {
            error: "Method not yet implemented".to_string(),
        }),
    )
        .into_response()
}

/// Get battery statistics
pub async fn get_battery_statistics(
    State(_st): State<AppState>,
    AuthBearer(_): AuthBearer,
) -> impl IntoResponse {
    // TODO: Implement once BatteryController has get_battery_statistics method
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(ErrorResponse {
            error: "Method not yet implemented".to_string(),
        }),
    )
        .into_response()
}

// Response types
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(utoipa::ToSchema))]
pub struct HealthResponse {
    pub health: HealthStatus,
}

#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(utoipa::ToSchema))]
pub struct SuccessResponse {
    pub message: String,
}

#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(utoipa::ToSchema))]
pub struct ErrorResponse {
    pub error: String,
}
