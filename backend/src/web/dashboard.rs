use crate::db::{self, DbUser};
use crate::domain::models::{DashboardPayload, UserRole};
use crate::state::SharedState;
use crate::web::session;
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::get,
    Json, Router,
};
use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize)]
pub struct CurrentUser {
    pub user_id: Uuid,
    pub name: String,
    pub role: UserRole,
    pub email: String,
}

#[derive(Serialize)]
pub struct TeamDashboard {
    pub members: Vec<DashboardPayload>,
}

#[derive(Serialize)]
pub struct MonthlyMetrics {
    pub year: i32,
    pub month: u32,
    pub month_name: String,
    pub who5_score: Option<f64>,
    pub phq9_score: Option<f64>,
    pub gad7_score: Option<f64>,
    pub burnout_percentage: Option<f64>,
    pub stress_level: Option<f64>,
    pub checkins_count: i64,
}

#[derive(Serialize)]
pub struct HistoricalData {
    pub months: Vec<MonthlyMetrics>,
}

pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/me", get(get_current_user))
        .route("/user/:id", get(user_view))
        .route("/user/:id/history", get(user_history))
        .route("/team", get(team_view))
        .with_state(state)
}

async fn get_current_user(
    headers: HeaderMap,
    State(state): State<SharedState>,
) -> Result<Json<CurrentUser>, StatusCode> {
    let token = session::extract_token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let claims = session::verify_session(&token, &state.session_key)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    let user = db::find_user_by_id(&state.pool, claims.user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let name = state
        .crypto
        .decrypt_str(&user.enc_name)
        .unwrap_or_else(|_| "User".to_string());

    Ok(Json(CurrentUser {
        user_id: user.id,
        name,
        role: user.role,
        email: user.email,
    }))
}

async fn user_view(
    headers: HeaderMap,
    State(state): State<SharedState>,
    Path(id): Path<Uuid>,
) -> Result<Json<DashboardPayload>, StatusCode> {
    let token = session::extract_token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let claims = session::verify_session(&token, &state.session_key)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
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

async fn user_history(
    headers: HeaderMap,
    State(state): State<SharedState>,
    Path(id): Path<Uuid>,
) -> Result<Json<HistoricalData>, StatusCode> {
    let token = session::extract_token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let claims = session::verify_session(&token, &state.session_key)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Users can only see their own history unless they're admin
    if claims.user_id != id && !matches!(claims.role, UserRole::Admin | UserRole::Founder) {
        return Err(StatusCode::FORBIDDEN);
    }

    // Query monthly aggregated check-in data
    #[derive(sqlx::FromRow)]
    struct MonthlyRow {
        year: Option<i32>,
        month: Option<i32>,
        who5_score: Option<f64>,
        phq9_score: Option<f64>,
        gad7_score: Option<f64>,
        burnout_percentage: Option<f64>,
        stress_level: Option<f64>,
        checkins_count: Option<i64>,
    }

    let monthly_data: Vec<MonthlyRow> = sqlx::query_as(
        "SELECT
            EXTRACT(YEAR FROM created_at)::int AS year,
            EXTRACT(MONTH FROM created_at)::int AS month,
            AVG(who5_score) AS who5_score,
            AVG(phq9_score) AS phq9_score,
            AVG(gad7_score) AS gad7_score,
            AVG(burnout_percentage) AS burnout_percentage,
            AVG(stress_level) AS stress_level,
            COUNT(*)::bigint AS checkins_count
        FROM checkin_answers
        WHERE user_id = $1
        GROUP BY year, month
        ORDER BY year DESC, month DESC",
    )
    .bind(id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch user history: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let months: Vec<MonthlyMetrics> = monthly_data
        .into_iter()
        .map(|row| {
            let month_num = row.month.unwrap_or(1) as u32;
            let month_name = match month_num {
                1 => "Січень",
                2 => "Лютий",
                3 => "Березень",
                4 => "Квітень",
                5 => "Травень",
                6 => "Червень",
                7 => "Липень",
                8 => "Серпень",
                9 => "Вересень",
                10 => "Жовтень",
                11 => "Листопад",
                12 => "Грудень",
                _ => "Unknown",
            }
            .to_string();

            MonthlyMetrics {
                year: row.year.unwrap_or(2025),
                month: month_num,
                month_name,
                who5_score: row.who5_score,
                phq9_score: row.phq9_score,
                gad7_score: row.gad7_score,
                burnout_percentage: row.burnout_percentage,
                stress_level: row.stress_level,
                checkins_count: row.checkins_count.unwrap_or(0),
            }
        })
        .collect();

    Ok(Json(HistoricalData { months }))
}

async fn team_view(
    headers: HeaderMap,
    State(state): State<SharedState>,
) -> Result<Json<TeamDashboard>, StatusCode> {
    let token = session::extract_token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let claims = session::verify_session(&token, &state.session_key)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
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
            outstanding_questions: outstanding,
        });
    }

    Ok(Json(TeamDashboard { members }))
}
