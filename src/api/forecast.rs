use axum::{extract::State, Json};
use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::{
    api::{error::ApiError, response::ApiResponse},
    controller::AppState,
    domain::forecast::{ConsumptionForecast, PriceForecast, ProductionForecast},
};

/// Combined forecast response
#[derive(Debug, Serialize)]
pub struct CombinedForecastResponse {
    timestamp: DateTime<Utc>,
    price: PriceForecastData,
    consumption: ConsumptionForecastData,
    production: ProductionForecastData,
}

/// Price forecast data
#[derive(Debug, Serialize)]
pub struct PriceForecastData {
    available: bool,
    last_update: Option<DateTime<Utc>>,
    points: Vec<PricePoint>,
}

/// Price point
#[derive(Debug, Serialize)]
pub struct PricePoint {
    timestamp: DateTime<Utc>,
    price_sek_kwh: f64,
    confidence: String,
}

/// Consumption forecast data
#[derive(Debug, Serialize)]
pub struct ConsumptionForecastData {
    available: bool,
    last_update: Option<DateTime<Utc>>,
    points: Vec<ConsumptionPoint>,
}

/// Consumption point
#[derive(Debug, Serialize)]
pub struct ConsumptionPoint {
    timestamp: DateTime<Utc>,
    power_w: f64,
    confidence: String,
}

/// Production forecast data
#[derive(Debug, Serialize)]
pub struct ProductionForecastData {
    available: bool,
    last_update: Option<DateTime<Utc>>,
    points: Vec<ProductionPoint>,
}

/// Production point
#[derive(Debug, Serialize)]
pub struct ProductionPoint {
    timestamp: DateTime<Utc>,
    power_w: f64,
    confidence: String,
}

/// GET /api/v1/forecast/price - Get price forecast
pub async fn get_price_forecast(
    State(_state): State<AppState>,
) -> Result<Json<ApiResponse<PriceForecastData>>, ApiError> {
    // TODO: Fetch from forecast engine
    Ok(Json(ApiResponse::success(PriceForecastData {
        available: false,
        last_update: None,
        points: vec![],
    })))
}

/// GET /api/v1/forecast/consumption - Get consumption forecast
pub async fn get_consumption_forecast(
    State(_state): State<AppState>,
) -> Result<Json<ApiResponse<ConsumptionForecastData>>, ApiError> {
    // TODO: Fetch from forecast engine
    Ok(Json(ApiResponse::success(ConsumptionForecastData {
        available: false,
        last_update: None,
        points: vec![],
    })))
}

/// GET /api/v1/forecast/production - Get production forecast
pub async fn get_production_forecast(
    State(_state): State<AppState>,
) -> Result<Json<ApiResponse<ProductionForecastData>>, ApiError> {
    // TODO: Fetch from forecast engine
    Ok(Json(ApiResponse::success(ProductionForecastData {
        available: false,
        last_update: None,
        points: vec![],
    })))
}

/// GET /api/v1/forecast/combined - Get combined forecast
pub async fn get_combined_forecast(
    State(_state): State<AppState>,
) -> Result<Json<ApiResponse<CombinedForecastResponse>>, ApiError> {
    // TODO: Fetch from forecast engine
    let response = CombinedForecastResponse {
        timestamp: Utc::now(),
        price: PriceForecastData {
            available: false,
            last_update: None,
            points: vec![],
        },
        consumption: ConsumptionForecastData {
            available: false,
            last_update: None,
            points: vec![],
        },
        production: ProductionForecastData {
            available: false,
            last_update: None,
            points: vec![],
        },
    };

    Ok(Json(ApiResponse::success(response)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_price_point_serialization() {
        let point = PricePoint {
            timestamp: Utc::now(),
            price_sek_kwh: 1.5,
            confidence: "high".to_string(),
        };

        let json = serde_json::to_string(&point).unwrap();
        assert!(json.contains("price_sek_kwh"));
    }
}
