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
    tracing::info!("user_history called for user_id: {}", id);

    let token = session::extract_token(&headers).ok_or_else(|| {
        tracing::error!("No session token found in headers");
        StatusCode::UNAUTHORIZED
    })?;

    let claims = session::verify_session(&token, &state.session_key)
        .map_err(|e| {
            tracing::error!("Session verification failed: {}", e);
            StatusCode::UNAUTHORIZED
        })?;

    tracing::info!("Session verified for user: {}, role: {:?}", claims.user_id, claims.role);

    // Users can only see their own history unless they're admin
    if claims.user_id != id && !matches!(claims.role, UserRole::Admin | UserRole::Founder) {
        tracing::warn!("User {} attempted to access history of user {}", claims.user_id, id);
        return Err(StatusCode::FORBIDDEN);
    }

    // Query monthly aggregated check-in data
    #[derive(sqlx::FromRow)]
    struct MonthlyRow {
        year: i32,
        month: i32,
        mood_avg: Option<f64>,
        energy_avg: Option<f64>,
        wellbeing_avg: Option<f64>,
        motivation_avg: Option<f64>,
        focus_avg: Option<f64>,
        stress_avg: Option<f64>,
        sleep_avg: Option<f64>,
        workload_avg: Option<f64>,
        checkins_count: i64,
    }

    let monthly_data: Vec<MonthlyRow> = sqlx::query_as(
        "SELECT
            EXTRACT(YEAR FROM created_at)::int AS year,
            EXTRACT(MONTH FROM created_at)::int AS month,
            AVG(CASE WHEN question_type = 'mood' THEN value END) AS mood_avg,
            AVG(CASE WHEN question_type = 'energy' THEN value END) AS energy_avg,
            AVG(CASE WHEN question_type = 'wellbeing' THEN value END) AS wellbeing_avg,
            AVG(CASE WHEN question_type = 'motivation' THEN value END) AS motivation_avg,
            AVG(CASE WHEN question_type = 'focus' THEN value END) AS focus_avg,
            AVG(CASE WHEN question_type = 'stress' THEN value END) AS stress_avg,
            AVG(CASE WHEN question_type = 'sleep' THEN value END) AS sleep_avg,
            AVG(CASE WHEN question_type = 'workload' THEN value END) AS workload_avg,
            COUNT(DISTINCT DATE(created_at))::bigint AS checkins_count
        FROM checkin_answers
        WHERE user_id = $1
        GROUP BY 1, 2
        ORDER BY 1 DESC, 2 DESC",
    )
    .bind(id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch user history for user {}: {}", id, e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    tracing::info!("Found {} months of data for user {}", monthly_data.len(), id);

    let months: Vec<MonthlyMetrics> = monthly_data
        .into_iter()
        .map(|row| {
            let month_num = row.month as u32;
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

            let mood = row.mood_avg.unwrap_or(0.0);
            let energy = row.energy_avg.unwrap_or(0.0);
            let wellbeing = row.wellbeing_avg.unwrap_or(0.0);
            let motivation = row.motivation_avg.unwrap_or(0.0);
            let focus = row.focus_avg.unwrap_or(0.0);
            let stress = row.stress_avg.unwrap_or(0.0);
            let _sleep = row.sleep_avg.unwrap_or(0.0);
            let workload = row.workload_avg.unwrap_or(0.0);

            let who5 = ((mood + energy + wellbeing) / 3.0 * 10.0).clamp(0.0, 100.0);
            let phq9 = (((10.0 - mood) + (10.0 - energy) + (10.0 - motivation)) / 3.0 * 2.7)
                .clamp(0.0, 27.0);
            let gad7 = ((stress + (10.0 - focus)) / 2.0 * 2.1).clamp(0.0, 21.0);
            let burnout = ((stress + workload + (10.0 - energy) + (10.0 - motivation)) / 4.0
                * 10.0)
                .clamp(0.0, 100.0);
            let stress_level = (stress * 4.0).clamp(0.0, 40.0);

            MonthlyMetrics {
                year: row.year,
                month: month_num,
                month_name,
                who5_score: Some(who5),
                phq9_score: Some(phq9),
                gad7_score: Some(gad7),
                burnout_percentage: Some(burnout),
                stress_level: Some(stress_level),
                checkins_count: row.checkins_count,
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
