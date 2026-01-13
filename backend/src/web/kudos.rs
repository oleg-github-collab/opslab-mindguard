use crate::db;
use crate::domain::models::UserRole;
use crate::services::moderation;
use crate::state::SharedState;
use crate::web::session::UserSession;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
struct KudosPayload {
    recipient_email: String,
    message: String,
}

#[derive(Debug, Deserialize)]
struct KudosQuery {
    user_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
struct KudosEntry {
    id: Uuid,
    from_name: String,
    to_name: String,
    message: String,
    created_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Serialize)]
struct KudosInsights {
    received_count_30d: usize,
    sent_count_30d: usize,
    top_keywords: Vec<String>,
    top_senders: Vec<String>,
}

#[derive(Debug, Serialize)]
struct KudosResponse {
    received: Vec<KudosEntry>,
    sent: Vec<KudosEntry>,
    insights: KudosInsights,
}

pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/", get(get_kudos))
        .route("/", post(send_kudos))
        .with_state(state)
}

async fn resolve_user_id(
    state: &SharedState,
    requester_id: Uuid,
    query: &KudosQuery,
) -> Result<Uuid, StatusCode> {
    if let Some(target_id) = query.user_id {
        let role = db::get_user_role(&state.pool, requester_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        if !matches!(role, UserRole::Admin | UserRole::Founder) {
            return Err(StatusCode::FORBIDDEN);
        }
        Ok(target_id)
    } else {
        Ok(requester_id)
    }
}

async fn get_kudos(
    UserSession(user_id): UserSession,
    State(state): State<SharedState>,
    Query(query): Query<KudosQuery>,
) -> Result<Json<KudosResponse>, StatusCode> {
    let target_id = resolve_user_id(&state, user_id, &query).await?;

    let received = db::get_recent_kudos_received(&state.pool, target_id, 30, 20)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let sent = db::get_recent_kudos_sent(&state.pool, target_id, 30, 20)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut received_entries = Vec::new();
    let mut received_messages = Vec::new();
    let mut sender_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();

    for record in received {
        let from_name = state
            .crypto
            .decrypt_str(&String::from_utf8_lossy(&record.from_user_enc_name))
            .unwrap_or_else(|_| "Unknown".to_string());
        let to_name = db::find_user_by_id(&state.pool, record.to_user_id)
            .await
            .ok()
            .flatten()
            .and_then(|u| state.crypto.decrypt_str(&u.enc_name).ok())
            .unwrap_or_else(|| "You".to_string());
        received_messages.push(record.message.clone());
        *sender_counts.entry(from_name.clone()).or_insert(0) += 1;
        received_entries.push(KudosEntry {
            id: record.id,
            from_name,
            to_name,
            message: record.message,
            created_at: record.created_at,
        });
    }

    let mut sent_entries = Vec::new();
    for record in sent {
        let to_name = state
            .crypto
            .decrypt_str(&String::from_utf8_lossy(&record.to_user_enc_name))
            .unwrap_or_else(|_| "Unknown".to_string());
        let from_name = db::find_user_by_id(&state.pool, record.from_user_id)
            .await
            .ok()
            .flatten()
            .and_then(|u| state.crypto.decrypt_str(&u.enc_name).ok())
            .unwrap_or_else(|| "You".to_string());
        sent_entries.push(KudosEntry {
            id: record.id,
            from_name,
            to_name,
            message: record.message,
            created_at: record.created_at,
        });
    }

    let sent_count_30d = sent_entries.len();

    let mut top_senders: Vec<(String, usize)> = sender_counts.into_iter().collect();
    top_senders.sort_by(|a, b| b.1.cmp(&a.1));
    let top_senders = top_senders
        .into_iter()
        .take(3)
        .map(|(name, _)| name)
        .collect();

    let top_keywords = moderation::extract_keywords(&received_messages, 6);

    Ok(Json(KudosResponse {
        received: received_entries,
        sent: sent_entries,
        insights: KudosInsights {
            received_count_30d: received_messages.len(),
            sent_count_30d: sent_count_30d,
            top_keywords,
            top_senders,
        },
    }))
}

async fn send_kudos(
    UserSession(user_id): UserSession,
    State(state): State<SharedState>,
    Json(payload): Json<KudosPayload>,
) -> Result<StatusCode, StatusCode> {
    let email = payload.recipient_email.trim().to_lowercase();
    if email.is_empty() || !email.contains('@') {
        return Err(StatusCode::BAD_REQUEST);
    }
    let message = payload.message.trim();
    if message.is_empty() || message.len() > 500 {
        return Err(StatusCode::BAD_REQUEST);
    }

    let recipient = db::get_user_by_email(&state.pool, &email)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    if recipient.id == user_id {
        return Err(StatusCode::BAD_REQUEST);
    }

    db::insert_kudos(&state.pool, user_id, recipient.id, message)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::CREATED)
}
