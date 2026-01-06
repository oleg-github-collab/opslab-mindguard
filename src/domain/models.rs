use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize, sqlx::Type, PartialEq, Eq)]
#[sqlx(type_name = "user_role", rename_all = "UPPERCASE")]
#[serde(rename_all = "UPPERCASE")]
pub enum UserRole {
    Admin,
    Founder,
    Employee,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Question {
    pub id: i32,
    pub text: String,
    pub order_index: i32,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Answer {
    pub id: Uuid,
    pub user_id: Uuid,
    pub question_id: i32,
    pub value: i16,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RollingScore {
    pub window_days: i64,
    pub total: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DashboardPayload {
    pub user_id: Uuid,
    pub role: UserRole,
    pub rolling_score: RollingScore,
    pub last_voice_risk: Option<i16>,
    pub outstanding_questions: Vec<Question>,
}
