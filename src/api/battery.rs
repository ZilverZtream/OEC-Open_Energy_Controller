//! Battery API endpoints

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
    State(st): State<AppState>,
    AuthBearer(_): AuthBearer,
) -> impl IntoResponse {
    match st.controller.get_battery_capabilities().await {
        Ok(caps) => (StatusCode::OK, Json(caps)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to get battery capabilities: {}", e),
            }),
        )
            .into_response(),
    }
}

/// Get battery health status
pub async fn get_battery_health(
    State(st): State<AppState>,
    AuthBearer(_): AuthBearer,
) -> impl IntoResponse {
    match st.controller.get_battery_health().await {
        Ok(health) => (StatusCode::OK, Json(HealthResponse { health })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to get battery health: {}", e),
            }),
        )
            .into_response(),
    }
}

/// Set battery power command
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(utoipa::ToSchema))]
pub struct SetPowerRequest {
    pub power_w: f64,
}

pub async fn set_battery_power(
    State(st): State<AppState>,
    AuthBearer(_): AuthBearer,
    Json(req): Json<SetPowerRequest>,
) -> impl IntoResponse {
    match st.controller.set_battery_power(req.power_w).await {
        Ok(_) => (
            StatusCode::OK,
            Json(SuccessResponse {
                message: format!("Battery power set to {} W", req.power_w),
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("Failed to set battery power: {}", e),
            }),
        )
            .into_response(),
    }
}

/// Get battery history for a time range
#[derive(Debug, Deserialize)]
pub struct BatteryHistoryQuery {
    pub hours: Option<u32>,
}

pub async fn get_battery_history(
    State(st): State<AppState>,
    AuthBearer(_): AuthBearer,
    Query(q): Query<BatteryHistoryQuery>,
) -> impl IntoResponse {
    let hours = q.hours.unwrap_or(24);

    match st.controller.get_battery_history(hours).await {
        Ok(history) => (StatusCode::OK, Json(history)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to get battery history: {}", e),
            }),
        )
            .into_response(),
    }
}

/// Get battery statistics
pub async fn get_battery_statistics(
    State(st): State<AppState>,
    AuthBearer(_): AuthBearer,
) -> impl IntoResponse {
    match st.controller.get_battery_statistics().await {
        Ok(stats) => (StatusCode::OK, Json(stats)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to get battery statistics: {}", e),
            }),
        )
            .into_response(),
    }
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
