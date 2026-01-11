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

#[derive(Deserialize)]
pub struct TokenLoginRequest {
    pub token: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub user_id: Uuid,
    pub role: UserRole,
    pub name: String,
}

pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/login", post(login))
        .route("/token-login", post(token_login))
        .with_state(state)
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
        role: user.role.clone(),
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

async fn token_login(
    State(state): State<SharedState>,
    Json(payload): Json<TokenLoginRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    // Verify token and get user
    #[derive(sqlx::FromRow)]
    struct TokenRecord {
        user_id: Uuid,
        used: bool,
        expires_at: chrono::DateTime<chrono::Utc>,
    }

    let token_record: TokenRecord = sqlx::query_as(
        "SELECT user_id, used, expires_at FROM telegram_login_tokens WHERE token = $1"
    )
    .bind(&payload.token)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Token lookup failed: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?
    .ok_or(StatusCode::UNAUTHORIZED)?;

    // Check if token is valid
    if token_record.used {
        tracing::warn!("Attempt to reuse token: {}", payload.token);
        return Err(StatusCode::UNAUTHORIZED);
    }

    if token_record.expires_at < chrono::Utc::now() {
        tracing::warn!("Expired token used: {}", payload.token);
        return Err(StatusCode::UNAUTHORIZED);
    }

    // Mark token as used
    sqlx::query("UPDATE telegram_login_tokens SET used = TRUE WHERE token = $1")
        .bind(&payload.token)
        .execute(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Get user details
    let user = db::find_user_by_id(&state.pool, token_record.user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let name = state
        .crypto
        .decrypt_str(&user.enc_name)
        .unwrap_or_else(|_| "User".to_string());

    let resp = LoginResponse {
        user_id: user.id,
        role: user.role.clone(),
        name,
    };

    let session_token = session::sign_session(user.id, &user.role, &state.session_key)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // SECURITY: Use Secure flag in production (HTTPS only)
    let is_production = std::env::var("RAILWAY_ENVIRONMENT").is_ok()
        || std::env::var("RENDER").is_ok()
        || std::env::var("FLY_APP_NAME").is_ok()
        || std::env::var("PRODUCTION").is_ok();

    let secure_flag = if is_production { "; Secure" } else { "" };

    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::SET_COOKIE,
        format!("session={}; HttpOnly; SameSite=Lax; Path={}{}", session_token, "/", secure_flag)
            .parse()
            .unwrap(),
    );

    tracing::info!("User {} logged in via Telegram token", user.id);

    Ok((headers, Json(resp)))
}
