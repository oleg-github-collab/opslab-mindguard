use crate::db;
use crate::services::ai::AiOutcome;
use crate::state::SharedState;
use anyhow::Result;
use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use chrono::Utc;
use serde_json::json;
use std::env;
use teloxide::net::Download;
use teloxide::prelude::*;
use teloxide::types::{ChatKind, Message, Update};
use sqlx;
use uuid::Uuid;

pub fn routes(state: SharedState) -> Router {
    Router::new()
        .route("/telegram/webhook", post(handle_update))
        .with_state(state)
}

async fn handle_update(
    State(state): State<SharedState>,
    Json(update): Json<Update>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let bot = bot();
    if let teloxide::types::UpdateKind::Message(message) = update.kind {
        match &message.chat.kind {
            ChatKind::Private(_) => {
                handle_private(&bot, state, message)
                    .await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            }
            ChatKind::Public(_) => {
                handle_group(&bot, state, message)
                    .await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            }
            _ => {}
        }
    }

    Ok(Json(json!({"status": "ok"})))
}

fn bot() -> teloxide::Bot {
    let token = env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN missing");
    teloxide::Bot::new(token)
}

async fn handle_private(bot: &teloxide::Bot, state: SharedState, msg: Message) -> Result<()> {
    let telegram_id = msg.chat.id.0;
    let user = db::find_user_by_telegram(&state.pool, telegram_id).await?;
    let Some(user) = user else {
        bot.send_message(msg.chat.id, "Спочатку авторизуйтесь у вебі (email + код), тоді напишіть сюди ще раз.")
            .await?;
        return Ok(());
    };

    if let Some(voice) = msg.voice() {
        let file_id = voice.file.id.clone();
        handle_voice(bot, state, msg, user.id, file_id).await?;
        return Ok(());
    }

    if let Some(text) = msg.text() {
        if text.starts_with("/start") {
            bot.send_message(msg.chat.id, "Щоденний чекін: надішліть /checkin, щоб отримати 2-3 питання + голосове промпт.")
                .await?;
            return Ok(());
        }
        if text.starts_with("/help") || text.contains("тривога") || text.contains("паніка") {
            bot.send_message(
                msg.chat.id,
                "Миттєва підтримка: спробуйте дихання 4-7-8 (4с вдих, 7с затримка, 8с видих, 4 цикли). Потім запишіть коротке голосове, як почуваєтесь.",
            )
            .await?;
            return Ok(());
        }
        if text.starts_with("/checkin") {
            let questions = state
                .poll_engine
                .next_questions(&state.pool, user.id, 3)
                .await
                .unwrap_or_default();
            let formatted = questions
                .iter()
                .map(|q| format!("{}: {}", q.id, q.text))
                .collect::<Vec<_>>()
                .join("\n");
            let prompt = format!(
                "Відповідай у форматі qid:value (0-3) для кожного рядка.\n{formatted}\nТакож запиши голосове: \"Як пройшов день?\"\nШвидка практика: 2 хвилини дихання 4-7-8 перед відповіддю."
            );
            bot.send_message(msg.chat.id, prompt).await?;
            return Ok(());
        }

        if let Some((qid, value)) = parse_answer(text) {
            db::insert_answer(&state.pool, user.id, qid, value).await?;
            bot.send_message(msg.chat.id, "Відповідь збережена ✅").await?;
        } else {
            bot.send_message(msg.chat.id, "Надішліть /checkin або голосове повідомлення.")
                .await?;
        }
    }

    Ok(())
}

async fn handle_voice(
    bot: &teloxide::Bot,
    state: SharedState,
    msg: Message,
    user_id: Uuid,
    file_id: String,
) -> Result<()> {
    let file = bot.get_file(file_id).await?;
    let mut bytes: Vec<u8> = Vec::new();
    bot.download_file(&file.path, &mut bytes).await?;

    let transcript = state.ai.transcribe_voice(bytes).await?;
    let context = recent_context(&state, user_id).await.unwrap_or_default();
    let outcome: AiOutcome = state.ai.analyze_transcript(&transcript, &context).await?;
    db::insert_voice_log(
        &state.pool,
        &state.crypto,
        user_id,
        &outcome.transcript,
        Some(&outcome.ai_json),
        outcome.risk_score,
        outcome.urgent,
    )
    .await?;

    bot.send_message(
        msg.chat.id,
        format!(
            "Дякуємо! Аналіз виконано. Ризик: {}/10. Порада: {}",
            outcome.risk_score,
            outcome
                .ai_json
                .get("advice")
                .and_then(|v| v.as_str())
                .unwrap_or("залишайтесь на зв'язку")
        ),
    )
    .await?;

    if outcome.urgent {
        bot.send_message(
            msg.chat.id,
            "⚠️ Високий ризик: зробіть паузу 5 хв. Практика: 4-7-8 дихання + складіть 3 пункти плану на найближчу годину. Якщо потрібно — напишіть \"паніка\" щоб отримати швидку підтримку.",
        )
        .await?;
        if let Some(admin_id) = env::var("ADMIN_TELEGRAM_ID")
            .ok()
            .and_then(|v| v.parse::<i64>().ok())
        {
            bot.send_message(
                teloxide::types::ChatId(admin_id),
                format!("⚠️ URGENT | User {user_id} flagged risk_score=10"),
            )
            .await?;
        }
    }

    Ok(())
}

async fn handle_group(bot: &teloxide::Bot, state: SharedState, msg: Message) -> Result<()> {
    if let Some(text) = msg.text() {
        let bot_name = env::var("BOT_USERNAME").unwrap_or_default();
        if !bot_name.is_empty() && !text.contains(&bot_name) {
            return Ok(()); // ignore messages without mention
        }
        let reply = state.ai.group_coach_response(text).await.unwrap_or_else(|_| {
            "Дихайте глибоко 4-4-4, зробіть перерву на 2 хвилини та поверніться до задачі.".to_string()
        });
        bot.send_message(msg.chat.id, reply).await?;
    }
    Ok(())
}

fn parse_answer(text: &str) -> Option<(i32, i16)> {
    let parts: Vec<&str> = text.split(':').collect();
    if parts.len() != 2 {
        return None;
    }
    let qid = parts[0].trim().parse::<i32>().ok()?;
    let value = parts[1].trim().parse::<i16>().ok()?;
    Some((qid, value))
}

async fn recent_context(state: &SharedState, user_id: Uuid) -> Result<String> {
    let logs = sqlx::query!(
        r#"
        SELECT enc_transcript, created_at
        FROM voice_logs
        WHERE user_id = $1 AND created_at > now() - interval '3 days'
        ORDER BY created_at DESC
        LIMIT 3
        "#,
        user_id
    )
    .fetch_all(&state.pool)
    .await?;

    let mut parts = Vec::new();
    for log in logs {
        if let Ok(text) = state.crypto.decrypt_str(&log.enc_transcript) {
            parts.push(format!(
                "{}: {}",
                log.created_at.with_timezone(&Utc).date_naive(),
                text
            ));
        }
    }
    Ok(parts.join("\n"))
}
