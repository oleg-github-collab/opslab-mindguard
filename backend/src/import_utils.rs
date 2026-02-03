use crate::crypto::Crypto;
use crate::db;
use anyhow::Result;
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct HistoricalAnswer {
    pub user_id: Uuid,
    pub question_id: i32,
    pub value: i16,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Deserialize)]
pub struct HistoricalVoiceLog {
    pub user_id: Uuid,
    pub transcript: String,
    pub ai_json: serde_json::Value,
    pub risk_score: i16,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
}

// Simple helper to import JSON exports without storing PII in plain text.
pub async fn import_answers(pool: &PgPool, payload: &[HistoricalAnswer]) -> Result<()> {
    for entry in payload {
        if let Some(created_at) = entry.created_at {
            sqlx::query!(
                r#"
                INSERT INTO answers (user_id, question_id, value, created_at)
                VALUES ($1, $2, $3, $4)
                "#,
                entry.user_id,
                entry.question_id,
                entry.value,
                created_at
            )
            .execute(pool)
            .await?;
        } else {
            db::insert_answer(pool, entry.user_id, entry.question_id, entry.value).await?;
        }
    }
    Ok(())
}

pub async fn import_voice_logs(
    pool: &PgPool,
    crypto: &Crypto,
    payload: &[HistoricalVoiceLog],
) -> Result<()> {
    for entry in payload {
        let enc_transcript = crypto.encrypt_str(&entry.transcript)?;
        let enc_ai = crypto.encrypt_str(&entry.ai_json.to_string())?;
        if let Some(created_at) = entry.created_at {
            sqlx::query!(
                r#"
                INSERT INTO voice_logs (user_id, enc_transcript, enc_ai_analysis, risk_score, urgent, created_at)
                VALUES ($1, $2, $3, $4, $5, $6)
                "#,
                entry.user_id,
                enc_transcript,
                enc_ai,
                entry.risk_score,
                entry.risk_score >= 9,
                created_at
            )
            .execute(pool)
            .await?;
        } else {
            db::insert_voice_log(
                pool,
                crypto,
                entry.user_id,
                &entry.transcript,
                Some(&entry.ai_json),
                entry.risk_score,
                entry.risk_score >= 9,
            )
            .await?;
        }
    }
    Ok(())
}
