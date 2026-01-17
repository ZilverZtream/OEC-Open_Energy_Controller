#![allow(dead_code)]
use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode, HeaderMap, Request},
    middleware::{self, Next},
    response::Response,
    body::Body,
};
use tower::{Layer, Service};
use std::task::{Context, Poll};
use std::future::Future;
use std::pin::Pin;

#[derive(Clone)]
pub struct AuthConfig {
    pub token: String,
}

/// Authentication middleware layer
///
/// # Security
/// - Validates Bearer token from Authorization header
/// - Returns 401 UNAUTHORIZED on missing or invalid token
/// - Uses constant-time comparison to prevent timing attacks
#[derive(Clone)]
pub struct AuthLayer {
    token: String,
}

impl AuthLayer {
    pub fn new(token: String) -> Self {
        Self { token }
    }
}

impl<S> Layer<S> for AuthLayer {
    type Service = AuthMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthMiddleware {
            inner,
            token: self.token.clone(),
        }
    }
}

#[derive(Clone)]
pub struct AuthMiddleware<S> {
    inner: S,
    token: String,
}

impl<S> Service<Request<Body>> for AuthMiddleware<S>
where
    S: Service<Request<Body>, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let token = self.token.clone();
        let mut inner = self.inner.clone();

        Box::pin(async move {
            // Check Authorization header
            let auth_header = req
                .headers()
                .get(axum::http::header::AUTHORIZATION)
                .and_then(|v| v.to_str().ok());

            match auth_header {
                Some(auth) if auth.starts_with("Bearer ") => {
                    let provided_token = &auth[7..];
                    // Constant-time comparison to prevent timing attacks
                    if constant_time_compare(provided_token.as_bytes(), token.as_bytes()) {
                        // Token valid - proceed with request
                        inner.call(req).await
                    } else {
                        // Invalid token
                        Ok(Response::builder()
                            .status(StatusCode::UNAUTHORIZED)
                            .body(Body::empty())
                            .unwrap())
                    }
                }
                _ => {
                    // Missing or malformed Authorization header
                    Ok(Response::builder()
                        .status(StatusCode::UNAUTHORIZED)
                        .body(Body::empty())
                        .unwrap())
                }
            }
        })
    }
}

/// Constant-time comparison to prevent timing attacks
fn constant_time_compare(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut result = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        result |= x ^ y;
    }
    result == 0
}

/// Create an authentication middleware layer
///
/// This returns a middleware layer that checks for Bearer token authentication
pub fn auth_layer(token: String) -> AuthLayer {
    AuthLayer::new(token)
}

#[derive(Debug, Clone)]
pub struct AuthBearer;

#[axum::async_trait]
impl<S> FromRequestParts<S> for AuthBearer
where
    S: Send + Sync,
{
    type Rejection = StatusCode;
    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        extract_bearer_token(&parts.headers)
            .map(|_| Self)
            .ok_or(StatusCode::UNAUTHORIZED)
    }
}

fn extract_bearer_token(headers: &HeaderMap) -> Option<String> {
    let auth_header = headers.get(axum::http::header::AUTHORIZATION)?;
    let auth_str = auth_header.to_str().ok()?;

    if auth_str.starts_with("Bearer ") {
        Some(auth_str[7..].to_string())
    } else {
        None
    }
}
