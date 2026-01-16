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
    State(st): State<AppState>,
    AuthBearer(_): AuthBearer,
    Query(q): Query<WeatherQuery>,
) -> impl IntoResponse {
    let location = GeoLocation {
        latitude: q.latitude.unwrap_or(59.3293), // Default to Stockholm
        longitude: q.longitude.unwrap_or(18.0686),
        name: Some("Stockholm".to_string()),
    };

    match st.controller.get_weather_forecast(location).await {
        Ok(forecast) => (StatusCode::OK, Json(forecast)).into_response(),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(ErrorResponse {
                error: format!("Failed to fetch weather forecast: {}", e),
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
