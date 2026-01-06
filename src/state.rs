use crate::crypto::Crypto;
use crate::domain::polling::PollEngine;
use crate::services::ai::AiService;
use crate::bot::daily_checkin::CheckIn;
use axum::extract::FromRef;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub crypto: Arc<Crypto>,
    pub ai: Arc<AiService>,
    pub poll_engine: PollEngine,
    pub session_key: Vec<u8>,
    pub checkin_sessions: Arc<RwLock<HashMap<i64, CheckIn>>>, // telegram_id -> CheckIn
}

pub type SharedState = Arc<AppState>;

// Implement FromRef for UserSession extractor
impl FromRef<SharedState> for SharedState {
    fn from_ref(state: &SharedState) -> Self {
        state.clone()
    }
}
