//! Grid connection API endpoints

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Serialize;

use crate::{
    auth::AuthBearer,
    controller::AppState,
    domain::{GridConnection, GridLimits},
};

/// Get current grid connection status
pub async fn get_grid_status(
    State(st): State<AppState>,
    AuthBearer(_): AuthBearer,
) -> impl IntoResponse {
    match st.controller.get_grid_status().await {
        Ok(status) => (StatusCode::OK, Json(status)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to get grid status: {}", e),
            }),
        )
            .into_response(),
    }
}

/// Get grid limits and tariff information
pub async fn get_grid_limits(
    State(st): State<AppState>,
    AuthBearer(_): AuthBearer,
) -> impl IntoResponse {
    match st.controller.get_grid_limits().await {
        Ok(limits) => (StatusCode::OK, Json(limits)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to get grid limits: {}", e),
            }),
        )
            .into_response(),
    }
}

/// Get grid import/export statistics
pub async fn get_grid_statistics(
    State(st): State<AppState>,
    AuthBearer(_): AuthBearer,
) -> impl IntoResponse {
    match st.controller.get_grid_statistics().await {
        Ok(stats) => (StatusCode::OK, Json(stats)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to get grid statistics: {}", e),
            }),
        )
            .into_response(),
    }
}

#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(utoipa::ToSchema))]
pub struct ErrorResponse {
    pub error: String,
}
