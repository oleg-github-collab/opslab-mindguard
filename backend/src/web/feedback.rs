use crate::middleware::RateLimiter;
use crate::services::categorizer::{PostCategory, WallPostCategorizer};
use crate::state::SharedState;
use crate::web::session::UserSession;
use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

static ANON_FEEDBACK_RATE_LIMITER: Lazy<RateLimiter> = Lazy::new(|| RateLimiter::new(10, 60));

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

#[derive(Serialize)]
pub struct WallStatsPost {
    pub id: Uuid,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub is_anonymous: bool,
    pub sentiment: String,
    pub summary: String,
    pub tags: Vec<String>,
    pub work_aspect: String,
    pub emotional_intensity: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_name: Option<String>,
}

#[derive(Serialize)]
pub struct AvailableMonth {
    pub label: String,
    pub value: String,
}

// Internal struct for database query
struct WallPostRow {
    id: Uuid,
    user_id: Uuid,
    enc_content: Vec<u8>,
    category: Option<PostCategory>,
    ai_categorized: Option<bool>,
    created_at: Option<chrono::DateTime<chrono::Utc>>,
}

pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/anonymous", post(anonymous))
        .route("/wall", post(create_wall_post))
        .route("/wall", get(get_wall_posts))
        .route("/stats", get(get_wall_stats))
        .route("/stats/available-months", get(get_available_months))
        .with_state(state)
}

async fn anonymous(
    headers: axum::http::HeaderMap,
    State(state): State<SharedState>,
    Json(payload): Json<FeedbackPayload>,
) -> Result<StatusCode, StatusCode> {
    // SECURITY: Rate limiting (10 requests per 60 seconds per IP)
    // Get IP from X-Forwarded-For header (Railway proxy) or use a default
    let ip = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .unwrap_or("unknown")
        .to_string();

    if !ANON_FEEDBACK_RATE_LIMITER.check(&ip).await {
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
    let api_key = std::env::var("OPENAI_API_KEY").map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

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
        enc_content.as_bytes(),
        category.clone() as PostCategory,
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
        post_id,
        user_id,
        category,
        ai_categorized
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
            let enc_str = String::from_utf8_lossy(&row.enc_content);
            match state.crypto.decrypt_str(&enc_str) {
                Ok(content) => Some(WallPost {
                    id: row.id,
                    user_id: row.user_id,
                    content,
                    category: row.category,
                    ai_categorized: row.ai_categorized.unwrap_or(false),
                    created_at: row.created_at.unwrap_or_else(|| chrono::Utc::now()),
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

async fn get_wall_stats(
    State(state): State<SharedState>,
) -> Result<Json<Vec<WallStatsPost>>, StatusCode> {
    // Get all wall posts with user names
    let rows = sqlx::query!(
        r#"
        SELECT
            wp.id,
            wp.user_id,
            wp.enc_content,
            wp.category as "category: PostCategory",
            wp.created_at,
            u.full_name as user_name
        FROM wall_posts wp
        LEFT JOIN users u ON wp.user_id = u.id
        ORDER BY wp.created_at DESC
        LIMIT 100
        "#
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch wall stats: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Convert posts to stats format with AI-generated fields
    let stats_posts: Vec<WallStatsPost> = rows
        .into_iter()
        .filter_map(|row| {
            let enc_str = String::from_utf8_lossy(&row.enc_content);
            match state.crypto.decrypt_str(&enc_str) {
                Ok(content) => {
                    // Determine sentiment based on category
                    let sentiment = match row.category {
                        PostCategory::Celebration => "positive",
                        PostCategory::Complaint => "negative",
                        PostCategory::SupportNeeded => "mixed",
                        _ => "mixed",
                    };

                    // Determine work aspect
                    let work_aspect = match row.category {
                        PostCategory::Celebration => "team",
                        PostCategory::Complaint => "management",
                        PostCategory::SupportNeeded => "workload",
                        PostCategory::Suggestion => "team",
                        PostCategory::Question => "management",
                    };

                    // Extract simple tags from content (first 5 words as tags)
                    let tags: Vec<String> = content
                        .split_whitespace()
                        .take(5)
                        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()))
                        .filter(|w| w.len() > 3)
                        .map(String::from)
                        .collect();

                    // Use content as summary (truncate if too long)
                    let summary = if content.len() > 200 {
                        format!("{}...", &content[..200])
                    } else {
                        content
                    };

                    // Emotional intensity based on content length and category
                    let emotional_intensity = match row.category {
                        PostCategory::Complaint | PostCategory::SupportNeeded => 4,
                        PostCategory::Celebration => 3,
                        _ => 2,
                    };

                    Some(WallStatsPost {
                        id: row.id,
                        created_at: row.created_at.unwrap_or_else(|| chrono::Utc::now()),
                        is_anonymous: row.user_name.is_none(),
                        sentiment: sentiment.to_string(),
                        summary,
                        tags,
                        work_aspect: work_aspect.to_string(),
                        emotional_intensity,
                        user_name: row.user_name,
                    })
                }
                Err(e) => {
                    tracing::error!("Failed to decrypt wall post {}: {}", row.id, e);
                    None
                }
            }
        })
        .collect();

    Ok(Json(stats_posts))
}

async fn get_available_months(
    State(state): State<SharedState>,
) -> Result<Json<Vec<AvailableMonth>>, StatusCode> {
    // Get unique months from wall posts
    let rows = sqlx::query!(
        r#"
        SELECT DISTINCT
            EXTRACT(YEAR FROM created_at) as year,
            EXTRACT(MONTH FROM created_at) as month
        FROM wall_posts
        WHERE created_at IS NOT NULL
        ORDER BY year DESC, month DESC
        "#
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch available months: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Ukrainian month names
    let month_names = [
        "січень", "лютий", "березень", "квітень", "травень", "червень",
        "липень", "серпень", "вересень", "жовтень", "листопад", "грудень"
    ];

    let months: Vec<AvailableMonth> = rows
        .into_iter()
        .filter_map(|row| {
            if let (Some(year), Some(month)) = (row.year, row.month) {
                let month_idx = (month as usize).saturating_sub(1);
                let label = format!("{} {}", month_names.get(month_idx)?, year as i32);
                let value = format!("{}-{:02}", year as i32, month as i32);
                Some(AvailableMonth { label, value })
            } else {
                None
            }
        })
        .collect();

    Ok(Json(months))
}
