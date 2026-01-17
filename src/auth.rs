#![allow(dead_code)]
use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode, HeaderMap, Request},
    middleware::{self, Next},
    response::Response,
    body::Body,
};

#[derive(Clone)]
pub struct AuthConfig {
    pub token: String,
}

/// Create an authentication middleware layer
///
/// This returns a middleware layer that checks for Bearer token authentication
pub fn auth_layer(token: String) -> impl Clone {
    middleware::from_fn::<_, Response>(move |req: Request<Body>, next: Next| {
        let token = token.clone();
        async move {
            let auth_header = req
                .headers()
                .get(axum::http::header::AUTHORIZATION)
                .and_then(|v| v.to_str().ok());

            match auth_header {
                Some(auth) if auth.starts_with("Bearer ") => {
                    let provided_token = &auth[7..];
                    if provided_token == token {
                        Ok::<_, StatusCode>(next.run(req).await)
                    } else {
                        Err(StatusCode::UNAUTHORIZED)
                    }
                }
                _ => Err(StatusCode::UNAUTHORIZED),
            }
        }
    })
}

#[derive(Debug, Clone)]
pub struct AuthBearer(pub uuid::Uuid);

#[axum::async_trait]
impl<S> FromRequestParts<S> for AuthBearer
where
    S: Send + Sync,
{
    type Rejection = StatusCode;
    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        extract_bearer_token(&parts.headers)
            .map(|_| Self(uuid::Uuid::new_v4()))
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
