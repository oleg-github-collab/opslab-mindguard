use crate::db;
use crate::domain::models::UserRole;
use crate::services::moderation;
use crate::state::SharedState;
use crate::web::session::UserSession;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
struct PulseMessagePayload {
    content: String,
    is_anonymous: Option<bool>,
}

#[derive(Debug, Serialize)]
struct PulseMessageResponse {
    id: Uuid,
    room_id: Uuid,
    content: String,
    is_anonymous: bool,
    status: String,
    created_at: chrono::DateTime<chrono::Utc>,
    author_name: Option<String>,
    moderation_reason: Option<String>,
}

#[derive(Debug, Serialize)]
struct PulsePostResponse {
    id: Uuid,
    status: String,
}

#[derive(Debug, Serialize)]
struct PulseModerationItem {
    id: Uuid,
    room_id: Uuid,
    room_slug: String,
    room_title: String,
    content: String,
    is_anonymous: bool,
    status: String,
    created_at: chrono::DateTime<chrono::Utc>,
    author_name: Option<String>,
    moderation_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ModerationDecisionPayload {
    reason: Option<String>,
}

pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/rooms", get(list_rooms))
        .route("/rooms/:slug/messages", get(list_messages))
        .route("/rooms/:slug/messages", post(create_message))
        .route("/moderation", get(list_pending_messages))
        .route("/messages/:id/approve", post(approve_message))
        .route("/messages/:id/reject", post(reject_message))
        .with_state(state)
}

async fn require_admin(state: &SharedState, user_id: Uuid) -> Result<db::DbUser, StatusCode> {
    let requesting_user = db::find_user_by_id(&state.pool, user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to find requesting user {}: {}", user_id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if !matches!(requesting_user.role, UserRole::Admin | UserRole::Founder) {
        return Err(StatusCode::FORBIDDEN);
    }

    Ok(requesting_user)
}

async fn is_admin(state: &SharedState, user_id: Uuid) -> Result<bool, StatusCode> {
    let user = db::find_user_by_id(&state.pool, user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;
    Ok(matches!(user.role, UserRole::Admin | UserRole::Founder))
}

async fn list_rooms(
    UserSession(_user_id): UserSession,
    State(state): State<SharedState>,
) -> Result<Json<Vec<db::PulseRoom>>, StatusCode> {
    let rooms = db::get_pulse_rooms(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(rooms))
}

async fn list_messages(
    UserSession(user_id): UserSession,
    State(state): State<SharedState>,
    Path(slug): Path<String>,
) -> Result<Json<Vec<PulseMessageResponse>>, StatusCode> {
    let room = db::get_pulse_room_by_slug(&state.pool, &slug)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let allow_pending = is_admin(&state, user_id).await.unwrap_or(false);
    let rows = db::get_pulse_messages(&state.pool, room.id, allow_pending, 100)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut out = Vec::new();
    for row in rows {
        let enc_str = String::from_utf8_lossy(&row.enc_content);
        let content = state.crypto.decrypt_str(&enc_str).unwrap_or_default();
        let author_name = if allow_pending && !row.is_anonymous {
            if let Some(uid) = row.user_id {
                db::find_user_by_id(&state.pool, uid)
                    .await
                    .ok()
                    .flatten()
                    .and_then(|user| state.crypto.decrypt_str(&user.enc_name).ok())
            } else {
                None
            }
        } else {
            None
        };

        out.push(PulseMessageResponse {
            id: row.id,
            room_id: row.room_id,
            content,
            is_anonymous: row.is_anonymous,
            status: row.status,
            created_at: row.created_at,
            author_name,
            moderation_reason: row.moderation_reason,
        });
    }

    Ok(Json(out))
}

async fn create_message(
    UserSession(user_id): UserSession,
    State(state): State<SharedState>,
    Path(slug): Path<String>,
    Json(payload): Json<PulseMessagePayload>,
) -> Result<Json<PulsePostResponse>, StatusCode> {
    let content = payload.content.trim();
    if content.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    if content.len() > 3000 {
        return Err(StatusCode::PAYLOAD_TOO_LARGE);
    }

    let room = db::get_pulse_room_by_slug(&state.pool, &slug)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let signal = moderation::analyze_toxicity(content);
    let needs_moderation = room.require_moderation || signal.flagged;
    let status = if needs_moderation { "PENDING" } else { "APPROVED" };
    let moderation_reason = if signal.flagged {
        Some(format!("themes: {}; severity: {}", signal.themes.join(","), signal.severity))
    } else if room.require_moderation {
        Some("room requires moderation".to_string())
    } else {
        None
    };

    let enc_content = state
        .crypto
        .encrypt_str(content)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let id = db::insert_pulse_message(
        &state.pool,
        room.id,
        user_id,
        enc_content.as_bytes(),
        payload.is_anonymous.unwrap_or(true),
        status,
        moderation_reason.as_deref(),
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(PulsePostResponse {
        id,
        status: status.to_string(),
    }))
}

async fn list_pending_messages(
    UserSession(user_id): UserSession,
    State(state): State<SharedState>,
) -> Result<Json<Vec<PulseModerationItem>>, StatusCode> {
    require_admin(&state, user_id).await?;

    let rows = sqlx::query(
        r#"
        SELECT pm.id, pm.room_id, pm.user_id, pm.enc_content, pm.is_anonymous, pm.status,
               pm.moderation_reason, pm.created_at, pr.slug, pr.title
        FROM pulse_messages pm
        JOIN pulse_rooms pr ON pm.room_id = pr.id
        WHERE pm.status = 'PENDING'
        ORDER BY pm.created_at DESC
        LIMIT 50
        "#,
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut out = Vec::new();
    for row in rows {
        let enc_content: Vec<u8> = row.try_get("enc_content").unwrap_or_default();
        let content = state
            .crypto
            .decrypt_str(&String::from_utf8_lossy(&enc_content))
            .unwrap_or_default();
        let author_name = if let Some(uid) = row.try_get::<Option<Uuid>, _>("user_id").ok().flatten()
        {
            db::find_user_by_id(&state.pool, uid)
                .await
                .ok()
                .flatten()
                .and_then(|user| state.crypto.decrypt_str(&user.enc_name).ok())
        } else {
            None
        };

        let id: Uuid = row
            .try_get("id")
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let room_id: Uuid = row
            .try_get("room_id")
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let room_slug: String = row
            .try_get("slug")
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let room_title: String = row
            .try_get("title")
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let is_anonymous: bool = row
            .try_get("is_anonymous")
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let status: String = row
            .try_get("status")
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let created_at: chrono::DateTime<chrono::Utc> = row
            .try_get("created_at")
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let moderation_reason: Option<String> = row
            .try_get("moderation_reason")
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        out.push(PulseModerationItem {
            id,
            room_id,
            room_slug,
            room_title,
            content,
            is_anonymous,
            status,
            created_at,
            author_name,
            moderation_reason,
        });
    }

    Ok(Json(out))
}

async fn approve_message(
    UserSession(user_id): UserSession,
    State(state): State<SharedState>,
    Path(message_id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    require_admin(&state, user_id).await?;
    db::update_pulse_message_status(&state.pool, message_id, "APPROVED", user_id, None)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn reject_message(
    UserSession(user_id): UserSession,
    State(state): State<SharedState>,
    Path(message_id): Path<Uuid>,
    Json(payload): Json<ModerationDecisionPayload>,
) -> Result<StatusCode, StatusCode> {
    require_admin(&state, user_id).await?;
    db::update_pulse_message_status(
        &state.pool,
        message_id,
        "REJECTED",
        user_id,
        payload.reason.as_deref(),
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}
