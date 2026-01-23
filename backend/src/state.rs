use crate::bot::daily_checkin::CheckIn;
use crate::crypto::Crypto;
use crate::domain::polling::PollEngine;
use crate::services::ai::AiService;
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
    pub checkin_sessions: Arc<RwLock<HashMap<i64, CheckInSession>>>, // telegram_id -> CheckInSession
}

#[derive(Clone)]
pub struct CheckInSession {
    pub checkin: CheckIn,
    pub current_index: usize,
    pub awaiting_open_question: Option<i32>,
    pub urgent_alerts_sent: Option<std::collections::HashSet<String>>,
    pub answered_questions: Option<std::collections::HashSet<i32>>,
}

pub type SharedState = Arc<AppState>;
