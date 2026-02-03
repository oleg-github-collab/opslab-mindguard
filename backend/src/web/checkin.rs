use crate::bot::daily_checkin::{CheckInGenerator, Question};
use crate::db;
use crate::domain::checkin::{schedule_for, CheckinFrequency, CheckinSchedule};
use crate::domain::models::UserRole;
use crate::state::{SharedState, WebCheckInSession};
use crate::time_utils;
use crate::web::session::UserSession;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use base64::{engine::general_purpose, Engine as _};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use sqlx::Row;
use uuid::Uuid;

pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/status", get(status))
        .route("/frequency", post(update_frequency))
        .route("/start", post(start))
        .route("/submit", post(submit))
        .with_state(state)
}

#[derive(Serialize)]
struct CheckinStatusResponse {
    enabled: bool,
    frequency: CheckinFrequency,
    frequency_label: String,
    due_today: bool,
    next_due_date: String,
    last_checkin_date: Option<String>,
    days_until: i64,
    question_count_label: String,
    estimated_time: String,
}

#[derive(Serialize)]
struct CheckinQuestionPayload {
    id: i32,
    qtype: String,
    text: String,
    emoji: String,
    scale: String,
}

#[derive(Serialize)]
struct CheckinStartResponse {
    checkin_id: String,
    intro_message: String,
    estimated_time: String,
    frequency: CheckinFrequency,
    questions: Vec<CheckinQuestionPayload>,
}

#[derive(Serialize)]
struct CheckinErrorResponse {
    error: String,
    next_due_date: Option<String>,
    days_until: Option<i64>,
}

#[derive(Serialize)]
struct CheckinSubmitResponse {
    saved_answers: usize,
    saved_open_responses: usize,
    urgent: bool,
    advice: Option<String>,
}

#[derive(Deserialize)]
struct CheckinFrequencyPayload {
    frequency: CheckinFrequency,
}

#[derive(Deserialize)]
struct CheckinStartParams {
    force: Option<String>,
}

#[derive(Deserialize)]
struct CheckinAnswerPayload {
    question_id: i32,
    value: Option<i16>,
    text: Option<String>,
    audio_base64: Option<String>,
    audio_mime: Option<String>,
    audio_duration_seconds: Option<i32>,
}

#[derive(Deserialize)]
struct CheckinSubmitPayload {
    checkin_id: String,
    answers: Vec<CheckinAnswerPayload>,
}

struct PreparedScaleAnswer {
    question_id: i32,
    qtype: String,
    value: i16,
}

struct PreparedOpenResponse {
    question_id: i32,
    qtype: String,
    source: String,
    response_text: String,
    analysis: serde_json::Value,
    risk_score: i16,
    urgent: bool,
    duration: Option<i32>,
}

async fn status(
    UserSession(user_id): UserSession,
    State(state): State<SharedState>,
) -> Result<Json<CheckinStatusResponse>, StatusCode> {
    let (_user, schedule, frequency) = load_schedule(&state, user_id).await?;

    let (frequency_label, question_count_label, estimated_time) =
        frequency_metadata(frequency);

    Ok(Json(CheckinStatusResponse {
        enabled: true,
        frequency,
        frequency_label,
        due_today: schedule.due,
        next_due_date: schedule.next_due_date.to_string(),
        last_checkin_date: schedule.last_date.map(|d| d.to_string()),
        days_until: schedule.days_until,
        question_count_label,
        estimated_time,
    }))
}

async fn update_frequency(
    UserSession(user_id): UserSession,
    State(state): State<SharedState>,
    Json(payload): Json<CheckinFrequencyPayload>,
) -> Result<Json<CheckinStatusResponse>, StatusCode> {
    let _user = db::find_user_by_id(&state.pool, user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    db::set_user_checkin_frequency(&state.pool, user_id, payload.frequency.as_str())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    status(UserSession(user_id), State(state)).await
}

async fn start(
    UserSession(user_id): UserSession,
    State(state): State<SharedState>,
    Query(params): Query<CheckinStartParams>,
) -> Result<Json<CheckinStartResponse>, (StatusCode, Json<CheckinErrorResponse>)> {
    let (user, schedule, frequency) = load_schedule(&state, user_id).await.map_err(|status| {
            (
                status,
                Json(CheckinErrorResponse {
                    error: "not_allowed".to_string(),
                    next_due_date: None,
                    days_until: None,
                }),
            )
        })?;

    let force = params
        .force
        .as_deref()
        .map(|val| matches!(val.trim().to_lowercase().as_str(), "1" | "true" | "yes"))
        .unwrap_or(false);
    let allow_force = matches!(user.role, UserRole::Admin | UserRole::Founder);
    let force = force && allow_force;

    if !schedule.due && !force {
        return Err((
            StatusCode::CONFLICT,
            Json(CheckinErrorResponse {
                error: "not_due".to_string(),
                next_due_date: Some(schedule.next_due_date.to_string()),
                days_until: Some(schedule.days_until),
            }),
        ));
    }

    let checkin = CheckInGenerator::generate_web_checkin(&state.pool, user_id, frequency)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(CheckinErrorResponse {
                    error: "generation_failed".to_string(),
                    next_due_date: None,
                    days_until: None,
                }),
            )
        })?;

    let payload_questions = checkin
        .questions
        .iter()
        .map(question_payload)
        .collect::<Vec<_>>();

    let expires_at = Utc::now() + Duration::hours(2);
    {
        let mut sessions = state.web_checkin_sessions.write().await;
        sessions.insert(
            user_id,
            WebCheckInSession {
                checkin: checkin.clone(),
                created_at: Utc::now(),
                expires_at,
            },
        );
    }

    Ok(Json(CheckinStartResponse {
        checkin_id: checkin.id,
        intro_message: checkin.intro_message,
        estimated_time: checkin.estimated_time,
        frequency,
        questions: payload_questions,
    }))
}

async fn submit(
    UserSession(user_id): UserSession,
    State(state): State<SharedState>,
    Json(payload): Json<CheckinSubmitPayload>,
) -> Result<Json<CheckinSubmitResponse>, StatusCode> {
    let _user = db::find_user_by_id(&state.pool, user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let session = {
        let sessions = state.web_checkin_sessions.read().await;
        sessions.get(&user_id).cloned()
    }
    .ok_or(StatusCode::CONFLICT)?;

    if session.expires_at < Utc::now() {
        return Err(StatusCode::CONFLICT);
    }

    if payload.checkin_id != session.checkin.id {
        return Err(StatusCode::BAD_REQUEST);
    }

    if payload.answers.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let answer_map = payload
        .answers
        .into_iter()
        .map(|answer| (answer.question_id, answer))
        .collect::<HashMap<_, _>>();

    let mut saved_answers = 0usize;
    let mut saved_open = 0usize;
    let mut highest_risk = 0i16;
    let mut advice: Option<String> = None;
    let mut urgent = false;
    let mut prepared_scales: Vec<PreparedScaleAnswer> = Vec::new();
    let mut prepared_open: Vec<PreparedOpenResponse> = Vec::new();

    for question in &session.checkin.questions {
        let Some(answer) = answer_map.get(&question.id) else {
            return Err(StatusCode::BAD_REQUEST);
        };

        if question.scale == "1-10" {
            let value = answer.value.ok_or(StatusCode::BAD_REQUEST)?;
            if !(1..=10).contains(&value) {
                return Err(StatusCode::BAD_REQUEST);
            }
            prepared_scales.push(PreparedScaleAnswer {
                question_id: question.id,
                qtype: question.qtype.clone(),
                value,
            });
            continue;
        }

        let text = answer
            .text
            .as_ref()
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty());
        let audio_b64 = answer
            .audio_base64
            .as_ref()
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty());

        if let Some(text) = text.as_ref() {
            if text.len() > 1500 {
                return Err(StatusCode::PAYLOAD_TOO_LARGE);
            }
        }

        if let Some(duration) = answer.audio_duration_seconds {
            if duration <= 0 || duration > 600 {
                return Err(StatusCode::BAD_REQUEST);
            }
        }

        if text.is_none() && audio_b64.is_none() {
            return Err(StatusCode::BAD_REQUEST);
        }

        let (response_text, source, duration) = if let Some(text) = text {
            (text, "text", None)
        } else {
            let audio_b64 = audio_b64.expect("audio payload missing");
            if !state.ai.is_enabled() {
                return Err(StatusCode::SERVICE_UNAVAILABLE);
            }
            if audio_b64.len() > 8_000_000 {
                return Err(StatusCode::PAYLOAD_TOO_LARGE);
            }
            let bytes = general_purpose::STANDARD
                .decode(audio_b64)
                .map_err(|_| StatusCode::BAD_REQUEST)?;
            let (mime, filename) = audio_mime_and_filename(answer.audio_mime.as_deref());
            let transcript = state
                .ai
                .transcribe_audio(bytes, mime, filename)
                .await
                .map_err(|_| StatusCode::BAD_REQUEST)?;
            (
                transcript,
                "voice",
                answer.audio_duration_seconds,
            )
        };

        if !state.ai.is_enabled() {
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        }

        let outcome = analyze_open_response(&state, user_id, question, &response_text)
            .await
            .map_err(|_| StatusCode::BAD_GATEWAY)?;

        prepared_open.push(PreparedOpenResponse {
            question_id: question.id,
            qtype: question.qtype.clone(),
            source: source.to_string(),
            response_text,
            analysis: outcome.ai_json,
            risk_score: outcome.risk_score,
            urgent: outcome.urgent,
            duration,
        });
    }

    for answer in prepared_scales {
        db::insert_checkin_answer(
            &state.pool,
            user_id,
            answer.question_id,
            &answer.qtype,
            answer.value,
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        saved_answers += 1;
    }

    for response in prepared_open {
        db::insert_checkin_open_response(
            &state.pool,
            &state.crypto,
            user_id,
            &session.checkin.id,
            response.question_id,
            &response.qtype,
            &response.source,
            &response.response_text,
            Some(&response.analysis),
            Some(response.risk_score),
            response.urgent,
            response.duration,
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        if response.source == "voice" {
            if let Err(err) = db::insert_voice_log(
                &state.pool,
                &state.crypto,
                user_id,
                &response.response_text,
                Some(&response.analysis),
                response.risk_score,
                response.urgent,
            )
            .await
            {
                tracing::warn!("Failed to store voice log for web check-in: {}", err);
            }
        }

        saved_open += 1;
        if response.risk_score > highest_risk {
            highest_risk = response.risk_score;
            advice = response
                .analysis
                .get("advice")
                .and_then(|v| v.as_str())
                .map(|val| val.to_string());
        }
        if response.urgent {
            urgent = true;
        }
    }

    {
        let mut sessions = state.web_checkin_sessions.write().await;
        sessions.remove(&user_id);
    }

    if urgent {
        if let Err(err) =
            crate::bot::enhanced_handlers::send_open_response_alert(&state, user_id, highest_risk)
                .await
        {
            tracing::warn!("Failed to send urgent alert for web check-in: {}", err);
        }
    }

    if let Err(err) = crate::bot::enhanced_handlers::send_web_checkin_followups(&state, user_id).await {
        tracing::warn!("Failed to send web check-in followups: {}", err);
    }

    Ok(Json(CheckinSubmitResponse {
        saved_answers,
        saved_open_responses: saved_open,
        urgent,
        advice,
    }))
}

fn question_payload(question: &Question) -> CheckinQuestionPayload {
    CheckinQuestionPayload {
        id: question.id,
        qtype: question.qtype.clone(),
        text: question.text.clone(),
        emoji: question.emoji.clone(),
        scale: question.scale.clone(),
    }
}

async fn load_schedule(
    state: &SharedState,
    user_id: Uuid,
) -> Result<(db::DbUser, CheckinSchedule, CheckinFrequency), StatusCode> {
    let user = db::find_user_by_id(&state.pool, user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let prefs = db::get_user_preferences(&state.pool, user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let frequency = CheckinFrequency::try_from(prefs.checkin_frequency.as_str())
        .unwrap_or(CheckinFrequency::Daily);

    let last_checkin_at = db::get_last_checkin_date(&state.pool, user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let today = time_utils::local_components(&prefs.timezone, Utc::now()).0;
    let last_local = last_checkin_at
        .map(|dt| time_utils::local_components(&prefs.timezone, dt).0);

    let schedule = schedule_for(frequency, last_local, today);
    Ok((user, schedule, frequency))
}

fn frequency_metadata(
    frequency: CheckinFrequency,
) -> (String, String, String) {
    match frequency {
        CheckinFrequency::Daily => (
            "Щодня".to_string(),
            "2-3".to_string(),
            "2-3 хвилини".to_string(),
        ),
        CheckinFrequency::Every3Days => (
            "Кожні 3 дні".to_string(),
            "10".to_string(),
            "6-8 хвилин".to_string(),
        ),
        CheckinFrequency::Weekly => (
            "Щотижня".to_string(),
            "12".to_string(),
            "10-12 хвилин".to_string(),
        ),
    }
}

fn audio_mime_and_filename(raw_mime: Option<&str>) -> (&'static str, &'static str) {
    let lower = raw_mime.unwrap_or("").to_lowercase();
    if lower.contains("webm") {
        ("audio/webm", "voice.webm")
    } else if lower.contains("mp4") || lower.contains("m4a") {
        ("audio/mp4", "voice.m4a")
    } else if lower.contains("aac") {
        ("audio/aac", "voice.aac")
    } else if lower.contains("wav") {
        ("audio/wav", "voice.wav")
    } else if lower.contains("mpeg") || lower.contains("mp3") {
        ("audio/mpeg", "voice.mp3")
    } else {
        ("audio/ogg", "voice.ogg")
    }
}

async fn analyze_open_response(
    state: &SharedState,
    user_id: uuid::Uuid,
    question: &Question,
    response_text: &str,
) -> Result<crate::services::ai::AiOutcome, anyhow::Error> {
    if !state.ai.is_enabled() {
        return Err(anyhow::anyhow!("AI disabled"));
    }

    let mut context = recent_context(state, user_id).await.unwrap_or_default();
    if !context.is_empty() {
        context.push('\n');
    }
    context.push_str(&format!("Question: {} ({})", question.text, question.qtype));

    let metrics = db::calculate_user_metrics(&state.pool, user_id)
        .await
        .ok()
        .flatten();

    state
        .ai
        .analyze_transcript(response_text, &context, metrics.as_ref())
        .await
}

async fn recent_context(state: &SharedState, user_id: uuid::Uuid) -> Result<String, anyhow::Error> {
    let rows = sqlx::query(
        r#"
        SELECT enc_transcript, created_at
        FROM voice_logs
        WHERE user_id = $1 AND created_at > now() - interval '3 days'
        ORDER BY created_at DESC
        LIMIT 3
        "#,
    )
    .bind(user_id)
    .fetch_all(&state.pool)
    .await?;

    let mut parts = Vec::new();
    for row in rows {
        let enc_transcript: String = row.try_get("enc_transcript")?;
        let created_at: chrono::DateTime<chrono::Utc> = row.try_get("created_at")?;
        if let Ok(text) = state.crypto.decrypt_str(&enc_transcript) {
            parts.push(format!(
                "{}: {}",
                created_at.with_timezone(&Utc).date_naive(),
                text
            ));
        }
    }
    Ok(parts.join("\n"))
}
