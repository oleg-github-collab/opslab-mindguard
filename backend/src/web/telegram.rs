///! Telegram integration endpoints - PIN generation and status
use crate::db;
use crate::state::SharedState;
use crate::web::session::UserSession;
use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::Serialize;

pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/generate-pin", post(generate_pin))
        .route("/status", get(telegram_status))
        .with_state(state)
}

#[derive(Serialize)]
struct PinResponse {
    pin_code: String,
    expires_in_seconds: i32,
}

#[derive(Serialize)]
struct TelegramStatus {
    connected: bool,
    telegram_id: Option<i64>,
    active_pin: Option<String>,
}

/// Generate new PIN code for Telegram linking
#[axum::debug_handler]
async fn generate_pin(
    UserSession(user_id): UserSession,
    State(state): State<SharedState>,
) -> Result<Json<PinResponse>, StatusCode> {
    let pin_code = db::generate_telegram_pin(&state.pool, user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to generate PIN: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(PinResponse {
        pin_code,
        expires_in_seconds: 300, // 5 minutes
    }))
}

/// Get Telegram connection status
async fn telegram_status(
    UserSession(user_id): UserSession,
    State(state): State<SharedState>,
) -> Result<Json<TelegramStatus>, StatusCode> {
    let user = db::find_user_by_id(&state.pool, user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let connected = user.as_ref().and_then(|u| u.telegram_id).is_some();
    let telegram_id = user.as_ref().and_then(|u| u.telegram_id);

    let active_pin = if !connected {
        db::get_active_pin(&state.pool, user_id)
            .await
            .ok()
            .flatten()
    } else {
        None
    };

    Ok(Json(TelegramStatus {
        connected,
        telegram_id,
        active_pin,
    }))
}
