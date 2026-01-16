#![allow(dead_code)]
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::Serialize;

use crate::controller::AppState;

/// Health check response
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    status: String,
    timestamp: chrono::DateTime<chrono::Utc>,
    checks: HealthChecks,
}

/// Individual health checks
#[derive(Debug, Serialize)]
pub struct HealthChecks {
    #[cfg(feature = "db")]
    database: ComponentHealth,
    controller: ComponentHealth,
}

/// Health status of a component
#[derive(Debug, Serialize)]
pub struct ComponentHealth {
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    latency_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

impl ComponentHealth {
    fn healthy(latency_ms: u64) -> Self {
        Self {
            status: "healthy".to_string(),
            latency_ms: Some(latency_ms),
            error: None,
        }
    }

    fn unhealthy(error: String) -> Self {
        Self {
            status: "unhealthy".to_string(),
            latency_ms: None,
            error: Some(error),
        }
    }
}

/// GET /health - Health check endpoint
///
/// Returns the health status of the application and its dependencies
#[cfg(feature = "db")]
pub async fn health_check(State(state): State<AppState>) -> impl IntoResponse {
    let start = Instant::now();

    // Check database connectivity
    let db_health = match check_database(&state).await {
        Ok(latency) => ComponentHealth::healthy(latency),
        Err(e) => ComponentHealth::unhealthy(e.to_string()),
    };

    // Check controller status
    let controller_health = check_controller(&state);

    let all_healthy = db_health.status == "healthy" && controller_health.status == "healthy";

    let response = HealthResponse {
        status: if all_healthy {
            "healthy".to_string()
        } else {
            "degraded".to_string()
        },
        timestamp: chrono::Utc::now(),
        checks: HealthChecks {
            database: db_health,
            controller: controller_health,
        },
    };

    let status_code = if all_healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    let total_duration = start.elapsed().as_millis() as u64;
    tracing::debug!(duration_ms = total_duration, healthy = all_healthy, "Health check completed");

    (status_code, Json(response))
}

/// GET /health - Health check endpoint (without database feature)
#[cfg(not(feature = "db"))]
pub async fn health_check(State(state): State<AppState>) -> impl IntoResponse {
    // Check controller status
    let controller_health = check_controller(&state);

    let all_healthy = controller_health.status == "healthy";

    let response = HealthResponse {
        status: if all_healthy {
            "healthy".to_string()
        } else {
            "degraded".to_string()
        },
        timestamp: chrono::Utc::now(),
        checks: HealthChecks {
            controller: controller_health,
        },
    };

    let status_code = if all_healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (status_code, Json(response))
}

/// Check database connectivity
#[cfg(feature = "db")]
async fn check_database(state: &AppState) -> anyhow::Result<u64> {
    let start = Instant::now();

    // Execute a simple query to check connectivity
    sqlx::query("SELECT 1")
        .execute(&state.repos.db.pool)
        .await?;

    let latency = start.elapsed().as_millis() as u64;
    Ok(latency)
}

/// Check controller status
fn check_controller(_state: &AppState) -> ComponentHealth {
    // For now, just return healthy if controller exists
    // In the future, check if control loop is running, last update time, etc.
    ComponentHealth::healthy(0)
}

/// GET /health/ready - Readiness probe for Kubernetes
///
/// Returns 200 if the application is ready to serve traffic
pub async fn readiness_check(State(state): State<AppState>) -> impl IntoResponse {
    #[cfg(feature = "db")]
    {
        match check_database(&state).await {
            Ok(_) => StatusCode::OK,
            Err(_) => StatusCode::SERVICE_UNAVAILABLE,
        }
    }

    #[cfg(not(feature = "db"))]
    {
        let _ = state;
        StatusCode::OK
    }
}

/// GET /health/live - Liveness probe for Kubernetes
///
/// Returns 200 if the application is running
pub async fn liveness_check() -> impl IntoResponse {
    StatusCode::OK
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_component_health_healthy() {
        let health = ComponentHealth::healthy(42);
        assert_eq!(health.status, "healthy");
        assert_eq!(health.latency_ms, Some(42));
        assert!(health.error.is_none());
    }

    #[test]
    fn test_component_health_unhealthy() {
        let health = ComponentHealth::unhealthy("Connection failed".to_string());
        assert_eq!(health.status, "unhealthy");
        assert!(health.latency_ms.is_none());
        assert_eq!(health.error, Some("Connection failed".to_string()));
    }
}
