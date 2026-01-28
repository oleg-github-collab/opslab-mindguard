pub mod admin;
pub mod analytics;
pub mod auth;
pub mod checkin;
pub mod dashboard;
pub mod insights;
pub mod kudos;
pub mod pulse;
pub mod session;
pub mod telegram;
pub mod wellness;

use crate::state::SharedState;
use axum::{routing::get, Router};

async fn health() -> &'static str {
    "OK"
}

pub fn routes(state: SharedState) -> Router {
    Router::new()
        .route("/health", get(health))
        .nest("/api", analytics::router(state.clone()))
        .nest("/auth", auth::router(state.clone()))
        .nest("/checkin", checkin::router(state.clone()))
        .nest("/dashboard", dashboard::router(state.clone()))
        .nest("/insights", insights::router(state.clone()))
        .nest("/kudos", kudos::router(state.clone()))
        .nest("/pulse", pulse::router(state.clone()))
        .nest("/wellness", wellness::router(state.clone()))
        .nest("/admin", admin::router(state.clone()))
        .nest("/telegram", telegram::router(state))
}
