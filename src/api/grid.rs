//! Grid connection API endpoints

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::Serialize;

use crate::{auth::AuthBearer, controller::AppState};

/// Get current grid connection status
pub async fn get_grid_status(
    State(st): State<AppState>,
    AuthBearer(_): AuthBearer,
) -> impl IntoResponse {
    match st.controller.get_grid_status().await {
        Ok(status) => (StatusCode::OK, Json(status)).into_response(),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(ErrorResponse {
                error: e.to_string(),
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
            StatusCode::BAD_GATEWAY,
            Json(ErrorResponse {
                error: e.to_string(),
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
            StatusCode::BAD_GATEWAY,
            Json(ErrorResponse {
                error: e.to_string(),
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
