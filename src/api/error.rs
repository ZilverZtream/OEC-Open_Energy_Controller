#![allow(dead_code)]
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use thiserror::Error;

/// API error types that can be returned from handlers
#[derive(Debug, Error)]
pub enum ApiError {
    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Forbidden")]
    Forbidden,

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Internal server error: {0}")]
    InternalError(String),

    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Hardware error: {0}")]
    HardwareError(String),

    #[error("Optimization error: {0}")]
    OptimizationError(String),
}

/// Error response that gets serialized to JSON
#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<String>,
}

impl ApiError {
    /// Get the HTTP status code for this error
    fn status_code(&self) -> StatusCode {
        match self {
            ApiError::NotFound(_) => StatusCode::NOT_FOUND,
            ApiError::BadRequest(_) | ApiError::ValidationError(_) => StatusCode::BAD_REQUEST,
            ApiError::Unauthorized => StatusCode::UNAUTHORIZED,
            ApiError::Forbidden => StatusCode::FORBIDDEN,
            ApiError::Conflict(_) => StatusCode::CONFLICT,
            ApiError::InternalError(_)
            | ApiError::DatabaseError(_)
            | ApiError::HardwareError(_)
            | ApiError::OptimizationError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::ServiceUnavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
        }
    }

    /// Get the error type string
    fn error_type(&self) -> &'static str {
        match self {
            ApiError::NotFound(_) => "NotFound",
            ApiError::BadRequest(_) => "BadRequest",
            ApiError::ValidationError(_) => "ValidationError",
            ApiError::Unauthorized => "Unauthorized",
            ApiError::Forbidden => "Forbidden",
            ApiError::Conflict(_) => "Conflict",
            ApiError::InternalError(_) => "InternalServerError",
            ApiError::ServiceUnavailable(_) => "ServiceUnavailable",
            ApiError::DatabaseError(_) => "DatabaseError",
            ApiError::HardwareError(_) => "HardwareError",
            ApiError::OptimizationError(_) => "OptimizationError",
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let error_type = self.error_type();

        let message = match &self {
            ApiError::InternalError(_)
            | ApiError::DatabaseError(_)
            | ApiError::HardwareError(_)
            | ApiError::OptimizationError(_) => {
                tracing::error!(error = %self, "API error occurred");
                "An internal error occurred".to_string()
            }
            ApiError::ServiceUnavailable(_) => {
                tracing::warn!(error = %self, "Service unavailable");
                "Service temporarily unavailable".to_string()
            }
            _ => {
                tracing::debug!(error = %self, "Client error");
                self.to_string()
            }
        };

        let error_response = ErrorResponse {
            error: error_type.to_string(),
            message,
            details: None,
        };

        (status, Json(error_response)).into_response()
    }
}

// Conversion from common error types

#[cfg(feature = "db")]
impl From<sqlx::Error> for ApiError {
    fn from(error: sqlx::Error) -> Self {
        match error {
            sqlx::Error::RowNotFound => ApiError::NotFound("Resource not found".to_string()),
            sqlx::Error::Database(db_err) => {
                ApiError::DatabaseError(format!("Database error: {}", db_err))
            }
            _ => ApiError::DatabaseError(format!("Database error: {}", error)),
        }
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(error: anyhow::Error) -> Self {
        ApiError::InternalError(error.to_string())
    }
}

impl From<validator::ValidationErrors> for ApiError {
    fn from(errors: validator::ValidationErrors) -> Self {
        ApiError::ValidationError(errors.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_status_codes() {
        assert_eq!(
            ApiError::NotFound("test".to_string()).status_code(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            ApiError::BadRequest("test".to_string()).status_code(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(ApiError::Unauthorized.status_code(), StatusCode::UNAUTHORIZED);
        assert_eq!(
            ApiError::InternalError("test".to_string()).status_code(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[test]
    fn test_error_types() {
        assert_eq!(
            ApiError::NotFound("test".to_string()).error_type(),
            "NotFound"
        );
        assert_eq!(
            ApiError::BadRequest("test".to_string()).error_type(),
            "BadRequest"
        );
        assert_eq!(ApiError::Unauthorized.error_type(), "Unauthorized");
    }

    #[test]
    fn test_error_display() {
        let error = ApiError::NotFound("User with ID 123".to_string());
        assert_eq!(error.to_string(), "Resource not found: User with ID 123");
    }
}
