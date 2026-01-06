use crate::middleware::RateLimiter;
use crate::services::categorizer::{PostCategory, WallPostCategorizer};
use crate::state::SharedState;
use crate::web::session::UserSession;
use axum::{
    extract::{ConnectInfo, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct FeedbackPayload {
    pub message: String,
}

#[derive(Deserialize)]
pub struct WallPostPayload {
    pub content: String,
    // SECURITY FIX: Remove user_id from payload - use authenticated user instead
}

#[derive(Serialize)]
pub struct WallPostResponse {
    pub id: Uuid,
    pub category: PostCategory,
}

#[derive(Serialize)]
pub struct WallPost {
    pub id: Uuid,
    pub user_id: Uuid,
    pub content: String, // SECURITY FIX: Decrypted content, not raw ciphertext
    pub category: Option<PostCategory>,
    pub ai_categorized: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

// Internal struct for database query
struct WallPostRow {
    id: Uuid,
    user_id: Uuid,
    enc_content: Vec<u8>,
    category: Option<PostCategory>,
    ai_categorized: bool,
    created_at: chrono::DateTime<chrono::Utc>,
}

pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/anonymous", post(anonymous))
        .route("/wall", post(create_wall_post))
        .route("/wall", get(get_wall_posts))
        .with_state(state)
}

async fn anonymous(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<SharedState>,
    Json(payload): Json<FeedbackPayload>,
) -> Result<StatusCode, StatusCode> {
    // SECURITY: Rate limiting (10 requests per 60 seconds per IP)
    let rate_limiter = RateLimiter::new(10, 60);
    let ip = addr.ip().to_string();

    if !rate_limiter.check(&ip).await {
        tracing::warn!("Rate limit exceeded for anonymous feedback from IP: {}", ip);
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    // SECURITY: Basic validation to prevent spam
    if payload.message.len() > 5000 {
        return Err(StatusCode::PAYLOAD_TOO_LARGE);
    }

    if payload.message.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let enc = state
        .crypto
        .encrypt_str(&payload.message)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    sqlx::query!(
        r#"
        INSERT INTO anonymous_feedback (id, enc_message)
        VALUES ($1, $2)
        "#,
        Uuid::new_v4(),
        enc
    )
    .execute(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::CREATED)
}

async fn create_wall_post(
    UserSession(user_id): UserSession,
    State(state): State<SharedState>,
    Json(payload): Json<WallPostPayload>,
) -> Result<Json<WallPostResponse>, StatusCode> {
    // SECURITY: user_id comes from authenticated session, not from request body

    // Validation
    if payload.content.len() > 5000 {
        return Err(StatusCode::PAYLOAD_TOO_LARGE);
    }

    if payload.content.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // #12 WOW Feature: Auto categorization with AI
    let api_key = std::env::var("OPENAI_API_KEY")
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let categorizer = WallPostCategorizer::new(api_key);

    let (category, ai_categorized) = match categorizer.categorize(&payload.content).await {
        Ok(cat) => (cat, true),
        Err(e) => {
            tracing::warn!("AI categorization failed, using keyword fallback: {}", e);
            // Fallback is already called inside categorize(), but if network error:
            (PostCategory::Complaint, false)
        }
    };

    let enc_content = state
        .crypto
        .encrypt_str(&payload.content)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let post_id = Uuid::new_v4();

    sqlx::query!(
        r#"
        INSERT INTO wall_posts (id, user_id, enc_content, category, ai_categorized)
        VALUES ($1, $2, $3, $4, $5)
        "#,
        post_id,
        user_id, // SECURITY FIX: Use authenticated user_id, not from payload
        enc_content,
        category as PostCategory,
        ai_categorized
    )
    .execute(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to insert wall post: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    tracing::info!(
        "Wall post created: id={}, user_id={}, category={:?}, ai_categorized={}",
        post_id, user_id, category, ai_categorized
    );

    Ok(Json(WallPostResponse {
        id: post_id,
        category,
    }))
}

async fn get_wall_posts(
    State(state): State<SharedState>,
) -> Result<Json<Vec<WallPost>>, StatusCode> {
    let rows = sqlx::query_as!(
        WallPostRow,
        r#"
        SELECT id, user_id, enc_content, category as "category: PostCategory", ai_categorized, created_at
        FROM wall_posts
        ORDER BY created_at DESC
        LIMIT 100
        "#
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch wall posts: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Decrypt content before returning
    let posts: Vec<WallPost> = rows
        .into_iter()
        .filter_map(|row| {
            match state.crypto.decrypt_str(&row.enc_content) {
                Ok(content) => Some(WallPost {
                    id: row.id,
                    user_id: row.user_id,
                    content,
                    category: row.category,
                    ai_categorized: row.ai_categorized,
                    created_at: row.created_at,
                }),
                Err(e) => {
                    tracing::error!("Failed to decrypt wall post {}: {}", row.id, e);
                    None
                }
            }
        })
        .collect();

    Ok(Json(posts))
}
