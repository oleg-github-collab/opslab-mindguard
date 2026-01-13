use crate::analytics::{correlations, early_signal, self_benchmark};
use crate::db;
use crate::domain::models::UserRole;
use crate::state::SharedState;
use crate::web::session::UserSession;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
struct InsightQuery {
    user_id: Option<Uuid>,
}

pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/self-benchmark", get(get_self_benchmark))
        .route("/correlations", get(get_correlations))
        .route("/early-signal", get(get_early_signal))
        .with_state(state)
}

async fn resolve_user_id(
    state: &SharedState,
    requester_id: Uuid,
    query: &InsightQuery,
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

async fn get_self_benchmark(
    UserSession(user_id): UserSession,
    State(state): State<SharedState>,
    Query(query): Query<InsightQuery>,
) -> Result<Json<Option<self_benchmark::SelfBenchmark>>, StatusCode> {
    let target_id = resolve_user_id(&state, user_id, &query).await?;
    let report = self_benchmark::build_self_benchmark(&state.pool, target_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(report))
}

async fn get_correlations(
    UserSession(user_id): UserSession,
    State(state): State<SharedState>,
    Query(query): Query<InsightQuery>,
) -> Result<Json<Vec<correlations::CorrelationInsight>>, StatusCode> {
    let target_id = resolve_user_id(&state, user_id, &query).await?;
    let insights = correlations::analyze_correlations(&state.pool, target_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(insights))
}

async fn get_early_signal(
    UserSession(user_id): UserSession,
    State(state): State<SharedState>,
    Query(query): Query<InsightQuery>,
) -> Result<Json<Option<early_signal::EarlySignal>>, StatusCode> {
    let target_id = resolve_user_id(&state, user_id, &query).await?;
    let signal = early_signal::detect_early_signal(&state.pool, target_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(signal))
}
