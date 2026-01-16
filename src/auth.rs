#![allow(dead_code)]
use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
};

#[derive(Clone)]
pub struct AuthConfig {
    pub token: String,
}

// Simplified auth layer that just returns an identity layer for now
// TODO: Implement proper authentication middleware
pub fn auth_layer(_token: String) -> tower::layer::util::Identity {
    tower::layer::util::Identity::new()
}

#[derive(Debug, Clone)]
pub struct AuthBearer(pub uuid::Uuid);

#[axum::async_trait]
impl<S> FromRequestParts<S> for AuthBearer
where
    S: Send + Sync,
{
    type Rejection = StatusCode;
    async fn from_request_parts(_parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(Self(uuid::Uuid::nil()))
    }
}
