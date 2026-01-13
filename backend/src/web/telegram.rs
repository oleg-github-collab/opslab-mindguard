///! Telegram integration endpoints - status and preferences
use crate::db;
use crate::state::SharedState;
use crate::time_utils;
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
        .route("/status", get(telegram_status))
        .route("/preferences", post(update_preferences))
        .with_state(state)
}

#[derive(Serialize)]
struct TelegramStatus {
    connected: bool,
    telegram_id: Option<i64>,
    reminder_hour: i16,
    reminder_minute: i16,
    timezone: String,
    notification_enabled: bool,
    onboarding_completed: bool,
}

#[derive(serde::Deserialize)]
struct PreferencesPayload {
    reminder_time: Option<String>,
    timezone: Option<String>,
    notification_enabled: Option<bool>,
    onboarding_complete: Option<bool>,
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

    let prefs = db::get_user_preferences(&state.pool, user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(TelegramStatus {
        connected,
        telegram_id,
        reminder_hour: prefs.reminder_hour,
        reminder_minute: prefs.reminder_minute,
        timezone: prefs.timezone,
        notification_enabled: prefs.notification_enabled,
        onboarding_completed: prefs.onboarding_completed,
    }))
}

/// Update Telegram reminder preferences from web UI
async fn update_preferences(
    UserSession(user_id): UserSession,
    State(state): State<SharedState>,
    Json(payload): Json<PreferencesPayload>,
) -> Result<Json<TelegramStatus>, StatusCode> {
    if let Some(reminder_time) = payload.reminder_time {
        let parts: Vec<&str> = reminder_time.split(':').collect();
        if parts.len() != 2 {
            return Err(StatusCode::BAD_REQUEST);
        }
        let hour: i16 = parts[0].parse().map_err(|_| StatusCode::BAD_REQUEST)?;
        let minute: i16 = parts[1].parse().map_err(|_| StatusCode::BAD_REQUEST)?;
        if hour < 0 || hour > 23 || minute < 0 || minute > 59 {
            return Err(StatusCode::BAD_REQUEST);
        }
        db::set_user_reminder_time(&state.pool, user_id, hour, minute)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    if let Some(timezone) = payload.timezone {
        let normalized =
            time_utils::normalize_timezone(&timezone).ok_or(StatusCode::BAD_REQUEST)?;
        db::set_user_timezone(&state.pool, user_id, &normalized)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    if let Some(enabled) = payload.notification_enabled {
        db::set_user_notification_enabled(&state.pool, user_id, enabled)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    if let Some(complete) = payload.onboarding_complete {
        if complete {
            let user = db::find_user_by_id(&state.pool, user_id)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
                .ok_or(StatusCode::UNAUTHORIZED)?;
            if user.telegram_id.is_none() {
                return Err(StatusCode::CONFLICT);
            }
        }
        db::set_user_onboarding_complete(&state.pool, user_id, complete)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    telegram_status(UserSession(user_id), State(state)).await
}
