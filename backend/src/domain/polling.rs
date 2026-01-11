use crate::domain::models::{Question, RollingScore};
use anyhow::Result;
use chrono::{Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone)]
pub struct PollEngine {
    decay_days: f32,
}

impl PollEngine {
    pub fn new() -> Self {
        Self { decay_days: 21.0 }
    }

    pub async fn next_questions(&self, pool: &PgPool, user_id: Uuid, limit: usize) -> Result<Vec<Question>> {
        // FIXED: Use checkin_answers instead of answers table
        // Group by question_type to find least recently answered types
        let question_types = vec!["mood", "energy", "stress", "sleep", "workload", "motivation", "focus", "wellbeing"];

        let mut type_scores: Vec<(String, chrono::DateTime<Utc>)> = Vec::new();

        for qtype in &question_types {
            let last_answer = sqlx::query_scalar!(
                r#"
                SELECT MAX(created_at) as "last_answered"
                FROM checkin_answers
                WHERE user_id = $1 AND question_type = $2
                "#,
                user_id,
                qtype
            )
            .fetch_one(pool)
            .await?;

            let timestamp = last_answer.unwrap_or_else(|| chrono::DateTime::from_timestamp(0, 0).unwrap());
            type_scores.push((qtype.to_string(), timestamp));
        }

        // Sort by oldest first
        type_scores.sort_by_key(|(_, ts)| *ts);

        // Take top N least recently answered
        let next: Vec<Question> = type_scores
            .into_iter()
            .take(limit)
            .enumerate()
            .map(|(idx, (qtype, _))| Question {
                id: idx as i32 + 1,
                text: format!("How is your {} today?", qtype),
                order_index: idx as i32,
            })
            .collect();

        Ok(next)
    }

    pub async fn calculate_rolling_score(
        &self,
        pool: &PgPool,
        user_id: Uuid,
        window_days: i64,
    ) -> Result<RollingScore> {
        let since = Utc::now() - Duration::days(window_days);

        // FIXED: Use checkin_answers instead of answers (1-10 scale instead of 0-3)
        let answers = sqlx::query!(
            r#"
            SELECT value, created_at
            FROM checkin_answers
            WHERE user_id = $1 AND created_at >= $2
            ORDER BY created_at DESC
            "#,
            user_id,
            since
        )
        .fetch_all(pool)
        .await?;

        let mut total = 0.0;
        for row in answers {
            let age_days = (Utc::now() - row.created_at).num_seconds() as f32 / 86_400.0;
            let weight = self.decay_weight(age_days);
            // Normalize 1-10 scale to 0-3 for backward compatibility with score calculation
            let normalized_value = (row.value as f32 - 1.0) / 9.0 * 3.0;
            total += normalized_value * weight;
        }

        Ok(RollingScore {
            window_days,
            total,
        })
    }

    fn decay_weight(&self, age_days: f32) -> f32 {
        // Exponential decay favoring recent answers; decay_days is ~1/e point.
        (-age_days / self.decay_days).exp()
    }
}
