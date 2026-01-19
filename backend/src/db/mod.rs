pub mod seed;

use crate::bot::daily_checkin::{CheckInAnswer, Metrics};
use crate::crypto::Crypto;
use crate::domain::models::UserRole;
use crate::time_utils;
use anyhow::Result;
use argon2::{password_hash::PasswordHash, Argon2, PasswordVerifier};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool, Row};
use std::collections::HashMap;
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
    pub is_active: bool,
    pub offboarded_at: Option<DateTime<Utc>>,
    pub offboarded_by: Option<Uuid>,
    pub offboarded_reason: Option<String>,
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

#[derive(Debug, Serialize, Deserialize)]
pub struct UserPreferences {
    pub reminder_hour: i16,
    pub reminder_minute: i16,
    pub timezone: String,
    pub notification_enabled: bool,
    pub last_reminder_date: Option<NaiveDate>,
    pub last_plan_nudge_date: Option<NaiveDate>,
    pub onboarding_completed: bool,
    pub onboarding_completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReminderCandidate {
    pub user_id: Uuid,
    pub telegram_id: i64,
    pub reminder_hour: i16,
    pub reminder_minute: i16,
    pub timezone: String,
    pub notification_enabled: bool,
    pub last_reminder_date: Option<NaiveDate>,
    pub last_plan_nudge_date: Option<NaiveDate>,
    pub onboarding_completed: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GoalSettings {
    pub sleep_target: i16,
    pub break_target: i16,
    pub move_target: i16,
    pub notifications_enabled: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WellnessPlan {
    pub id: Uuid,
    pub plan_date: NaiveDate,
    pub items: serde_json::Value,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct PulseRoom {
    pub id: Uuid,
    pub slug: String,
    pub title: String,
    pub description: Option<String>,
    pub require_moderation: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PulseMessage {
    pub id: Uuid,
    pub room_id: Uuid,
    pub user_id: Uuid,
    pub content: String,
    pub is_anonymous: bool,
    pub status: String,
    pub moderation_reason: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
pub struct PulseMessageRow {
    pub id: Uuid,
    pub room_id: Uuid,
    pub user_id: Option<Uuid>,
    pub enc_content: Vec<u8>,
    pub is_anonymous: bool,
    pub status: String,
    pub moderation_reason: Option<String>,
    pub created_at: DateTime<Utc>,
}

pub async fn find_user_by_email(pool: &PgPool, email: &str) -> Result<Option<DbUser>> {
    let user = sqlx::query_as::<_, DbUser>(
        r#"
        SELECT
            id,
            email,
            hash,
            telegram_id,
            role,
            enc_name,
            note,
            created_at,
            is_active,
            offboarded_at,
            offboarded_by,
            offboarded_reason
        FROM users
        WHERE email = $1
          AND is_active = true
        "#,
    )
    .bind(email)
    .fetch_optional(pool)
    .await?;
    Ok(user)
}

/// Find user by email including inactive (admin-only use)
pub async fn find_user_by_email_any(pool: &PgPool, email: &str) -> Result<Option<DbUser>> {
    let user = sqlx::query_as::<_, DbUser>(
        r#"
        SELECT
            id,
            email,
            hash,
            telegram_id,
            role,
            enc_name,
            note,
            created_at,
            is_active,
            offboarded_at,
            offboarded_by,
            offboarded_reason
        FROM users
        WHERE email = $1
        "#,
    )
    .bind(email)
    .fetch_optional(pool)
    .await?;
    Ok(user)
}

pub async fn find_user_by_id(pool: &PgPool, id: Uuid) -> Result<Option<DbUser>> {
    let user = sqlx::query_as::<_, DbUser>(
        r#"
        SELECT
            id,
            email,
            hash,
            telegram_id,
            role,
            enc_name,
            note,
            created_at,
            is_active,
            offboarded_at,
            offboarded_by,
            offboarded_reason
        FROM users
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(user)
}

pub async fn find_user_by_telegram(pool: &PgPool, telegram_id: i64) -> Result<Option<DbUser>> {
    let user = sqlx::query_as::<_, DbUser>(
        r#"
        SELECT
            id,
            email,
            hash,
            telegram_id,
            role,
            enc_name,
            note,
            created_at,
            is_active,
            offboarded_at,
            offboarded_by,
            offboarded_reason
        FROM users
        WHERE telegram_id = $1
        "#,
    )
    .bind(telegram_id)
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TelegramLinkOutcome {
    Linked(Uuid),
    AlreadyLinked { user_id: Uuid, same_telegram: bool },
    InvalidCredentials,
    TelegramIdInUse,
}

/// Link Telegram to user using pre-issued access code (email + 4-digit code).
pub async fn link_telegram_by_email_code(
    pool: &PgPool,
    email: &str,
    code: &str,
    telegram_id: i64,
) -> Result<TelegramLinkOutcome> {
    let email = email.trim().to_lowercase();
    if email.is_empty() || code.is_empty() {
        return Ok(TelegramLinkOutcome::InvalidCredentials);
    }

    let Some(user) = find_user_by_email(pool, &email).await? else {
        return Ok(TelegramLinkOutcome::InvalidCredentials);
    };

    if let Some(existing) = user.telegram_id {
        return Ok(TelegramLinkOutcome::AlreadyLinked {
            user_id: user.id,
            same_telegram: existing == telegram_id,
        });
    }

    if find_user_by_telegram(pool, telegram_id).await?.is_some() {
        return Ok(TelegramLinkOutcome::TelegramIdInUse);
    }

    let pin = sqlx::query!(
        r#"
        SELECT id
        FROM telegram_pins
        WHERE user_id = $1
          AND pin_code = $2
          AND used = false
          AND expires_at > NOW()
        ORDER BY created_at DESC
        LIMIT 1
        "#,
        user.id,
        code
    )
    .fetch_optional(pool)
    .await?;

    if let Some(pin) = pin {
        sqlx::query!(
            r#"
            UPDATE telegram_pins
            SET used = true, used_at = NOW()
            WHERE id = $1
            "#,
            pin.id
        )
        .execute(pool)
        .await?;

        sqlx::query!(
            r#"
            UPDATE users
            SET telegram_id = $1, updated_at = NOW()
            WHERE id = $2
            "#,
            telegram_id,
            user.id
        )
        .execute(pool)
        .await?;

        return Ok(TelegramLinkOutcome::Linked(user.id));
    }

    let parsed_hash = PasswordHash::new(&user.hash)
        .map_err(|e| anyhow::anyhow!("Invalid password hash: {}", e))?;
    if Argon2::default()
        .verify_password(code.as_bytes(), &parsed_hash)
        .is_err()
    {
        return Ok(TelegramLinkOutcome::InvalidCredentials);
    }

    sqlx::query!(
        r#"
        UPDATE users
        SET telegram_id = $1, updated_at = NOW()
        WHERE id = $2
        "#,
        telegram_id,
        user.id
    )
    .execute(pool)
    .await?;

    Ok(TelegramLinkOutcome::Linked(user.id))
}

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
    sqlx::query(
        r#"
        INSERT INTO user_preferences (user_id, reminder_hour, reminder_minute)
        VALUES ($1, $2, $3)
        ON CONFLICT (user_id) DO UPDATE
        SET reminder_hour = $2,
            reminder_minute = $3,
            last_reminder_date = NULL,
            updated_at = NOW()
        "#,
    )
    .bind(user_id)
    .bind(hour)
    .bind(minute)
    .execute(pool)
    .await?;
    Ok(())
}

/// Update user's timezone (IANA or UTC offset)
pub async fn set_user_timezone(pool: &PgPool, user_id: Uuid, timezone: &str) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO user_preferences (user_id, timezone)
        VALUES ($1, $2)
        ON CONFLICT (user_id) DO UPDATE
        SET timezone = $2,
            updated_at = NOW()
        "#,
    )
    .bind(user_id)
    .bind(timezone)
    .execute(pool)
    .await?;
    Ok(())
}

/// Toggle notification setting for reminders
pub async fn set_user_notification_enabled(
    pool: &PgPool,
    user_id: Uuid,
    enabled: bool,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO user_preferences (user_id, notification_enabled)
        VALUES ($1, $2)
        ON CONFLICT (user_id) DO UPDATE
        SET notification_enabled = $2,
            updated_at = NOW()
        "#,
    )
    .bind(user_id)
    .bind(enabled)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn set_user_onboarding_complete(
    pool: &PgPool,
    user_id: Uuid,
    completed: bool,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO user_preferences (user_id, onboarding_completed, onboarding_completed_at)
        VALUES ($1, $2, CASE WHEN $2 THEN NOW() ELSE NULL END)
        ON CONFLICT (user_id) DO UPDATE
        SET onboarding_completed = $2,
            onboarding_completed_at = CASE WHEN $2 THEN NOW() ELSE NULL END,
            updated_at = NOW()
        "#,
    )
    .bind(user_id)
    .bind(completed)
    .execute(pool)
    .await?;
    Ok(())
}

/// Get user preferences with defaults
pub async fn get_user_preferences(pool: &PgPool, user_id: Uuid) -> Result<UserPreferences> {
    let row = sqlx::query(
        r#"
        SELECT
            COALESCE(reminder_hour, 10)::SMALLINT as reminder_hour,
            COALESCE(reminder_minute, 0)::SMALLINT as reminder_minute,
            COALESCE(timezone, 'Europe/Kyiv') as timezone,
            COALESCE(notification_enabled, true) as notification_enabled,
            last_reminder_date,
            last_plan_nudge_date,
            COALESCE(onboarding_completed, false) as onboarding_completed,
            onboarding_completed_at
        FROM user_preferences
        WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    if let Some(row) = row {
        Ok(UserPreferences {
            reminder_hour: row.try_get::<i16, _>("reminder_hour")?,
            reminder_minute: row.try_get::<i16, _>("reminder_minute")?,
            timezone: row.try_get::<String, _>("timezone")?,
            notification_enabled: row.try_get::<bool, _>("notification_enabled")?,
            last_reminder_date: row.try_get::<Option<NaiveDate>, _>("last_reminder_date")?,
            last_plan_nudge_date: row.try_get::<Option<NaiveDate>, _>("last_plan_nudge_date")?,
            onboarding_completed: row.try_get::<bool, _>("onboarding_completed")?,
            onboarding_completed_at: row.try_get::<Option<DateTime<Utc>>, _>("onboarding_completed_at")?,
        })
    } else {
        Ok(UserPreferences {
            reminder_hour: 10,
            reminder_minute: 0,
            timezone: "Europe/Kyiv".to_string(),
            notification_enabled: true,
            last_reminder_date: None,
            last_plan_nudge_date: None,
            onboarding_completed: false,
            onboarding_completed_at: None,
        })
    }
}

/// Mark reminder as sent for a local date (idempotent)
pub async fn mark_reminder_sent(
    pool: &PgPool,
    user_id: Uuid,
    local_date: NaiveDate,
) -> Result<bool> {
    let result = sqlx::query(
        r#"
        INSERT INTO user_preferences (user_id, last_reminder_date)
        VALUES ($1, $2)
        ON CONFLICT (user_id) DO UPDATE
        SET last_reminder_date = EXCLUDED.last_reminder_date,
            updated_at = NOW()
        WHERE user_preferences.last_reminder_date IS NULL
           OR user_preferences.last_reminder_date < EXCLUDED.last_reminder_date
        "#,
    )
    .bind(user_id)
    .bind(local_date)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

/// Get reminder candidates with preferences
pub async fn get_reminder_candidates(pool: &PgPool) -> Result<Vec<ReminderCandidate>> {
    let rows = sqlx::query(
        r#"
        SELECT
            u.id as user_id,
            u.telegram_id as telegram_id,
            COALESCE(p.reminder_hour, 10)::SMALLINT as reminder_hour,
            COALESCE(p.reminder_minute, 0)::SMALLINT as reminder_minute,
            COALESCE(p.timezone, 'Europe/Kyiv') as timezone,
            COALESCE(p.notification_enabled, true) as notification_enabled,
            p.last_reminder_date as last_reminder_date,
            p.last_plan_nudge_date as last_plan_nudge_date,
            COALESCE(p.onboarding_completed, false) as onboarding_completed
        FROM users u
        LEFT JOIN user_preferences p ON u.id = p.user_id
        WHERE u.telegram_id IS NOT NULL
          AND u.role != 'ADMIN'
          AND u.is_active = true
          AND COALESCE(p.onboarding_completed, false) = true
        "#,
    )
    .fetch_all(pool)
    .await?;

    let mut out = Vec::new();
    for row in rows {
        let telegram_id: Option<i64> = row.try_get("telegram_id")?;
        let Some(telegram_id) = telegram_id else {
            continue;
        };
        out.push(ReminderCandidate {
            user_id: row.try_get("user_id")?,
            telegram_id,
            reminder_hour: row.try_get("reminder_hour")?,
            reminder_minute: row.try_get("reminder_minute")?,
            timezone: row.try_get("timezone")?,
            notification_enabled: row.try_get("notification_enabled")?,
            last_reminder_date: row.try_get("last_reminder_date")?,
            last_plan_nudge_date: row.try_get("last_plan_nudge_date")?,
            onboarding_completed: row.try_get("onboarding_completed")?,
        });
    }

    Ok(out)
}

/// Calculate best reminder time based on user's local activity
pub async fn calculate_best_reminder_time_local(
    pool: &PgPool,
    user_id: Uuid,
    timezone: &str,
) -> Result<(i16, i16)> {
    let rows = sqlx::query(
        r#"
        SELECT created_at
        FROM checkin_answers
        WHERE user_id = $1
          AND created_at >= NOW() - INTERVAL '60 days'
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    if rows.is_empty() {
        return Ok((10, 0));
    }

    let mut counts: HashMap<(i16, i16), i32> = HashMap::new();
    for row in rows {
        let created_at: DateTime<Utc> = row.try_get("created_at")?;
        let (_, hour, minute) = time_utils::local_components(timezone, created_at);
        let mut rounded_minute = ((minute as i32 + 7) / 15) * 15;
        let mut rounded_hour = hour as i32;
        if rounded_minute == 60 {
            rounded_minute = 0;
            rounded_hour = (rounded_hour + 1) % 24;
        }
        let key = (rounded_hour as i16, rounded_minute as i16);
        *counts.entry(key).or_insert(0) += 1;
    }

    let ((hour, minute), _) = counts
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .unwrap_or(((10, 0), 0));

    Ok((hour, minute))
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
            FROM checkin_answers ca
            JOIN users u ON u.id = ca.user_id
            WHERE ca.created_at >= NOW() - INTERVAL '7 days'
              AND u.is_active = true
            GROUP BY ca.user_id
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
    let rows = sqlx::query(
        r#"
        SELECT u.id, u.telegram_id
        FROM users u
        LEFT JOIN user_preferences p ON u.id = p.user_id
        WHERE u.telegram_id IS NOT NULL
          AND u.role != 'ADMIN'
          AND u.is_active = true
          AND COALESCE(p.onboarding_completed, false) = true
          AND COALESCE(p.notification_enabled, true) = true
        "#,
    )
    .fetch_all(pool)
    .await?;

    let mut out = Vec::new();
    for row in rows {
        let user_id: Uuid = row.try_get("id")?;
        let telegram_id: Option<i64> = row.try_get("telegram_id")?;
        if let Some(telegram_id) = telegram_id {
            out.push((user_id, telegram_id));
        }
    }

    Ok(out)
}

// ---------- Kudos System (#17) ----------

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct KudosRecord {
    pub id: Uuid,
    pub from_user_id: Uuid,
    pub to_user_id: Uuid,
    pub message: String,
    pub created_at: Option<DateTime<Utc>>,
    pub from_user_enc_name: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct KudosSentRecord {
    pub id: Uuid,
    pub from_user_id: Uuid,
    pub to_user_id: Uuid,
    pub message: String,
    pub created_at: Option<DateTime<Utc>>,
    pub to_user_enc_name: Vec<u8>,
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

/// Get all users (admin/founder scope, includes inactive)
pub async fn get_all_users(pool: &PgPool) -> Result<Vec<DbUser>> {
    let users = sqlx::query_as::<_, DbUser>(
        r#"
        SELECT
            id,
            email,
            hash,
            telegram_id,
            role,
            enc_name,
            note,
            created_at,
            is_active,
            offboarded_at,
            offboarded_by,
            offboarded_reason
        FROM users
        ORDER BY created_at ASC
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(users)
}

/// Get active non-admin users (for team metrics/heatmap)
pub async fn get_active_users(pool: &PgPool) -> Result<Vec<DbUser>> {
    let users = sqlx::query_as::<_, DbUser>(
        r#"
        SELECT
            id,
            email,
            hash,
            telegram_id,
            role,
            enc_name,
            note,
            created_at,
            is_active,
            offboarded_at,
            offboarded_by,
            offboarded_reason
        FROM users
        WHERE is_active = true
          AND role != 'ADMIN'
        ORDER BY created_at ASC
        "#,
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

// ---------- Pulse Rooms ----------

pub async fn get_pulse_rooms(pool: &PgPool) -> Result<Vec<PulseRoom>> {
    let rooms = sqlx::query_as::<_, PulseRoom>(
        r#"
        SELECT id, slug, title, description, require_moderation, created_at
        FROM pulse_rooms
        WHERE is_active = true
        ORDER BY created_at ASC
        "#,
    )
    .fetch_all(pool)
    .await?;
    Ok(rooms)
}

pub async fn get_pulse_room_by_slug(pool: &PgPool, slug: &str) -> Result<Option<PulseRoom>> {
    let room = sqlx::query_as::<_, PulseRoom>(
        r#"
        SELECT id, slug, title, description, require_moderation, created_at
        FROM pulse_rooms
        WHERE slug = $1
          AND is_active = true
        "#,
    )
    .bind(slug)
    .fetch_optional(pool)
    .await?;
    Ok(room)
}

pub async fn get_pulse_messages(
    pool: &PgPool,
    room_id: Uuid,
    include_pending: bool,
    limit: i64,
) -> Result<Vec<PulseMessageRow>> {
    let rows = if include_pending {
        sqlx::query_as::<_, PulseMessageRow>(
            r#"
            SELECT id, room_id, user_id, enc_content, is_anonymous, status, moderation_reason, created_at
            FROM pulse_messages
            WHERE room_id = $1
            ORDER BY created_at DESC
            LIMIT $2
            "#,
        )
        .bind(room_id)
        .bind(limit)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, PulseMessageRow>(
            r#"
            SELECT id, room_id, user_id, enc_content, is_anonymous, status, moderation_reason, created_at
            FROM pulse_messages
            WHERE room_id = $1
              AND status = 'APPROVED'
            ORDER BY created_at DESC
            LIMIT $2
            "#,
        )
        .bind(room_id)
        .bind(limit)
        .fetch_all(pool)
        .await?
    };
    Ok(rows)
}

pub async fn insert_pulse_message(
    pool: &PgPool,
    room_id: Uuid,
    user_id: Uuid,
    enc_content: &[u8],
    is_anonymous: bool,
    status: &str,
    moderation_reason: Option<&str>,
) -> Result<Uuid> {
    let id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO pulse_messages (id, room_id, user_id, enc_content, is_anonymous, status, moderation_reason)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
    )
    .bind(id)
    .bind(room_id)
    .bind(user_id)
    .bind(enc_content)
    .bind(is_anonymous)
    .bind(status)
    .bind(moderation_reason)
    .execute(pool)
    .await?;
    Ok(id)
}

pub async fn update_pulse_message_status(
    pool: &PgPool,
    message_id: Uuid,
    status: &str,
    moderator_id: Uuid,
    moderation_reason: Option<&str>,
) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE pulse_messages
        SET status = $1,
            moderated_by = $2,
            moderated_at = NOW(),
            moderation_reason = $3
        WHERE id = $4
        "#,
    )
    .bind(status)
    .bind(moderator_id)
    .bind(moderation_reason)
    .bind(message_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_pending_pulse_messages(
    pool: &PgPool,
    limit: i64,
) -> Result<Vec<PulseMessageRow>> {
    let rows = sqlx::query_as::<_, PulseMessageRow>(
        r#"
        SELECT id, room_id, user_id, enc_content, is_anonymous, status, moderation_reason, created_at
        FROM pulse_messages
        WHERE status = 'PENDING'
        ORDER BY created_at DESC
        LIMIT $1
        "#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

// ---------- Wellness Goals + Plans ----------

pub async fn get_user_goal_settings(pool: &PgPool, user_id: Uuid) -> Result<GoalSettings> {
    let row = sqlx::query(
        r#"
        SELECT sleep_target, break_target, move_target, notifications_enabled
        FROM user_goal_settings
        WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    if let Some(row) = row {
        Ok(GoalSettings {
            sleep_target: row.try_get::<i16, _>("sleep_target")?,
            break_target: row.try_get::<i16, _>("break_target")?,
            move_target: row.try_get::<i16, _>("move_target")?,
            notifications_enabled: row.try_get::<bool, _>("notifications_enabled")?,
        })
    } else {
        Ok(GoalSettings {
            sleep_target: 7,
            break_target: 3,
            move_target: 20,
            notifications_enabled: true,
        })
    }
}

pub async fn upsert_user_goal_settings(
    pool: &PgPool,
    user_id: Uuid,
    settings: &GoalSettings,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO user_goal_settings (user_id, sleep_target, break_target, move_target, notifications_enabled)
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (user_id) DO UPDATE
        SET sleep_target = $2,
            break_target = $3,
            move_target = $4,
            notifications_enabled = $5,
            updated_at = NOW()
        "#,
    )
    .bind(user_id)
    .bind(settings.sleep_target)
    .bind(settings.break_target)
    .bind(settings.move_target)
    .bind(settings.notifications_enabled)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_wellness_plan(
    pool: &PgPool,
    user_id: Uuid,
    plan_date: NaiveDate,
) -> Result<Option<WellnessPlan>> {
    let row = sqlx::query(
        r#"
        SELECT id, plan_date, items, completed_at
        FROM wellness_plans
        WHERE user_id = $1
          AND plan_date = $2
        "#,
    )
    .bind(user_id)
    .bind(plan_date)
    .fetch_optional(pool)
    .await?;

    if let Some(row) = row {
        Ok(Some(WellnessPlan {
            id: row.try_get("id")?,
            plan_date: row.try_get("plan_date")?,
            items: row.try_get("items")?,
            completed_at: row.try_get("completed_at")?,
        }))
    } else {
        Ok(None)
    }
}

pub async fn upsert_wellness_plan(
    pool: &PgPool,
    user_id: Uuid,
    plan_date: NaiveDate,
    items: &serde_json::Value,
) -> Result<WellnessPlan> {
    let row = sqlx::query(
        r#"
        INSERT INTO wellness_plans (user_id, plan_date, items)
        VALUES ($1, $2, $3)
        ON CONFLICT (user_id, plan_date) DO UPDATE
        SET items = EXCLUDED.items,
            updated_at = NOW()
        RETURNING id, plan_date, items, completed_at
        "#,
    )
    .bind(user_id)
    .bind(plan_date)
    .bind(items)
    .fetch_one(pool)
    .await?;

    Ok(WellnessPlan {
        id: row.try_get("id")?,
        plan_date: row.try_get("plan_date")?,
        items: row.try_get("items")?,
        completed_at: row.try_get("completed_at")?,
    })
}

pub async fn mark_wellness_plan_completed(
    pool: &PgPool,
    user_id: Uuid,
    plan_date: NaiveDate,
) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE wellness_plans
        SET completed_at = NOW(),
            updated_at = NOW()
        WHERE user_id = $1
          AND plan_date = $2
        "#,
    )
    .bind(user_id)
    .bind(plan_date)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn mark_plan_nudge_sent(
    pool: &PgPool,
    user_id: Uuid,
    local_date: NaiveDate,
) -> Result<bool> {
    let result = sqlx::query(
        r#"
        INSERT INTO user_preferences (user_id, last_plan_nudge_date)
        VALUES ($1, $2)
        ON CONFLICT (user_id) DO UPDATE
        SET last_plan_nudge_date = EXCLUDED.last_plan_nudge_date,
            updated_at = NOW()
        WHERE user_preferences.last_plan_nudge_date IS NULL
           OR user_preferences.last_plan_nudge_date < EXCLUDED.last_plan_nudge_date
        "#,
    )
    .bind(user_id)
    .bind(local_date)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

// ---------- Kudos Insights ----------

pub async fn get_recent_kudos_received(
    pool: &PgPool,
    user_id: Uuid,
    days: i64,
    limit: i64,
) -> Result<Vec<KudosRecord>> {
    let records = sqlx::query_as::<_, KudosRecord>(
        r#"
        SELECT k.id, k.from_user_id, k.to_user_id, k.message, k.created_at,
               u.enc_name as from_user_enc_name
        FROM kudos k
        JOIN users u ON k.from_user_id = u.id
        WHERE k.to_user_id = $1
          AND k.created_at >= NOW() - ($2 || ' days')::INTERVAL
        ORDER BY k.created_at DESC
        LIMIT $3
        "#,
    )
    .bind(user_id)
    .bind(days.to_string())
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(records)
}

pub async fn get_recent_kudos_sent(
    pool: &PgPool,
    user_id: Uuid,
    days: i64,
    limit: i64,
) -> Result<Vec<KudosSentRecord>> {
    let records = sqlx::query_as::<_, KudosSentRecord>(
        r#"
        SELECT k.id, k.from_user_id, k.to_user_id, k.message, k.created_at,
               u.enc_name as to_user_enc_name
        FROM kudos k
        JOIN users u ON k.to_user_id = u.id
        WHERE k.from_user_id = $1
          AND k.created_at >= NOW() - ($2 || ' days')::INTERVAL
        ORDER BY k.created_at DESC
        LIMIT $3
        "#,
    )
    .bind(user_id)
    .bind(days.to_string())
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(records)
}

// ---------- Wall Toxicity Signals ----------

pub async fn insert_wall_toxic_signal(
    pool: &PgPool,
    post_id: Uuid,
    severity: i16,
    flagged: bool,
    themes: &serde_json::Value,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO wall_toxic_signals (post_id, severity, flagged, themes)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (post_id) DO UPDATE
        SET severity = EXCLUDED.severity,
            flagged = EXCLUDED.flagged,
            themes = EXCLUDED.themes,
            created_at = NOW()
        "#,
    )
    .bind(post_id)
    .bind(severity)
    .bind(flagged)
    .bind(themes)
    .execute(pool)
    .await?;
    Ok(())
}

// ---------- Analytics Storage ----------

#[derive(Debug, Clone, FromRow)]
pub struct MonthlyMetricRow {
    pub user_id: Uuid,
    pub period: NaiveDate,
    pub who5: f64,
    pub phq9: f64,
    pub gad7: f64,
    pub mbi: f64,
    pub sleep_duration: f64,
    pub sleep_quality: f64,
    pub work_life_balance: f64,
    pub stress_level: f64,
    pub source: String,
}

#[derive(Debug, Clone)]
pub struct MonthlyMetricInput {
    pub who5: f64,
    pub phq9: f64,
    pub gad7: f64,
    pub mbi: f64,
    pub sleep_duration: f64,
    pub sleep_quality: f64,
    pub work_life_balance: f64,
    pub stress_level: f64,
}

#[derive(Debug, Clone)]
pub struct AnalyticsMetadataRow {
    pub company: String,
    pub data_collection_period: Option<String>,
    pub update_frequency: Option<String>,
    pub next_assessment: Option<NaiveDate>,
    pub participation_rate: Option<String>,
    pub assessment_tools: Vec<String>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct IndustryBenchmarkRow {
    pub sector: String,
    pub who5: Option<f64>,
    pub phq9: Option<f64>,
    pub gad7: Option<f64>,
    pub mbi: Option<f64>,
    pub sleep_duration: Option<f64>,
    pub work_life_balance: Option<f64>,
    pub stress_level: Option<f64>,
}

#[derive(Debug, Clone, FromRow)]
pub struct AnalyticsAlertRow {
    pub severity: String,
    pub employee_name: String,
    pub message: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct AnalyticsRecommendationRow {
    pub category: String,
    pub title: String,
    pub description: String,
    pub affected_employees: Vec<String>,
    pub priority: String,
}

pub async fn get_monthly_metric_overrides(
    pool: &PgPool,
    user_ids: &[Uuid],
) -> Result<Vec<MonthlyMetricRow>> {
    if user_ids.is_empty() {
        return Ok(Vec::new());
    }
    let rows = sqlx::query_as::<_, MonthlyMetricRow>(
        r#"
        SELECT
            user_id,
            period,
            who5,
            phq9,
            gad7,
            mbi,
            sleep_duration,
            sleep_quality,
            work_life_balance,
            stress_level,
            source
        FROM analytics_monthly_metrics
        WHERE user_id = ANY($1)
        ORDER BY period ASC
        "#,
    )
    .bind(user_ids)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn upsert_monthly_metric(
    pool: &PgPool,
    user_id: Uuid,
    period: NaiveDate,
    metrics: &MonthlyMetricInput,
    source: &str,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO analytics_monthly_metrics (
            user_id,
            period,
            who5,
            phq9,
            gad7,
            mbi,
            sleep_duration,
            sleep_quality,
            work_life_balance,
            stress_level,
            source
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        ON CONFLICT (user_id, period) DO UPDATE
        SET who5 = EXCLUDED.who5,
            phq9 = EXCLUDED.phq9,
            gad7 = EXCLUDED.gad7,
            mbi = EXCLUDED.mbi,
            sleep_duration = EXCLUDED.sleep_duration,
            sleep_quality = EXCLUDED.sleep_quality,
            work_life_balance = EXCLUDED.work_life_balance,
            stress_level = EXCLUDED.stress_level,
            source = EXCLUDED.source,
            updated_at = NOW()
        "#,
    )
    .bind(user_id)
    .bind(period)
    .bind(metrics.who5)
    .bind(metrics.phq9)
    .bind(metrics.gad7)
    .bind(metrics.mbi)
    .bind(metrics.sleep_duration)
    .bind(metrics.sleep_quality)
    .bind(metrics.work_life_balance)
    .bind(metrics.stress_level)
    .bind(source)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_analytics_metadata(pool: &PgPool) -> Result<Option<AnalyticsMetadataRow>> {
    let row = sqlx::query(
        r#"
        SELECT
            company,
            data_collection_period,
            update_frequency,
            next_assessment,
            participation_rate,
            assessment_tools,
            updated_at
        FROM analytics_metadata
        ORDER BY updated_at DESC
        LIMIT 1
        "#,
    )
    .fetch_optional(pool)
    .await?;

    let Some(row) = row else {
        return Ok(None);
    };

    let tools_value: Option<serde_json::Value> = row.try_get("assessment_tools")?;
    let assessment_tools = tools_value
        .and_then(|value| serde_json::from_value::<Vec<String>>(value).ok())
        .unwrap_or_default();

    Ok(Some(AnalyticsMetadataRow {
        company: row.try_get("company")?,
        data_collection_period: row.try_get("data_collection_period")?,
        update_frequency: row.try_get("update_frequency")?,
        next_assessment: row.try_get("next_assessment")?,
        participation_rate: row.try_get("participation_rate")?,
        assessment_tools,
        updated_at: row.try_get("updated_at")?,
    }))
}

pub async fn get_industry_benchmarks(pool: &PgPool) -> Result<Vec<IndustryBenchmarkRow>> {
    let rows = sqlx::query_as::<_, IndustryBenchmarkRow>(
        r#"
        SELECT
            sector,
            who5,
            phq9,
            gad7,
            mbi,
            sleep_duration,
            work_life_balance,
            stress_level
        FROM analytics_industry_benchmarks
        ORDER BY sector ASC
        "#,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn get_analytics_alerts(pool: &PgPool) -> Result<Vec<AnalyticsAlertRow>> {
    let rows = sqlx::query_as::<_, AnalyticsAlertRow>(
        r#"
        SELECT severity, employee_name, message, timestamp
        FROM analytics_alerts
        ORDER BY timestamp DESC
        "#,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn get_analytics_recommendations(
    pool: &PgPool,
) -> Result<Vec<AnalyticsRecommendationRow>> {
    let rows = sqlx::query(
        r#"
        SELECT category, title, description, affected_employees, priority
        FROM analytics_recommendations
        ORDER BY created_at DESC
        "#,
    )
    .fetch_all(pool)
    .await?;

    let mut out = Vec::new();
    for row in rows {
        let affected: Option<serde_json::Value> = row.try_get("affected_employees")?;
        let affected_employees = affected
            .and_then(|value| serde_json::from_value::<Vec<String>>(value).ok())
            .unwrap_or_default();
        out.push(AnalyticsRecommendationRow {
            category: row.try_get("category")?,
            title: row.try_get("title")?,
            description: row.try_get("description")?,
            affected_employees,
            priority: row.try_get("priority")?,
        });
    }
    Ok(out)
}
