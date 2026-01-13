use crate::analytics::early_signal;
use crate::db;
use crate::domain::models::UserRole;
use crate::services::moderation;
use crate::state::SharedState;
use crate::web::session::UserSession;
use argon2::{
    password_hash::{PasswordHasher, SaltString},
    Argon2,
};
use axum::{extract::{Path, State}, http::StatusCode, routing::{get, post}, Json, Router};
use chrono::{DateTime, Utc};
use rand_core::OsRng;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct TeamHeatmapData {
    pub users: Vec<UserHeatmapEntry>,
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserHeatmapEntry {
    pub user_id: Uuid,
    pub name: String, // Decrypted for admin view
    pub status: UserStatus,
    pub who5_score: f64,
    pub phq9_score: f64,
    pub gad7_score: f64,
    pub burnout_percentage: f64,
    pub last_checkin: Option<DateTime<Utc>>,
    pub streak: i32,
    pub early_signal: Option<early_signal::EarlySignal>,
    pub action_cards: Vec<ActionCard>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UserStatus {
    Excellent,  // All green
    Good,       // Mostly good
    Concerning, // Some yellow flags
    Critical,   // Red flags - needs attention
    NoData,     // No recent check-ins
}

#[derive(Debug, Serialize)]
pub struct AdminUser {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub role: UserRole,
    pub note: Option<String>,
    pub telegram_id: Option<i64>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub offboarded_at: Option<DateTime<Utc>>,
    pub offboarded_reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ActionCard {
    pub title: String,
    pub description: String,
    pub priority: String,
}

#[derive(Debug, Serialize)]
pub struct ToxicityThemeCount {
    pub theme: String,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct WallToxicitySummary {
    pub total_posts: i64,
    pub flagged_posts: i64,
    pub avg_severity: f64,
    pub top_themes: Vec<ToxicityThemeCount>,
    pub keywords: Vec<String>,
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateUserPayload {
    pub name: String,
    pub email: String,
    pub code: String,
    pub role: Option<UserRole>,
    pub note: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct OffboardPayload {
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ResetCodePayload {
    pub code: String,
}

pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/heatmap", get(get_team_heatmap))
        .route("/users", get(list_users))
        .route("/users", post(create_user))
        .route("/users/:id/deactivate", post(deactivate_user))
        .route("/users/:id/reactivate", post(reactivate_user))
        .route("/users/:id/reset-code", post(reset_user_code))
        .route("/wall/toxicity", get(get_wall_toxicity_summary))
        .with_state(state)
}

async fn require_admin(
    state: &SharedState,
    user_id: Uuid,
) -> Result<db::DbUser, StatusCode> {
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

fn normalize_email(email: &str) -> String {
    email.trim().to_lowercase()
}

fn is_valid_code(code: &str) -> bool {
    code.len() == 4 && code.chars().all(|c| c.is_ascii_digit())
}

fn can_manage_role(requester: &db::DbUser, target_role: &UserRole) -> bool {
    match target_role {
        UserRole::Admin | UserRole::Founder => matches!(requester.role, UserRole::Founder),
        UserRole::Employee => matches!(requester.role, UserRole::Admin | UserRole::Founder),
    }
}

fn map_admin_user(state: &SharedState, user: db::DbUser) -> AdminUser {
    let name = state
        .crypto
        .decrypt_str(&user.enc_name)
        .unwrap_or_else(|_| "Unknown".to_string());

    AdminUser {
        id: user.id,
        name,
        email: user.email,
        role: user.role,
        note: user.note,
        telegram_id: user.telegram_id,
        is_active: user.is_active,
        created_at: user.created_at,
        offboarded_at: user.offboarded_at,
        offboarded_reason: user.offboarded_reason,
    }
}

async fn list_users(
    UserSession(user_id): UserSession,
    State(state): State<SharedState>,
) -> Result<Json<Vec<AdminUser>>, StatusCode> {
    require_admin(&state, user_id).await?;

    let users = db::get_all_users(&state.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to load users: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let mut out: Vec<AdminUser> = users.into_iter().map(|u| map_admin_user(&state, u)).collect();
    out.sort_by_key(|u| (!u.is_active, u.created_at));

    Ok(Json(out))
}

async fn create_user(
    UserSession(user_id): UserSession,
    State(state): State<SharedState>,
    Json(payload): Json<CreateUserPayload>,
) -> Result<Json<AdminUser>, StatusCode> {
    let requester = require_admin(&state, user_id).await?;

    let name = payload.name.trim();
    if name.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let email = normalize_email(&payload.email);
    if email.is_empty() || !email.contains('@') {
        return Err(StatusCode::BAD_REQUEST);
    }

    if !is_valid_code(&payload.code) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let role = payload.role.unwrap_or(UserRole::Employee);
    if !can_manage_role(&requester, &role) {
        return Err(StatusCode::FORBIDDEN);
    }

    let salt = SaltString::generate(OsRng);
    let hash = Argon2::default()
        .hash_password(payload.code.as_bytes(), &salt)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .to_string();

    let enc_name = state
        .crypto
        .encrypt_str(name)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if let Some(existing) = db::find_user_by_email_any(&state.pool, &email)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        if existing.is_active {
            return Err(StatusCode::CONFLICT);
        }
        if !can_manage_role(&requester, &existing.role) {
            return Err(StatusCode::FORBIDDEN);
        }

        sqlx::query(
            r#"
            UPDATE users
            SET email = $1,
                hash = $2,
                enc_name = $3,
                role = $4,
                note = $5,
                is_active = true,
                telegram_id = NULL,
                offboarded_at = NULL,
                offboarded_by = NULL,
                offboarded_reason = NULL,
                updated_at = NOW()
            WHERE id = $6
            "#,
        )
        .bind(&email)
        .bind(&hash)
        .bind(&enc_name)
        .bind(role.clone())
        .bind(payload.note.clone())
        .bind(existing.id)
        .execute(&state.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to reactivate user {}: {}", existing.id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        let user = db::find_user_by_id(&state.pool, existing.id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .ok_or(StatusCode::NOT_FOUND)?;

        return Ok(Json(map_admin_user(&state, user)));
    }

    let user_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO users (id, email, hash, enc_name, role, note)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(user_id)
    .bind(&email)
    .bind(&hash)
    .bind(&enc_name)
    .bind(role)
    .bind(payload.note.clone())
    .execute(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create user {}: {}", email, e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let user = db::find_user_by_id(&state.pool, user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(map_admin_user(&state, user)))
}

async fn deactivate_user(
    UserSession(user_id): UserSession,
    State(state): State<SharedState>,
    Path(target_id): Path<Uuid>,
    Json(payload): Json<OffboardPayload>,
) -> Result<Json<AdminUser>, StatusCode> {
    let requester = require_admin(&state, user_id).await?;

    if requester.id == target_id {
        return Err(StatusCode::FORBIDDEN);
    }

    let target = db::find_user_by_id(&state.pool, target_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    if !can_manage_role(&requester, &target.role) {
        return Err(StatusCode::FORBIDDEN);
    }

    sqlx::query(
        r#"
        UPDATE users
        SET is_active = false,
            telegram_id = NULL,
            offboarded_at = NOW(),
            offboarded_by = $1,
            offboarded_reason = $2,
            updated_at = NOW()
        WHERE id = $3
        "#,
    )
    .bind(requester.id)
    .bind(payload.reason.clone())
    .bind(target_id)
    .execute(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to deactivate user {}: {}", target_id, e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let user = db::find_user_by_id(&state.pool, target_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(map_admin_user(&state, user)))
}

async fn reactivate_user(
    UserSession(user_id): UserSession,
    State(state): State<SharedState>,
    Path(target_id): Path<Uuid>,
) -> Result<Json<AdminUser>, StatusCode> {
    let requester = require_admin(&state, user_id).await?;

    let target = db::find_user_by_id(&state.pool, target_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    if !can_manage_role(&requester, &target.role) {
        return Err(StatusCode::FORBIDDEN);
    }

    sqlx::query(
        r#"
        UPDATE users
        SET is_active = true,
            offboarded_at = NULL,
            offboarded_by = NULL,
            offboarded_reason = NULL,
            updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(target_id)
    .execute(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to reactivate user {}: {}", target_id, e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let user = db::find_user_by_id(&state.pool, target_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(map_admin_user(&state, user)))
}

async fn reset_user_code(
    UserSession(user_id): UserSession,
    State(state): State<SharedState>,
    Path(target_id): Path<Uuid>,
    Json(payload): Json<ResetCodePayload>,
) -> Result<StatusCode, StatusCode> {
    let requester = require_admin(&state, user_id).await?;

    if !is_valid_code(&payload.code) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let target = db::find_user_by_id(&state.pool, target_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    if !can_manage_role(&requester, &target.role) {
        return Err(StatusCode::FORBIDDEN);
    }

    let salt = SaltString::generate(OsRng);
    let hash = Argon2::default()
        .hash_password(payload.code.as_bytes(), &salt)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .to_string();

    sqlx::query(
        r#"
        UPDATE users
        SET hash = $1,
            updated_at = NOW()
        WHERE id = $2
        "#,
    )
    .bind(hash)
    .bind(target_id)
    .execute(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to reset code for {}: {}", target_id, e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(StatusCode::NO_CONTENT)
}

async fn get_team_heatmap(
    UserSession(user_id): UserSession,
    State(state): State<SharedState>,
) -> Result<Json<TeamHeatmapData>, StatusCode> {
    tracing::info!("get_team_heatmap called by user_id: {}", user_id);
    let _requesting_user = require_admin(&state, user_id).await?;

    let users = db::get_active_users(&state.pool).await.map_err(|e| {
        tracing::error!("Failed to get all users: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    tracing::info!("Processing heatmap for {} users", users.len());

    let mut heatmap_entries = Vec::new();

    for user in users {
        let name = state
            .crypto
            .decrypt_str(&user.enc_name)
            .unwrap_or_else(|_| "Unknown".to_string());

        // Gracefully handle metrics calculation failures - just mark as NoData
        let metrics = match db::calculate_user_metrics(&state.pool, user.id).await {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!("Failed to calculate metrics for user {} ({}): {}", user.id, name, e);
                None
            }
        };

        let streak = db::get_user_current_streak(&state.pool, user.id)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!("Failed to get streak for user {} ({}): {}", user.id, name, e);
                0
            });

        let last_checkin = db::get_last_checkin_date(&state.pool, user.id)
            .await
            .ok()
            .flatten();

        let early_signal = if metrics.is_some() {
            early_signal::detect_early_signal(&state.pool, user.id)
                .await
                .ok()
                .flatten()
        } else {
            None
        };

        let (status, who5, phq9, gad7, burnout) = if let Some(m) = metrics.as_ref() {
            let status = calculate_user_status(m);
            (
                status,
                m.who5_score,
                m.phq9_score,
                m.gad7_score,
                m.burnout_percentage(),
            )
        } else {
            (UserStatus::NoData, 0.0, 0.0, 0.0, 0.0)
        };

        let action_cards = build_action_cards(metrics.as_ref(), early_signal.as_ref(), last_checkin);

        heatmap_entries.push(UserHeatmapEntry {
            user_id: user.id,
            name,
            status,
            who5_score: who5,
            phq9_score: phq9,
            gad7_score: gad7,
            burnout_percentage: burnout,
            last_checkin,
            streak,
            early_signal,
            action_cards,
        });
    }

    // Sort by status (Critical first, then Concerning, etc.)
    heatmap_entries.sort_by(|a, b| {
        let a_priority = status_priority(a.status);
        let b_priority = status_priority(b.status);
        a_priority.cmp(&b_priority)
    });

    Ok(Json(TeamHeatmapData {
        users: heatmap_entries,
        generated_at: Utc::now(),
    }))
}

fn calculate_user_status(metrics: &crate::bot::daily_checkin::Metrics) -> UserStatus {
    let mut red_flags = 0;
    let mut yellow_flags = 0;

    // Critical indicators (Red flags)
    if metrics.who5_score < 35.0 {
        red_flags += 1;
    }
    if metrics.phq9_score >= 15.0 {
        red_flags += 1; // Moderately severe depression
    }
    if metrics.gad7_score >= 15.0 {
        red_flags += 1; // Severe anxiety
    }
    if metrics.burnout_percentage() > 75.0 {
        red_flags += 1;
    }

    // Warning indicators (Yellow flags)
    if metrics.who5_score >= 35.0 && metrics.who5_score < 50.0 {
        yellow_flags += 1;
    }
    if metrics.phq9_score >= 10.0 && metrics.phq9_score < 15.0 {
        yellow_flags += 1; // Moderate depression
    }
    if metrics.gad7_score >= 10.0 && metrics.gad7_score < 15.0 {
        yellow_flags += 1; // Moderate anxiety
    }
    if metrics.burnout_percentage() > 60.0 && metrics.burnout_percentage() <= 75.0 {
        yellow_flags += 1;
    }

    if red_flags >= 2 {
        UserStatus::Critical
    } else if red_flags == 1 || yellow_flags >= 2 {
        UserStatus::Concerning
    } else if yellow_flags == 1 {
        UserStatus::Good
    } else {
        UserStatus::Excellent
    }
}

fn status_priority(status: UserStatus) -> u8 {
    match status {
        UserStatus::Critical => 0,
        UserStatus::Concerning => 1,
        UserStatus::NoData => 2,
        UserStatus::Good => 3,
        UserStatus::Excellent => 4,
    }
}

fn build_action_cards(
    metrics: Option<&crate::bot::daily_checkin::Metrics>,
    early_signal: Option<&early_signal::EarlySignal>,
    last_checkin: Option<DateTime<Utc>>,
) -> Vec<ActionCard> {
    let mut cards: Vec<(i32, ActionCard)> = Vec::new();

    if let Some(last) = last_checkin {
        let days = (Utc::now() - last).num_days();
        if days >= 10 {
            cards.push((
                85,
                ActionCard {
                    title: "Re-engage check-in".to_string(),
                    description: format!(
                        "No recent check-ins for {} days. Send a gentle ping or 1:1 invite.",
                        days
                    ),
                    priority: "medium".to_string(),
                },
            ));
        }
    }

    if let Some(m) = metrics {
        if m.burnout_percentage() > 75.0 {
            cards.push((
                100,
                ActionCard {
                    title: "Immediate recovery plan".to_string(),
                    description: "Reduce workload, schedule time-off, and organize a 1:1 within 24h."
                        .to_string(),
                    priority: "high".to_string(),
                },
            ));
        } else if m.burnout_percentage() > 60.0 {
            cards.push((
                80,
                ActionCard {
                    title: "Workload recalibration".to_string(),
                    description: "Review sprint scope, move 1-2 tasks, and add recovery blocks."
                        .to_string(),
                    priority: "medium".to_string(),
                },
            ));
        }

        if m.phq9_score >= 15.0 || m.gad7_score >= 15.0 {
            cards.push((
                95,
                ActionCard {
                    title: "Support escalation".to_string(),
                    description: "Offer confidential support resources and a private conversation."
                        .to_string(),
                    priority: "high".to_string(),
                },
            ));
        }

        if m.who5_score < 50.0 {
            cards.push((
                70,
                ActionCard {
                    title: "Wellbeing boost".to_string(),
                    description: "Check workload clarity and suggest a recovery-focused day plan."
                        .to_string(),
                    priority: "medium".to_string(),
                },
            ));
        }

        if m.stress_level >= 24.0 {
            cards.push((
                75,
                ActionCard {
                    title: "Stress relief".to_string(),
                    description: "Encourage breaks and reduce meeting load for 2-3 days.".to_string(),
                    priority: "medium".to_string(),
                },
            ));
        }
    }

    if let Some(signal) = early_signal {
        if signal.level == "critical" || signal.level == "alert" {
            cards.push((
                90,
                ActionCard {
                    title: "Early signal detected".to_string(),
                    description: "Proactive check-in recommended. Aim for a 15 min pulse talk."
                        .to_string(),
                    priority: "high".to_string(),
                },
            ));
        } else {
            cards.push((
                60,
                ActionCard {
                    title: "Monitor drift".to_string(),
                    description: "Keep an eye on trends and offer a lightweight support option."
                        .to_string(),
                    priority: "low".to_string(),
                },
            ));
        }
    }

    cards.sort_by(|a, b| b.0.cmp(&a.0));
    cards.into_iter().take(3).map(|c| c.1).collect()
}

async fn get_wall_toxicity_summary(
    UserSession(user_id): UserSession,
    State(state): State<SharedState>,
) -> Result<Json<WallToxicitySummary>, StatusCode> {
    require_admin(&state, user_id).await?;

    let rows = sqlx::query(
        r#"
        SELECT wp.id, wp.enc_content, wp.created_at,
               wts.severity, wts.flagged, wts.themes
        FROM wall_posts wp
        LEFT JOIN wall_toxic_signals wts ON wp.id = wts.post_id
        WHERE wp.created_at >= NOW() - INTERVAL '90 days'
        ORDER BY wp.created_at DESC
        LIMIT 400
        "#,
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut total_posts = 0i64;
    let mut flagged_posts = 0i64;
    let mut severity_sum = 0i64;
    let mut theme_counts: std::collections::HashMap<String, i64> =
        std::collections::HashMap::new();
    let mut flagged_messages = Vec::new();

    for row in rows {
        total_posts += 1;
        let enc_content: Vec<u8> = row.try_get("enc_content").unwrap_or_default();
        let content = state
            .crypto
            .decrypt_str(&String::from_utf8_lossy(&enc_content))
            .unwrap_or_default();

        let stored_themes: Option<serde_json::Value> = row.try_get("themes").ok();
        let stored_flagged: Option<bool> = row.try_get("flagged").ok();
        let stored_severity: Option<i16> = row.try_get("severity").ok();

        let (flagged, severity, themes) = if let Some(themes_val) = stored_themes {
            let themes: Vec<String> = serde_json::from_value(themes_val).unwrap_or_default();
            (
                stored_flagged.unwrap_or(false),
                stored_severity.unwrap_or(0),
                themes,
            )
        } else {
            let signal = moderation::analyze_toxicity(&content);
            (signal.flagged, signal.severity, signal.themes)
        };

        if flagged {
            flagged_posts += 1;
            severity_sum += severity as i64;
            flagged_messages.push(content);
        }

        for theme in themes {
            *theme_counts.entry(theme).or_insert(0) += 1;
        }
    }

    let mut themes: Vec<ToxicityThemeCount> = theme_counts
        .into_iter()
        .map(|(theme, count)| ToxicityThemeCount { theme, count })
        .collect();
    themes.sort_by(|a, b| b.count.cmp(&a.count));
    let top_themes = themes.into_iter().take(6).collect::<Vec<_>>();

    let keywords = moderation::extract_keywords(&flagged_messages, 8);
    let avg_severity = if flagged_posts > 0 {
        severity_sum as f64 / flagged_posts as f64
    } else {
        0.0
    };

    Ok(Json(WallToxicitySummary {
        total_posts,
        flagged_posts,
        avg_severity,
        top_themes,
        keywords,
        generated_at: Utc::now(),
    }))
}
