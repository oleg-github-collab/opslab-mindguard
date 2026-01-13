use crate::db;
use crate::services::wellness::{generate_daily_plan, PlanItem};
use crate::state::SharedState;
use crate::time_utils;
use crate::web::session::UserSession;
use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct GoalSettingsPayload {
    sleep_target: Option<i16>,
    break_target: Option<i16>,
    move_target: Option<i16>,
    notifications_enabled: Option<bool>,
}

#[derive(Debug, Serialize)]
struct WellnessPlanResponse {
    plan_date: NaiveDate,
    items: Vec<PlanItem>,
    completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/goals", get(get_goals))
        .route("/goals", post(update_goals))
        .route("/plan", get(get_plan))
        .route("/plan/complete", post(complete_plan))
        .with_state(state)
}

async fn get_goals(
    UserSession(user_id): UserSession,
    State(state): State<SharedState>,
) -> Result<Json<db::GoalSettings>, StatusCode> {
    let goals = db::get_user_goal_settings(&state.pool, user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(goals))
}

async fn update_goals(
    UserSession(user_id): UserSession,
    State(state): State<SharedState>,
    Json(payload): Json<GoalSettingsPayload>,
) -> Result<Json<db::GoalSettings>, StatusCode> {
    let mut current = db::get_user_goal_settings(&state.pool, user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if let Some(val) = payload.sleep_target {
        if !(4..=10).contains(&val) {
            return Err(StatusCode::BAD_REQUEST);
        }
        current.sleep_target = val;
    }
    if let Some(val) = payload.break_target {
        if !(1..=10).contains(&val) {
            return Err(StatusCode::BAD_REQUEST);
        }
        current.break_target = val;
    }
    if let Some(val) = payload.move_target {
        if !(5..=120).contains(&val) {
            return Err(StatusCode::BAD_REQUEST);
        }
        current.move_target = val;
    }
    if let Some(val) = payload.notifications_enabled {
        current.notifications_enabled = val;
    }

    db::upsert_user_goal_settings(&state.pool, user_id, &current)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(current))
}

async fn get_plan(
    UserSession(user_id): UserSession,
    State(state): State<SharedState>,
) -> Result<Json<WellnessPlanResponse>, StatusCode> {
    let prefs = db::get_user_preferences(&state.pool, user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let (local_date, _, _) = time_utils::local_components(&prefs.timezone, chrono::Utc::now());

    if let Some(plan) = db::get_wellness_plan(&state.pool, user_id, local_date)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        let items: Vec<PlanItem> =
            serde_json::from_value(plan.items).unwrap_or_else(|_| Vec::new());
        return Ok(Json(WellnessPlanResponse {
            plan_date: plan.plan_date,
            items,
            completed_at: plan.completed_at,
        }));
    }

    let goals = db::get_user_goal_settings(&state.pool, user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let metrics = db::calculate_user_metrics(&state.pool, user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .as_ref()
        .map(|m| m.clone());
    let items = generate_daily_plan(metrics.as_ref(), &goals);
    let items_json = serde_json::to_value(&items).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let plan = db::upsert_wellness_plan(&state.pool, user_id, local_date, &items_json)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(WellnessPlanResponse {
        plan_date: plan.plan_date,
        items,
        completed_at: plan.completed_at,
    }))
}

async fn complete_plan(
    UserSession(user_id): UserSession,
    State(state): State<SharedState>,
) -> Result<StatusCode, StatusCode> {
    let prefs = db::get_user_preferences(&state.pool, user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let (local_date, _, _) = time_utils::local_components(&prefs.timezone, chrono::Utc::now());

    db::mark_wellness_plan_completed(&state.pool, user_id, local_date)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}
