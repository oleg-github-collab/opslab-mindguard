pub mod seed;

use crate::bot::daily_checkin::{CheckInAnswer, Metrics};
use crate::crypto::Crypto;
use crate::domain::models::UserRole;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool, Row};
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
pub async fn insert_answer(
    pool: &PgPool,
    user_id: Uuid,
    question_id: i32,
    value: i16,
) -> Result<()> {
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
        &days.to_string()
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
pub async fn get_checkin_answer_count(pool: &PgPool, user_id: Uuid, days: i32) -> Result<i64> {
    let result = sqlx::query!(
        r#"
        SELECT COUNT(*) as count
        FROM checkin_answers
        WHERE user_id = $1
          AND created_at >= NOW() - ($2 || ' days')::INTERVAL
        "#,
        user_id,
        &days.to_string()
    )
    .fetch_one(pool)
    .await?;
    Ok(result.count.unwrap_or(0))
}

// ========== Telegram PIN Functions ==========

/// Generate a new PIN code for Telegram linking
pub async fn generate_telegram_pin(pool: &PgPool, user_id: Uuid) -> Result<String> {
    use rand::Rng;

    // Generate 4-digit PIN (before any await to ensure Send)
    let pin_code = {
        let mut rng = rand::thread_rng();
        format!("{:04}", rng.gen_range(1000..10000))
    };

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
        hour as i32,
        minute as i32
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
pub async fn get_last_checkin_date(pool: &PgPool, user_id: Uuid) -> Result<Option<DateTime<Utc>>> {
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
    let row = sqlx::query(
        r#"
        WITH recent AS (
            SELECT
                user_id,
                AVG(CASE WHEN question_type = 'mood' THEN value END) as mood_avg,
                AVG(CASE WHEN question_type = 'energy' THEN value END) as energy_avg,
                AVG(CASE WHEN question_type = 'wellbeing' THEN value END) as wellbeing_avg,
                AVG(CASE WHEN question_type = 'motivation' THEN value END) as motivation_avg,
                AVG(CASE WHEN question_type = 'focus' THEN value END) as focus_avg,
                AVG(CASE WHEN question_type = 'stress' THEN value END) as stress_avg
            FROM checkin_answers
            WHERE created_at >= NOW() - INTERVAL '7 days'
            GROUP BY user_id
        ),
        per_user AS (
            SELECT
                ((COALESCE(mood_avg, 0) + COALESCE(energy_avg, 0) + COALESCE(wellbeing_avg, 0)) / 3.0 * 10.0) as who5,
                (((10.0 - COALESCE(mood_avg, 0)) + (10.0 - COALESCE(energy_avg, 0)) + (10.0 - COALESCE(motivation_avg, 0))) / 3.0 * 2.7) as phq9,
                ((COALESCE(stress_avg, 0) + (10.0 - COALESCE(focus_avg, 0))) / 2.0 * 2.1) as gad7
            FROM recent
        )
        SELECT
            CAST(COALESCE(AVG(who5), 0.0) AS DOUBLE PRECISION) as avg_who5,
            CAST(COALESCE(AVG(phq9), 0.0) AS DOUBLE PRECISION) as avg_phq9,
            CAST(COALESCE(AVG(gad7), 0.0) AS DOUBLE PRECISION) as avg_gad7
        FROM per_user
        "#
    )
    .fetch_one(pool)
    .await?;

    let who5: f64 = row.try_get("avg_who5")?;
    let phq9: f64 = row.try_get("avg_phq9")?;
    let gad7: f64 = row.try_get("avg_gad7")?;

    Ok(TeamAverage { who5, phq9, gad7 })
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
    pub created_at: Option<DateTime<Utc>>,
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
pub async fn get_user_recent_pattern(pool: &PgPool, user_id: Uuid) -> Result<Vec<(String, f64)>> {
    let patterns: Vec<_> = sqlx::query!(
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
        .map(|p| (p.question_type, p.avg_value.unwrap_or(0.0)))
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
    let row = sqlx::query(
        r#"
        SELECT
            AVG(CASE WHEN question_type = 'mood' THEN value END) as mood_avg,
            AVG(CASE WHEN question_type = 'energy' THEN value END) as energy_avg,
            AVG(CASE WHEN question_type = 'wellbeing' THEN value END) as wellbeing_avg,
            AVG(CASE WHEN question_type = 'motivation' THEN value END) as motivation_avg,
            AVG(CASE WHEN question_type = 'focus' THEN value END) as focus_avg,
            AVG(CASE WHEN question_type = 'stress' THEN value END) as stress_avg,
            AVG(CASE WHEN question_type = 'sleep' THEN value END) as sleep_avg,
            AVG(CASE WHEN question_type = 'workload' THEN value END) as workload_avg
        FROM checkin_answers
        WHERE user_id = $1
          AND created_at >= $2
          AND created_at < $3
        "#
    )
    .bind(user_id)
    .bind(start)
    .bind(end)
    .fetch_one(pool)
    .await?;

    let mood = row
        .try_get::<Option<f64>, _>("mood_avg")?
        .unwrap_or(0.0);
    let energy = row
        .try_get::<Option<f64>, _>("energy_avg")?
        .unwrap_or(0.0);
    let wellbeing = row
        .try_get::<Option<f64>, _>("wellbeing_avg")?
        .unwrap_or(0.0);
    let motivation = row
        .try_get::<Option<f64>, _>("motivation_avg")?
        .unwrap_or(0.0);
    let focus = row
        .try_get::<Option<f64>, _>("focus_avg")?
        .unwrap_or(0.0);
    let stress = row
        .try_get::<Option<f64>, _>("stress_avg")?
        .unwrap_or(0.0);
    let sleep = row
        .try_get::<Option<f64>, _>("sleep_avg")?
        .unwrap_or(0.0);
    let workload = row
        .try_get::<Option<f64>, _>("workload_avg")?
        .unwrap_or(0.0);

    let who5 = ((mood + energy + wellbeing) / 3.0 * 10.0).clamp(0.0, 100.0);
    if who5 == 0.0 {
        return Ok(None);
    }

    let phq9 = (((10.0 - mood) + (10.0 - energy) + (10.0 - motivation)) / 3.0 * 2.7)
        .clamp(0.0, 27.0);
    let gad7 = ((stress + (10.0 - focus)) / 2.0 * 2.1).clamp(0.0, 21.0);
    let mbi = ((stress + workload + (10.0 - energy) + (10.0 - motivation)) / 4.0 * 10.0)
        .clamp(0.0, 100.0);
    let sleep_duration = sleep.clamp(0.0, 10.0);
    let work_life_balance = (10.0 - workload).clamp(0.0, 10.0);
    let stress_level = (stress * 4.0).clamp(0.0, 40.0);

    Ok(Some(Metrics {
        who5_score: who5,
        phq9_score: phq9,
        gad7_score: gad7,
        mbi_score: mbi,
        sleep_duration,
        work_life_balance,
        stress_level,
    }))
}
