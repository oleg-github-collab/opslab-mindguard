///! Розширені handlers для Telegram бота з щоденними чекінами
use crate::bot::daily_checkin::{CheckInGenerator, MetricsCalculator, CheckInAnswer, Metrics};
use crate::db;
use crate::services::ai::AiOutcome;
use crate::state::SharedState;
use anyhow::Result;
use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use chrono::{Datelike, Utc};
use serde_json::json;
use std::env;
use teloxide::net::Download;
use teloxide::prelude::*;
use teloxide::types::{ChatKind, InlineKeyboardButton, InlineKeyboardMarkup, Message, Update, ParseMode};
use sqlx;
use uuid::Uuid;

// ========== WOW Features Helper Functions ==========

/// #5 Quick Actions after check-in
async fn send_quick_actions(
    bot: &teloxide::Bot,
    state: &SharedState,
    chat_id: ChatId,
    user_id: Uuid,
) -> Result<()> {
    // Отримати метрики користувача
    let metrics = match db::calculate_user_metrics(&state.pool, user_id).await? {
        Some(m) => m,
        None => return Ok(()), // Недостатньо даних
    };

    let mut actions = Vec::new();

    // Аналізувати metrics і пропонувати дії
    if metrics.stress_level >= 28.0 {
        // ~7/10 stress
        actions.push(("🎵 Meditation 5 min", "action_meditation"));
        actions.push(("🚶 Прогулянка 10 хв", "action_walk"));
    }

    if metrics.who5_score < 60.0 {
        actions.push(("📝 Написати на Wall", "action_wall_post"));
        actions.push(("💬 Поговорити з кимось", "action_talk"));
    }

    if metrics.sleep_quality() < 6.0 {
        actions.push(("😴 Поради для сну", "action_sleep_tips"));
    }

    if metrics.burnout_percentage() > 60.0 {
        actions.push(("🌴 Планувати відпочинок", "action_vacation"));
    }

    // Якщо немає специфічних рекомендацій
    if actions.is_empty() {
        actions.push(("📊 Подивитись статистику", "action_status"));
        return Ok(());
    }

    // Створити inline keyboard
    let mut rows = Vec::new();
    for (text, callback_data) in actions {
        rows.push(vec![InlineKeyboardButton::callback(text, callback_data)]);
    }

    let keyboard = InlineKeyboardMarkup::new(rows);

    bot.send_message(
        chat_id,
        "💡 *На основі твоїх відповідей:*\n\nРекомендовані дії:",
    )
    .parse_mode(ParseMode::Markdown)
    .reply_markup(keyboard)
    .await?;

    Ok(())
}

/// Handle action callbacks (#5 Quick Actions)
async fn handle_action_callback(
    bot: &teloxide::Bot,
    callback: &teloxide::types::CallbackQuery,
    action: &str,
) -> Result<()> {
    let msg = callback.message.as_ref().unwrap();

    match action {
        "meditation" => {
            bot.send_message(
                msg.chat.id,
                "🎵 *Meditation 5 min*\n\n\
                1. Знайди тихе місце\n\
                2. Заплющ очі\n\
                3. Дихай 4-7-8:\n\
                   • 4 сек вдих\n\
                   • 7 сек затримка\n\
                   • 8 сек видих\n\
                4. Повтори 5 циклів\n\n\
                _Це допоможе знизити стрес і заспокоїтись_ 🧘",
            )
            .parse_mode(ParseMode::Markdown)
            .await?;
        }
        "walk" => {
            bot.send_message(
                msg.chat.id,
                "🚶 *10-хвилинна прогулянка*\n\n\
                ✅ Покращує настрій на 20%\n\
                ✅ Знижує stress\n\
                ✅ Очищує голову\n\n\
                Встав і йди ЗАРАЗ! Я нагадаю через 10 хв ⏰",
            )
            .parse_mode(ParseMode::Markdown)
            .await?;
        }
        "wall_post" => {
            bot.send_message(
                msg.chat.id,
                "📝 *Стіна плачу*\n\n\
                Поділись своїми думками анонімно:\n\
                https://mindguard.opslab.uk/wall\n\n\
                Написати голосовим сюди - також працює!",
            )
            .parse_mode(ParseMode::Markdown)
            .await?;
        }
        "talk" => {
            bot.send_message(
                msg.chat.id,
                "💬 *Поговорити з кимось*\n\n\
                Іноді розмова - найкраще рішення.\n\n\
                Кому написати:\n\
                • Твоєму керівнику\n\
                • HR/Jane\n\
                • Колезі, якому довіряєш\n\n\
                Твоє здоров'я важливіше за все! 💚",
            )
            .parse_mode(ParseMode::Markdown)
            .await?;
        }
        "sleep_tips" => {
            bot.send_message(
                msg.chat.id,
                "😴 *Поради для якісного сну:*\n\n\
                1. Лягай в один час (10-11 PM)\n\
                2. Вимкни екрани за 1 годину\n\
                3. Температура 18-20°C\n\
                4. Темрява повна\n\
                5. Без кави після 14:00\n\
                6. Легка вечеря за 2-3 години\n\n\
                💡 Спробуй сьогодні!",
            )
            .parse_mode(ParseMode::Markdown)
            .await?;
        }
        "vacation" => {
            bot.send_message(
                msg.chat.id,
                "🌴 *Час відпочити!*\n\n\
                Твої показники вказують на burnout.\n\n\
                Рекомендації:\n\
                • Візьми 2-3 дні off\n\
                • Повністю відключись від роботи\n\
                • Займи улюбленою справою\n\n\
                Поговори з Jane про відпустку! 💙",
            )
            .parse_mode(ParseMode::Markdown)
            .await?;
        }
        "status" => {
            bot.send_message(
                msg.chat.id,
                "Використай команду /status щоб побачити детальну статистику! 📊",
            )
            .await?;
        }
        _ => {}
    }

    bot.answer_callback_query(&callback.id)
        .text("✅ Готово!")
        .await?;

    Ok(())
}

/// #4 Mood-Based Emoji Reactions
fn get_emoji_reaction(qtype: &str, value: i16) -> String {
    match qtype {
        "mood" => match value {
            9..=10 => "🎉 Чудово! Такий настрій - рідкість, насолоджуйся!",
            7..=8 => "😊 Супер! Продовжуй в тому ж дусі!",
            5..=6 => "😌 Норм, стабільно",
            3..=4 => "💙 Розумію, важкий день. Це тимчасово",
            1..=2 => "🤗 Тримайся, ти не один. Поговори з кимось якщо потрібно",
            _ => "✅ Дякую",
        },
        "energy" => match value {
            9..=10 => "⚡ Wow! Де береш таку енергію?",
            7..=8 => "💪 Чудовий рівень!",
            5..=6 => "🔋 Норм, але можна краще",
            3..=4 => "😴 Трохи втомився? Кава допоможе!",
            1..=2 => "😓 Дуже низько... Може відпочинок?",
            _ => "✅ Дякую",
        },
        "stress" => match value {
            9..=10 => "🚨 Дуже високо! Зроби паузу ЗАРАЗ. Дихай 4-7-8",
            7..=8 => "😰 Багато стресу. Прогулянка 10 хв?",
            5..=6 => "😐 Помірно, контролюй",
            3..=4 => "😌 Непогано, так тримати",
            1..=2 => "🧘 Чудово! Майже zen",
            _ => "✅ Дякую",
        },
        "sleep" => match value {
            9..=10 => "😴 Ідеальний сон! 8+ годин?",
            7..=8 => "💤 Добре виспався!",
            5..=6 => "🌙 Норм, але краще раніше лягати",
            3..=4 => "⏰ Мало спав... Сьогодні раніше спати!",
            1..=2 => "🚨 Критично! Sleep debt накопичується",
            _ => "✅ Дякую",
        },
        "workload" => match value {
            9..=10 => "😱 Занадто багато! Делегуй завдання",
            7..=8 => "📊 Високе навантаження, стеж за burnout",
            5..=6 => "⚖️ Збалансовано",
            3..=4 => "✅ Комфортний рівень",
            1..=2 => "🌴 Спокійно зараз, чудово!",
            _ => "✅ Дякую",
        },
        "focus" | "concentration" => match value {
            9..=10 => "🎯 Лазерний фокус!",
            7..=8 => "🧠 Добра концентрація",
            5..=6 => "😐 Норм, але є відволікання",
            3..=4 => "📱 Важко зосередитись? Вимкни сповіщення",
            1..=2 => "💭 Дуже розсіяно... Meditation 5 min?",
            _ => "✅ Дякую",
        },
        "motivation" => match value {
            9..=10 => "🚀 Супер мотивація! Вперед!",
            7..=8 => "💡 Гарний настрій до праці",
            5..=6 => "😐 Нейтрально",
            3..=4 => "😔 Низька мотивація... Відпочинок?",
            1..=2 => "💤 Burnout ознаки? Поговори з кимось",
            _ => "✅ Дякую",
        },
        "wellbeing" | "anxiety" => match value {
            9..=10 => "✨ Чудове самопочуття!",
            7..=8 => "😊 Добре себе почуваєш",
            5..=6 => "😌 Норм стан",
            3..=4 => "💙 Підтримка потрібна?",
            1..=2 => "🤗 Важко зараз... Ти не один",
            _ => "✅ Дякую",
        },
        _ => "✅ Відповідь збережена",
    }
    .to_string()
}

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

    match update.kind {
        teloxide::types::UpdateKind::Message(message) => {
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
        teloxide::types::UpdateKind::CallbackQuery(callback) => {
            handle_callback(&bot, state, callback)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        }
        _ => {}
    }

    Ok(Json(json!({"status": "ok"})))
}

fn bot() -> teloxide::Bot {
    let token = env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN missing");
    teloxide::Bot::new(token)
}

async fn handle_private(bot: &teloxide::Bot, state: SharedState, msg: Message) -> Result<()> {
    let telegram_id = msg.chat.id.0;

    // Handle /start with PIN code
    if let Some(text) = msg.text() {
        if text.starts_with("/start ") {
            let parts: Vec<&str> = text.split_whitespace().collect();
            if parts.len() == 2 {
                let pin = parts[1];
                return handle_pin_verification(bot, &state, msg.chat.id, telegram_id, pin).await;
            }
        }
    }

    let user = db::find_user_by_telegram(&state.pool, telegram_id).await?;
    let Some(user) = user else {
        bot.send_message(
            msg.chat.id,
            "👋 *Привіт! Ласкаво просимо до OpsLab Mindguard!*\n\n\
            Для початку роботи:\n\
            1️⃣ Увійдіть на платформу: https://mindguard.opslab.uk\n\
            2️⃣ Отримайте PIN-код на dashboard\n\
            3️⃣ Напишіть сюди: `/start ВАШ-PIN`\n\n\
            _Приклад: /start 1234_",
        )
        .parse_mode(teloxide::types::ParseMode::Markdown)
        .await?;
        return Ok(());
    };

    // Handle voice messages
    if let Some(voice) = msg.voice() {
        let file_id = voice.file.id.clone();
        handle_voice(bot, state, msg, user.id, file_id).await?;
        return Ok(());
    }

    // Handle text commands
    if let Some(text) = msg.text() {
        if text.starts_with("/start") {
            send_start_message(bot, msg.chat.id).await?;
            return Ok(());
        }

        if text.starts_with("/checkin") {
            start_daily_checkin(bot, &state, msg.chat.id, user.id).await?;
            return Ok(());
        }

        if text.starts_with("/status") {
            send_user_status(bot, &state, msg.chat.id, user.id).await?;
            return Ok(());
        }

        if text.starts_with("/wall") {
            send_wall_info(bot, msg.chat.id).await?;
            return Ok(());
        }

        // #2 WOW Feature: Smart Reminders
        if text.starts_with("/settime") {
            let args = text.trim_start_matches("/settime").trim();
            handle_settime_command(bot, &state, msg.chat.id, user.id, args).await?;
            return Ok(());
        }

        // #17 WOW Feature: Kudos System
        if text.starts_with("/kudos") {
            let args = text.trim_start_matches("/kudos").trim();
            handle_kudos_command(bot, &state, msg.chat.id, user.id, args).await?;
            return Ok(());
        }

        if text.starts_with("/help") || text.contains("тривога") || text.contains("паніка") {
            bot.send_message(
                msg.chat.id,
                "💆 *Миттєва підтримка*\n\n\
                Спробуйте дихання 4-7-8:\n\
                • 4 секунди вдих\n\
                • 7 секунд затримка\n\
                • 8 секунд видих\n\
                • Повторити 4 цикли\n\n\
                Потім запишіть коротке голосове про те, як почуваєтесь.\n\n\
                Якщо потрібна термінова допомога - зверніться до психолога або вашого керівника.",
            )
            .parse_mode(teloxide::types::ParseMode::Markdown)
            .await?;
            return Ok(());
        }

        // Fallback
        bot.send_message(
            msg.chat.id,
            "📱 *Команди бота:*\n\n\
            /checkin - Щоденний чекін (2-3 хв)\n\
            /status - Ваш поточний стан\n\
            /wall - Стіна плачу\n\
            /settime - Встановити час чекіну ⏰\n\
            /kudos - Подякувати колезі 🎉\n\
            /help - Допомога",
        )
        .parse_mode(ParseMode::Markdown)
        .await?;
    }

    Ok(())
}

/// Обробка PIN-коду для зв'язування Telegram
async fn handle_pin_verification(
    bot: &teloxide::Bot,
    state: &SharedState,
    chat_id: ChatId,
    telegram_id: i64,
    pin: &str,
) -> Result<()> {
    // Verify PIN and link Telegram ID
    match db::verify_and_link_telegram(&state.pool, pin, telegram_id).await {
        Ok(Some(user_id)) => {
            // Success! Telegram linked
            let user = db::find_user_by_id(&state.pool, user_id).await?;
            let name = if let Some(user) = user {
                state.crypto.decrypt_str(&user.enc_name).unwrap_or("користувач".to_string())
            } else {
                "користувач".to_string()
            };

            bot.send_message(
                chat_id,
                format!(
                    "✅ *Вітаємо, {}!*\n\n\
                    Telegram успішно підключено до вашого акаунту!\n\n\
                    🎉 Тепер ви будете отримувати:\n\
                    • Щоденні чекіни о 10:00 AM\n\
                    • Критичні сповіщення\n\
                    • Можливість відправляти голосові для AI аналізу\n\n\
                    *Доступні команди:*\n\
                    /checkin - Пройти чекін зараз\n\
                    /status - Переглянути свої метрики\n\
                    /wall - Стіна плачу\n\
                    /help - Допомога\n\n\
                    Побачимось завтра о 10:00! 👋",
                    name
                ),
            )
            .parse_mode(teloxide::types::ParseMode::Markdown)
            .await?;
        }
        Ok(None) => {
            // Invalid or expired PIN
            bot.send_message(
                chat_id,
                "❌ *Невірний або прострочений PIN-код*\n\n\
                PIN-код дійсний тільки 5 хвилин.\n\n\
                Будь ласка:\n\
                1️⃣ Увійдіть на платформу знову\n\
                2️⃣ Згенеруйте новий PIN-код\n\
                3️⃣ Напишіть: `/start НОВИЙ-PIN`",
            )
            .parse_mode(teloxide::types::ParseMode::Markdown)
            .await?;
        }
        Err(e) => {
            tracing::error!("Error verifying PIN: {}", e);
            bot.send_message(
                chat_id,
                "⚠️ Виникла помилка при підключенні.\n\
                Спробуйте ще раз або зверніться до адміністратора.",
            )
            .await?;
        }
    }

    Ok(())
}

/// Відправка привітального повідомлення
async fn send_start_message(bot: &teloxide::Bot, chat_id: ChatId) -> Result<()> {
    bot.send_message(
        chat_id,
        "👋 *Привіт! Я OpsLab Mindguard Bot*\n\n\
        Допомагаю відстежувати твоє ментальне здоров'я:\n\n\
        🔹 *Щоденні чекіни* (2-3 хв) - автоматична розсилка о 10:00\n\
        🔹 *Голосова підтримка* - запиши голосове і отримай аналіз\n\
        🔹 *Стіна плачу* - анонімний зворотній зв'язок\n\
        🔹 *Критичні алерти* - сповіщення для адмінів\n\n\
        *Команди:*\n\
        /checkin - Пройти чекін зараз\n\
        /status - Мій поточний стан\n\
        /wall - Стіна плачу\n\
        /help - Допомога\n\n\
        _Щоденні чекіни надсилаються автоматично о 10:00_",
    )
    .parse_mode(teloxide::types::ParseMode::Markdown)
    .await?;
    Ok(())
}

/// Початок щоденного чекіну
pub async fn start_daily_checkin(
    bot: &teloxide::Bot,
    state: &SharedState,
    chat_id: ChatId,
    user_id: Uuid,
) -> Result<()> {
    // #1 WOW Feature: Use adaptive check-in generation
    let checkin = match CheckInGenerator::generate_adaptive_checkin(&state.pool, user_id).await {
        Ok(c) => c,
        Err(_) => {
            // Fallback to standard if adaptive fails
            let day_of_week = Utc::now().weekday().num_days_from_monday();
            CheckInGenerator::generate_checkin(user_id, day_of_week)
        }
    };

    // Зберегти чекін в сесії
    {
        let mut sessions = state.checkin_sessions.write().await;
        sessions.insert(chat_id.0, checkin.clone());
    }

    // Відправка привітання
    bot.send_message(
        chat_id,
        format!(
            "📋 *Щоденний чекін*\n\n{}\n\n⏱️ Займе {}",
            checkin.intro_message, checkin.estimated_time
        ),
    )
    .parse_mode(teloxide::types::ParseMode::Markdown)
    .await?;

    // Відправка першого питання
    send_checkin_question(bot, chat_id, &checkin, 0).await?;

    Ok(())
}

/// Відправка питання чекіну
async fn send_checkin_question(
    bot: &teloxide::Bot,
    chat_id: ChatId,
    checkin: &crate::bot::daily_checkin::CheckIn,
    question_index: usize,
) -> Result<()> {
    if question_index >= checkin.questions.len() {
        return Ok(());
    }

    let question = &checkin.questions[question_index];

    // Створення inline клавіатури з кнопками 1-10
    let mut rows = vec![];

    // Перший ряд: 1-5
    let row1: Vec<InlineKeyboardButton> = (1..=5)
        .map(|i| {
            InlineKeyboardButton::callback(
                i.to_string(),
                format!("ans_{}_{}", question.id, i),
            )
        })
        .collect();
    rows.push(row1);

    // Другий ряд: 6-10
    let row2: Vec<InlineKeyboardButton> = (6..=10)
        .map(|i| {
            InlineKeyboardButton::callback(
                i.to_string(),
                format!("ans_{}_{}", question.id, i),
            )
        })
        .collect();
    rows.push(row2);

    // Третій ряд: пропустити
    rows.push(vec![InlineKeyboardButton::callback(
        "⏭️ Пропустити",
        "skip_checkin".to_string(),
    )]);

    let keyboard = InlineKeyboardMarkup::new(rows);

    bot.send_message(
        chat_id,
        format!(
            "{} *Питання {}/{}*\n\n{}\n\n_Оцініть від 1 до 10_",
            question.emoji,
            question_index + 1,
            checkin.questions.len(),
            question.text
        ),
    )
    .parse_mode(teloxide::types::ParseMode::Markdown)
    .reply_markup(keyboard)
    .await?;

    Ok(())
}

/// Обробка callback queries (відповіді на кнопки)
async fn handle_callback(
    bot: &teloxide::Bot,
    state: SharedState,
    callback: teloxide::types::CallbackQuery,
) -> Result<()> {
    let Some(ref data) = callback.data else {
        return Ok(());
    };

    if data.starts_with("ans_") {
        // Формат: ans_{question_id}_{value}
        let parts: Vec<&str> = data.split('_').collect();
        if parts.len() == 3 {
            let question_id: i32 = parts[1].parse().unwrap_or(0);
            let value: i16 = parts[2].parse().unwrap_or(0);

            if let Some(msg) = &callback.message {
                let telegram_id = msg.chat.id.0;

                // Отримати чекін з сесії
                let checkin = {
                    let sessions = state.checkin_sessions.read().await;
                    sessions.get(&telegram_id).cloned()
                };

                let Some(checkin) = checkin else {
                    bot.answer_callback_query(&callback.id)
                        .text("❌ Сесія чекіну завершена. Натисни /checkin щоб почати знову")
                        .await?;
                    return Ok(());
                };

                if let Ok(Some(user)) = db::find_user_by_telegram(&state.pool, telegram_id).await {
                    // Знайти питання за ID в поточному чекіні
                    if let Some(question) = checkin.questions.iter().find(|q| q.id == question_id) {
                        // Зберегти відповідь в БД
                        db::insert_checkin_answer(
                            &state.pool,
                            user.id,
                            question_id,
                            &question.qtype,
                            value
                        ).await?;

                        // #4 WOW Feature: Emoji reactions based on mood
                        let reaction = get_emoji_reaction(&question.qtype, value);

                        bot.answer_callback_query(&callback.id)
                            .text(reaction)
                            .await?;

                        // Видалити попереднє повідомлення
                        bot.delete_message(msg.chat.id, msg.id).await.ok();

                        // Знайти індекс поточного питання
                        let current_index = checkin.questions.iter()
                            .position(|q| q.id == question_id)
                            .unwrap_or(0);
                        let next_index = current_index + 1;

                        if next_index < checkin.questions.len() {
                            // Відправити наступне питання
                            send_checkin_question(bot, msg.chat.id, &checkin, next_index).await?;
                        } else {
                            // Чекін завершено - видалити з сесії
                            {
                                let mut sessions = state.checkin_sessions.write().await;
                                sessions.remove(&telegram_id);
                            }

                            bot.send_message(
                                msg.chat.id,
                                "✅ *Чекін завершено! Дякую!* 🙏\n\n\
                                Твої дані збережені та будуть використані для аналізу.\n\
                                Продовжуй проходити щоденні чекіни для повної картини.\n\n\
                                Побачимось завтра! 👋"
                            )
                            .parse_mode(teloxide::types::ParseMode::Markdown)
                            .await?;

                            // #5 WOW Feature: Quick Actions after check-in
                            send_quick_actions(bot, &state, msg.chat.id, user.id).await.ok();

                            // Перевірити чи потрібно надіслати критичний алерт
                            let count = db::get_checkin_answer_count(&state.pool, user.id, 10).await?;
                            if count >= 21 {
                                if let Ok(Some(metrics)) = db::calculate_user_metrics(&state.pool, user.id).await {
                                    if MetricsCalculator::is_critical(&metrics) {
                                        send_critical_alert(bot, &state, user.id, &metrics).await?;

                                        // Сповістити користувача
                                        bot.send_message(
                                            msg.chat.id,
                                            "⚠️ *Важливе повідомлення*\n\n\
                                            Твої показники вказують на необхідність звернення до фахівця.\n\n\
                                            Рекомендуємо:\n\
                                            • Поговорити з керівником\n\
                                            • Звернутися до психолога\n\
                                            • Взяти відпочинок\n\n\
                                            Твоє здоров'я - найважливіше! 💚"
                                        )
                                        .parse_mode(teloxide::types::ParseMode::Markdown)
                                        .await?;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    } else if data == "skip_checkin" {
        bot.answer_callback_query(&callback.id)
            .text("Чекін пропущено")
            .await?;

        if let Some(msg) = callback.message {
            bot.delete_message(msg.chat.id, msg.id).await.ok();
            bot.send_message(
                msg.chat.id,
                "⏭️ Чекін пропущено.\n\n\
                Пам'ятай, що регулярні чекіни допомагають краще розуміти твій стан.\n\
                Завтра спробуй пройти повністю! 💪",
            )
            .await?;
        }
    } else if data.starts_with("action_") {
        // #5 WOW Feature: Quick Actions callbacks
        let action = data.strip_prefix("action_").unwrap_or("");
        handle_action_callback(bot, &callback, action).await?;
    }

    Ok(())
}

/// Відправка статусу користувача
async fn send_user_status(
    bot: &teloxide::Bot,
    state: &SharedState,
    chat_id: ChatId,
    user_id: Uuid,
) -> Result<()> {
    // Отримати відповіді з БД за останні 10 днів
    let answers = db::get_recent_checkin_answers(&state.pool, user_id, 10).await?;
    let answer_count = answers.len();

    if answers.is_empty() {
        bot.send_message(
            chat_id,
            "📊 У тебе ще немає даних.\n\
            Пройди кілька щоденних чекінів для отримання статистики!",
        )
        .await?;
        return Ok(());
    }

    // Спробувати розрахувати метрики через БД функцію
    let metrics = db::calculate_user_metrics(&state.pool, user_id).await?;

    let Some(metrics) = metrics else {
        bot.send_message(
            chat_id,
            format!(
                "📊 *Твій статус*\n\n\
                Чекінів пройдено: {}\n\
                Потрібно мінімум 7 днів (21 відповідь) для повної картини.\n\n\
                Продовжуй проходити щоденні чекіни! 💪",
                answer_count
            ),
        )
        .parse_mode(teloxide::types::ParseMode::Markdown)
        .await?;
        return Ok(());
    };

    let risk = MetricsCalculator::risk_level(&metrics);
    let risk_emoji = match risk {
        "critical" => "🔴",
        "high" => "🟡",
        "medium" => "🟠",
        _ => "🟢",
    };

    bot.send_message(
        chat_id,
        format!(
            "📊 *Твій статус за останній тиждень*\n\n\
            {} Рівень ризику: *{}*\n\n\
            🌟 Благополуччя (WHO-5): {}/100\n\
            😔 Депресія (PHQ-9): {}/27\n\
            😰 Тривожність (GAD-7): {}/21\n\
            🔥 Вигорання (MBI): {:.1}%\n\n\
            😴 Сон: {:.1}h (якість {:.1}/10)\n\
            ⚖️ Work-Life Balance: {:.1}/10\n\
            ⚠️ Рівень стресу: {:.1}/40\n\n\
            _Дані за {} відповідей_",
            risk_emoji,
            risk,
            metrics.who5_score,
            metrics.phq9_score,
            metrics.gad7_score,
            metrics.mbi_score,
            metrics.sleep_duration,
            metrics.sleep_quality(),
            metrics.work_life_balance,
            metrics.stress_level,
            answer_count
        ),
    )
    .parse_mode(teloxide::types::ParseMode::Markdown)
    .await?;

    // Якщо критичні показники - надіслати алерт
    if MetricsCalculator::is_critical(&metrics) {
        send_critical_alert(bot, state, user_id, &metrics).await?;
    }

    Ok(())
}

/// Відправка інформації про Стіну плачу
async fn send_wall_info(bot: &teloxide::Bot, chat_id: ChatId) -> Result<()> {
    bot.send_message(
        chat_id,
        "📝 *Стіна плачу*\n\n\
        Місце для анонімного зворотного зв'язку.\n\
        Поділися своїми думками, ідеями або переживаннями.\n\n\
        Всі пости анонімні та конфіденційні.\n\n\
        🔗 https://mindguard.opslab.uk/wall",
    )
    .parse_mode(teloxide::types::ParseMode::Markdown)
    .await?;
    Ok(())
}

/// Відправка критичного алерту адмінам
async fn send_critical_alert(
    bot: &teloxide::Bot,
    state: &SharedState,
    user_id: Uuid,
    metrics: &crate::bot::daily_checkin::Metrics,
) -> Result<()> {
    let admin_id = env::var("ADMIN_TELEGRAM_ID")
        .ok()
        .and_then(|v| v.parse::<i64>().ok());
    let jane_id = env::var("JANE_TELEGRAM_ID")
        .ok()
        .and_then(|v| v.parse::<i64>().ok());

    let alert_message = format!(
        "🚨 *КРИТИЧНИЙ АЛЕРТ!*\n\n\
        Користувач: {}\n\n\
        📊 *Критичні показники:*\n\
        • WHO-5 (благополуччя): {}/100\n\
        • PHQ-9 (депресія): {}/27\n\
        • GAD-7 (тривожність): {}/21\n\
        • MBI (вигорання): {:.1}%\n\
        • Стрес: {:.1}/40\n\n\
        ⚠️ *ТЕРМІНОВА ДІЯ НЕОБХІДНА!*\n\n\
        Рекомендації:\n\
        1. Негайна консультація з психологом\n\
        2. Зменшення робочого навантаження\n\
        3. 1-на-1 зустріч протягом 24 годин",
        user_id,
        metrics.who5_score,
        metrics.phq9_score,
        metrics.gad7_score,
        metrics.mbi_score,
        metrics.stress_level
    );

    // Відправка Олегу (admin)
    if let Some(admin) = admin_id {
        bot.send_message(ChatId(admin), &alert_message)
            .parse_mode(teloxide::types::ParseMode::Markdown)
            .await
            .ok();
    }

    // Відправка Джейн (manager)
    if let Some(jane) = jane_id {
        bot.send_message(ChatId(jane), &alert_message)
            .parse_mode(teloxide::types::ParseMode::Markdown)
            .await
            .ok();
    }

    Ok(())
}

// Існуючі функції (голосові, група) залишаються без змін
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
                ChatId(admin_id),
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

        // Проста логіка відповідей
        let response = if text.contains("стрес") || text.contains("тривога") {
            "💆 *Поради при стресі:*\n\n\
            1. Зроби глибокий вдих (4-7-8)\n\
            2. Вийди на прогулянку\n\
            3. Поговори з колегою\n\
            4. Зроби перерву\n\n\
            Пам'ятай: /checkin для відстеження стану".to_string()
        } else if text.contains("втома") || text.contains("вигорання") {
            "🔥 *При вигоранні:*\n\n\
            1. Візьми відпочинок\n\
            2. Встанови межі\n\
            3. Делегуй задачі\n\
            4. Поговори з HR\n\n\
            Твоє здоров'я важливіше!".to_string()
        } else {
            // AI відповідь
            state
                .ai
                .group_coach_response(text)
                .await
                .unwrap_or_else(|_| {
                    "Дихайте глибоко 4-4-4, зробіть перерву на 2 хвилини та поверніться до задачі.".to_string()
                })
        };

        bot.send_message(msg.chat.id, response)
            .parse_mode(teloxide::types::ParseMode::Markdown)
            .await?;
    }
    Ok(())
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

// ========== WOW Features Command Handlers ==========

/// #2 Smart Reminders: /settime command
async fn handle_settime_command(
    bot: &teloxide::Bot,
    state: &SharedState,
    chat_id: ChatId,
    user_id: Uuid,
    args: &str,
) -> Result<()> {
    if args.is_empty() {
        bot.send_message(
            chat_id,
            "⏰ *Встановити час чекіну*\n\n\
            Формат: `/settime ГГ:ХХ` або `/settime auto`\n\n\
            Приклади:\n\
            • `/settime 09:00` - щодня о 9:00\n\
            • `/settime 14:30` - щодня о 14:30\n\
            • `/settime auto` - автоматично визначити найкращий час\n\n\
            Поточний час: 10:00 (за замовчуванням)",
        )
        .parse_mode(ParseMode::Markdown)
        .await?;
        return Ok(());
    }

    if args == "auto" {
        // Автоматичний вибір часу на основі активності
        let (hour, minute) = db::calculate_best_reminder_time(&state.pool, user_id).await?;

        db::set_user_reminder_time(&state.pool, user_id, hour, minute).await?;

        bot.send_message(
            chat_id,
            format!(
                "✅ *Встановлено автоматичний час!*\n\n\
                На основі твоєї активності найкращий час: *{:02}:{:02}*\n\n\
                Завтра отримаєш чекін саме тоді! ⏰",
                hour, minute
            ),
        )
        .parse_mode(ParseMode::Markdown)
        .await?;

        return Ok(());
    }

    // Parse time (09:00, 14:30, etc)
    let parts: Vec<&str> = args.split(':').collect();
    if parts.len() != 2 {
        bot.send_message(
            chat_id,
            "❌ Неправильний формат.\n\nВикористай: `/settime 09:00` або `/settime auto`",
        )
        .parse_mode(ParseMode::Markdown)
        .await?;
        return Ok(());
    }

    let hour: i16 = match parts[0].parse() {
        Ok(h) => h,
        Err(_) => {
            bot.send_message(chat_id, "❌ Неправильний формат години")
                .await?;
            return Ok(());
        }
    };

    let minute: i16 = match parts[1].parse() {
        Ok(m) => m,
        Err(_) => {
            bot.send_message(chat_id, "❌ Неправильний формат хвилин")
                .await?;
            return Ok(());
        }
    };

    if hour < 0 || hour > 23 || minute < 0 || minute > 59 {
        bot.send_message(chat_id, "❌ Час має бути в форматі 00:00 - 23:59")
            .await?;
        return Ok(());
    }

    db::set_user_reminder_time(&state.pool, user_id, hour, minute).await?;

    bot.send_message(
        chat_id,
        format!(
            "✅ *Час чекіну оновлено!*\n\n\
            Новий час: *{:02}:{:02}*\n\
            Завтра отримаєш чекін саме тоді! ⏰",
            hour, minute
        ),
    )
    .parse_mode(ParseMode::Markdown)
    .await?;

    Ok(())
}

/// #17 Kudos System: /kudos command
async fn handle_kudos_command(
    bot: &teloxide::Bot,
    state: &SharedState,
    chat_id: ChatId,
    user_id: Uuid,
    args: &str,
) -> Result<()> {
    if args.is_empty() {
        bot.send_message(
            chat_id,
            "🎉 *Kudos - подяка колезі!*\n\n\
            Формат: `/kudos @email повідомлення`\n\n\
            Приклад:\n\
            `/kudos @jane.davydiuk@opslab.uk Дякую за підтримку! 💙`\n\n\
            Колега отримає твоє повідомлення в Telegram!",
        )
        .parse_mode(ParseMode::Markdown)
        .await?;
        return Ok(());
    }

    // Parse: @email message
    let parts: Vec<&str> = args.splitn(2, ' ').collect();
    if parts.len() < 2 {
        bot.send_message(
            chat_id,
            "❌ Неправильний формат.\n\n\
            Використай: `/kudos @email повідомлення`\n\n\
            Приклад: `/kudos @jane.davydiuk@opslab.uk дякую! 💙`",
        )
        .parse_mode(ParseMode::Markdown)
        .await?;
        return Ok(());
    }

    let recipient_email = parts[0].trim_start_matches('@');
    let kudos_message = parts[1];

    // Find recipient
    let recipient = match db::get_user_by_email(&state.pool, recipient_email).await? {
        Some(u) => u,
        None => {
            bot.send_message(
                chat_id,
                format!("❌ Користувача {} не знайдено.\n\nПеревір email!", recipient_email),
            )
            .await?;
            return Ok(());
        }
    };

    if user_id == recipient.id {
        bot.send_message(chat_id, "😅 Не можна давати kudos собі!")
            .await?;
        return Ok(());
    }

    // Save kudos
    db::insert_kudos(&state.pool, user_id, recipient.id, kudos_message).await?;

    // Notify sender
    bot.send_message(
        chat_id,
        format!("✅ Kudos відправлено *{}*! 🎉", recipient_email),
    )
    .parse_mode(ParseMode::Markdown)
    .await?;

    // Notify recipient (if has Telegram)
    if let Some(recipient_tg_id) = recipient.telegram_id {
        let sender = db::find_user_by_id(&state.pool, user_id).await?;
        if let Some(sender) = sender {
            let sender_name = state
                .crypto
                .decrypt_str(&sender.enc_name)
                .unwrap_or_else(|_| "Colleague".to_string());

            bot.send_message(
                ChatId(recipient_tg_id),
                format!(
                    "🎉 *Kudos від {}!*\n\n\
                    {}\n\n\
                    _Продовжуй в тому ж дусі!_ 💪",
                    sender_name, kudos_message
                ),
            )
            .parse_mode(ParseMode::Markdown)
            .await?;
        }
    }

    Ok(())
}
