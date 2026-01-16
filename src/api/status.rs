use axum::{extract::State, Json};
use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::{
    api::{error::ApiError, response::ApiResponse},
    controller::AppState,
    domain::{battery::BatteryState, types::Power},
};

/// System status response
#[derive(Debug, Serialize)]
pub struct SystemStatus {
    timestamp: DateTime<Utc>,
    battery: BatteryStatusInfo,
    schedule: ScheduleInfo,
    forecast: ForecastInfo,
    system: SystemInfo,
}

/// Battery status information
#[derive(Debug, Serialize)]
pub struct BatteryStatusInfo {
    soc_percent: f64,
    power_w: f64,
    voltage_v: f64,
    temperature_c: f64,
    health_percent: f64,
    status: String,
}

/// Schedule information
#[derive(Debug, Serialize)]
pub struct ScheduleInfo {
    active: bool,
    last_update: Option<DateTime<Utc>>,
    next_optimization: Option<DateTime<Utc>>,
    upcoming_intervals: Vec<ScheduleInterval>,
}

/// Schedule interval
#[derive(Debug, Serialize)]
pub struct ScheduleInterval {
    time: DateTime<Utc>,
    target_power_w: f64,
}

/// Forecast information
#[derive(Debug, Serialize)]
pub struct ForecastInfo {
    last_update: Option<DateTime<Utc>>,
    next_update: Option<DateTime<Utc>>,
    price_available: bool,
    consumption_available: bool,
    production_available: bool,
}

/// System information
#[derive(Debug, Serialize)]
pub struct SystemInfo {
    uptime_seconds: u64,
    version: String,
    mode: String,
}

/// GET /api/v1/status - Get current system status
///
/// Returns a comprehensive overview of the current system state including:
/// - Battery status (SoC, power, health)
/// - Active schedule and upcoming intervals
/// - Forecast availability and timestamps
/// - System information
pub async fn get_status(State(state): State<AppState>) -> Result<Json<ApiResponse<SystemStatus>>, ApiError> {
    // Get battery state
    let battery_info = get_battery_status(&state).await?;

    // Get schedule info
    let schedule_info = get_schedule_info(&state);

    // Get forecast info
    let forecast_info = get_forecast_info(&state);

    // Get system info
    let system_info = get_system_info(&state);

    let status = SystemStatus {
        timestamp: Utc::now(),
        battery: battery_info,
        schedule: schedule_info,
        forecast: forecast_info,
        system: system_info,
    };

    Ok(Json(ApiResponse::success(status)))
}

async fn get_battery_status(state: &AppState) -> Result<BatteryStatusInfo, ApiError> {
    // Read current battery state
    let battery_state = state
        .controller
        .battery
        .read_state()
        .await
        .map_err(|e| ApiError::HardwareError(format!("Failed to read battery state: {}", e)))?;

    Ok(BatteryStatusInfo {
        soc_percent: battery_state.soc_percent,
        power_w: battery_state.power_w,
        voltage_v: battery_state.voltage_v,
        temperature_c: battery_state.temperature_c,
        health_percent: battery_state.health_percent,
        status: format!("{:?}", battery_state.status),
    })
}

fn get_schedule_info(_state: &AppState) -> ScheduleInfo {
    // TODO: Read actual schedule from controller
    // For now, return placeholder data
    ScheduleInfo {
        active: false,
        last_update: None,
        next_optimization: None,
        upcoming_intervals: vec![],
    }
}

fn get_forecast_info(_state: &AppState) -> ForecastInfo {
    // TODO: Check forecast cache/engine
    // For now, return placeholder data
    ForecastInfo {
        last_update: None,
        next_update: None,
        price_available: false,
        consumption_available: false,
        production_available: false,
    }
}

fn get_system_info(_state: &AppState) -> SystemInfo {
    SystemInfo {
        uptime_seconds: 0, // TODO: Track actual uptime
        version: env!("CARGO_PKG_VERSION").to_string(),
        mode: if cfg!(feature = "sim") {
            "simulated".to_string()
        } else {
            "production".to_string()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_info() {
        let info = SystemInfo {
            uptime_seconds: 3600,
            version: "0.2.0".to_string(),
            mode: "simulated".to_string(),
        };

        assert_eq!(info.uptime_seconds, 3600);
        assert_eq!(info.version, "0.2.0");
    }
}
