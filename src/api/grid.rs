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
    State(_st): State<AppState>,
    AuthBearer(_): AuthBearer,
) -> impl IntoResponse {
    // TODO: Implement once BatteryController has get_grid_status method
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(ErrorResponse {
            error: "Method not yet implemented".to_string(),
        }),
    )
        .into_response()
}

/// Get grid limits and tariff information
pub async fn get_grid_limits(
    State(_st): State<AppState>,
    AuthBearer(_): AuthBearer,
) -> impl IntoResponse {
    // TODO: Implement once BatteryController has get_grid_limits method
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(ErrorResponse {
            error: "Method not yet implemented".to_string(),
        }),
    )
        .into_response()
}

/// Get grid import/export statistics
pub async fn get_grid_statistics(
    State(_st): State<AppState>,
    AuthBearer(_): AuthBearer,
) -> impl IntoResponse {
    // TODO: Implement once BatteryController has get_grid_statistics method
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(ErrorResponse {
            error: "Method not yet implemented".to_string(),
        }),
    )
        .into_response()
}

#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(utoipa::ToSchema))]
pub struct ErrorResponse {
    pub error: String,
}
