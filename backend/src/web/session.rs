use crate::db;
use crate::domain::models::UserRole;
use axum::{
    async_trait,
    extract::{FromRef, FromRequestParts},
    http::{request::Parts, HeaderMap, StatusCode},
};
use base64::{engine::general_purpose, Engine as _};
use chrono::{Duration, Utc};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone)]
pub struct SessionClaims {
    pub user_id: Uuid,
    pub role: UserRole,
    pub exp: i64,
}

#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("invalid token format")]
    Invalid,
    #[error("signature mismatch")]
    Signature,
    #[error("expired")]
    Expired,
    #[error("bad role")]
    Role,
}

pub fn sign_session(user_id: Uuid, role: &UserRole, key: &[u8]) -> Result<String, SessionError> {
    let exp = Utc::now() + Duration::hours(24);
    let payload = format!("{}|{}|{}", user_id, role_string(role), exp.timestamp());
    let mut mac = HmacSha256::new_from_slice(key).map_err(|_| SessionError::Invalid)?;
    mac.update(payload.as_bytes());
    let sig = mac.finalize().into_bytes();
    let token = format!(
        "{}.{}",
        general_purpose::STANDARD.encode(payload.as_bytes()),
        general_purpose::STANDARD.encode(sig)
    );
    Ok(token)
}

pub fn verify_session(token: &str, key: &[u8]) -> Result<SessionClaims, SessionError> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 2 {
        return Err(SessionError::Invalid);
    }
    let payload_bytes = general_purpose::STANDARD
        .decode(parts[0])
        .map_err(|_| SessionError::Invalid)?;
    let sig_bytes = general_purpose::STANDARD
        .decode(parts[1])
        .map_err(|_| SessionError::Invalid)?;

    let mut mac = HmacSha256::new_from_slice(key).map_err(|_| SessionError::Invalid)?;
    mac.update(&payload_bytes);
    mac.verify_slice(&sig_bytes)
        .map_err(|_| SessionError::Signature)?;

    let payload = String::from_utf8(payload_bytes).map_err(|_| SessionError::Invalid)?;
    let pieces: Vec<&str> = payload.split('|').collect();
    if pieces.len() != 3 {
        return Err(SessionError::Invalid);
    }
    let user_id = Uuid::parse_str(pieces[0]).map_err(|_| SessionError::Invalid)?;
    let role = parse_role(pieces[1])?;
    let exp: i64 = pieces[2].parse().map_err(|_| SessionError::Invalid)?;
    if Utc::now().timestamp() > exp {
        return Err(SessionError::Expired);
    }
    Ok(SessionClaims { user_id, role, exp })
}

pub fn extract_token(headers: &HeaderMap) -> Option<String> {
    if let Some(auth) = headers.get(axum::http::header::AUTHORIZATION) {
        if let Ok(val) = auth.to_str() {
            if let Some(bearer) = val.strip_prefix("Bearer ") {
                return Some(bearer.trim().to_string());
            }
        }
    }
    if let Some(cookie) = headers.get(axum::http::header::COOKIE) {
        if let Ok(val) = cookie.to_str() {
            for pair in val.split(';') {
                let trimmed = pair.trim();
                if let Some(rest) = trimmed.strip_prefix("session=") {
                    return Some(rest.to_string());
                }
            }
        }
    }
    None
}

fn role_string(role: &UserRole) -> &'static str {
    match role {
        UserRole::Admin => "ADMIN",
        UserRole::Founder => "FOUNDER",
        UserRole::Employee => "EMPLOYEE",
    }
}

fn parse_role(raw: &str) -> Result<UserRole, SessionError> {
    match raw {
        "ADMIN" => Ok(UserRole::Admin),
        "FOUNDER" => Ok(UserRole::Founder),
        "EMPLOYEE" => Ok(UserRole::Employee),
        _ => Err(SessionError::Role),
    }
}

// ============================================
// Axum Extractor for UserSession
// ============================================

/// Axum extractor that validates session and extracts user_id
///
/// Usage:
/// ```rust
/// async fn handler(UserSession(user_id): UserSession) -> Result<...> {
///     // user_id is authenticated Uuid
/// }
/// ```
pub struct UserSession(pub Uuid);

#[async_trait]
impl<S> FromRequestParts<S> for UserSession
where
    S: Send + Sync,
    crate::state::SharedState: FromRef<S>,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let shared_state = crate::state::SharedState::from_ref(state);

        let token = extract_token(&parts.headers).ok_or(StatusCode::UNAUTHORIZED)?;

        let claims = verify_session(&token, &shared_state.session_key).map_err(|e| {
            tracing::warn!("Session verification failed: {}", e);
            StatusCode::UNAUTHORIZED
        })?;

        let user = db::find_user_by_id(&shared_state.pool, claims.user_id)
            .await
            .map_err(|e| {
                tracing::warn!("User lookup failed for session: {}", e);
                StatusCode::UNAUTHORIZED
            })?;

        let Some(user) = user else {
            return Err(StatusCode::UNAUTHORIZED);
        };

        if !user.is_active {
            return Err(StatusCode::UNAUTHORIZED);
        }

        Ok(UserSession(claims.user_id))
    }
}
