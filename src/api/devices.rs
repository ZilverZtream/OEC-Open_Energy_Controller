use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    api::{error::ApiError, response::ApiResponse},
    controller::AppState,
};

#[cfg(feature = "db")]
use crate::database::models::device::{Device, DeviceType};

/// Device list response
#[derive(Debug, Serialize)]
pub struct DeviceListResponse {
    devices: Vec<DeviceInfo>,
    total: usize,
}

/// Device information
#[derive(Debug, Serialize)]
pub struct DeviceInfo {
    id: Uuid,
    device_type: String,
    manufacturer: Option<String>,
    model: Option<String>,
    ip_address: Option<String>,
    port: Option<i32>,
    online: bool,
    last_seen: Option<chrono::DateTime<chrono::Utc>>,
}

/// Request to add a new device
#[derive(Debug, Deserialize)]
pub struct AddDeviceRequest {
    device_type: String,
    manufacturer: Option<String>,
    model: Option<String>,
    ip_address: String,
    port: i32,
    config: Option<serde_json::Value>,
}

/// Request to update a device
#[derive(Debug, Deserialize)]
pub struct UpdateDeviceRequest {
    manufacturer: Option<String>,
    model: Option<String>,
    ip_address: Option<String>,
    port: Option<i32>,
    config: Option<serde_json::Value>,
}

/// GET /api/v1/devices - List all devices
#[cfg(feature = "db")]
pub async fn list_devices(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<DeviceListResponse>>, ApiError> {
    let devices = state.repos.db.devices.list_all().await?;

    let device_infos: Vec<DeviceInfo> = devices
        .iter()
        .map(|d| DeviceInfo {
            id: d.id,
            device_type: format!("{:?}", d.device_type),
            manufacturer: d.manufacturer.clone(),
            model: d.model.clone(),
            ip_address: d.ip_address.clone(),
            port: d.port,
            online: d.is_online(300),  // Consider online if seen in last 5 minutes
            last_seen: d.last_seen,
        })
        .collect();

    let total = device_infos.len();

    Ok(Json(ApiResponse::success(DeviceListResponse {
        devices: device_infos,
        total,
    })))
}

/// GET /api/v1/devices/:id - Get device by ID
#[cfg(feature = "db")]
pub async fn get_device(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<DeviceInfo>>, ApiError> {
    let device = state
        .repos
        .db
        .devices
        .find_by_id(id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Device with ID {} not found", id)))?;

    let device_info = DeviceInfo {
        id: device.id,
        device_type: format!("{:?}", device.device_type),
        manufacturer: device.manufacturer.clone(),
        model: device.model.clone(),
        ip_address: device.ip_address.clone(),
        port: device.port,
        online: device.is_online(300),
        last_seen: device.last_seen,
    };

    Ok(Json(ApiResponse::success(device_info)))
}

/// POST /api/v1/devices - Add a new device
#[cfg(feature = "db")]
pub async fn add_device(
    State(state): State<AppState>,
    Json(request): Json<AddDeviceRequest>,
) -> Result<Json<ApiResponse<DeviceInfo>>, ApiError> {
    // Parse device type
    let device_type = parse_device_type(&request.device_type)?;

    // Create new device
    let device = Device {
        id: Uuid::new_v4(),
        device_type,
        manufacturer: request.manufacturer,
        model: request.model,
        ip_address: Some(request.ip_address),
        port: Some(request.port),
        config: request.config,
        discovered_at: chrono::Utc::now(),
        last_seen: chrono::Utc::now(),
        enabled: true,
    };

    // Insert into database
    state.repos.db.devices.insert(&device).await?;

    let device_info = DeviceInfo {
        id: device.id,
        device_type: format!("{:?}", device.device_type),
        manufacturer: device.manufacturer,
        model: device.model,
        ip_address: device.ip_address,
        port: device.port,
        online: true,
        last_seen: device.last_seen,
    };

    Ok(Json(ApiResponse::success(device_info)))
}

/// PUT /api/v1/devices/:id - Update a device
#[cfg(feature = "db")]
pub async fn update_device(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(_request): Json<UpdateDeviceRequest>,
) -> Result<Json<ApiResponse<DeviceInfo>>, ApiError> {
    // Check if device exists
    let device = state
        .repos
        .db
        .devices
        .find_by_id(id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Device with ID {} not found", id)))?;

    // TODO: Implement update logic
    // For now, just return the existing device

    let device_info = DeviceInfo {
        id: device.id,
        device_type: format!("{:?}", device.device_type),
        manufacturer: device.manufacturer,
        model: device.model,
        ip_address: device.ip_address,
        port: device.port,
        online: device.is_online(300),
        last_seen: device.last_seen,
    };

    Ok(Json(ApiResponse::success(device_info)))
}

/// DELETE /api/v1/devices/:id - Delete a device
#[cfg(feature = "db")]
pub async fn delete_device(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    // Check if device exists
    let _ = state
        .repos
        .db
        .devices
        .find_by_id(id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Device with ID {} not found", id)))?;

    // Delete the device
    state.repos.db.devices.delete(id).await?;

    Ok(Json(ApiResponse::success(())))
}

#[cfg(feature = "db")]
fn parse_device_type(type_str: &str) -> Result<DeviceType, ApiError> {
    match type_str.to_lowercase().as_str() {
        "battery" => Ok(DeviceType::Battery),
        "inverter" => Ok(DeviceType::Inverter),
        "ev_charger" | "evcharger" => Ok(DeviceType::EvCharger),
        "meter" => Ok(DeviceType::Meter),
        _ => Err(ApiError::BadRequest(format!(
            "Invalid device type: {}. Must be one of: battery, inverter, ev_charger, meter",
            type_str
        ))),
    }
}

// Stub implementations when db feature is not enabled
#[cfg(not(feature = "db"))]
pub async fn list_devices(
    State(_state): State<AppState>,
) -> Result<Json<ApiResponse<DeviceListResponse>>, ApiError> {
    Ok(Json(ApiResponse::success(DeviceListResponse {
        devices: vec![],
        total: 0,
    })))
}

#[cfg(not(feature = "db"))]
pub async fn get_device(
    State(_state): State<AppState>,
    Path(_id): Path<Uuid>,
) -> Result<Json<ApiResponse<DeviceInfo>>, ApiError> {
    Err(ApiError::ServiceUnavailable(
        "Database feature not enabled".to_string(),
    ))
}

#[cfg(not(feature = "db"))]
pub async fn add_device(
    State(_state): State<AppState>,
    Json(_request): Json<AddDeviceRequest>,
) -> Result<Json<ApiResponse<DeviceInfo>>, ApiError> {
    Err(ApiError::ServiceUnavailable(
        "Database feature not enabled".to_string(),
    ))
}

#[cfg(not(feature = "db"))]
pub async fn update_device(
    State(_state): State<AppState>,
    Path(_id): Path<Uuid>,
    Json(_request): Json<UpdateDeviceRequest>,
) -> Result<Json<ApiResponse<DeviceInfo>>, ApiError> {
    Err(ApiError::ServiceUnavailable(
        "Database feature not enabled".to_string(),
    ))
}

#[cfg(not(feature = "db"))]
pub async fn delete_device(
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
    #[cfg(feature = "db")]
    fn test_parse_device_type() {
        assert!(matches!(
            parse_device_type("battery").unwrap(),
            DeviceType::Battery
        ));
        assert!(matches!(
            parse_device_type("inverter").unwrap(),
            DeviceType::Inverter
        ));
        assert!(matches!(
            parse_device_type("ev_charger").unwrap(),
            DeviceType::EvCharger
        ));
        assert!(parse_device_type("invalid").is_err());
    }
}
