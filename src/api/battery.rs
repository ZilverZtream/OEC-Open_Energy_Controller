#![allow(dead_code)]
//! Battery API endpoints

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    auth::AuthBearer,
    controller::{AppState, BatteryStateSample},
    domain::HealthStatus,
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
    let caps = st.controller.get_battery_capabilities().await;
    (StatusCode::OK, Json(caps)).into_response()
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
        Ok(()) => (
            StatusCode::OK,
            Json(SuccessResponse {
                message: "Power command applied".to_string(),
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
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub interval_minutes: Option<i64>,
}

pub async fn get_battery_history(
    State(st): State<AppState>,
    AuthBearer(_): AuthBearer,
    Query(q): Query<BatteryHistoryQuery>,
) -> impl IntoResponse {
    let end_time = q.end_time.unwrap_or_else(Utc::now);
    let start_time = q
        .start_time
        .unwrap_or_else(|| end_time - Duration::hours(24));
    let interval = q.interval_minutes.map(Duration::minutes);

    let history = st
        .controller
        .get_battery_history(start_time, end_time, interval)
        .await;
    (StatusCode::OK, Json(BatteryHistoryResponse { history })).into_response()
}

/// Get battery statistics
pub async fn get_battery_statistics(
    State(st): State<AppState>,
    AuthBearer(_): AuthBearer,
    Query(q): Query<BatteryStatisticsQuery>,
) -> impl IntoResponse {
    let end_time = q.end_time.unwrap_or_else(Utc::now);
    let start_time = q
        .start_time
        .unwrap_or_else(|| end_time - Duration::hours(24));

    match st
        .controller
        .get_battery_statistics(start_time, end_time)
        .await
    {
        Some(stats) => (StatusCode::OK, Json(stats)).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "No battery history available".to_string(),
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
pub struct BatteryHistoryResponse {
    pub history: Vec<BatteryStateSample>,
}

#[derive(Debug, Deserialize)]
pub struct BatteryStatisticsQuery {
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
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
