#![allow(dead_code)]
use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
};

#[derive(Clone)]
pub struct AuthConfig {
    pub token: String,
}

// ============================================================================
// ⚠️  CRITICAL SECURITY WARNING ⚠️
// ============================================================================
// This authentication system is currently DISABLED and provides NO security.
//
// ISSUES:
// 1. auth_layer() returns Identity (no-op) - ALL requests bypass authentication
// 2. AuthBearer always returns Uuid::nil() - NO user identification
// 3. Any attacker can send power commands without authorization
// 4. This allows unauthorized control of battery charging/discharging
//
// RISKS:
// - Unauthorized power commands could damage battery or electrical system
// - Attackers could cause financial loss by manipulating charging schedules
// - No audit trail of who issued commands
//
// TODO (URGENT):
// - Implement Bearer token validation using JWT or similar
// - Add token expiry and refresh mechanism
// - Implement rate limiting to prevent abuse
// - Add audit logging for all power commands
// - Consider mutual TLS for critical installations
// ============================================================================

pub fn auth_layer(_token: String) -> tower::layer::util::Identity {
    // SECURITY ISSUE: This returns a no-op layer that doesn't validate anything
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
        // SECURITY ISSUE: This always returns nil UUID without checking credentials
        Ok(Self(uuid::Uuid::nil()))
    }
}
