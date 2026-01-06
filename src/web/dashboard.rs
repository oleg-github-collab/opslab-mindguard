use crate::db::{self, DbUser};
use crate::domain::models::{DashboardPayload, UserRole};
use crate::web::session;
use crate::state::SharedState;
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::get,
    Json, Router,
};
use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize)]
pub struct TeamDashboard {
    pub members: Vec<DashboardPayload>,
}

pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/user/:id", get(user_view))
        .route("/team", get(team_view))
        .with_state(state)
}

async fn user_view(headers: HeaderMap, State(state): State<SharedState>, Path(id): Path<Uuid>) -> Result<Json<DashboardPayload>, StatusCode> {
    let token = session::extract_token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let claims = session::verify_session(&token, &state.session_key).map_err(|_| StatusCode::UNAUTHORIZED)?;
    if claims.user_id != id && !matches!(claims.role, UserRole::Admin | UserRole::Founder) {
        return Err(StatusCode::FORBIDDEN);
    }

    let user = db::find_user_by_id(&state.pool, id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let outstanding = state
        .poll_engine
        .next_questions(&state.pool, id, 3)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let rolling = state
        .poll_engine
        .calculate_rolling_score(&state.pool, id, 14)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let last_voice = sqlx::query_scalar!(
        r#"SELECT risk_score FROM voice_logs WHERE user_id = $1 ORDER BY created_at DESC LIMIT 1"#,
        id
    )
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let payload = DashboardPayload {
        user_id: user.id,
        role: user.role,
        rolling_score: rolling,
        last_voice_risk: last_voice.map(|v| v as i16),
        outstanding_questions: outstanding,
    };
    Ok(Json(payload))
}

async fn team_view(headers: HeaderMap, State(state): State<SharedState>) -> Result<Json<TeamDashboard>, StatusCode> {
    let token = session::extract_token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let claims = session::verify_session(&token, &state.session_key).map_err(|_| StatusCode::UNAUTHORIZED)?;
    if !matches!(claims.role, UserRole::Admin | UserRole::Founder) {
        return Err(StatusCode::FORBIDDEN);
    }

    let users = sqlx::query_as!(
        DbUser,
        r#"
        SELECT id, email, hash, telegram_id, role as "role: UserRole", enc_name, note, created_at
        FROM users
        ORDER BY created_at ASC
        "#
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut members = Vec::new();
    for user in users {
        let rolling = state
            .poll_engine
            .calculate_rolling_score(&state.pool, user.id, 14)
            .await
            .unwrap_or_else(|_| crate::domain::models::RollingScore {
                window_days: 14,
                total: 0.0,
            });
        let outstanding = state
            .poll_engine
            .next_questions(&state.pool, user.id, 3)
            .await
            .unwrap_or_default();
        let last_voice = sqlx::query_scalar!(
            r#"SELECT risk_score FROM voice_logs WHERE user_id = $1 ORDER BY created_at DESC LIMIT 1"#,
            user.id
        )
        .fetch_optional(&state.pool)
        .await
        .ok()
        .flatten();

        members.push(DashboardPayload {
            user_id: user.id,
            role: user.role,
            rolling_score: rolling,
            last_voice_risk: last_voice.map(|v| v as i16),
            outstanding_questions,
        });
    }

    Ok(Json(TeamDashboard { members }))
}
