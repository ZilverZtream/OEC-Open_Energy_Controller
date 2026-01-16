//! Weather forecast API endpoints

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
    forecast::{GeoLocation, WeatherForecast},
};

/// Get weather forecast for a location
#[derive(Debug, Deserialize)]
pub struct WeatherQuery {
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

pub async fn get_weather_forecast(
    State(_st): State<AppState>,
    AuthBearer(_): AuthBearer,
    Query(_q): Query<WeatherQuery>,
) -> impl IntoResponse {
    // TODO: Implement once BatteryController has get_weather_forecast method
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
