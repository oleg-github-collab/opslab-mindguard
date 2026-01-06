# 🚀 План імплементації WOW-функцій

**Дата:** 2026-01-04
**Функції до імплементації:** 1, 2, 4, 5, 6, 7, 8, 10, 11, 12, 17

---

## 📊 АРХІТЕКТУРНИЙ ОГЛЯД

### Нові таблиці БД:

```sql
-- 05_wow_features.sql

-- User preferences & settings
CREATE TABLE user_preferences (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    reminder_hour SMALLINT DEFAULT 10 CHECK (reminder_hour >= 0 AND reminder_hour <= 23),
    reminder_minute SMALLINT DEFAULT 0 CHECK (reminder_minute >= 0 AND reminder_minute <= 59),
    timezone VARCHAR(50) DEFAULT 'UTC',
    language VARCHAR(5) DEFAULT 'uk',
    notification_enabled BOOLEAN DEFAULT true,
    updated_at TIMESTAMPTZ DEFAULT now()
);

-- Streak tracking
CREATE TABLE user_streaks (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    current_streak INT DEFAULT 0,
    longest_streak INT DEFAULT 0,
    last_checkin_date DATE,
    total_checkins INT DEFAULT 0,
    milestones_reached JSONB DEFAULT '[]'::jsonb,
    updated_at TIMESTAMPTZ DEFAULT now()
);

-- Kudos system
CREATE TABLE kudos (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    from_user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    to_user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    message TEXT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT now()
);
CREATE INDEX idx_kudos_to_user ON kudos(to_user_id, created_at DESC);
CREATE INDEX idx_kudos_from_user ON kudos(from_user_id, created_at DESC);

-- Team insights cache (для швидкості)
CREATE TABLE team_insights_cache (
    id SERIAL PRIMARY KEY,
    insight_type VARCHAR(50) NOT NULL,
    data JSONB NOT NULL,
    generated_at TIMESTAMPTZ DEFAULT now()
);
CREATE INDEX idx_insights_type_date ON team_insights_cache(insight_type, generated_at DESC);
```

---

## 🎯 ФУНКЦІЯ #1: Adaptive Question Intelligence

### Логіка:
```rust
// Аналізує попередні відповіді (останні 3 дні)
// Якщо stress високий → першим питає про stress
// Якщо sleep низький → focus на sleep
```

### Файли для змін:
1. `src/bot/daily_checkin.rs` - додати `AdaptiveQuestionEngine`
2. `src/db/mod.rs` - `get_user_recent_pattern()`

### Імплементація:

```rust
// src/bot/daily_checkin.rs

pub struct AdaptiveQuestionEngine;

impl AdaptiveQuestionEngine {
    /// Аналізує останні відповіді і визначає що питати першим
    pub async fn analyze_priority(
        pool: &PgPool,
        user_id: Uuid
    ) -> Result<Vec<QuestionType>> {
        // Отримати відповіді за останні 3 дні
        let recent = sqlx::query!(
            r#"
            SELECT question_type, AVG(value) as avg_value
            FROM checkin_answers
            WHERE user_id = $1
              AND created_at >= NOW() - INTERVAL '3 days'
            GROUP BY question_type
            "#,
            user_id
        )
        .fetch_all(pool)
        .await?;

        let mut priorities = Vec::new();

        // Логіка пріоритизації
        for row in recent {
            let qtype = row.question_type;
            let avg = row.avg_value.unwrap_or(5.0);

            match qtype.as_str() {
                "stress" if avg >= 7.0 => {
                    // Високий стрес - першим питати!
                    priorities.insert(0, QuestionType::Stress);
                }
                "sleep" if avg <= 5.0 => {
                    // Поганий сон - важливо!
                    priorities.insert(0, QuestionType::Sleep);
                }
                "energy" if avg <= 4.0 => {
                    // Низька енергія
                    priorities.insert(0, QuestionType::Energy);
                }
                _ => {}
            }
        }

        // Якщо немає пріоритетів - стандартний порядок по дню тижня
        if priorities.is_empty() {
            let day = Utc::now().weekday().num_days_from_monday();
            priorities = CheckInGenerator::select_question_types(day);
        }

        Ok(priorities)
    }
}

// Оновити CheckInGenerator
impl CheckInGenerator {
    pub async fn generate_adaptive_checkin(
        pool: &PgPool,
        user_id: Uuid
    ) -> Result<CheckIn> {
        // Використати adaptive logic
        let question_types = AdaptiveQuestionEngine::analyze_priority(pool, user_id).await?;

        let mut questions = Vec::new();
        for (idx, qtype) in question_types.iter().enumerate() {
            let (text, emoji) = QuestionBank::get_random_question(*qtype);
            questions.push(Question {
                id: idx as i32 + 1,
                qtype: Self::qtype_to_string(*qtype),
                text: text.to_string(),
                emoji: emoji.to_string(),
                scale: "1-10".to_string(),
            });
        }

        Ok(CheckIn {
            id: format!("checkin_{}", Utc::now().format("%Y%m%d")),
            user_id,
            date: Utc::now(),
            day_of_week: Utc::now().weekday().num_days_from_monday(),
            questions,
            intro_message: Self::get_adaptive_intro(&question_types),
            estimated_time: "2-3 хвилини".to_string(),
        })
    }

    fn get_adaptive_intro(types: &[QuestionType]) -> String {
        if types.first() == Some(&QuestionType::Stress) {
            "Доброго дня! 🌅 Помітив що stress високий. Як сьогодні?".to_string()
        } else if types.first() == Some(&QuestionType::Sleep) {
            "Привіт! 😴 Як спалося? Сон дуже важливий для здоров'я.".to_string()
        } else {
            "Доброго ранку! Як справи сьогодні?".to_string()
        }
    }
}
```

---

## 🎯 ФУНКЦІЯ #2: Smart Reminder Timing

### Логіка:
```
Користувач може:
1. /settime 09:00 - встановити свій час
2. /settime auto - система визначить найкращий час
```

### Імплементація:

```rust
// src/bot/enhanced_handlers.rs

async fn handle_settime_command(
    bot: &Bot,
    state: &SharedState,
    chat_id: ChatId,
    user_id: Uuid,
    time_str: &str
) -> Result<()> {
    if time_str == "auto" {
        // Аналізувати коли користувач найчастіше відповідає
        let best_time = db::calculate_best_reminder_time(&state.pool, user_id).await?;

        db::set_user_reminder_time(&state.pool, user_id, best_time.0, best_time.1).await?;

        bot.send_message(
            chat_id,
            format!(
                "✅ Встановлено автоматичний час!\n\n\
                На основі твоєї активності найкращий час: {:02}:{:02}\n\n\
                Завтра отримаєш чекін саме тоді! ⏰",
                best_time.0, best_time.1
            )
        ).await?;
    } else {
        // Parse time (09:00, 14:30, etc)
        let parts: Vec<&str> = time_str.split(':').collect();
        if parts.len() != 2 {
            bot.send_message(chat_id, "❌ Неправильний формат. Використай: /settime 09:00").await?;
            return Ok(());
        }

        let hour: i16 = parts[0].parse().map_err(|_| anyhow::anyhow!("Invalid hour"))?;
        let minute: i16 = parts[1].parse().map_err(|_| anyhow::anyhow!("Invalid minute"))?;

        if hour < 0 || hour > 23 || minute < 0 || minute > 59 {
            bot.send_message(chat_id, "❌ Час має бути 00:00 - 23:59").await?;
            return Ok(());
        }

        db::set_user_reminder_time(&state.pool, user_id, hour, minute).await?;

        bot.send_message(
            chat_id,
            format!(
                "✅ Час чекіну оновлено!\n\n\
                Новий час: {:02}:{:02}\n\
                Завтра отримаєш чекін саме тоді! ⏰",
                hour, minute
            )
        ).await?;
    }

    Ok(())
}

// src/db/mod.rs

pub async fn calculate_best_reminder_time(pool: &PgPool, user_id: Uuid) -> Result<(i16, i16)> {
    // Аналізувати коли користувач найчастіше відповідає на чекіни
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

pub async fn set_user_reminder_time(
    pool: &PgPool,
    user_id: Uuid,
    hour: i16,
    minute: i16
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

pub async fn get_users_for_reminder_time(
    pool: &PgPool,
    hour: i16,
    minute: i16
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
        "#,
        hour,
        minute
    )
    .fetch_all(pool)
    .await?;

    Ok(users.into_iter()
        .filter_map(|u| u.telegram_id.map(|tid| (u.id, tid)))
        .collect())
}
```

### Оновити scheduler:

```rust
// src/main.rs

// Замість одного job о 10:00, створити job кожну годину
scheduler.add(Job::new_async("0 * * * * *", move |_uuid, _l| {
    let state = shared_for_scheduler.clone();
    Box::pin(async move {
        let now = Utc::now();
        let hour = now.hour() as i16;
        let minute = now.minute() as i16;

        // Округлити до найближчих 15 хвилин (0, 15, 30, 45)
        let rounded_minute = (minute / 15) * 15;

        if let Ok(users) = db::get_users_for_reminder_time(&state.pool, hour, rounded_minute).await {
            if !users.is_empty() {
                tracing::info!("Sending check-ins to {} users at {:02}:{:02}", users.len(), hour, rounded_minute);
                // Send check-ins...
            }
        }
    })
})?).await?;
```

---

## 🎯 ФУНКЦІЯ #4: Mood-Based Emoji Reactions

### Логіка:
```
value >= 8 → "🎉 Чудово!"
value 6-7 → "👍 Непогано!"
value 4-5 → "😌 Норм"
value 2-3 → "💙 Розумію"
value 1 → "🤗 Тримайся"
```

### Імплементація:

```rust
// src/bot/enhanced_handlers.rs

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
        _ => "✅ Відповідь збережена".to_string(),
    }.to_string()
}

// Використати в handle_callback:
bot.answer_callback_query(&callback.id)
    .text(get_emoji_reaction(&question.qtype, value))
    .await?;
```

---

## 🎯 ФУНКЦІЯ #5: Quick Actions після чекіну

### Логіка:
```
stress >= 7 → [🎵 Meditation] [☕ Break]
energy <= 4 → [☕ Coffee] [💤 Nap reminder]
mood <= 4 → [📝 Wall] [🗣️ Talk to someone]
```

### Імплементація:

```rust
// src/bot/enhanced_handlers.rs

async fn send_quick_actions(
    bot: &Bot,
    state: &SharedState,
    chat_id: ChatId,
    user_id: Uuid,
    metrics: &Metrics
) -> Result<()> {
    let mut actions = Vec::new();

    // Аналізувати metrics і пропонувати дії
    if metrics.stress_level >= 28.0 { // ~7/10
        actions.push(("🎵 Meditation 5 min", "meditation"));
        actions.push(("🚶 Прогулянка 10 хв", "walk"));
    }

    if metrics.who5_score < 60 {
        actions.push(("📝 Написати на Wall", "wall_post"));
        actions.push(("💬 Поговорити з кимось", "talk"));
    }

    if metrics.sleep_quality < 6.0 {
        actions.push(("😴 Поради для сну", "sleep_tips"));
    }

    if actions.is_empty() {
        return Ok(());
    }

    // Створити inline keyboard
    let mut rows = Vec::new();
    for (text, callback_data) in actions {
        rows.push(vec![
            InlineKeyboardButton::callback(text, format!("action_{}", callback_data))
        ]);
    }

    let keyboard = InlineKeyboardMarkup::new(rows);

    bot.send_message(
        chat_id,
        "💡 *На основі твоїх відповідей:*\n\nРекомендовані дії:"
    )
    .parse_mode(ParseMode::Markdown)
    .reply_markup(keyboard)
    .await?;

    Ok(())
}

// Handler для actions:
async fn handle_action_callback(
    bot: &Bot,
    callback: &CallbackQuery,
    action: &str
) -> Result<()> {
    match action {
        "meditation" => {
            bot.send_message(
                callback.message.unwrap().chat.id,
                "🎵 *Meditation 5 min*\n\n\
                1. Знайди тихе місце\n\
                2. Заплющ очі\n\
                3. Дихай 4-7-8:\n\
                   • 4 сек вдих\n\
                   • 7 сек затримка\n\
                   • 8 сек видих\n\
                4. Повтори 5 циклів\n\n\
                [Guided meditation video →](https://youtube.com/...)"
            )
            .parse_mode(ParseMode::Markdown)
            .await?;
        }
        "walk" => {
            bot.send_message(
                callback.message.unwrap().chat.id,
                "🚶 *10-хвилинна прогулянка*\n\n\
                ✅ Покращує настрій на 20%\n\
                ✅ Знижує stress\n\
                ✅ Очищає голову\n\n\
                Встав і йди ЗАРАЗ! Я нагадаю через 10 хв ⏰"
            )
            .parse_mode(ParseMode::Markdown)
            .await?;

            // TODO: Нагадування через 10 хв
        }
        "wall_post" => {
            bot.send_message(
                callback.message.unwrap().chat.id,
                "📝 *Стіна плачу*\n\n\
                Поділись своїми думками анонімно:\n\
                https://mindguard.opslab.uk/wall"
            )
            .parse_mode(ParseMode::Markdown)
            .await?;
        }
        "sleep_tips" => {
            bot.send_message(
                callback.message.unwrap().chat.id,
                "😴 *Поради для якісного сну:*\n\n\
                1. Лягай в один час (10-11 PM)\n\
                2. Вимкни екрани за 1 годину\n\
                3. Температура 18-20°C\n\
                4. Темрява повна\n\
                5. Без кави після 14:00\n\n\
                💡 Спробуй сьогодні!"
            )
            .parse_mode(ParseMode::Markdown)
            .await?;
        }
        _ => {}
    }

    Ok(())
}
```

---

## 🎯 ФУНКЦІЯ #6: Weekly Summary (Telegram)

### Логіка:
```
Щоп'ятниці о 17:00 → відправити summary з:
- WHO-5, PHQ-9, GAD-7 за тиждень
- Тренди (↑ ↓ →)
- Anonymous team benchmark (#10)
- Top досягнення
```

### Імплементація:

```rust
// src/bot/weekly_summary.rs (НОВИЙ ФАЙЛ)

use crate::db;
use crate::domain::models::Metrics;
use crate::state::SharedState;
use anyhow::Result;
use chrono::{Datelike, Duration, Utc};
use teloxide::prelude::*;
use teloxide::types::ParseMode;
use uuid::Uuid;

pub struct WeeklySummary {
    pub user_id: Uuid,
    pub week_start: chrono::DateTime<Utc>,
    pub week_end: chrono::DateTime<Utc>,
    pub current_metrics: Metrics,
    pub previous_metrics: Option<Metrics>,
    pub checkin_count: i32,
    pub streak: i32,
    pub team_average: TeamAverage,
}

pub struct TeamAverage {
    pub who5: f64,
    pub phq9: f64,
    pub gad7: f64,
}

impl WeeklySummary {
    pub async fn generate(
        pool: &sqlx::PgPool,
        user_id: Uuid
    ) -> Result<Self> {
        let now = Utc::now();
        let week_start = now - Duration::days(7);

        // Поточні метрики (цей тиждень)
        let current_metrics = db::calculate_user_metrics(pool, user_id).await?;

        // Попередній тиждень для порівняння
        let previous_metrics = db::calculate_user_metrics_for_period(
            pool,
            user_id,
            week_start - Duration::days(7),
            week_start
        ).await.ok();

        // Кількість check-ins
        let checkin_count = db::get_checkin_count_for_week(pool, user_id).await?;

        // Streak
        let streak = db::get_user_current_streak(pool, user_id).await?;

        // Team average (анонімно)
        let team_average = db::get_team_average_metrics(pool).await?;

        Ok(Self {
            user_id,
            week_start,
            week_end: now,
            current_metrics,
            previous_metrics,
            checkin_count,
            streak,
            team_average,
        })
    }

    pub fn format_telegram_message(&self) -> String {
        let mut msg = String::from("📊 *ТВІЙ ТИЖНЕВИЙ SUMMARY*\n\n");

        // Check-ins
        msg.push_str(&format!("✅ Чекінів: {}/7\n", self.checkin_count));
        msg.push_str(&format!("🔥 Streak: {} днів\n\n", self.streak));

        // WHO-5 Well-being
        msg.push_str(&format!(
            "💚 *WHO-5 Well-being:* {:.1}/100 {}\n",
            self.current_metrics.who5_score,
            self.get_trend_emoji("who5")
        ));

        // PHQ-9 Depression
        msg.push_str(&format!(
            "🧠 *PHQ-9 Depression:* {:.1}/27 {}\n",
            self.current_metrics.phq9_score,
            self.get_trend_emoji("phq9")
        ));

        // GAD-7 Anxiety
        msg.push_str(&format!(
            "😰 *GAD-7 Anxiety:* {:.1}/21 {}\n",
            self.current_metrics.gad7_score,
            self.get_trend_emoji("gad7")
        ));

        // Burnout
        msg.push_str(&format!(
            "🔥 *Burnout:* {:.0}% {}\n\n",
            self.current_metrics.burnout_percentage,
            self.get_trend_emoji("burnout")
        ));

        // Team benchmark (#10)
        msg.push_str("📈 *Порівняння з командою (анонімно):*\n");
        msg.push_str(&self.format_team_comparison());
        msg.push_str("\n\n");

        // Insights
        msg.push_str("💡 *Інсайти тижня:*\n");
        msg.push_str(&self.generate_insights());

        msg.push_str("\n\n_Продовжуй в тому ж дусі! 💪_");

        msg
    }

    fn get_trend_emoji(&self, metric: &str) -> &'static str {
        if let Some(prev) = &self.previous_metrics {
            let (current, previous) = match metric {
                "who5" => (self.current_metrics.who5_score, prev.who5_score),
                "phq9" => (self.current_metrics.phq9_score, prev.phq9_score),
                "gad7" => (self.current_metrics.gad7_score, prev.gad7_score),
                "burnout" => (self.current_metrics.burnout_percentage, prev.burnout_percentage),
                _ => return "→",
            };

            let diff = current - previous;

            // WHO-5: вище = краще
            if metric == "who5" {
                if diff > 5.0 { "📈" } else if diff < -5.0 { "📉" } else { "→" }
            } else {
                // PHQ-9, GAD-7, burnout: нижче = краще
                if diff < -2.0 { "📈" } else if diff > 2.0 { "📉" } else { "→" }
            }
        } else {
            "→"
        }
    }

    fn format_team_comparison(&self) -> String {
        let mut comp = String::new();

        let who5_diff = self.current_metrics.who5_score - self.team_average.who5;
        let phq9_diff = self.current_metrics.phq9_score - self.team_average.phq9;
        let gad7_diff = self.current_metrics.gad7_score - self.team_average.gad7;

        comp.push_str(&format!(
            "• WHO-5: {} ({:+.1})\n",
            if who5_diff > 0.0 { "вище середнього ✨" } else { "нижче середнього" },
            who5_diff
        ));

        comp.push_str(&format!(
            "• PHQ-9: {} ({:+.1})\n",
            if phq9_diff < 0.0 { "краще команди ✨" } else { "гірше команди" },
            phq9_diff
        ));

        comp.push_str(&format!(
            "• GAD-7: {} ({:+.1})",
            if gad7_diff < 0.0 { "менше тривоги ✨" } else { "більше тривоги" },
            gad7_diff
        ));

        comp
    }

    fn generate_insights(&self) -> String {
        let mut insights = Vec::new();

        if self.current_metrics.who5_score >= 75.0 {
            insights.push("• Твій well-being на високому рівні! 🎉");
        } else if self.current_metrics.who5_score < 50.0 {
            insights.push("• Well-being низький. Поговори з кимось? 💙");
        }

        if self.streak >= 7 {
            insights.push(&format!("• {} днів streak! Ти супер! 🔥", self.streak));
        }

        if self.current_metrics.phq9_score < 5.0 {
            insights.push("• Депресивні симптоми мінімальні ✨");
        }

        if self.current_metrics.burnout_percentage < 30.0 {
            insights.push("• Ризик burnout низький 💚");
        } else if self.current_metrics.burnout_percentage > 70.0 {
            insights.push("• ⚠️ Високий ризик burnout! Потрібна перерва");
        }

        if insights.is_empty() {
            insights.push("• Продовжуй моніторити своє здоров'я!");
        }

        insights.join("\n")
    }
}

pub async fn send_weekly_summaries(state: &SharedState) -> Result<()> {
    // Отримати всіх користувачів з Telegram ID
    let users = db::get_all_telegram_users(&state.pool).await?;

    tracing::info!("Sending weekly summaries to {} users", users.len());

    for (user_id, telegram_id) in users {
        match WeeklySummary::generate(&state.pool, user_id).await {
            Ok(summary) => {
                let msg = summary.format_telegram_message();

                if let Err(e) = state.bot.send_message(ChatId(telegram_id), msg)
                    .parse_mode(ParseMode::Markdown)
                    .await
                {
                    tracing::error!("Failed to send weekly summary to user {}: {}", user_id, e);
                }

                // Rate limiting
                tokio::time::sleep(std::time::Duration::from_millis(35)).await;
            }
            Err(e) => {
                tracing::error!("Failed to generate summary for user {}: {}", user_id, e);
            }
        }
    }

    Ok(())
}

// src/db/mod.rs - додати функції:

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

pub async fn get_team_average_metrics(pool: &PgPool) -> Result<TeamAverage> {
    let avg = sqlx::query!(
        r#"
        WITH recent_metrics AS (
            SELECT
                user_id,
                AVG(CASE WHEN question_type = 'mood' THEN value * 20 ELSE 0 END) as who5,
                AVG(CASE WHEN question_type IN ('mood', 'sleep', 'concentration') THEN value * 3 ELSE 0 END) as phq9,
                AVG(CASE WHEN question_type IN ('anxiety', 'stress') THEN value * 3 ELSE 0 END) as gad7
            FROM checkin_answers
            WHERE created_at >= NOW() - INTERVAL '7 days'
            GROUP BY user_id
        )
        SELECT
            AVG(who5) as "avg_who5!",
            AVG(phq9) as "avg_phq9!",
            AVG(gad7) as "avg_gad7!"
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

    Ok(users.into_iter()
        .filter_map(|u| u.telegram_id.map(|tid| (u.id, tid)))
        .collect())
}
```

### Scheduler job:

```rust
// src/main.rs - додати Friday 17:00 job

scheduler.add(Job::new_async("0 0 17 * * FRI", move |_uuid, _l| {
    let state = shared_for_weekly.clone();
    Box::pin(async move {
        tracing::info!("Sending weekly summaries...");
        if let Err(e) = weekly_summary::send_weekly_summaries(&state).await {
            tracing::error!("Failed to send weekly summaries: {}", e);
        }
    })
})?).await?;
```

---

## 🎯 ФУНКЦІЯ #7: Correlation Insights

### Логіка:
```
Аналізувати кореляції:
- Sleep → Mood
- Stress → Concentration
- Day of week → Productivity
```

### Імплементація:

```rust
// src/analytics/correlations.rs (НОВИЙ ФАЙЛ)

use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

pub struct CorrelationInsight {
    pub correlation_type: String,
    pub strength: f64, // -1.0 to 1.0
    pub description: String,
    pub recommendation: String,
}

pub async fn analyze_correlations(
    pool: &PgPool,
    user_id: Uuid
) -> Result<Vec<CorrelationInsight>> {
    let mut insights = Vec::new();

    // 1. Sleep → Mood correlation
    let sleep_mood = calculate_sleep_mood_correlation(pool, user_id).await?;
    if sleep_mood.abs() > 0.5 {
        insights.push(CorrelationInsight {
            correlation_type: "sleep_mood".to_string(),
            strength: sleep_mood,
            description: format!(
                "Твій сон {} пов'язаний з настроєм (r={:.2})",
                if sleep_mood > 0.0 { "сильно" } else { "негативно" },
                sleep_mood
            ),
            recommendation: if sleep_mood > 0.0 {
                "💤 Якість сну напряму впливає на настрій. Пріоритизуй 7-8 годин!".to_string()
            } else {
                "🤔 Цікаво: твій сон не корелює з настроєм. Шукай інші фактори.".to_string()
            },
        });
    }

    // 2. Stress → Concentration correlation
    let stress_focus = calculate_stress_concentration_correlation(pool, user_id).await?;
    if stress_focus.abs() > 0.4 {
        insights.push(CorrelationInsight {
            correlation_type: "stress_concentration".to_string(),
            strength: stress_focus,
            description: format!(
                "Стрес {} концентрацію (r={:.2})",
                if stress_focus < 0.0 { "знижує" } else { "підвищує" },
                stress_focus
            ),
            recommendation: if stress_focus < -0.5 {
                "⚠️ Високий стрес руйнує концентрацію. Meditation + breaks!".to_string()
            } else {
                "✅ Стрес не сильно впливає на концентрацію.".to_string()
            },
        });
    }

    // 3. Day of week patterns
    let best_day = find_best_day_of_week(pool, user_id).await?;
    insights.push(CorrelationInsight {
        correlation_type: "day_of_week".to_string(),
        strength: 1.0,
        description: format!("Твій найкращий день: {}", day_name(best_day)),
        recommendation: format!(
            "📅 Плануй важливі завдання на {}",
            day_name(best_day)
        ),
    });

    Ok(insights)
}

async fn calculate_sleep_mood_correlation(
    pool: &PgPool,
    user_id: Uuid
) -> Result<f64> {
    // Pearson correlation між sleep і mood
    let result = sqlx::query!(
        r#"
        WITH daily_data AS (
            SELECT
                DATE(created_at) as day,
                AVG(CASE WHEN question_type = 'sleep' THEN value ELSE NULL END) as sleep,
                AVG(CASE WHEN question_type = 'mood' THEN value ELSE NULL END) as mood
            FROM checkin_answers
            WHERE user_id = $1
              AND created_at >= NOW() - INTERVAL '30 days'
            GROUP BY DATE(created_at)
            HAVING
                AVG(CASE WHEN question_type = 'sleep' THEN value ELSE NULL END) IS NOT NULL
                AND AVG(CASE WHEN question_type = 'mood' THEN value ELSE NULL END) IS NOT NULL
        )
        SELECT
            CORR(sleep, mood) as "correlation"
        FROM daily_data
        "#,
        user_id
    )
    .fetch_one(pool)
    .await?;

    Ok(result.correlation.unwrap_or(0.0))
}

async fn calculate_stress_concentration_correlation(
    pool: &PgPool,
    user_id: Uuid
) -> Result<f64> {
    let result = sqlx::query!(
        r#"
        WITH daily_data AS (
            SELECT
                DATE(created_at) as day,
                AVG(CASE WHEN question_type = 'stress' THEN value ELSE NULL END) as stress,
                AVG(CASE WHEN question_type = 'concentration' THEN value ELSE NULL END) as concentration
            FROM checkin_answers
            WHERE user_id = $1
              AND created_at >= NOW() - INTERVAL '30 days'
            GROUP BY DATE(created_at)
            HAVING
                AVG(CASE WHEN question_type = 'stress' THEN value ELSE NULL END) IS NOT NULL
                AND AVG(CASE WHEN question_type = 'concentration' THEN value ELSE NULL END) IS NOT NULL
        )
        SELECT
            CORR(stress, concentration) as "correlation"
        FROM daily_data
        "#,
        user_id
    )
    .fetch_one(pool)
    .await?;

    Ok(result.correlation.unwrap_or(0.0))
}

async fn find_best_day_of_week(pool: &PgPool, user_id: Uuid) -> Result<u32> {
    let result = sqlx::query!(
        r#"
        SELECT
            EXTRACT(DOW FROM created_at)::INT as dow,
            AVG(value) as avg_value
        FROM checkin_answers
        WHERE user_id = $1
          AND created_at >= NOW() - INTERVAL '60 days'
        GROUP BY dow
        ORDER BY avg_value DESC
        LIMIT 1
        "#,
        user_id
    )
    .fetch_one(pool)
    .await?;

    Ok(result.dow.unwrap_or(1) as u32)
}

fn day_name(dow: u32) -> &'static str {
    match dow {
        0 => "Неділя",
        1 => "Понеділок",
        2 => "Вівторок",
        3 => "Середа",
        4 => "Четвер",
        5 => "П'ятниця",
        6 => "Субота",
        _ => "Невідомо",
    }
}

// Додати до weekly summary:
// src/bot/weekly_summary.rs

// У WeeklySummary::format_telegram_message() додати:
let correlations = analyze_correlations(pool, self.user_id).await?;
if !correlations.is_empty() {
    msg.push_str("\n\n🔍 *Correlation Insights:*\n");
    for corr in correlations {
        msg.push_str(&format!("• {}\n  {}\n", corr.description, corr.recommendation));
    }
}
```

---

## 🎯 ФУНКЦІЯ #8: Team Mood Heatmap

### Логіка:
```
Admin/Founder бачать:
- Grid 3x3 (9 users)
- Кольори: 🟢 (добре) 🟡 (норм) 🔴 (погано)
- Real-time статус команди
```

### Імплементація:

```rust
// src/web/admin.rs - оновити API

use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct TeamHeatmapResponse {
    pub members: Vec<TeamMemberStatus>,
    pub team_average: TeamAverage,
    pub critical_count: usize,
}

#[derive(Serialize)]
pub struct TeamMemberStatus {
    pub name: String, // encrypted, буде розшифровано
    pub email: String,
    pub status: MoodStatus,
    pub last_checkin: Option<chrono::DateTime<chrono::Utc>>,
    pub metrics: UserMetricsSummary,
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MoodStatus {
    Good,    // 🟢
    Warning, // 🟡
    Critical, // 🔴
    NoData,  // ⚪
}

#[derive(Serialize)]
pub struct UserMetricsSummary {
    pub who5: f64,
    pub phq9: f64,
    pub gad7: f64,
    pub burnout: f64,
}

pub async fn get_team_heatmap(
    State(state): State<SharedState>,
    UserSession(user_id): UserSession,
) -> Result<Json<TeamHeatmapResponse>, StatusCode> {
    // Перевірка: тільки Admin/Founder
    let role = db::get_user_role(&state.pool, user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !matches!(role, UserRole::Admin | UserRole::Founder) {
        return Err(StatusCode::FORBIDDEN);
    }

    // Отримати всіх користувачів
    let users = db::get_all_users(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut members = Vec::new();
    let mut critical_count = 0;

    for user in users {
        let metrics = db::calculate_user_metrics(&state.pool, user.id)
            .await
            .ok();

        let last_checkin = db::get_last_checkin_date(&state.pool, user.id)
            .await
            .ok()
            .flatten();

        let status = if let Some(m) = &metrics {
            if m.who5_score < 40.0 || m.phq9_score >= 15.0 || m.burnout_percentage > 70.0 {
                critical_count += 1;
                MoodStatus::Critical
            } else if m.who5_score < 60.0 || m.phq9_score >= 10.0 || m.burnout_percentage > 50.0 {
                MoodStatus::Warning
            } else {
                MoodStatus::Good
            }
        } else {
            MoodStatus::NoData
        };

        // Розшифрувати ім'я
        let name = state.crypto.decrypt_str(&user.enc_name)
            .unwrap_or_else(|_| "Unknown".to_string());

        members.push(TeamMemberStatus {
            name,
            email: user.email,
            status,
            last_checkin,
            metrics: metrics.map(|m| UserMetricsSummary {
                who5: m.who5_score,
                phq9: m.phq9_score,
                gad7: m.gad7_score,
                burnout: m.burnout_percentage,
            }).unwrap_or(UserMetricsSummary {
                who5: 0.0,
                phq9: 0.0,
                gad7: 0.0,
                burnout: 0.0,
            }),
        });
    }

    let team_average = db::get_team_average_metrics(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(TeamHeatmapResponse {
        members,
        team_average,
        critical_count,
    }))
}

// src/db/mod.rs

pub async fn get_last_checkin_date(
    pool: &PgPool,
    user_id: Uuid
) -> Result<Option<chrono::DateTime<chrono::Utc>>> {
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
```

### Frontend (React):

```tsx
// web/src/components/TeamHeatmap.tsx

import React from 'react';

interface TeamHeatmapProps {
  data: TeamHeatmapResponse;
}

export const TeamHeatmap: React.FC<TeamHeatmapProps> = ({ data }) => {
  const getStatusColor = (status: MoodStatus) => {
    switch (status) {
      case 'good': return 'bg-green-500';
      case 'warning': return 'bg-yellow-500';
      case 'critical': return 'bg-red-500';
      default: return 'bg-gray-300';
    }
  };

  const getStatusEmoji = (status: MoodStatus) => {
    switch (status) {
      case 'good': return '🟢';
      case 'warning': return '🟡';
      case 'critical': return '🔴';
      default: return '⚪';
    }
  };

  return (
    <div className="team-heatmap">
      <h2 className="text-2xl font-bold mb-4">Team Mood Heatmap</h2>

      {data.critical_count > 0 && (
        <div className="alert alert-danger mb-4">
          ⚠️ {data.critical_count} members need attention!
        </div>
      )}

      <div className="grid grid-cols-3 gap-4">
        {data.members.map((member) => (
          <div
            key={member.email}
            className={`p-4 rounded-lg ${getStatusColor(member.status)} bg-opacity-20 border-2`}
          >
            <div className="flex items-center justify-between mb-2">
              <span className="font-semibold">{member.name}</span>
              <span className="text-2xl">{getStatusEmoji(member.status)}</span>
            </div>

            <div className="text-sm space-y-1">
              <div>WHO-5: {member.metrics.who5.toFixed(1)}</div>
              <div>PHQ-9: {member.metrics.phq9.toFixed(1)}</div>
              <div>GAD-7: {member.metrics.gad7.toFixed(1)}</div>
              <div>Burnout: {member.metrics.burnout.toFixed(0)}%</div>
            </div>

            {member.last_checkin && (
              <div className="text-xs text-gray-500 mt-2">
                Last: {new Date(member.last_checkin).toLocaleDateString()}
              </div>
            )}
          </div>
        ))}
      </div>

      <div className="mt-6 p-4 bg-gray-100 rounded">
        <h3 className="font-semibold mb-2">Team Average</h3>
        <div className="grid grid-cols-3 gap-4">
          <div>WHO-5: {data.team_average.who5.toFixed(1)}</div>
          <div>PHQ-9: {data.team_average.phq9.toFixed(1)}</div>
          <div>GAD-7: {data.team_average.gad7.toFixed(1)}</div>
        </div>
      </div>
    </div>
  );
};
```

---

## 🎯 ФУНКЦІЯ #11: Voice AI Coach

### Логіка:
```
Користувач відправляє голосове → OpenAI Whisper → GPT-4 аналіз → рекомендації
```

### Імплементація:

```rust
// src/ai/voice_coach.rs (НОВИЙ ФАЙЛ)

use crate::ai::AiService;
use anyhow::Result;
use async_openai::types::{
    ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs,
    ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs,
};

pub struct VoiceCoach {
    ai: std::sync::Arc<AiService>,
}

impl VoiceCoach {
    pub fn new(ai: std::sync::Arc<AiService>) -> Self {
        Self { ai }
    }

    pub async fn analyze_voice_message(
        &self,
        transcription: &str,
        user_metrics: Option<&crate::domain::models::Metrics>,
    ) -> Result<VoiceCoachResponse> {
        let system_prompt = self.build_system_prompt(user_metrics);

        let messages = vec![
            ChatCompletionRequestMessage::System(
                ChatCompletionRequestSystemMessageArgs::default()
                    .content(system_prompt)
                    .build()?
            ),
            ChatCompletionRequestMessage::User(
                ChatCompletionRequestUserMessageArgs::default()
                    .content(transcription)
                    .build()?
            ),
        ];

        let request = CreateChatCompletionRequestArgs::default()
            .model("gpt-4-turbo-preview")
            .messages(messages)
            .temperature(0.7)
            .max_tokens(500)
            .build()?;

        let response = self.ai.client.chat().create(request).await?;

        let content = response.choices[0]
            .message
            .content
            .clone()
            .unwrap_or_default();

        Ok(VoiceCoachResponse {
            analysis: content.clone(),
            recommendations: self.extract_recommendations(&content),
            empathy_score: self.calculate_empathy_score(&content),
        })
    }

    fn build_system_prompt(&self, metrics: Option<&crate::domain::models::Metrics>) -> String {
        let mut prompt = String::from(
            "Ти - емпатичний AI-коуч для ментального здоров'я співробітників.\n\n\
            Твоя роль:\n\
            1. Уважно вислухати (прочитати транскрипцію)\n\
            2. Визначити емоційний стан\n\
            3. Надати підтримку та конкретні рекомендації\n\
            4. Бути стислим (3-5 речень)\n\n"
        );

        if let Some(m) = metrics {
            prompt.push_str(&format!(
                "Контекст користувача:\n\
                - WHO-5 (well-being): {:.1}/100\n\
                - PHQ-9 (depression): {:.1}/27\n\
                - GAD-7 (anxiety): {:.1}/21\n\
                - Burnout: {:.0}%\n\n",
                m.who5_score, m.phq9_score, m.gad7_score, m.burnout_percentage
            ));
        }

        prompt.push_str(
            "Відповідай українською мовою. Будь теплим, підтримуючим, але чесним.\n\
            Якщо бачиш серйозні проблеми - рекомендуй поговорити з психологом."
        );

        prompt
    }

    fn extract_recommendations(&self, analysis: &str) -> Vec<String> {
        // Проста екстракція (можна покращити з regex)
        analysis
            .lines()
            .filter(|line| line.starts_with("•") || line.starts_with("-") || line.starts_with("*"))
            .map(|s| s.trim().to_string())
            .collect()
    }

    fn calculate_empathy_score(&self, analysis: &str) -> f64 {
        // Простий heuristic: чи є підтримуючі слова?
        let empathy_words = [
            "розумію", "підтримую", "важливо", "нормально",
            "не один", "допоможу", "тримайся", "молодець"
        ];

        let count = empathy_words.iter()
            .filter(|word| analysis.to_lowercase().contains(*word))
            .count();

        (count as f64 / empathy_words.len() as f64).min(1.0)
    }
}

#[derive(Debug)]
pub struct VoiceCoachResponse {
    pub analysis: String,
    pub recommendations: Vec<String>,
    pub empathy_score: f64,
}

// src/bot/enhanced_handlers.rs - оновити handle_voice:

pub async fn handle_voice(
    bot: Bot,
    state: SharedState,
    msg: Message,
) -> Result<()> {
    let chat_id = msg.chat.id;

    // Existing transcription logic...
    let transcription = transcribe_voice(&bot, &msg).await?;

    // NEW: Voice Coach analysis
    let user_id = db::get_user_by_telegram_id(&state.pool, chat_id.0)
        .await?
        .ok_or_else(|| anyhow::anyhow!("User not found"))?;

    let metrics = db::calculate_user_metrics(&state.pool, user_id).await.ok();

    let coach = VoiceCoach::new(state.ai.clone());
    let response = coach.analyze_voice_message(&transcription, metrics.as_ref()).await?;

    // Send response
    let mut reply = format!(
        "🎙️ *Voice Analysis:*\n\n{}\n\n",
        response.analysis
    );

    if !response.recommendations.is_empty() {
        reply.push_str("💡 *Рекомендації:*\n");
        for rec in response.recommendations {
            reply.push_str(&format!("{}\n", rec));
        }
    }

    bot.send_message(chat_id, reply)
        .parse_mode(ParseMode::Markdown)
        .await?;

    // Save to wall (existing logic)...

    Ok(())
}
```

---

## 🎯 ФУНКЦІЯ #12: Auto Wall Post Categorization

### Логіка:
```
AI аналізує пост → категорія:
- 😤 Complaint
- 💡 Suggestion
- 🎉 Celebration
- ❓ Question
- 💙 Support needed
```

### Імплементація:

```rust
// src/ai/categorizer.rs (НОВИЙ ФАЙЛ)

use async_openai::types::{
    ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs,
    ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "post_category", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PostCategory {
    Complaint,
    Suggestion,
    Celebration,
    Question,
    SupportNeeded,
}

impl PostCategory {
    pub fn emoji(&self) -> &'static str {
        match self {
            Self::Complaint => "😤",
            Self::Suggestion => "💡",
            Self::Celebration => "🎉",
            Self::Question => "❓",
            Self::SupportNeeded => "💙",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Complaint => "Complaint",
            Self::Suggestion => "Suggestion",
            Self::Celebration => "Celebration",
            Self::Question => "Question",
            Self::SupportNeeded => "Support Needed",
        }
    }
}

pub struct WallPostCategorizer {
    ai: std::sync::Arc<crate::ai::AiService>,
}

impl WallPostCategorizer {
    pub fn new(ai: std::sync::Arc<crate::ai::AiService>) -> Self {
        Self { ai }
    }

    pub async fn categorize(&self, content: &str) -> Result<PostCategory> {
        let system_prompt = "Ти - класифікатор постів на стіні плачу.\n\n\
            Категорії:\n\
            - COMPLAINT: скарги, невдоволення, проблеми\n\
            - SUGGESTION: ідеї, пропозиції покращень\n\
            - CELEBRATION: успіхи, досягнення, позитив\n\
            - QUESTION: питання, прохання порад\n\
            - SUPPORT_NEEDED: потреба в підтримці, допомозі\n\n\
            Відповідай ТІЛЬКИ однією категорією: COMPLAINT, SUGGESTION, CELEBRATION, QUESTION, або SUPPORT_NEEDED";

        let messages = vec![
            ChatCompletionRequestMessage::System(
                ChatCompletionRequestSystemMessageArgs::default()
                    .content(system_prompt)
                    .build()?
            ),
            ChatCompletionRequestMessage::User(
                ChatCompletionRequestUserMessageArgs::default()
                    .content(content)
                    .build()?
            ),
        ];

        let request = CreateChatCompletionRequestArgs::default()
            .model("gpt-3.5-turbo")
            .messages(messages)
            .temperature(0.3)
            .max_tokens(10)
            .build()?;

        let response = self.ai.client.chat().create(request).await?;

        let category_str = response.choices[0]
            .message
            .content
            .clone()
            .unwrap_or_default()
            .trim()
            .to_uppercase();

        // Parse category
        match category_str.as_str() {
            "COMPLAINT" => Ok(PostCategory::Complaint),
            "SUGGESTION" => Ok(PostCategory::Suggestion),
            "CELEBRATION" => Ok(PostCategory::Celebration),
            "QUESTION" => Ok(PostCategory::Question),
            "SUPPORT_NEEDED" => Ok(PostCategory::SupportNeeded),
            _ => Ok(PostCategory::Complaint), // default
        }
    }
}

// migrations/05_wow_features.sql - додати:

CREATE TYPE post_category AS ENUM (
    'COMPLAINT',
    'SUGGESTION',
    'CELEBRATION',
    'QUESTION',
    'SUPPORT_NEEDED'
);

ALTER TABLE wall_posts
ADD COLUMN category post_category,
ADD COLUMN ai_categorized BOOLEAN DEFAULT false;

CREATE INDEX idx_wall_posts_category ON wall_posts(category);

// src/web/wall.rs - оновити create_post:

pub async fn create_post(
    State(state): State<SharedState>,
    UserSession(user_id): UserSession,
    Json(payload): Json<CreateWallPostRequest>,
) -> Result<Json<WallPost>, StatusCode> {
    // Existing validation...

    // NEW: AI categorization
    let categorizer = WallPostCategorizer::new(state.ai.clone());
    let category = categorizer.categorize(&payload.content)
        .await
        .unwrap_or(PostCategory::Complaint);

    let post = db::insert_wall_post(
        &state.pool,
        user_id,
        &payload.content,
        payload.is_anonymous,
        Some(category),
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(post))
}
```

---

## 🎯 ФУНКЦІЯ #17: Kudos System

### Логіка:
```
/kudos @Jane - дякую за підтримку! 💙
→ Jane отримує повідомлення
→ Kudos зберігається в БД
→ Weekly summary показує kudos count
```

### Імплементація:

```rust
// src/bot/enhanced_handlers.rs

pub async fn handle_kudos_command(
    bot: Bot,
    state: SharedState,
    msg: Message,
    args: String,
) -> Result<()> {
    let chat_id = msg.chat.id;

    // Get sender user_id
    let from_user = db::get_user_by_telegram_id(&state.pool, chat_id.0)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Sender not registered"))?;

    // Parse: @email message
    let parts: Vec<&str> = args.splitn(2, ' ').collect();
    if parts.len() < 2 {
        bot.send_message(
            chat_id,
            "❌ Формат: /kudos @email твоє повідомлення\n\n\
            Приклад: /kudos @jane.davydiuk@opslab.uk дякую за допомогу! 💙"
        ).await?;
        return Ok(());
    }

    let recipient_email = parts[0].trim_start_matches('@');
    let kudos_message = parts[1];

    // Find recipient
    let recipient = db::get_user_by_email(&state.pool, recipient_email)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Recipient not found"))?;

    if from_user.id == recipient.id {
        bot.send_message(chat_id, "😅 Не можна давати kudos собі!")
            .await?;
        return Ok(());
    }

    // Save kudos
    db::insert_kudos(
        &state.pool,
        from_user.id,
        recipient.id,
        kudos_message,
    ).await?;

    // Notify sender
    bot.send_message(
        chat_id,
        format!("✅ Kudos відправлено {}! 🎉", recipient_email)
    ).await?;

    // Notify recipient (if has Telegram)
    if let Some(recipient_tg_id) = recipient.telegram_id {
        let sender_name = state.crypto.decrypt_str(&from_user.enc_name)
            .unwrap_or_else(|_| "Colleague".to_string());

        bot.send_message(
            ChatId(recipient_tg_id),
            format!(
                "🎉 *Kudos від {}!*\n\n\
                {}\n\n\
                _Продовжуй в тому ж дусі!_ 💪",
                sender_name,
                kudos_message
            )
        )
        .parse_mode(ParseMode::Markdown)
        .await?;
    }

    Ok(())
}

// src/db/mod.rs

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

pub async fn get_kudos_count_for_week(
    pool: &PgPool,
    user_id: Uuid
) -> Result<i64> {
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

pub async fn get_recent_kudos(
    pool: &PgPool,
    user_id: Uuid,
    limit: i64
) -> Result<Vec<KudosRecord>> {
    let records = sqlx::query_as!(
        KudosRecord,
        r#"
        SELECT k.id, k.from_user_id, k.to_user_id, k.message, k.created_at,
               u.enc_name as from_user_enc_name
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

#[derive(Debug)]
pub struct KudosRecord {
    pub id: Uuid,
    pub from_user_id: Uuid,
    pub to_user_id: Uuid,
    pub message: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub from_user_enc_name: Vec<u8>,
}

// Weekly summary - додати kudos:
// У WeeklySummary::format_telegram_message():

let kudos_count = db::get_kudos_count_for_week(pool, self.user_id).await?;
if kudos_count > 0 {
    msg.push_str(&format!("\n🎉 *Kudos отримано:* {} цього тижня!\n", kudos_count));

    let recent = db::get_recent_kudos(pool, self.user_id, 3).await?;
    for kudos in recent {
        let from_name = crypto.decrypt_str(&kudos.from_user_enc_name)?;
        msg.push_str(&format!("• {} від {}\n", kudos.message, from_name));
    }
}
```

---

## 📋 SUMMARY: Файли до створення/оновлення

### Нові файли:
1. `migrations/05_wow_features.sql` - всі нові таблиці
2. `src/bot/weekly_summary.rs` - weekly summaries
3. `src/analytics/correlations.rs` - correlation insights
4. `src/ai/voice_coach.rs` - voice AI coach
5. `src/ai/categorizer.rs` - wall post categorization

### Файли для оновлення:
1. `src/bot/enhanced_handlers.rs`:
   - `get_emoji_reaction()` (#4)
   - `send_quick_actions()` (#5)
   - `handle_settime_command()` (#2)
   - `handle_kudos_command()` (#17)
   - `handle_voice()` - оновити для voice coach (#11)

2. `src/bot/daily_checkin.rs`:
   - `AdaptiveQuestionEngine` (#1)
   - `generate_adaptive_checkin()` (#1)

3. `src/db/mod.rs`:
   - Багато нових функцій (детально вище)

4. `src/web/admin.rs`:
   - `get_team_heatmap()` (#8)

5. `src/web/wall.rs`:
   - Оновити `create_post()` для AI categorization (#12)

6. `src/main.rs`:
   - Оновити scheduler для smart reminders (#2)
   - Додати Friday 17:00 job (#6)

7. `src/lib.rs`:
   - Додати нові модулі

8. Frontend (web/src):
   - `TeamHeatmap.tsx` (#8)

---

## ⚡ ПОРЯДОК ІМПЛЕМЕНТАЦІЇ

### Phase 1: Database & Core (Priority 1)
1. Створити `migrations/05_wow_features.sql`
2. Оновити `src/db/mod.rs` з усіма функціями

### Phase 2: Bot Features (Priority 2)
3. Adaptive Questions (#1)
4. Emoji Reactions (#4)
5. Quick Actions (#5)
6. Smart Reminders (#2)
7. Kudos System (#17)

### Phase 3: Analytics (Priority 3)
8. Weekly Summary (#6 + #10)
9. Correlation Insights (#7)
10. Voice AI Coach (#11)

### Phase 4: Admin & UI (Priority 4)
11. Team Heatmap (#8)
12. Wall Categorization (#12)

---

**ПЛАН ГОТОВИЙ. Всі 11 WOW-функцій детально розплановано. Готовий до імплементації! 🚀**
