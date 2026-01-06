use crate::db;
use crate::domain::models::UserRole;
use crate::middleware::RateLimiter;
use crate::state::SharedState;
use crate::web::session;
use argon2::{password_hash::PasswordHash, Argon2, PasswordVerifier};
use axum::{
    extract::{ConnectInfo, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub code: String,
    pub telegram_id: Option<i64>,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub user_id: Uuid,
    pub role: UserRole,
    pub name: String,
}

pub fn router(state: SharedState) -> Router {
    Router::new().route("/login", post(login)).with_state(state)
}

async fn login(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<SharedState>,
    Json(payload): Json<LoginRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    // SECURITY: Rate limiting to prevent brute force attacks (5 attempts per 60 seconds per IP)
    let rate_limiter = RateLimiter::new(5, 60);
    let ip = addr.ip().to_string();

    if !rate_limiter.check(&ip).await {
        tracing::warn!("Login rate limit exceeded for IP: {}", ip);
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    let user = db::find_user_by_email(&state.pool, &payload.email)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let parsed_hash = PasswordHash::new(&user.hash).map_err(|_| StatusCode::UNAUTHORIZED)?;
    Argon2::default()
        .verify_password(payload.code.as_bytes(), &parsed_hash)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    if let Some(tg) = payload.telegram_id {
        if user.telegram_id.is_none() {
            db::attach_telegram(&state.pool, user.id, tg)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        } else if user.telegram_id != Some(tg) {
            return Err(StatusCode::CONFLICT);
        }
    }

    let name = state
        .crypto
        .decrypt_str(&user.enc_name)
        .unwrap_or_else(|_| "User".to_string());

    let resp = LoginResponse {
        user_id: user.id,
        role: user.role,
        name,
    };

    let token = session::sign_session(user.id, &user.role, &state.session_key).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // SECURITY: Use Secure flag in production (HTTPS only)
    let is_production = std::env::var("RAILWAY_ENVIRONMENT").is_ok()
        || std::env::var("RENDER").is_ok()
        || std::env::var("FLY_APP_NAME").is_ok()
        || std::env::var("PRODUCTION").is_ok();

    let secure_flag = if is_production { "; Secure" } else { "" };

    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::SET_COOKIE,
        format!("session={token}; HttpOnly; SameSite=Lax; Path={}{}", "/", secure_flag).parse().unwrap(),
    );
    Ok((headers, Json(resp)))
}
