#![allow(dead_code)]
use axum::{
    extract::{Path, State},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    api::{error::ApiError, response::ApiResponse},
    controller::AppState,
};

/// Schedule response
#[derive(Debug, Serialize)]
pub struct ScheduleResponse {
    id: Uuid,
    device_id: Uuid,
    created_at: DateTime<Utc>,
    valid_from: DateTime<Utc>,
    valid_until: DateTime<Utc>,
    intervals: Vec<ScheduleInterval>,
    optimizer_version: Option<String>,
    estimated_cost: Option<f64>,
}

/// Schedule interval
#[derive(Debug, Serialize, Deserialize)]
pub struct ScheduleInterval {
    timestamp: DateTime<Utc>,
    power_w: f64,
}

/// Request to create a manual schedule
#[derive(Debug, Deserialize)]
pub struct CreateScheduleRequest {
    device_id: Uuid,
    valid_from: DateTime<Utc>,
    valid_until: DateTime<Utc>,
    intervals: Vec<ScheduleInterval>,
}

/// GET /api/v1/schedule/current - Get current active schedule
#[cfg(feature = "db")]
pub async fn get_current_schedule(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Option<ScheduleResponse>>>, ApiError> {
    // Get the battery device ID (first battery found)
    let devices = state.repos.db.devices.list_all().await?;
    let battery = devices
        .iter()
        .find(|d| matches!(d.device_type, crate::database::models::device::DeviceType::Battery));

    if let Some(battery_device) = battery {
        // Find active schedule
        let schedule = state
            .repos
            .db
            .schedules
            .find_active(battery_device.id, Utc::now())
            .await?;

        if let Some(schedule_row) = schedule {
            let schedule_json: serde_json::Value = schedule_row.schedule_json.clone();
            let intervals: Vec<ScheduleInterval> = serde_json::from_value(schedule_json)
                .unwrap_or_else(|_| vec![]);

            let response = ScheduleResponse {
                id: schedule_row.id,
                device_id: schedule_row.device_id,
                created_at: schedule_row.created_at,
                valid_from: schedule_row.valid_from,
                valid_until: schedule_row.valid_until,
                intervals,
                optimizer_version: schedule_row.optimizer_version,
                estimated_cost: schedule_row.cost_estimate,
            };

            Ok(Json(ApiResponse::success(Some(response))))
        } else {
            Ok(Json(ApiResponse::success(None)))
        }
    } else {
        Ok(Json(ApiResponse::success(None)))
    }
}

/// GET /api/v1/schedule/:id - Get schedule by ID
#[cfg(feature = "db")]
pub async fn get_schedule_by_id(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<ScheduleResponse>>, ApiError> {
    let schedule_row = state
        .repos
        .db
        .schedules
        .find_by_id(id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Schedule with ID {} not found", id)))?;

    let schedule_json: serde_json::Value = schedule_row.schedule_json.clone();
    let intervals: Vec<ScheduleInterval> = serde_json::from_value(schedule_json)
        .unwrap_or_else(|_| vec![]);

    let response = ScheduleResponse {
        id: schedule_row.id,
        device_id: schedule_row.device_id,
        created_at: schedule_row.created_at,
        valid_from: schedule_row.valid_from,
        valid_until: schedule_row.valid_until,
        intervals,
        optimizer_version: schedule_row.optimizer_version,
        estimated_cost: schedule_row.cost_estimate,
    };

    Ok(Json(ApiResponse::success(response)))
}

/// POST /api/v1/schedule - Create a manual schedule
#[cfg(feature = "db")]
pub async fn create_schedule(
    State(state): State<AppState>,
    Json(request): Json<CreateScheduleRequest>,
) -> Result<Json<ApiResponse<ScheduleResponse>>, ApiError> {
    // Validate intervals
    if request.intervals.is_empty() {
        return Err(ApiError::BadRequest(
            "Schedule must have at least one interval".to_string(),
        ));
    }

    // Validate time range
    if request.valid_from >= request.valid_until {
        return Err(ApiError::BadRequest(
            "valid_from must be before valid_until".to_string(),
        ));
    }

    // Create schedule row
    let schedule_row = crate::database::models::schedule::ScheduleRow {
        id: Uuid::new_v4(),
        device_id: request.device_id,
        created_at: Utc::now(),
        valid_from: request.valid_from,
        valid_until: request.valid_until,
        schedule_json: serde_json::to_value(&request.intervals).unwrap(),
        optimizer_version: Some("manual".to_string()),
        cost_estimate: None,
    };

    // Insert into database
    state.repos.db.schedules.insert(&schedule_row).await?;

    let response = ScheduleResponse {
        id: schedule_row.id,
        device_id: schedule_row.device_id,
        created_at: schedule_row.created_at,
        valid_from: schedule_row.valid_from,
        valid_until: schedule_row.valid_until,
        intervals: request.intervals,
        optimizer_version: schedule_row.optimizer_version,
        estimated_cost: schedule_row.cost_estimate,
    };

    Ok(Json(ApiResponse::success(response)))
}

/// DELETE /api/v1/schedule/:id - Invalidate a schedule
#[cfg(feature = "db")]
pub async fn invalidate_schedule(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    state.repos.db.schedules.invalidate(id).await?;
    Ok(Json(ApiResponse::success(())))
}

// Stub implementations when db feature is not enabled
#[cfg(not(feature = "db"))]
pub async fn get_current_schedule(
    State(_state): State<AppState>,
) -> Result<Json<ApiResponse<Option<ScheduleResponse>>>, ApiError> {
    Ok(Json(ApiResponse::success(None)))
}

#[cfg(not(feature = "db"))]
pub async fn get_schedule_by_id(
    State(_state): State<AppState>,
    Path(_id): Path<Uuid>,
) -> Result<Json<ApiResponse<ScheduleResponse>>, ApiError> {
    Err(ApiError::ServiceUnavailable(
        "Database feature not enabled".to_string(),
    ))
}

#[cfg(not(feature = "db"))]
pub async fn create_schedule(
    State(_state): State<AppState>,
    Json(_request): Json<CreateScheduleRequest>,
) -> Result<Json<ApiResponse<ScheduleResponse>>, ApiError> {
    Err(ApiError::ServiceUnavailable(
        "Database feature not enabled".to_string(),
    ))
}

#[cfg(not(feature = "db"))]
pub async fn invalidate_schedule(
    State(_state): State<AppState>,
    Path(_id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    Err(ApiError::ServiceUnavailable(
        "Database feature not enabled".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schedule_interval_serialization() {
        let interval = ScheduleInterval {
            timestamp: Utc::now(),
            power_w: 5000.0,
        };

        let json = serde_json::to_string(&interval).unwrap();
        assert!(json.contains("power_w"));
    }
}
