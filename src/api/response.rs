use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use chrono::{DateTime, Utc};
use serde::Serialize;

/// Standard API response wrapper
#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    /// Whether the request was successful
    pub success: bool,
    /// Response data (present on success)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    /// Error message (present on failure)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Response timestamp
    pub timestamp: DateTime<Utc>,
    /// Request metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ResponseMetadata>,
}

/// Additional metadata about the response
#[derive(Debug, Serialize)]
pub struct ResponseMetadata {
    /// Total count of items (for paginated responses)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_count: Option<usize>,
    /// Current page number
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<usize>,
    /// Page size
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_size: Option<usize>,
    /// Processing duration in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

impl<T: Serialize> ApiResponse<T> {
    /// Create a successful response with data
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            timestamp: Utc::now(),
            metadata: None,
        }
    }

    /// Create an error response
    pub fn error(message: impl Into<String>) -> ApiResponse<()> {
        ApiResponse {
            success: false,
            data: None,
            error: Some(message.into()),
            timestamp: Utc::now(),
            metadata: None,
        }
    }

    /// Add metadata to the response
    pub fn with_metadata(mut self, metadata: ResponseMetadata) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Add total count to metadata
    pub fn with_count(mut self, count: usize) -> Self {
        let metadata = self.metadata.take().unwrap_or_else(|| ResponseMetadata {
            total_count: None,
            page: None,
            page_size: None,
            duration_ms: None,
        });

        self.metadata = Some(ResponseMetadata {
            total_count: Some(count),
            ..metadata
        });
        self
    }

    /// Add pagination info to metadata
    pub fn with_pagination(mut self, page: usize, page_size: usize, total: usize) -> Self {
        let metadata = self.metadata.take().unwrap_or_else(|| ResponseMetadata {
            total_count: None,
            page: None,
            page_size: None,
            duration_ms: None,
        });

        self.metadata = Some(ResponseMetadata {
            total_count: Some(total),
            page: Some(page),
            page_size: Some(page_size),
            ..metadata
        });
        self
    }

    /// Add processing duration to metadata
    pub fn with_duration(mut self, duration_ms: u64) -> Self {
        let metadata = self.metadata.take().unwrap_or_else(|| ResponseMetadata {
            total_count: None,
            page: None,
            page_size: None,
            duration_ms: None,
        });

        self.metadata = Some(ResponseMetadata {
            duration_ms: Some(duration_ms),
            ..metadata
        });
        self
    }
}

impl<T: Serialize> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> Response {
        let status = if self.success {
            StatusCode::OK
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        };

        (status, Json(self)).into_response()
    }
}

/// Helper to create a success response
pub fn success<T: Serialize>(data: T) -> ApiResponse<T> {
    ApiResponse::success(data)
}

/// Helper to create an error response
pub fn error(message: impl Into<String>) -> ApiResponse<()> {
    ApiResponse::<()>::error(message)
}

/// Helper to create a success response with pagination
pub fn success_with_pagination<T: Serialize>(
    data: T,
    page: usize,
    page_size: usize,
    total: usize,
) -> ApiResponse<T> {
    ApiResponse::success(data).with_pagination(page, page_size, total)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_success_response() {
        let response = ApiResponse::success("test data");
        assert!(response.success);
        assert_eq!(response.data, Some("test data"));
        assert!(response.error.is_none());
    }

    #[test]
    fn test_error_response() {
        let response: ApiResponse<()> = ApiResponse::error("test error");
        assert!(!response.success);
        assert!(response.data.is_none());
        assert_eq!(response.error, Some("test error".to_string()));
    }

    #[test]
    fn test_response_with_metadata() {
        let response = ApiResponse::success("data")
            .with_count(100)
            .with_duration(50);

        assert!(response.success);
        assert!(response.metadata.is_some());

        let metadata = response.metadata.unwrap();
        assert_eq!(metadata.total_count, Some(100));
        assert_eq!(metadata.duration_ms, Some(50));
    }

    #[test]
    fn test_response_with_pagination() {
        let response = ApiResponse::success("data")
            .with_pagination(2, 20, 100);

        let metadata = response.metadata.unwrap();
        assert_eq!(metadata.page, Some(2));
        assert_eq!(metadata.page_size, Some(20));
        assert_eq!(metadata.total_count, Some(100));
    }

    #[test]
    fn test_helper_functions() {
        let success_resp = success("data");
        assert!(success_resp.success);

        let error_resp = error("error message");
        assert!(!error_resp.success);

        let paginated = success_with_pagination("data", 1, 10, 50);
        assert!(paginated.metadata.is_some());
    }
}
