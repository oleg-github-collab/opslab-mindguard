///! Розширені handlers для Telegram бота з щоденними чекінами
use crate::analytics::correlations;
use crate::bot::daily_checkin::{CheckInGenerator, Metrics, MetricsCalculator};
use crate::bot::markdown::mdv2;
use crate::db;
use crate::services::ai::AiOutcome;
use crate::services::wellness;
use crate::state::SharedState;
use crate::time_utils;
use anyhow::Result;
use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use chrono::{Datelike, Utc};
use serde_json::json;
use sqlx;
use std::env;
use teloxide::net::Download;
use teloxide::prelude::*;
use teloxide::types::{
    ChatKind, InlineKeyboardButton, InlineKeyboardMarkup, Message, ParseMode, Update,
};
use uuid::Uuid;

// ========== WOW Features Helper Functions ==========

fn app_base_url() -> String {
    let raw = env::var("APP_BASE_URL")
        .or_else(|_| env::var("PUBLIC_BASE_URL"))
        .unwrap_or_else(|_| "http://localhost:3000".to_string());
    raw.trim_end_matches('/').to_string()
}

fn env_chat_id(keys: &[&str]) -> Option<i64> {
    for key in keys {
        if let Ok(val) = env::var(key) {
            if let Ok(id) = val.parse::<i64>() {
                return Some(id);
            }
        }
    }
    None
}

fn bot_username() -> Option<String> {
    env::var("BOT_USERNAME")
        .ok()
        .map(|raw| raw.trim().trim_start_matches('@').to_string())
        .filter(|val| !val.is_empty())
}

fn is_group_command(text: &str, bot_name: Option<&str>) -> bool {
    let trimmed = text.trim();
    let commands = [
        "/mindguard",
        "/help",
        "/support",
        "/checkin",
        "/status",
        "/weblogin",
        "/settings",
        "/kudos",
        "/plan",
        "/goals",
        "/pulse",
        "/insight",
        "/wall",
        "/link",
    ];
    if commands.iter().any(|cmd| trimmed.starts_with(cmd)) {
        return true;
    }
    if let Some(name) = bot_name {
        return commands
            .iter()
            .any(|cmd| trimmed.starts_with(&format!("{cmd}@{name}")));
    }
    false
}

fn is_personal_request(text: &str) -> bool {
    let lowered = text.to_lowercase();
    let keywords = [
        "/status",
        "/checkin",
        "/weblogin",
        "/settings",
        "/kudos",
        "/plan",
        "/goals",
        "/insight",
        "/link",
        "мій",
        "мої",
        "моє",
        "статист",
        "метрик",
        "дані",
        "ризик",
        "streak",
        "status",
        "checkin",
        "my stats",
        "my data",
    ];

    keywords.iter().any(|k| lowered.contains(k))
}

fn is_valid_code(code: &str) -> bool {
    code.len() == 4 && code.chars().all(|c| c.is_ascii_digit())
}

fn is_valid_email(email: &str) -> bool {
    let trimmed = email.trim_start_matches('@');
    trimmed.contains('@') && trimmed.len() <= 254
}

fn parse_link_command(text: &str) -> Option<(String, String)> {
    let mut parts = text.trim().split_whitespace();
    let cmd = parts.next()?;
    if !(cmd.starts_with("/start") || cmd.starts_with("/link")) {
        return None;
    }
    let email = parts.next()?;
    let code = parts.next()?;
    if is_valid_email(email) && is_valid_code(code) {
        return Some((email.to_string(), code.to_string()));
    }
    None
}

fn parse_plain_link(text: &str) -> Option<(String, String)> {
    let mut parts = text.trim().split_whitespace();
    let email = parts.next()?;
    let code = parts.next()?;
    if is_valid_email(email) && is_valid_code(code) {
        return Some((email.to_string(), code.to_string()));
    }
    None
}

struct ParsedCommand {
    name: String,
    args: String,
}

fn normalize_command(text: &str, bot_name: Option<&str>) -> Option<ParsedCommand> {
    let trimmed = text.trim();
    if !trimmed.starts_with('/') {
        return None;
    }

    let mut parts = trimmed.splitn(2, ' ');
    let mut cmd = parts.next()?.to_string();
    let args = parts.next().unwrap_or("").trim().to_string();

    if let Some(name) = bot_name {
        let suffix = format!("@{name}");
        if cmd.ends_with(&suffix) {
            cmd.truncate(cmd.len() - suffix.len());
        }
    }

    Some(ParsedCommand { name: cmd, args })
}

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
        actions.push(("📝 Дати фідбек", "action_feedback"));
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
    }

    // Створити inline keyboard
    let mut rows = Vec::new();
    for (text, callback_data) in actions {
        rows.push(vec![InlineKeyboardButton::callback(text, callback_data)]);
    }

    let keyboard = InlineKeyboardMarkup::new(rows);

    bot.send_message(
        chat_id,
        mdv2("💡 На основі твоїх відповідей:\n\nРекомендовані дії:"),
    )
    .parse_mode(ParseMode::MarkdownV2)
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
                mdv2(
                    "🎵 Meditation 5 min\n\n\
                    1. Знайди тихе місце\n\
                    2. Заплющ очі\n\
                    3. Дихай 4-7-8:\n\
                       • 4 сек вдих\n\
                       • 7 сек затримка\n\
                       • 8 сек видих\n\
                    4. Повтори 5 циклів\n\n\
                    Це допоможе знизити стрес і заспокоїтись 🧘",
                ),
            )
            .parse_mode(ParseMode::MarkdownV2)
            .await?;
        }
        "walk" => {
            bot.send_message(
                msg.chat.id,
                mdv2(
                    "🚶 10-хвилинна прогулянка\n\n\
                    ✅ Покращує настрій на 20%\n\
                    ✅ Знижує stress\n\
                    ✅ Очищує голову\n\n\
                    Встав і йди ЗАРАЗ! Я нагадаю через 10 хв ⏰",
                ),
            )
            .parse_mode(ParseMode::MarkdownV2)
            .await?;
        }
        "feedback" | "wall_post" => {
            let feedback_url = "https://opslab-feedback-production.up.railway.app/";
            bot.send_message(
                msg.chat.id,
                mdv2(format!(
                    "📝 OpsLab Feedback\n\n\
                    Анонімний або публічний фідбек доступний тут:\n\
                    {}\n\n\
                    Це окремий сервіс — без передачі твоїх приватних даних у групи.",
                    feedback_url
                )),
            )
            .parse_mode(ParseMode::MarkdownV2)
            .await?;
        }
        "talk" => {
            bot.send_message(
                msg.chat.id,
                mdv2(
                    "💬 Поговорити з кимось\n\n\
                    Іноді розмова - найкраще рішення.\n\n\
                    Кому написати:\n\
                    • Твоєму керівнику\n\
                    • HR/Jane\n\
                    • Колезі, якому довіряєш\n\n\
                    Твоє здоров'я важливіше за все! 💚",
                ),
            )
            .parse_mode(ParseMode::MarkdownV2)
            .await?;
        }
        "sleep_tips" => {
            bot.send_message(
                msg.chat.id,
                mdv2(
                    "😴 Поради для якісного сну:\n\n\
                    1. Лягай в один час (10-11 PM)\n\
                    2. Вимкни екрани за 1 годину\n\
                    3. Температура 18-20°C\n\
                    4. Темрява повна\n\
                    5. Без кави після 14:00\n\
                    6. Легка вечеря за 2-3 години\n\n\
                    💡 Спробуй сьогодні!",
                ),
            )
            .parse_mode(ParseMode::MarkdownV2)
            .await?;
        }
        "vacation" => {
            bot.send_message(
                msg.chat.id,
                mdv2(
                    "🌴 Час відпочити!\n\n\
                    Твої показники вказують на burnout.\n\n\
                    Рекомендації:\n\
                    • Візьми 2-3 дні off\n\
                    • Повністю відключись від роботи\n\
                    • Займи улюбленою справою\n\n\
                    Поговори з Jane про відпустку! 💙",
                ),
            )
            .parse_mode(ParseMode::MarkdownV2)
            .await?;
        }
        "status" => {
            bot.send_message(
                msg.chat.id,
                mdv2("Використай команду /status щоб побачити детальну статистику! 📊"),
            )
            .parse_mode(ParseMode::MarkdownV2)
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
        "reflection" => match value {
            9..=10 => "🧭 Дякую за глибину. Це важливо.",
            7..=8 => "💬 Ціную відкритість, це допомагає.",
            5..=6 => "🫶 Дякую, що поділився.",
            3..=4 => "💙 Звучить непросто. Ми поруч.",
            1..=2 => "🤝 Тримайся. Якщо потрібно — напиши /support.",
            _ => "✅ Дякую",
        },
        "support" => match value {
            9..=10 => "🤝 Супер, є опора.",
            7..=8 => "💙 Добре, що підтримка відчувається.",
            5..=6 => "🫶 Якщо потрібно більше підтримки — скажи.",
            3..=4 => "💬 Можемо подумати як додати підтримку.",
            1..=2 => "🛟 Дуже важливо не залишатись одному. Ми поруч.",
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
        teloxide::types::UpdateKind::Message(message) => match &message.chat.kind {
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
        },
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
    let text = msg.text().map(|t| t.trim().to_string());
    let bot_name = bot_username();
    let command = text
        .as_deref()
        .and_then(|t| normalize_command(t, bot_name.as_deref()));

    // Handle /start or /link with linking payload
    if let Some(text) = text.as_deref() {
        if let Some((email, code)) = parse_link_command(text) {
            return handle_link_by_code(bot, &state, msg.chat.id, telegram_id, &email, &code).await;
        }
    }

    let user = db::find_user_by_telegram(&state.pool, telegram_id).await?;
    let Some(user) = user else {
        if let Some(text) = text.as_deref() {
            if let Some((email, code)) = parse_plain_link(text) {
                return handle_link_by_code(bot, &state, msg.chat.id, telegram_id, &email, &code)
                    .await;
            }
            if let Some(cmd) = command.as_ref() {
                if cmd.name == "/start" || cmd.name == "/link" {
                    bot.send_message(
                        msg.chat.id,
                        mdv2(
                            "🧩 Для привʼязки потрібні email та 4-значний код доступу.\n\n\
                            Формат:\n\
                            /start email@opslab.uk 1234\n\n\
                            Або:\n\
                            /link email@opslab.uk 1234\n\n\
                            Якщо ви втратили код — зверніться до адміністратора.",
                        ),
                    )
                    .parse_mode(teloxide::types::ParseMode::MarkdownV2)
                    .await?;
                    return Ok(());
                }
            }
            if text.trim().starts_with('/') {
                let base_url = app_base_url();
                bot.send_message(
                    msg.chat.id,
                    mdv2(format!(
                        "🔒 Щоб команди працювали, пройдіть короткий онбординг:\n\n\
                        1) Відкрийте web: {}\n\
                        2) Увійдіть (email + 4-значний код)\n\
                        3) Налаштуйте час і пояс\n\
                        4) Поверніться в бот і привʼяжіть Telegram:\n\
                        /link email@opslab.uk 1234\n\n\
                        Після цього бот почне надсилати чекіни та звіти.",
                        base_url
                    )),
                )
                .parse_mode(teloxide::types::ParseMode::MarkdownV2)
                .await?;
                return Ok(());
            }
        }
        let base_url = app_base_url();
        bot.send_message(
            msg.chat.id,
            mdv2(format!(
                "👋 Привіт! Ласкаво просимо до OpsLab Mindguard!\n\n\
                🧠 Що це за платформа?\n\
                OpsLab Mindguard — система для моніторингу та підтримки ментального здоров'я команди.\n\n\
                🔐 Як почати?\n\
                1. Відкрийте web: {0}\n\
                2. Увійдіть (email + 4-значний код)\n\
                3. Пройдіть онбординг і встановіть час нагадувань\n\
                4. Поверніться в бот і привʼяжіть Telegram:\n\
                   /link email@opslab.uk 1234\n\n\
                💡 Код доступу ви отримали від адміністратора.\n\
                🔒 Привʼязка одноразова — для зміни зверніться до адміністратора.\n\n\
                📋 Доступні команди:\n\
                /help - Показати всі команди\n\
                /checkin - Пройти щоденний чекін\n\
                /status - Подивитись свій стан\n\
                /weblogin - Отримати посилання для входу\n\
                /wall - OpsLab Feedback (зовнішній)\n\n\
                Веб-платформа: {0}",
                base_url
            )),
        )
        .parse_mode(teloxide::types::ParseMode::MarkdownV2)
        .await?;
        return Ok(());
    };

    if !user.is_active {
        bot.send_message(
            msg.chat.id,
            "⛔ Ваш доступ до платформи призупинено.\n\
            Якщо це помилка — зверніться до адміністратора або HR.",
        )
        .await?;
        return Ok(());
    }

    let prefs = db::get_user_preferences(&state.pool, user.id)
        .await
        .unwrap_or(crate::db::UserPreferences {
            reminder_hour: 10,
            reminder_minute: 0,
            timezone: "Europe/Kyiv".to_string(),
            notification_enabled: true,
            last_reminder_date: None,
            last_plan_nudge_date: None,
            onboarding_completed: false,
            onboarding_completed_at: None,
        });

    if !prefs.onboarding_completed {
        if let Some(cmd) = command.as_ref() {
            if cmd.name == "/weblogin" {
                send_web_login_link(bot, &state, msg.chat.id, user.id).await?;
                return Ok(());
            }
            if cmd.name == "/link" {
                bot.send_message(
                    msg.chat.id,
                    mdv2(
                        "✅ Telegram уже привʼязаний до вашого акаунту.\n\n\
                        Для зміни зверніться до адміністратора.",
                    ),
                )
                .parse_mode(ParseMode::MarkdownV2)
                .await?;
                return Ok(());
            }
        }

        send_onboarding_gate(bot, msg.chat.id, &prefs).await?;
        return Ok(());
    }

    // Handle voice messages
    if let Some(voice) = msg.voice() {
        let file_id = voice.file.id.clone();
        handle_voice(bot, state, msg, user.id, file_id).await?;
        return Ok(());
    }

    // Handle text commands
    if let Some(cmd) = command {
        match cmd.name.as_str() {
            "/start" => {
                send_start_message(bot, msg.chat.id).await?;
                return Ok(());
            }
            "/link" => {
                bot.send_message(
                    msg.chat.id,
                    mdv2(
                        "✅ Telegram уже привʼязаний до вашого акаунту.\n\n\
                        Для зміни зверніться до адміністратора.",
                    ),
                )
                .parse_mode(ParseMode::MarkdownV2)
                .await?;
                return Ok(());
            }
            "/checkin" => {
            start_daily_checkin(bot, &state, msg.chat.id, user.id).await?;
            return Ok(());
            }
            "/status" => {
            send_user_status(bot, &state, msg.chat.id, user.id).await?;
            return Ok(());
            }
            "/wall" => {
            send_wall_info(bot, msg.chat.id).await?;
            return Ok(());
            }
            "/weblogin" => {
            send_web_login_link(bot, &state, msg.chat.id, user.id).await?;
            return Ok(());
            }
            "/settime" => {
                handle_settime_command(bot, &state, msg.chat.id, user.id, &cmd.args).await?;
                return Ok(());
            }
            "/timezone" => {
                handle_timezone_command(bot, &state, msg.chat.id, user.id, &cmd.args).await?;
                return Ok(());
            }
            "/notify" => {
                handle_notify_command(bot, &state, msg.chat.id, user.id, &cmd.args).await?;
                return Ok(());
            }
            "/settings" => {
            send_settings(bot, &state, msg.chat.id, user.id).await?;
            return Ok(());
            }
            "/kudos" => {
                handle_kudos_command(bot, &state, msg.chat.id, user.id, &cmd.args).await?;
                return Ok(());
            }
            "/plan" => {
            send_wellness_plan(bot, &state, msg.chat.id, user.id).await?;
            return Ok(());
            }
            "/goals" => {
                handle_goals_command(bot, &state, msg.chat.id, user.id, &cmd.args).await?;
                return Ok(());
            }
            "/pulse" => {
            send_pulse_info(bot, msg.chat.id).await?;
            return Ok(());
            }
            "/insight" => {
            send_personal_insight(bot, &state, msg.chat.id, user.id).await?;
            return Ok(());
            }
            "/help" => {
                send_help_message(bot, msg.chat.id).await?;
                return Ok(());
            }
            _ => {}
        }
    }

    if let Some(text) = text.as_deref() {
        let lowered = text.to_lowercase();
        if lowered.contains("тривога") || lowered.contains("паніка") {
            send_help_message(bot, msg.chat.id).await?;
            return Ok(());
        }
    }

    // Fallback
    bot.send_message(
        msg.chat.id,
        mdv2(
            "📱 Команди бота:\n\n\
            /checkin - Щоденний чекін (2-3 хв)\n\
            /status - Ваш поточний стан\n\
            /wall - OpsLab Feedback\n\
            /settings - Налаштування\n\
            /settime - Встановити час чекіну ⏰\n\
            /timezone - Часовий пояс\n\
            /notify - Нагадування on/off\n\
            /kudos - Подякувати колезі 🎉\n\
            /plan - План Wellness OS\n\
            /goals - Персональні цілі\n\
            /pulse - Pulse rooms\n\
            /insight - Персональний інсайт\n\
            /help - Допомога\n\
            /weblogin - Вхід у web\n\
            /link email@opslab.uk 1234 - Привʼязка Telegram",
        ),
    )
    .parse_mode(ParseMode::MarkdownV2)
    .await?;

    Ok(())
}

/// Обробка коду доступу для зв'язування Telegram
async fn handle_link_by_code(
    bot: &teloxide::Bot,
    state: &SharedState,
    chat_id: ChatId,
    telegram_id: i64,
    email: &str,
    code: &str,
) -> Result<()> {
    let email = email.trim_start_matches('@');
    match db::link_telegram_by_email_code(&state.pool, email, code, telegram_id).await {
        Ok(db::TelegramLinkOutcome::Linked(user_id)) => {
            let user = db::find_user_by_id(&state.pool, user_id).await?;
            let name = user
                .and_then(|u| state.crypto.decrypt_str(&u.enc_name).ok())
                .unwrap_or_else(|| "користувач".to_string());
            let base_url = app_base_url();

            bot.send_message(
                chat_id,
                mdv2(format!(
                    "✅ Вітаємо, {}!\n\n\
                    Telegram успішно підключено до вашого акаунту.\n\n\
                    🧭 Наступний крок:\n\
                    1) Відкрий web: {base_url}\n\
                    2) Пройди онбординг і задай час нагадувань\n\
                    3) Натисни \"Завершити онбординг\"\n\n\
                    Швидкий вхід у web: /weblogin\n\n\
                    Після завершення стануть доступні чекіни, звіти та персональні інсайти.\n\
                    Побачимось у твій обраний час! 👋",
                    name,
                )),
            )
            .parse_mode(teloxide::types::ParseMode::MarkdownV2)
            .await?;
        }
        Ok(db::TelegramLinkOutcome::AlreadyLinked {
            same_telegram: true,
            ..
        }) => {
            bot.send_message(
                chat_id,
                mdv2(
                    "✅ Telegram вже привʼязаний до вашого акаунту.\n\n\
                    Використайте /help для списку команд або /weblogin для швидкого входу.",
                ),
            )
            .parse_mode(teloxide::types::ParseMode::MarkdownV2)
            .await?;
        }
        Ok(db::TelegramLinkOutcome::AlreadyLinked {
            same_telegram: false,
            ..
        }) => {
            bot.send_message(
                chat_id,
                mdv2(
                    "⚠️ Цей акаунт вже привʼязаний до іншого Telegram.\n\n\
                    Для зміни зверніться до адміністратора.",
                ),
            )
            .parse_mode(teloxide::types::ParseMode::MarkdownV2)
            .await?;
        }
        Ok(db::TelegramLinkOutcome::TelegramIdInUse) => {
            bot.send_message(
                chat_id,
                mdv2(
                    "⚠️ Цей Telegram вже привʼязаний до іншого акаунту.\n\n\
                    Якщо це помилка — зверніться до адміністратора.",
                ),
            )
            .parse_mode(teloxide::types::ParseMode::MarkdownV2)
            .await?;
        }
        Ok(db::TelegramLinkOutcome::InvalidCredentials) => {
            bot.send_message(
                chat_id,
                mdv2(
                    "❌ Невірний email або код доступу.\n\n\
                    Формат:\n\
                    /start email@opslab.uk 1234\n\n\
                    Якщо код втрачено — зверніться до адміністратора.",
                ),
            )
            .parse_mode(teloxide::types::ParseMode::MarkdownV2)
            .await?;
        }
        Err(e) => {
            tracing::error!("Error linking Telegram: {}", e);
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
    let base_url = app_base_url();
    bot.send_message(
        chat_id,
        mdv2(format!(
            "👋 Привіт! Я OpsLab Mindguard Bot\n\n\
            Допомагаю відстежувати твоє ментальне здоров'я:\n\n\
            🔹 Щоденні чекіни (2-3 хв) - автоматична розсилка у твій час\n\
            🔹 Голосова підтримка - запиши голосове і отримай аналіз\n\
            🔹 OpsLab Feedback - окремий сервіс для фідбеку\n\
            🔹 Web dashboard - детальна статистика\n\n\
            Головні команди:\n\
            /checkin - Пройти чекін зараз\n\
            /status - Мій поточний стан\n\
            /weblogin - Отримати посилання для входу в dashboard\n\
            /wall - OpsLab Feedback\n\
            /plan - План Wellness OS\n\
            /goals - Персональні цілі\n\
            /pulse - Pulse rooms\n\
            /insight - Персональний інсайт\n\
            /settings - Налаштування та час нагадувань\n\
            /help - Допомога\n\
            /link email@opslab.uk 1234 - Привʼязка Telegram\n\n\
            💡 Швидкий старт:\n\
            1. Відкрий web dashboard: {base_url}\n\
            2. Переглянь метрики та оновлюй час нагадувань\n\
            3. Чекіни приходять у вибраний час\n\n\
            Час нагадувань можна змінити в /settings або /settime",
        )),
    )
    .parse_mode(teloxide::types::ParseMode::MarkdownV2)
    .await?;
    Ok(())
}

async fn send_onboarding_gate(
    bot: &teloxide::Bot,
    chat_id: ChatId,
    prefs: &crate::db::UserPreferences,
) -> Result<()> {
    let base_url = app_base_url();
    let time = format!("{:02}:{:02}", prefs.reminder_hour, prefs.reminder_minute);
    let notifications = if prefs.notification_enabled {
        "увімкнені"
    } else {
        "вимкнені"
    };

    bot.send_message(
        chat_id,
        mdv2(format!(
            "🧭 Ще один крок до активації Mindguard\n\n\
            1) Відкрий web: {base_url}\n\
            2) Пройди онбординг і задай час нагадувань\n\
            3) Натисни кнопку \"Завершити онбординг\"\n\n\
            Поточні налаштування: {time} · {} · сповіщення {}\n\n\
            Після завершення будуть доступні /checkin, /status, /plan та інші команди.\n\
            Швидкий вхід у web: /weblogin",
            prefs.timezone, notifications
        )),
    )
    .parse_mode(ParseMode::MarkdownV2)
    .await?;
    Ok(())
}

async fn send_help_message(bot: &teloxide::Bot, chat_id: ChatId) -> Result<()> {
    bot.send_message(
        chat_id,
        mdv2(
            "📱 Команди бота:\n\n\
            /checkin - Щоденний чекін\n\
            /status - Поточний стан\n\
            /wall - OpsLab Feedback\n\
            /weblogin - Вхід у web dashboard\n\
            /settime - Час нагадувань\n\
            /timezone - Часовий пояс\n\
            /notify - Нагадування on/off\n\
            /settings - Налаштування\n\
            /kudos - Подяка колезі\n\
            /plan - План Wellness OS\n\
            /goals - Персональні цілі\n\
            /pulse - Pulse rooms\n\
            /insight - Персональний інсайт\n\n\
            🔗 Привʼязка Telegram:\n\
            /start email@opslab.uk 1234\n\
            /link email@opslab.uk 1234\n\n\
            🧑‍🤝‍🧑 У груповому чаті:\n\
            Звертайтесь до бота через /mindguard або @mention для загальних порад.\n\
            Персональні дані доступні лише в приваті.\n\n\
            💆 Миттєва підтримка\n\
            Дихання 4-7-8: 4с вдих → 7с затримка → 8с видих (4 цикли).\n\n\
            Якщо потрібна термінова допомога — зверніться до психолога або керівника.",
        ),
    )
    .parse_mode(teloxide::types::ParseMode::MarkdownV2)
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
        mdv2(format!(
            "📋 Щоденний чекін\n\n{}\n\n⏱️ Займе {}",
            checkin.intro_message, checkin.estimated_time
        )),
    )
    .parse_mode(teloxide::types::ParseMode::MarkdownV2)
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
            InlineKeyboardButton::callback(i.to_string(), format!("ans_{}_{}", question.id, i))
        })
        .collect();
    rows.push(row1);

    // Другий ряд: 6-10
    let row2: Vec<InlineKeyboardButton> = (6..=10)
        .map(|i| {
            InlineKeyboardButton::callback(i.to_string(), format!("ans_{}_{}", question.id, i))
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
        mdv2(format!(
            "{} Питання {}/{}\n\n{}\n\nОцініть від 1 до 10",
            question.emoji,
            question_index + 1,
            checkin.questions.len(),
            question.text
        )),
    )
    .parse_mode(teloxide::types::ParseMode::MarkdownV2)
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
                    if !user.is_active {
                        bot.answer_callback_query(&callback.id)
                            .text("⛔ Доступ призупинено. Зверніться до адміністратора.")
                            .await?;
                        return Ok(());
                    }
                    // Знайти питання за ID в поточному чекіні
                    if let Some(question) = checkin.questions.iter().find(|q| q.id == question_id) {
                        // Зберегти відповідь в БД
                        db::insert_checkin_answer(
                            &state.pool,
                            user.id,
                            question_id,
                            &question.qtype,
                            value,
                        )
                        .await?;

                        // #4 WOW Feature: Emoji reactions based on mood
                        let reaction = get_emoji_reaction(&question.qtype, value);

                        bot.answer_callback_query(&callback.id)
                            .text(reaction)
                            .await?;

                        // Видалити попереднє повідомлення
                        bot.delete_message(msg.chat.id, msg.id).await.ok();

                        // Знайти індекс поточного питання
                        let current_index = checkin
                            .questions
                            .iter()
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
                                mdv2(
                                    "✅ Чекін завершено! Дякую! 🙏\n\n\
                                Твої дані збережені та будуть використані для аналізу.\n\
                                Продовжуй проходити щоденні чекіни для повної картини.\n\n\
                                Побачимось завтра! 👋",
                                ),
                            )
                            .parse_mode(teloxide::types::ParseMode::MarkdownV2)
                            .await?;

                            // #5 WOW Feature: Quick Actions after check-in
                            send_quick_actions(bot, &state, msg.chat.id, user.id)
                                .await
                                .ok();

                            // Gentle nudge for Wellness OS plan
                            if let Err(e) = maybe_send_plan_nudge(bot, &state, msg.chat.id, user.id).await {
                                tracing::warn!("Failed to send plan nudge: {}", e);
                            }

                            // Перевірити чи потрібно надіслати критичний алерт
                            let count =
                                db::get_checkin_answer_count(&state.pool, user.id, 10).await?;
                            if count >= 21 {
                                if let Ok(Some(metrics)) =
                                    db::calculate_user_metrics(&state.pool, user.id).await
                                {
                                    if MetricsCalculator::is_critical(&metrics) {
                                        send_critical_alert(bot, &state, user.id, &metrics).await?;

                                        // Сповістити користувача
                                        bot.send_message(
                                            msg.chat.id,
                                            mdv2(
                                                "⚠️ Важливе повідомлення\n\n\
                                            Твої показники вказують на необхідність звернення до фахівця.\n\n\
                                            Рекомендуємо:\n\
                                            • Поговорити з керівником\n\
                                            • Звернутися до психолога\n\
                                            • Взяти відпочинок\n\n\
                                            Твоє здоров'я - найважливіше! 💚",
                                            )
                                        )
                                        .parse_mode(teloxide::types::ParseMode::MarkdownV2)
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
            mdv2(format!(
                "📊 Твій статус\n\n\
                Чекінів пройдено: {}\n\
                Потрібно мінімум 7 днів (21 відповідь) для повної картини.\n\n\
                Продовжуй проходити щоденні чекіни! 💪",
                answer_count
            )),
        )
        .parse_mode(teloxide::types::ParseMode::MarkdownV2)
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
        mdv2(format!(
            "📊 Твій статус за останній тиждень\n\n\
            {} Рівень ризику: {}\n\n\
            🌟 Благополуччя (WHO-5): {}/100\n\
            😔 Депресія (PHQ-9): {}/27\n\
            😰 Тривожність (GAD-7): {}/21\n\
            🔥 Вигорання (MBI): {:.1}%\n\n\
            😴 Сон: {:.1}h (якість {:.1}/10)\n\
            ⚖️ Work-Life Balance: {:.1}/10\n\
            ⚠️ Рівень стресу: {:.1}/40\n\n\
            Дані за {} відповідей",
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
        )),
    )
    .parse_mode(teloxide::types::ParseMode::MarkdownV2)
    .await?;

    // Якщо критичні показники - надіслати алерт
    if MetricsCalculator::is_critical(&metrics) {
        send_critical_alert(bot, state, user_id, &metrics).await?;
    }

    Ok(())
}

/// Відправка інформації про OpsLab Feedback
async fn send_wall_info(bot: &teloxide::Bot, chat_id: ChatId) -> Result<()> {
    let feedback_url = "https://opslab-feedback-production.up.railway.app/";
    bot.send_message(
        chat_id,
        mdv2(format!(
            "📝 OpsLab Feedback\n\n\
            Простір для чесного зворотного зв'язку.\n\
            Анонімно або публічно — у зовнішньому сервісі.\n\n\
            🔗 {}",
            feedback_url
        )),
    )
    .parse_mode(teloxide::types::ParseMode::MarkdownV2)
    .await?;
    Ok(())
}

/// Generate web login link for Telegram user
async fn send_web_login_link(
    bot: &teloxide::Bot,
    state: &SharedState,
    chat_id: ChatId,
    user_id: Uuid,
) -> Result<()> {
    // Generate secure random token
    let token: String = (0..32)
        .map(|_| format!("{:02x}", rand::random::<u8>()))
        .collect();

    // Store token in database (expires in 5 minutes)
    sqlx::query(
        "INSERT INTO telegram_login_tokens (user_id, token, expires_at) VALUES ($1, $2, now() + INTERVAL '5 minutes')"
    )
    .bind(user_id)
    .bind(&token)
    .execute(&state.pool)
    .await?;

    let base_url = app_base_url();
    let login_url = format!("{}/?token={}", base_url, token);

    bot.send_message(
        chat_id,
        mdv2(format!(
            "🔐 Ваше персональне посилання для входу:\n\n\
            {}\n\n\
            ⏱ Посилання дійсне 5 хвилин\n\
            🔒 Одноразове використання\n\n\
            Просто перейдіть за посиланням - вхід виконається автоматично!",
            login_url
        )),
    )
    .parse_mode(teloxide::types::ParseMode::MarkdownV2)
    .await?;

    Ok(())
}

/// Відправка критичного алерту адмінам
async fn send_critical_alert(
    bot: &teloxide::Bot,
    state: &SharedState,
    user_id: Uuid,
    metrics: &Metrics,
) -> Result<()> {
    let admin_id = env_chat_id(&["ADMIN_TELEGRAM_ID", "TELEGRAM_ADMIN_CHAT_ID"]);
    let jane_id = env_chat_id(&["JANE_TELEGRAM_ID", "TELEGRAM_JANE_CHAT_ID"]);

    let alert_message = mdv2(format!(
        "🚨 КРИТИЧНИЙ АЛЕРТ!\n\n\
        Користувач: {}\n\n\
        📊 Критичні показники:\n\
        • WHO-5 (благополуччя): {}/100\n\
        • PHQ-9 (депресія): {}/27\n\
        • GAD-7 (тривожність): {}/21\n\
        • MBI (вигорання): {:.1}%\n\
        • Стрес: {:.1}/40\n\n\
        ⚠️ ТЕРМІНОВА ДІЯ НЕОБХІДНА!\n\n\
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
    ));

    // Відправка Олегу (admin)
    if let Some(admin) = admin_id {
        bot.send_message(ChatId(admin), alert_message.clone())
            .parse_mode(teloxide::types::ParseMode::MarkdownV2)
            .await
            .ok();
    }

    // Відправка Джейн (manager)
    if let Some(jane) = jane_id {
        bot.send_message(ChatId(jane), alert_message.clone())
            .parse_mode(teloxide::types::ParseMode::MarkdownV2)
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
    bot.send_message(
        msg.chat.id,
        "🎧 Отримав голосове. Аналізую, це займе до 30 секунд...",
    )
    .await?;

    let file = bot.get_file(file_id).await?;
    let mut bytes: Vec<u8> = Vec::new();
    bot.download_file(&file.path, &mut bytes).await?;

    let transcript = match state.ai.transcribe_voice(bytes).await {
        Ok(text) => text,
        Err(e) => {
            tracing::error!("Voice transcription failed: {}", e);
            bot.send_message(
                msg.chat.id,
                "Не вдалося розпізнати голосове. Спробуй ще раз або напиши текстом.",
            )
            .await?;
            return Ok(());
        }
    };
    let context = recent_context(&state, user_id).await.unwrap_or_default();
    let metrics = db::calculate_user_metrics(&state.pool, user_id)
        .await
        .ok()
        .flatten();

    let outcome: AiOutcome = match state
        .ai
        .analyze_transcript(&transcript, &context, metrics.as_ref())
        .await
    {
        Ok(result) => result,
        Err(e) => {
            tracing::error!("Voice analysis failed: {}", e);
            AiOutcome {
                transcript: transcript.clone(),
                ai_json: json!({
                    "sentiment": "unknown",
                    "emotion_tags": [],
                    "risk_score": 1,
                    "topics": [],
                    "advice": "Дякую, що поділився. Зроби коротку паузу, попий води та обери одну маленьку дію на зараз."
                }),
                risk_score: 1,
                urgent: false,
            }
        }
    };
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

    let advice = outcome
        .ai_json
        .get("advice")
        .and_then(|v| v.as_str())
        .unwrap_or("Зроби коротку паузу та подбай про себе.");
    let sentiment = outcome
        .ai_json
        .get("sentiment")
        .and_then(|v| v.as_str())
        .unwrap_or("невідомо");
    let emotion_tags = outcome
        .ai_json
        .get("emotion_tags")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        })
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "—".to_string());
    let topics = outcome
        .ai_json
        .get("topics")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        })
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "—".to_string());

    bot.send_message(
        msg.chat.id,
        format!(
            "🎧 Голосовий аналіз готовий.\n\n\
            Стан: {sentiment}\n\
            Емоції: {emotion_tags}\n\
            Теми: {topics}\n\
            Ризик: {}/10\n\n\
            Порада на сьогодні: {advice}",
            outcome.risk_score
        ),
    )
    .await?;

    if outcome.urgent {
        bot.send_message(
            msg.chat.id,
            "⚠️ Високий ризик: зробіть паузу 5 хв. Практика: 4-7-8 дихання + складіть 3 пункти плану на найближчу годину. Якщо потрібно — напишіть \"паніка\" щоб отримати швидку підтримку.",
        )
        .await?;
        if let Some(admin_id) = env_chat_id(&["ADMIN_TELEGRAM_ID", "TELEGRAM_ADMIN_CHAT_ID"]) {
            bot.send_message(
                ChatId(admin_id),
                format!("⚠️ URGENT | User {user_id} flagged risk_score=10"),
            )
            .await?;
        }
        if let Some(jane_id) = env_chat_id(&["JANE_TELEGRAM_ID", "TELEGRAM_JANE_CHAT_ID"]) {
            bot.send_message(
                ChatId(jane_id),
                format!("⚠️ URGENT | User {user_id} flagged risk_score=10"),
            )
            .await?;
        }
    }

    Ok(())
}

async fn handle_group(bot: &teloxide::Bot, state: SharedState, msg: Message) -> Result<()> {
    if let Some(text) = msg.text() {
        let bot_name = bot_username();
        let mention = bot_name
            .as_ref()
            .map(|name| format!("@{name}"))
            .unwrap_or_default();
        let is_reply_to_bot = msg
            .reply_to_message()
            .and_then(|m| m.from())
            .map(|u| u.is_bot)
            .unwrap_or(false);
        let has_mention = !mention.is_empty() && text.contains(&mention);
        let is_command = is_group_command(text, bot_name.as_deref());

        if !is_reply_to_bot && !has_mention && !is_command {
            return Ok(());
        }

        if is_personal_request(text) {
            bot.send_message(
                msg.chat.id,
                "🔒 Персональні метрики та чекіни доступні лише у приватному чаті.\n\
                Напиши мені в особисті повідомлення: /start",
            )
            .await?;
            return Ok(());
        }

        let trimmed = text.trim();
        if is_command {
            if trimmed.starts_with("/wall") {
                send_wall_info(bot, msg.chat.id).await?;
                return Ok(());
            }
            if trimmed.starts_with("/pulse") {
                send_pulse_info(bot, msg.chat.id).await?;
                return Ok(());
            }
            if trimmed == "/mindguard"
                || trimmed.starts_with("/mindguard@")
                || trimmed == "/help"
                || trimmed.starts_with("/help@")
                || trimmed == "/support"
                || trimmed.starts_with("/support@")
            {
                bot.send_message(
                    msg.chat.id,
                    "💬 Я можу допомогти з загальними порадами у групі.\n\
                    Напиши питання після /mindguard або з @mention.\n\
                    Наприклад: /mindguard як зняти стрес?",
                )
                .await?;
                return Ok(());
            }
        }

        // Проста логіка відповідей
        let response = if text.contains("стрес") || text.contains("тривога") {
            "💆 Поради при стресі:\n\n\
            1. Зроби глибокий вдих (4-7-8)\n\
            2. Вийди на прогулянку\n\
            3. Поговори з колегою\n\
            4. Зроби перерву\n\n\
            Пам'ятай: /checkin для відстеження стану"
                .to_string()
        } else if text.contains("втома") || text.contains("вигорання") {
            "🔥 При вигоранні:\n\n\
            1. Візьми відпочинок\n\
            2. Встанови межі\n\
            3. Делегуй задачі\n\
            4. Поговори з HR\n\n\
            Твоє здоров'я важливіше!"
                .to_string()
        } else {
            // AI відповідь
            state
                .ai
                .group_coach_response(text)
                .await
                .unwrap_or_else(|_| {
                    "Дихайте глибоко 4-4-4, зробіть перерву на 2 хвилини та поверніться до задачі."
                        .to_string()
                })
        };

        bot.send_message(msg.chat.id, mdv2(response))
            .parse_mode(teloxide::types::ParseMode::MarkdownV2)
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
        let prefs = db::get_user_preferences(&state.pool, user_id)
            .await
            .unwrap_or(crate::db::UserPreferences {
                reminder_hour: 10,
                reminder_minute: 0,
                timezone: "Europe/Kyiv".to_string(),
                notification_enabled: true,
                last_reminder_date: None,
                last_plan_nudge_date: None,
                onboarding_completed: false,
                onboarding_completed_at: None,
            });
        bot.send_message(
            chat_id,
            mdv2(format!(
                "⏰ Встановити час чекіну\n\n\
                Формат: /settime ГГ:ХХ або /settime auto\n\n\
                Приклади:\n\
                • /settime 09:00 - щодня о 9:00\n\
                • /settime 14:30 - щодня о 14:30\n\
                • /settime auto - автоматично визначити найкращий час\n\n\
                Поточний час: {:02}:{:02} ({})",
                prefs.reminder_hour,
                prefs.reminder_minute,
                prefs.timezone
            )),
        )
        .parse_mode(ParseMode::MarkdownV2)
        .await?;
        return Ok(());
    }

    if args == "auto" {
        // Автоматичний вибір часу на основі активності
        let prefs = db::get_user_preferences(&state.pool, user_id)
            .await
            .unwrap_or(crate::db::UserPreferences {
                reminder_hour: 10,
                reminder_minute: 0,
                timezone: "Europe/Kyiv".to_string(),
                notification_enabled: true,
                last_reminder_date: None,
                last_plan_nudge_date: None,
                onboarding_completed: false,
                onboarding_completed_at: None,
            });
        let (hour, minute) =
            db::calculate_best_reminder_time_local(&state.pool, user_id, &prefs.timezone).await?;

        db::set_user_reminder_time(&state.pool, user_id, hour, minute).await?;

        bot.send_message(
            chat_id,
            mdv2(format!(
                "✅ Встановлено автоматичний час!\n\n\
                На основі твоєї активності найкращий час: {:02}:{:02} ({})\n\n\
                Завтра отримаєш чекін саме тоді! ⏰",
                hour, minute, prefs.timezone
            )),
        )
        .parse_mode(ParseMode::MarkdownV2)
        .await?;

        return Ok(());
    }

    // Parse time (09:00, 14:30, etc)
    let parts: Vec<&str> = args.split(':').collect();
    if parts.len() != 2 {
        bot.send_message(
            chat_id,
            mdv2("❌ Неправильний формат.\n\nВикористай: /settime 09:00 або /settime auto"),
        )
        .parse_mode(ParseMode::MarkdownV2)
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

    let prefs = db::get_user_preferences(&state.pool, user_id)
        .await
        .unwrap_or(crate::db::UserPreferences {
            reminder_hour: hour,
            reminder_minute: minute,
            timezone: "Europe/Kyiv".to_string(),
            notification_enabled: true,
            last_reminder_date: None,
            last_plan_nudge_date: None,
            onboarding_completed: false,
            onboarding_completed_at: None,
        });

    bot.send_message(
        chat_id,
        mdv2(format!(
            "✅ Час чекіну оновлено!\n\n\
            Новий час: {:02}:{:02} ({})\n\
            Завтра отримаєш чекін саме тоді! ⏰",
            hour, minute, prefs.timezone
        )),
    )
    .parse_mode(ParseMode::MarkdownV2)
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
            mdv2(
                "🎉 Kudos - подяка колезі!\n\n\
            Формат: /kudos @email повідомлення\n\n\
            Приклад:\n\
            /kudos @jane.davydiuk@opslab.uk Дякую за підтримку! 💙\n\n\
            Колега отримає твоє повідомлення в Telegram!",
            ),
        )
        .parse_mode(ParseMode::MarkdownV2)
        .await?;
        return Ok(());
    }

    // Parse: @email message
    let parts: Vec<&str> = args.splitn(2, ' ').collect();
    if parts.len() < 2 {
        bot.send_message(
            chat_id,
            mdv2(
                "❌ Неправильний формат.\n\n\
            Використай: /kudos @email повідомлення\n\n\
            Приклад: /kudos @jane.davydiuk@opslab.uk дякую! 💙",
            ),
        )
        .parse_mode(ParseMode::MarkdownV2)
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
                format!(
                    "❌ Користувача {} не знайдено.\n\nПеревір email!",
                    recipient_email
                ),
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
        mdv2(format!("✅ Kudos відправлено {}! 🎉", recipient_email)),
    )
    .parse_mode(ParseMode::MarkdownV2)
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
                mdv2(format!(
                    "🎉 Kudos від {}!\n\n\
                    {}\n\n\
                    Продовжуй в тому ж дусі! 💪",
                    sender_name, kudos_message
                )),
            )
            .parse_mode(ParseMode::MarkdownV2)
            .await?;
        }
    }

    Ok(())
}

async fn send_wellness_plan(
    bot: &teloxide::Bot,
    state: &SharedState,
    chat_id: ChatId,
    user_id: Uuid,
) -> Result<()> {
    let prefs = db::get_user_preferences(&state.pool, user_id)
        .await
        .unwrap_or(crate::db::UserPreferences {
            reminder_hour: 10,
            reminder_minute: 0,
            timezone: "Europe/Kyiv".to_string(),
            notification_enabled: true,
            last_reminder_date: None,
            last_plan_nudge_date: None,
            onboarding_completed: false,
            onboarding_completed_at: None,
        });
    let (local_date, _, _) = time_utils::local_components(&prefs.timezone, Utc::now());

    let mut plan = db::get_wellness_plan(&state.pool, user_id, local_date).await?;
    let goals = db::get_user_goal_settings(&state.pool, user_id).await?;
    let metrics = db::calculate_user_metrics(&state.pool, user_id)
        .await
        .ok()
        .flatten();

    if plan.is_none() {
        let items = wellness::generate_daily_plan(metrics.as_ref(), &goals);
        let items_json = serde_json::to_value(&items).unwrap_or_else(|_| serde_json::json!([]));
        plan = Some(db::upsert_wellness_plan(&state.pool, user_id, local_date, &items_json).await?);
    }

    let plan = plan.unwrap();
    let items: Vec<wellness::PlanItem> =
        serde_json::from_value(plan.items).unwrap_or_else(|_| Vec::new());
    let plan_text = wellness::plan_to_text(&items);
    let completed = if plan.completed_at.is_some() {
        "✅ План відмічено виконаним."
    } else {
        "Позначити виконання можна у web або командою /plan після завершення."
    };

    bot.send_message(
        chat_id,
        mdv2(format!(
            "🌿 Wellness OS · План на сьогодні\n\n{}\n\n{}",
            plan_text, completed
        )),
    )
    .parse_mode(ParseMode::MarkdownV2)
    .await?;

    db::mark_plan_nudge_sent(&state.pool, user_id, local_date)
        .await
        .ok();

    Ok(())
}

async fn handle_goals_command(
    bot: &teloxide::Bot,
    state: &SharedState,
    chat_id: ChatId,
    user_id: Uuid,
    args: &str,
) -> Result<()> {
    let mut current = db::get_user_goal_settings(&state.pool, user_id).await?;

    if args.is_empty() {
        bot.send_message(
            chat_id,
            mdv2(format!(
                "🎯 Твої цілі\n\n\
                Сон: {} год/ніч\n\
                Пауза: {} раз/день\n\
                Рух: {} хв/день\n\
                Gentle nudges: {}\n\n\
                Оновити:\n\
                /goals sleep=7 breaks=3 move=20 nudges=on\n\
                або /goals 7 3 20",
                current.sleep_target,
                current.break_target,
                current.move_target,
                if current.notifications_enabled { "on" } else { "off" }
            )),
        )
        .parse_mode(ParseMode::MarkdownV2)
        .await?;
        return Ok(());
    }

    let mut sleep = None;
    let mut breaks = None;
    let mut move_target = None;
    let mut nudges = None;

    let parts: Vec<&str> = args.split_whitespace().collect();
    let mut numeric_parts = Vec::new();
    for part in parts {
        if let Some((key, val)) = part.split_once('=') {
            match key {
                "sleep" => sleep = val.parse::<i16>().ok(),
                "breaks" => breaks = val.parse::<i16>().ok(),
                "move" => move_target = val.parse::<i16>().ok(),
                "nudges" | "notify" => {
                    nudges = Some(matches!(val, "on" | "true" | "yes"))
                }
                _ => {}
            }
        } else {
            numeric_parts.push(part);
        }
    }

    if sleep.is_none() && breaks.is_none() && move_target.is_none() && !numeric_parts.is_empty() {
        if numeric_parts.len() >= 1 {
            sleep = numeric_parts[0].parse::<i16>().ok();
        }
        if numeric_parts.len() >= 2 {
            breaks = numeric_parts[1].parse::<i16>().ok();
        }
        if numeric_parts.len() >= 3 {
            move_target = numeric_parts[2].parse::<i16>().ok();
        }
    }

    if let Some(val) = sleep {
        current.sleep_target = val.clamp(4, 10);
    }
    if let Some(val) = breaks {
        current.break_target = val.clamp(1, 10);
    }
    if let Some(val) = move_target {
        current.move_target = val.clamp(5, 120);
    }
    if let Some(val) = nudges {
        current.notifications_enabled = val;
    }

    db::upsert_user_goal_settings(&state.pool, user_id, &current).await?;

    bot.send_message(
        chat_id,
        mdv2(format!(
            "✅ Цілі оновлено\n\n\
            Сон: {} год\n\
            Пауза: {} раз\n\
            Рух: {} хв\n\
            Gentle nudges: {}",
            current.sleep_target,
            current.break_target,
            current.move_target,
            if current.notifications_enabled { "on" } else { "off" }
        )),
    )
    .parse_mode(ParseMode::MarkdownV2)
    .await?;

    Ok(())
}

async fn send_pulse_info(bot: &teloxide::Bot, chat_id: ChatId) -> Result<()> {
    let base_url = app_base_url();
    bot.send_message(
        chat_id,
        mdv2(format!(
            "🗣 Pulse rooms\n\n\
            Анонімні командні обговорення з модерацією.\n\
            Перейди у web та відкрий Pulse Rooms.\n\n\
            🔗 {base_url}"
        )),
    )
    .parse_mode(ParseMode::MarkdownV2)
    .await?;
    Ok(())
}

async fn send_personal_insight(
    bot: &teloxide::Bot,
    state: &SharedState,
    chat_id: ChatId,
    user_id: Uuid,
) -> Result<()> {
    let metrics = db::calculate_user_metrics(&state.pool, user_id)
        .await
        .ok()
        .flatten();

    let Some(metrics) = metrics else {
        bot.send_message(
            chat_id,
            "Потрібно більше чекінів для персонального інсайту. Спробуй /checkin кілька днів.",
        )
        .await?;
        return Ok(());
    };

    let correlations = correlations::analyze_correlations(&state.pool, user_id)
        .await
        .unwrap_or_default();

    let insight = state
        .ai
        .generate_personal_insight(&metrics, &correlations)
        .await
        .unwrap_or_else(|_| {
            "Оціни сьогоднішній стрес, додай коротку паузу та одну маленьку перемогу."
                .to_string()
        });

    bot.send_message(chat_id, mdv2(format!("✨ Персональний інсайт\n\n{insight}")))
        .parse_mode(ParseMode::MarkdownV2)
        .await?;

    Ok(())
}

async fn maybe_send_plan_nudge(
    bot: &teloxide::Bot,
    state: &SharedState,
    chat_id: ChatId,
    user_id: Uuid,
) -> Result<()> {
    let goals = db::get_user_goal_settings(&state.pool, user_id).await?;
    if !goals.notifications_enabled {
        return Ok(());
    }

    let prefs = db::get_user_preferences(&state.pool, user_id).await?;
    let (local_date, _, _) = time_utils::local_components(&prefs.timezone, Utc::now());
    if prefs.last_plan_nudge_date == Some(local_date) {
        return Ok(());
    }

    let mut plan = db::get_wellness_plan(&state.pool, user_id, local_date).await?;
    let metrics = db::calculate_user_metrics(&state.pool, user_id)
        .await
        .ok()
        .flatten();

    if plan.is_none() {
        let items = wellness::generate_daily_plan(metrics.as_ref(), &goals);
        let items_json = serde_json::to_value(&items).unwrap_or_else(|_| serde_json::json!([]));
        plan = Some(db::upsert_wellness_plan(&state.pool, user_id, local_date, &items_json).await?);
    }

    let plan = plan.unwrap();
    if plan.completed_at.is_some() {
        return Ok(());
    }

    let items: Vec<wellness::PlanItem> =
        serde_json::from_value(plan.items).unwrap_or_else(|_| Vec::new());
    if items.is_empty() {
        return Ok(());
    }

    let preview = items
        .iter()
        .take(3)
        .enumerate()
        .map(|(idx, item)| format!("{}. {}", idx + 1, item.title))
        .collect::<Vec<_>>()
        .join("\n");

    bot.send_message(
        chat_id,
        mdv2(format!(
            "🌿 Wellness OS\n\n\
            Твій план на сьогодні вже готовий:\n\
            {}\n\n\
            Деталі: /plan",
            preview
        )),
    )
    .parse_mode(ParseMode::MarkdownV2)
    .await?;

    db::mark_plan_nudge_sent(&state.pool, user_id, local_date)
        .await
        .ok();

    Ok(())
}

/// /timezone command - set user's timezone
async fn handle_timezone_command(
    bot: &teloxide::Bot,
    state: &SharedState,
    chat_id: ChatId,
    user_id: Uuid,
    args: &str,
) -> Result<()> {
    if args.is_empty() {
        bot.send_message(
            chat_id,
            mdv2(
                "🌍 Часовий пояс\n\n\
            Формат: /timezone Europe/Kyiv або /timezone UTC+2\n\n\
            Приклади:\n\
            • /timezone Europe/Kyiv\n\
            • /timezone Europe/Warsaw\n\
            • /timezone UTC+2\n\n\
            Підказка: список IANA таймзон https://en.wikipedia.org/wiki/List_of_tz_database_time_zones",
            ),
        )
        .parse_mode(ParseMode::MarkdownV2)
        .await?;
        return Ok(());
    }

    let normalized = match time_utils::normalize_timezone(args) {
        Some(value) => value,
        None => {
            bot.send_message(
                chat_id,
                mdv2("❌ Невірний часовий пояс. Спробуй Europe/Kyiv або UTC+2."),
            )
            .parse_mode(ParseMode::MarkdownV2)
            .await?;
            return Ok(());
        }
    };

    db::set_user_timezone(&state.pool, user_id, &normalized).await?;

    let now_local = time_utils::format_local_time(&normalized, chrono::Utc::now());
    bot.send_message(
        chat_id,
        mdv2(format!(
            "✅ Часовий пояс оновлено: {}\n\
            Поточний локальний час: {}",
            normalized, now_local
        )),
    )
    .parse_mode(ParseMode::MarkdownV2)
    .await?;

    Ok(())
}

/// /notify command - enable/disable reminders
async fn handle_notify_command(
    bot: &teloxide::Bot,
    state: &SharedState,
    chat_id: ChatId,
    user_id: Uuid,
    args: &str,
) -> Result<()> {
    if args.is_empty() {
        let prefs = db::get_user_preferences(&state.pool, user_id).await?;
        let status = if prefs.notification_enabled {
            "увімкнено ✅"
        } else {
            "вимкнено ⛔"
        };
        bot.send_message(
            chat_id,
            mdv2(format!(
                "🔔 Нагадування зараз {}.\n\
                Використай /notify on або /notify off.",
                status
            )),
        )
        .parse_mode(ParseMode::MarkdownV2)
        .await?;
        return Ok(());
    }

    let arg = args.to_lowercase();
    let enabled = match arg.as_str() {
        "on" | "true" | "yes" | "1" | "увімкнути" | "увімкнено" => true,
        "off" | "false" | "no" | "0" | "вимкнути" | "вимкнено" => false,
        _ => {
            bot.send_message(
                chat_id,
                mdv2("❌ Невірна команда. Використай /notify on або /notify off."),
            )
            .parse_mode(ParseMode::MarkdownV2)
            .await?;
            return Ok(());
        }
    };

    db::set_user_notification_enabled(&state.pool, user_id, enabled).await?;

    let msg = if enabled {
        "✅ Нагадування увімкнено. Я напишу в заданий час."
    } else {
        "⛔ Нагадування вимкнено. Можеш повернути через /notify on."
    };

    bot.send_message(chat_id, msg).await?;
    Ok(())
}

/// /settings - show reminder settings
async fn send_settings(
    bot: &teloxide::Bot,
    state: &SharedState,
    chat_id: ChatId,
    user_id: Uuid,
) -> Result<()> {
    let prefs = db::get_user_preferences(&state.pool, user_id).await?;
    let now_local = time_utils::format_local_time(&prefs.timezone, chrono::Utc::now());
    let status = if prefs.notification_enabled {
        "увімкнено ✅"
    } else {
        "вимкнено ⛔"
    };

    bot.send_message(
        chat_id,
        mdv2(format!(
            "⚙️ Налаштування\n\n\
            ⏰ Час нагадувань: {:02}:{:02}\n\
            🌍 Часовий пояс: {}\n\
            🕒 Локальний час: {}\n\
            🔔 Нагадування: {}\n\n\
            Команди:\n\
            • /settime – змінити час\n\
            • /timezone – змінити часовий пояс\n\
            • /notify on|off – нагадування",
            prefs.reminder_hour,
            prefs.reminder_minute,
            prefs.timezone,
            now_local,
            status
        )),
    )
    .parse_mode(ParseMode::MarkdownV2)
    .await?;

    Ok(())
}
