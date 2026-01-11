pub mod admin;
pub mod auth;
pub mod dashboard;
pub mod feedback;
pub mod session;
pub mod telegram;

use crate::state::SharedState;
use axum::Router;

pub fn routes(state: SharedState) -> Router {
    Router::new()
        .nest("/auth", auth::router(state.clone()))
        .nest("/dashboard", dashboard::router(state.clone()))
        .nest("/feedback", feedback::router(state.clone()))
        .nest("/admin", admin::router(state.clone()))
        .nest("/telegram", telegram::router(state))
}
