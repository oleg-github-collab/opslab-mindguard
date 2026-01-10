pub mod seed;

use crate::crypto::Crypto;
use crate::domain::models::UserRole;
use crate::bot::daily_checkin::{CheckInAnswer, Metrics};
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct DbUser {
    pub id: Uuid,
    pub email: String,
    pub hash: String,
    pub telegram_id: Option<i64>,
    pub role: UserRole,
    pub enc_name: String,
    pub note: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct VoiceLog {
    pub id: Uuid,
    pub user_id: Uuid,
    pub enc_transcript: String,
    pub enc_ai_analysis: Option<String>,
    pub risk_score: i16,
    pub urgent: bool,
    pub created_at: DateTime<Utc>,
}

pub async fn find_user_by_email(pool: &PgPool, email: &str) -> Result<Option<DbUser>> {
    let user = sqlx::query_as!(
        DbUser,
        r#"
        SELECT id, email, hash, telegram_id, role as "role: UserRole", enc_name, note, created_at
        FROM users
        WHERE email = $1
        "#,
        email
    )
    .fetch_optional(pool)
    .await?;
    Ok(user)
}

pub async fn find_user_by_id(pool: &PgPool, id: Uuid) -> Result<Option<DbUser>> {
    let user = sqlx::query_as!(
        DbUser,
        r#"
        SELECT id, email, hash, telegram_id, role as "role: UserRole", enc_name, note, created_at
        FROM users
        WHERE id = $1
        "#,
        id
    )
    .fetch_optional(pool)
    .await?;
    Ok(user)
}

pub async fn find_user_by_telegram(pool: &PgPool, telegram_id: i64) -> Result<Option<DbUser>> {
    let user = sqlx::query_as!(
        DbUser,
        r#"
        SELECT id, email, hash, telegram_id, role as "role: UserRole", enc_name, note, created_at
        FROM users
        WHERE telegram_id = $1
        "#,
        telegram_id
    )
    .fetch_optional(pool)
    .await?;
    Ok(user)
}

pub async fn attach_telegram(pool: &PgPool, user_id: Uuid, telegram_id: i64) -> Result<()> {
    sqlx::query!(
        r#"
        UPDATE users
        SET telegram_id = $2, updated_at = now()
        WHERE id = $1
        "#,
        user_id,
        telegram_id
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// DEPRECATED: Legacy function for backward compatibility with old handlers.rs
/// Converts 0-3 scale answers to 1-10 scale and stores in checkin_answers
/// New code should use insert_checkin_answer() directly
pub async fn insert_answer(pool: &PgPool, user_id: Uuid, question_id: i32, value: i16) -> Result<()> {
    // Map question_id to question_type (legacy mapping)
    let question_type = match question_id {
        1 => "mood",
        2 => "energy",
        3 => "stress",
        4 => "sleep",
        5 => "workload",
        6 => "motivation",
        7 => "focus",
        8 => "wellbeing",
        _ => "mood", // fallback
    };

    // Convert 0-3 scale to 1-10 scale
    // 0 -> 1, 1 -> 4, 2 -> 7, 3 -> 10
    let normalized_value = ((value as f32 / 3.0) * 9.0 + 1.0).round() as i16;

    // Insert into new table
    sqlx::query!(
        r#"
        INSERT INTO checkin_answers (user_id, question_id, question_type, value)
        VALUES ($1, $2, $3, $4)
        "#,
        user_id,
        question_id,
        question_type,
        normalized_value
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn insert_voice_log(
    pool: &PgPool,
    crypto: &Crypto,
    user_id: Uuid,
    transcript: &str,
    analysis: Option<&serde_json::Value>,
    risk_score: i16,
    urgent: bool,
) -> Result<Uuid> {
    let enc_transcript = crypto.encrypt_str(transcript)?;
    let enc_ai_analysis = match analysis {
        Some(val) => Some(crypto.encrypt_str(&val.to_string())?),
        None => None,
    };
    let id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO voice_logs (id, user_id, enc_transcript, enc_ai_analysis, risk_score, urgent)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
        id,
        user_id,
        enc_transcript,
        enc_ai_analysis,
        risk_score,
        urgent
    )
    .execute(pool)
    .await?;
    Ok(id)
}

// ========== Daily Check-in Functions ==========

/// Insert a single check-in answer
pub async fn insert_checkin_answer(
    pool: &PgPool,
    user_id: Uuid,
    question_id: i32,
    qtype: &str,
    value: i16,
) -> Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO checkin_answers (user_id, question_id, question_type, value)
        VALUES ($1, $2, $3, $4)
        "#,
        user_id,
        question_id,
        qtype,
        value
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Get recent check-in answers for a user (last N days)
pub async fn get_recent_checkin_answers(
    pool: &PgPool,
    user_id: Uuid,
    days: i32,
) -> Result<Vec<CheckInAnswer>> {
    let answers = sqlx::query_as!(
        CheckInAnswer,
        r#"
        SELECT question_id, question_type as qtype, value
        FROM checkin_answers
        WHERE user_id = $1
          AND created_at >= NOW() - ($2 || ' days')::INTERVAL
        ORDER BY created_at DESC
        "#,
        user_id,
        days
    )
    .fetch_all(pool)
    .await?;
    Ok(answers)
}

/// Calculate and return metrics for a user
pub async fn calculate_user_metrics(pool: &PgPool, user_id: Uuid) -> Result<Option<Metrics>> {
    let result = sqlx::query!(
        r#"
        SELECT calculate_user_metrics($1) as metrics
        "#,
        user_id
    )
    .fetch_one(pool)
    .await?;

    match result.metrics {
        Some(json_value) => {
            let metrics: Metrics = serde_json::from_value(json_value)?;
            Ok(Some(metrics))
        }
        None => Ok(None),
    }
}

/// Get the count of check-in answers for a user in the last N days
pub async fn get_checkin_answer_count(
    pool: &PgPool,
    user_id: Uuid,
    days: i32,
) -> Result<i64> {
    let result = sqlx::query!(
        r#"
        SELECT COUNT(*) as count
        FROM checkin_answers
        WHERE user_id = $1
          AND created_at >= NOW() - ($2 || ' days')::INTERVAL
        "#,
        user_id,
        days
    )
    .fetch_one(pool)
    .await?;
    Ok(result.count.unwrap_or(0))
}

// ========== Telegram PIN Functions ==========

/// Generate a new PIN code for Telegram linking
pub async fn generate_telegram_pin(pool: &PgPool, user_id: Uuid) -> Result<String> {
    use rand::Rng;

    // Generate 4-digit PIN
    let mut rng = rand::thread_rng();
    let pin_code = format!("{:04}", rng.gen_range(1000..10000));

    // Mark old PINs as used
    sqlx::query!(
        r#"
        UPDATE telegram_pins
        SET used = true
        WHERE user_id = $1 AND used = false
        "#,
        user_id
    )
    .execute(pool)
    .await?;

    // Insert new PIN
    sqlx::query!(
        r#"
        INSERT INTO telegram_pins (user_id, pin_code, expires_at)
        VALUES ($1, $2, NOW() + INTERVAL '5 minutes')
        "#,
        user_id,
        &pin_code
    )
    .execute(pool)
    .await?;

    Ok(pin_code)
}

/// Verify PIN and link Telegram ID to user
pub async fn verify_and_link_telegram(
    pool: &PgPool,
    pin_code: &str,
    telegram_id: i64,
) -> Result<Option<Uuid>> {
    // Find valid PIN
    let pin = sqlx::query!(
        r#"
        SELECT user_id, expires_at
        FROM telegram_pins
        WHERE pin_code = $1
          AND used = false
          AND expires_at > NOW()
        ORDER BY created_at DESC
        LIMIT 1
        "#,
        pin_code
    )
    .fetch_optional(pool)
    .await?;

    let Some(pin) = pin else {
        return Ok(None); // Invalid or expired PIN
    };

    // Mark PIN as used
    sqlx::query!(
        r#"
        UPDATE telegram_pins
        SET used = true, used_at = NOW()
        WHERE pin_code = $1
        "#,
        pin_code
    )
    .execute(pool)
    .await?;

    // Link Telegram ID to user
    sqlx::query!(
        r#"
        UPDATE users
        SET telegram_id = $1, updated_at = NOW()
        WHERE id = $2
        "#,
        telegram_id,
        pin.user_id
    )
    .execute(pool)
    .await?;

    Ok(Some(pin.user_id))
}

/// Get active PIN for user (for display on dashboard)
pub async fn get_active_pin(pool: &PgPool, user_id: Uuid) -> Result<Option<String>> {
    let pin = sqlx::query!(
        r#"
        SELECT pin_code
        FROM telegram_pins
        WHERE user_id = $1
          AND used = false
          AND expires_at > NOW()
        ORDER BY created_at DESC
        LIMIT 1
        "#,
        user_id
    )
    .fetch_optional(pool)
    .await?;

    Ok(pin.map(|p| p.pin_code))
}

// ========== WOW Features Database Functions ==========

// ---------- Smart Reminders (#2) ----------

/// Set user's preferred reminder time
pub async fn set_user_reminder_time(
    pool: &PgPool,
    user_id: Uuid,
    hour: i16,
    minute: i16,
) -> Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO user_preferences (user_id, reminder_hour, reminder_minute)
        VALUES ($1, $2, $3)
        ON CONFLICT (user_id) DO UPDATE
        SET reminder_hour = $2, reminder_minute = $3, updated_at = NOW()
        "#,
        user_id,
        hour,
        minute
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Calculate best reminder time based on user's answer patterns
pub async fn calculate_best_reminder_time(pool: &PgPool, user_id: Uuid) -> Result<(i16, i16)> {
    let result = sqlx::query!(
        r#"
        SELECT
            EXTRACT(HOUR FROM created_at AT TIME ZONE 'UTC')::INT as hour,
            COUNT(*) as count
        FROM checkin_answers
        WHERE user_id = $1
        GROUP BY hour
        ORDER BY count DESC
        LIMIT 1
        "#,
        user_id
    )
    .fetch_optional(pool)
    .await?;

    if let Some(row) = result {
        Ok((row.hour.unwrap_or(10) as i16, 0))
    } else {
        Ok((10, 0)) // Default 10:00
    }
}

/// Get all users who should receive check-in at specific time
pub async fn get_users_for_reminder_time(
    pool: &PgPool,
    hour: i16,
    minute: i16,
) -> Result<Vec<(Uuid, i64)>> {
    let users = sqlx::query!(
        r#"
        SELECT u.id, u.telegram_id
        FROM users u
        LEFT JOIN user_preferences p ON u.id = p.user_id
        WHERE u.telegram_id IS NOT NULL
          AND u.role != 'ADMIN'
          AND COALESCE(p.reminder_hour, 10) = $1
          AND COALESCE(p.reminder_minute, 0) = $2
          AND COALESCE(p.notification_enabled, true) = true
        "#,
        hour,
        minute
    )
    .fetch_all(pool)
    .await?;

    Ok(users
        .into_iter()
        .filter_map(|u| u.telegram_id.map(|tid| (u.id, tid)))
        .collect())
}

// ---------- User Streaks (#6) ----------

/// Get user's current streak
pub async fn get_user_current_streak(pool: &PgPool, user_id: Uuid) -> Result<i32> {
    let streak = sqlx::query_scalar!(
        r#"
        SELECT COALESCE(current_streak, 0) as "streak!"
        FROM user_streaks
        WHERE user_id = $1
        "#,
        user_id
    )
    .fetch_optional(pool)
    .await?;

    Ok(streak.unwrap_or(0))
}

/// Get check-in count for the last week
pub async fn get_checkin_count_for_week(pool: &PgPool, user_id: Uuid) -> Result<i32> {
    let count = sqlx::query_scalar!(
        r#"
        SELECT COUNT(DISTINCT DATE(created_at)) as "count!"
        FROM checkin_answers
        WHERE user_id = $1
          AND created_at >= NOW() - INTERVAL '7 days'
        "#,
        user_id
    )
    .fetch_one(pool)
    .await?;

    Ok(count as i32)
}

/// Get last check-in date for user
pub async fn get_last_checkin_date(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Option<DateTime<Utc>>> {
    let result = sqlx::query_scalar!(
        r#"
        SELECT MAX(created_at) as "last_checkin"
        FROM checkin_answers
        WHERE user_id = $1
        "#,
        user_id
    )
    .fetch_one(pool)
    .await?;

    Ok(result)
}

// ---------- Team Average Metrics (#10) ----------

#[derive(Debug, Serialize, Deserialize)]
pub struct TeamAverage {
    pub who5: f64,
    pub phq9: f64,
    pub gad7: f64,
}

/// Get team average metrics (anonymized)
/// FIXED: Use actual question types (focus, stress) instead of (concentration, anxiety)
pub async fn get_team_average_metrics(pool: &PgPool) -> Result<TeamAverage> {
    let avg = sqlx::query!(
        r#"
        WITH recent_metrics AS (
            SELECT
                user_id,
                AVG(CASE WHEN question_type = 'mood' THEN value * 20.0 ELSE 0 END) as who5,
                AVG(CASE WHEN question_type IN ('mood', 'sleep', 'focus') THEN value * 3.0 ELSE 0 END) as phq9,
                AVG(CASE WHEN question_type = 'stress' THEN value * 3.0 ELSE 0 END) as gad7
            FROM checkin_answers
            WHERE created_at >= NOW() - INTERVAL '7 days'
            GROUP BY user_id
        )
        SELECT
            CAST(COALESCE(AVG(who5), 0.0) AS DOUBLE PRECISION) as "avg_who5: f64",
            CAST(COALESCE(AVG(phq9), 0.0) AS DOUBLE PRECISION) as "avg_phq9: f64",
            CAST(COALESCE(AVG(gad7), 0.0) AS DOUBLE PRECISION) as "avg_gad7: f64"
        FROM recent_metrics
        "#
    )
    .fetch_one(pool)
    .await?;

    Ok(TeamAverage {
        who5: avg.avg_who5,
        phq9: avg.avg_phq9,
        gad7: avg.avg_gad7,
    })
}

// ---------- All Telegram Users (#6) ----------

/// Get all users with Telegram ID (for broadcasting)
pub async fn get_all_telegram_users(pool: &PgPool) -> Result<Vec<(Uuid, i64)>> {
    let users = sqlx::query!(
        r#"
        SELECT id, telegram_id
        FROM users
        WHERE telegram_id IS NOT NULL
          AND role != 'ADMIN'
        "#
    )
    .fetch_all(pool)
    .await?;

    Ok(users
        .into_iter()
        .filter_map(|u| u.telegram_id.map(|tid| (u.id, tid)))
        .collect())
}

// ---------- Kudos System (#17) ----------

#[derive(Debug, Serialize, Deserialize)]
pub struct KudosRecord {
    pub id: Uuid,
    pub from_user_id: Uuid,
    pub to_user_id: Uuid,
    pub message: String,
    pub created_at: DateTime<Utc>,
    pub from_user_enc_name: Vec<u8>,
}

/// Insert a new kudos
pub async fn insert_kudos(
    pool: &PgPool,
    from_user_id: Uuid,
    to_user_id: Uuid,
    message: &str,
) -> Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO kudos (from_user_id, to_user_id, message)
        VALUES ($1, $2, $3)
        "#,
        from_user_id,
        to_user_id,
        message
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Get kudos count for the last week
pub async fn get_kudos_count_for_week(pool: &PgPool, user_id: Uuid) -> Result<i64> {
    let count = sqlx::query_scalar!(
        r#"
        SELECT COUNT(*) as "count!"
        FROM kudos
        WHERE to_user_id = $1
          AND created_at >= NOW() - INTERVAL '7 days'
        "#,
        user_id
    )
    .fetch_one(pool)
    .await?;

    Ok(count)
}

/// Get recent kudos for user
pub async fn get_recent_kudos(
    pool: &PgPool,
    user_id: Uuid,
    limit: i64,
) -> Result<Vec<KudosRecord>> {
    let records = sqlx::query_as!(
        KudosRecord,
        r#"
        SELECT k.id, k.from_user_id, k.to_user_id, k.message, k.created_at,
               u.enc_name as "from_user_enc_name!"
        FROM kudos k
        JOIN users u ON k.from_user_id = u.id
        WHERE k.to_user_id = $1
        ORDER BY k.created_at DESC
        LIMIT $2
        "#,
        user_id,
        limit
    )
    .fetch_all(pool)
    .await?;

    Ok(records)
}

// ---------- Get User by Email (#17) ----------

/// Find user by email (for kudos system)
pub async fn get_user_by_email(pool: &PgPool, email: &str) -> Result<Option<DbUser>> {
    find_user_by_email(pool, email).await
}

/// Get user by Telegram ID
pub async fn get_user_by_telegram_id(pool: &PgPool, telegram_id: i64) -> Result<Option<DbUser>> {
    find_user_by_telegram(pool, telegram_id).await
}

// ---------- All Users (#8) ----------

/// Get all users (for admin/founder heatmap)
pub async fn get_all_users(pool: &PgPool) -> Result<Vec<DbUser>> {
    let users = sqlx::query_as!(
        DbUser,
        r#"
        SELECT id, email, hash, telegram_id, role as "role: UserRole", enc_name, note, created_at
        FROM users
        WHERE role != 'ADMIN'
        ORDER BY created_at ASC
        "#
    )
    .fetch_all(pool)
    .await?;

    Ok(users)
}

/// Get user role
pub async fn get_user_role(pool: &PgPool, user_id: Uuid) -> Result<UserRole> {
    let role = sqlx::query_scalar!(
        r#"
        SELECT role as "role: UserRole"
        FROM users
        WHERE id = $1
        "#,
        user_id
    )
    .fetch_one(pool)
    .await?;

    Ok(role)
}

// ---------- Adaptive Questions (#1) ----------

/// Get question type pattern for adaptive logic
pub async fn get_user_recent_pattern(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<(String, f64)>> {
    let patterns = sqlx::query!(
        r#"
        SELECT question_type, CAST(COALESCE(AVG(value), 0.0) AS DOUBLE PRECISION) as "avg_value: f64"
        FROM checkin_answers
        WHERE user_id = $1
          AND created_at >= NOW() - INTERVAL '3 days'
        GROUP BY question_type
        "#,
        user_id
    )
    .fetch_all(pool)
    .await?;

    Ok(patterns
        .into_iter()
        .map(|p| (p.question_type, p.avg_value))
        .collect())
}

// ---------- Metrics for Period (#6) ----------

/// Calculate user metrics for a specific time period
pub async fn calculate_user_metrics_for_period(
    pool: &PgPool,
    user_id: Uuid,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Result<Option<Metrics>> {
    // FIXED: Use actual question types (focus, stress) instead of (concentration, anxiety)
    let result = sqlx::query!(
        r#"
        SELECT
            CAST(COALESCE(AVG(CASE WHEN question_type = 'mood' THEN value * 20.0 ELSE NULL END), 0.0) AS DOUBLE PRECISION) as "who5: f64",
            CAST(COALESCE(AVG(CASE WHEN question_type IN ('mood', 'sleep', 'focus') THEN value * 3.0 ELSE NULL END), 0.0) AS DOUBLE PRECISION) as "phq9: f64",
            CAST(COALESCE(AVG(CASE WHEN question_type = 'stress' THEN value * 3.0 ELSE NULL END), 0.0) AS DOUBLE PRECISION) as "gad7: f64",
            CAST(COALESCE(AVG(CASE WHEN question_type IN ('energy', 'stress', 'workload') THEN value * 10.0 ELSE NULL END), 0.0) AS DOUBLE PRECISION) as "mbi: f64",
            CAST(COALESCE(AVG(CASE WHEN question_type = 'sleep' THEN value ELSE NULL END), 0.0) AS DOUBLE PRECISION) as "sleep_duration: f64",
            CAST(COALESCE(AVG(CASE WHEN question_type = 'workload' THEN 10.0 - value ELSE NULL END), 0.0) AS DOUBLE PRECISION) as "work_life_balance: f64",
            CAST(COALESCE(AVG(CASE WHEN question_type = 'stress' THEN value * 4.0 ELSE NULL END), 0.0) AS DOUBLE PRECISION) as "stress_level: f64"
        FROM checkin_answers
        WHERE user_id = $1
          AND created_at >= $2
          AND created_at < $3
        "#,
        user_id,
        start,
        end
    )
    .fetch_one(pool)
    .await?;

    if result.who5 == 0.0 {
        return Ok(None);
    }

    Ok(Some(Metrics {
        who5_score: result.who5,
        phq9_score: result.phq9,
        gad7_score: result.gad7,
        mbi_score: result.mbi,
        sleep_duration: result.sleep_duration,
        work_life_balance: result.work_life_balance,
        stress_level: result.stress_level,
    }))
}
