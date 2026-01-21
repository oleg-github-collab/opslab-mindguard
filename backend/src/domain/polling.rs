use crate::domain::models::{Question, RollingScore};
use anyhow::Result;
use chrono::{Duration, Utc};
use sqlx::{PgPool, Row};
use uuid::Uuid;

#[derive(Clone)]
pub struct PollEngine {
    decay_days: f32,
}

impl PollEngine {
    pub fn new() -> Self {
        Self { decay_days: 21.0 }
    }

    pub async fn next_questions(
        &self,
        pool: &PgPool,
        user_id: Uuid,
        limit: usize,
    ) -> Result<Vec<Question>> {
        // FIXED: Use checkin_answers instead of answers table
        // Group by question_type to find least recently answered types
        let question_types = vec![
            "mood",
            "energy",
            "stress",
            "sleep",
            "workload",
            "motivation",
            "focus",
            "wellbeing",
        ];

        let mut type_scores: Vec<(String, chrono::DateTime<Utc>)> = Vec::new();

        for qtype in &question_types {
            let last_answer: Option<chrono::DateTime<Utc>> = sqlx::query_scalar(
                r#"
                SELECT MAX(created_at)
                FROM checkin_answers
                WHERE user_id = $1 AND question_type = $2
                "#,
            )
            .bind(user_id)
            .bind(qtype)
            .fetch_one(pool)
            .await?;

            let timestamp =
                last_answer.unwrap_or_else(|| chrono::DateTime::from_timestamp(0, 0).unwrap());
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
        let answers = sqlx::query(
            r#"
            SELECT question_type, value, created_at
            FROM checkin_answers
            WHERE user_id = $1 AND created_at >= $2
            ORDER BY created_at DESC
            "#,
        )
        .bind(user_id)
        .bind(since)
        .fetch_all(pool)
        .await?;

        let mut weighted_sum = 0.0;
        let mut weight_total = 0.0;

        for row in answers {
            let qtype: String = row.try_get("question_type")?;
            let value: i16 = row.try_get("value")?;
            let created_at: chrono::DateTime<Utc> = row.try_get("created_at")?;

            let age_days = (Utc::now() - created_at).num_seconds() as f32 / 86_400.0;
            let weight = self.decay_weight(age_days);

            let adjusted_value = match qtype.as_str() {
                "stress" | "workload" => 11 - value,
                _ => value,
            } as f32;

            // Normalize 1-10 scale to 0-3 and compute weighted average
            let normalized_value = (adjusted_value - 1.0) / 9.0 * 3.0;
            weighted_sum += normalized_value * weight;
            weight_total += weight;
        }

        let avg = if weight_total > 0.0 {
            weighted_sum / weight_total
        } else {
            0.0
        };
        let total = (avg / 3.0 * 100.0).clamp(0.0, 100.0);

        Ok(RollingScore { window_days, total })
    }

    fn decay_weight(&self, age_days: f32) -> f32 {
        // Exponential decay favoring recent answers; decay_days is ~1/e point.
        (-age_days / self.decay_days).exp()
    }
}
