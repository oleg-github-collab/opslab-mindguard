use crate::db;
use crate::domain::models::UserRole;
use crate::state::SharedState;
use crate::web::session::UserSession;
use axum::{
    extract::State,
    http::StatusCode,
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
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
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UserStatus {
    Excellent,   // All green
    Good,        // Mostly good
    Concerning,  // Some yellow flags
    Critical,    // Red flags - needs attention
    NoData,      // No recent check-ins
}

pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/heatmap", get(get_team_heatmap))
        .with_state(state)
}

async fn get_team_heatmap(
    UserSession(user_id): UserSession,
    State(state): State<SharedState>,
) -> Result<Json<TeamHeatmapData>, StatusCode> {
    // SECURITY: Only ADMIN and FOUNDER can access team heatmap
    let requesting_user = db::find_user_by_id(&state.pool, user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if !matches!(requesting_user.role, UserRole::Admin | UserRole::Founder) {
        tracing::warn!(
            "Unauthorized heatmap access attempt by user {} with role {:?}",
            user_id,
            requesting_user.role
        );
        return Err(StatusCode::FORBIDDEN);
    }

    let users = db::get_all_users(&state.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get all users: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let mut heatmap_entries = Vec::new();

    for user in users {
        let name = state
            .crypto
            .decrypt_str(&user.enc_name)
            .unwrap_or_else(|_| "Unknown".to_string());

        let metrics = db::calculate_user_metrics(&state.pool, user.id)
            .await
            .map_err(|e| {
                tracing::error!("Failed to calculate metrics for user {}: {}", user.id, e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        let streak = db::get_user_current_streak(&state.pool, user.id)
            .await
            .unwrap_or(0);

        let last_checkin = db::get_last_checkin_date(&state.pool, user.id)
            .await
            .ok()
            .flatten();

        let (status, who5, phq9, gad7, burnout) = if let Some(m) = metrics {
            let status = calculate_user_status(&m);
            (status, m.who5_score, m.phq9_score, m.gad7_score, m.burnout_percentage())
        } else {
            (UserStatus::NoData, 0.0, 0.0, 0.0, 0.0)
        };

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
