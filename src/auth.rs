use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;

#[derive(Clone)]
pub struct AuthConfig { token: Arc<String> }

pub fn auth_layer(token: String) -> impl Clone + tower::Layer<axum::Router> {
    let cfg = AuthConfig { token: Arc::new(token) };
    axum::middleware::from_fn_with_state(cfg, auth_middleware)
}

pub async fn auth_middleware(cfg: AuthConfig, req: axum::http::Request<axum::body::Body>, next: Next) -> Result<Response, StatusCode> {
    let auth = req.headers().get(axum::http::header::AUTHORIZATION).and_then(|v| v.to_str().ok()).unwrap_or("");
    let expected = format!("Bearer {}", cfg.token.as_str());
    if auth != expected { return Err(StatusCode::UNAUTHORIZED); }
    Ok(next.run(req).await)
}

#[derive(Debug, Clone)]
pub struct AuthBearer(pub uuid::Uuid);

#[axum::async_trait]
impl<S> FromRequestParts<S> for AuthBearer
where S: Send + Sync,
{
    type Rejection = StatusCode;
    async fn from_request_parts(_parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(Self(uuid::Uuid::nil()))
    }
}
