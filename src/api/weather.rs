//! Weather forecast API endpoints

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::{auth::AuthBearer, controller::AppState, forecast::GeoLocation};

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
    let (latitude, longitude) = match (q.latitude, q.longitude) {
        (Some(lat), Some(lon)) => (lat, lon),
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "latitude and longitude are required".to_string(),
                }),
            )
                .into_response();
        }
    };

    let location = GeoLocation {
        latitude,
        longitude,
        name: None,
    };

    match st.controller.get_weather_forecast(location).await {
        Ok(forecast) => (StatusCode::OK, Json(forecast)).into_response(),
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
