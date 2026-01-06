# 🚨 Critical Issues & Fixes - OpsLab Mindguard

## Status: Аудит завершено - Виявлено критичні проблеми

---

## ✅ ВИПРАВЛЕНО

### 1. ✅ Конфлікт маршрутизації (CRITICAL - FIXED)
**Проблема:** Два handler файли (`handlers.rs` і `enhanced_handlers.rs`) використовували однаковий шлях `/telegram/webhook`

**Виправлення:**
- `src/main.rs:61` - Тепер використовується тільки `bot::enhanced_handlers::routes()`
- `enhanced_handlers.rs` містить ВСІ функції з `handlers.rs` ПЛЮС нові для чекінів
- `handlers.rs` можна видалити або залишити як backup

**Файли змінені:**
- [src/main.rs:61](src/main.rs#L61)

---

## 🔴 КРИТИЧНІ ПРОБЛЕМИ (Потребують негайного виправлення)

### 2. 🔴 Відсутність управління станом чекінів (CRITICAL)

**Проблема:**
У файлі `src/bot/enhanced_handlers.rs:261-262` кожен раз при відповіді на кнопку генерується НОВИЙ чекін з НОВИМИ випадковими питаннями:

```rust
// ❌ ПРОБЛЕМА: Питання будуть різні щоразу!
let day_of_week = Utc::now().weekday().num_days_from_monday();
let checkin = CheckInGenerator::generate_checkin(user.id, day_of_week);
```

**Наслідки:**
- Користувач отримує різні питання під час одного чекіну
- Неможливо зберегти правильний `qtype` для кожної відповіді
- Metrics будуть некоректними

**Рішення (ЧАСТКОВО РЕАЛІЗОВАНО):**

1. ✅ Додано `checkin_sessions` в `AppState` ([src/state.rs:17](src/state.rs#L17))
2. ✅ Ініціалізовано в `main.rs:49` ([src/main.rs:49](src/main.rs#L49))
3. ⚠️ **ПОТРІБНО:** Оновити `enhanced_handlers.rs` для використання сесій

**Код для виправлення:**

#### А) Оновити `start_daily_checkin`:
```rust
async fn start_daily_checkin(
    bot: &teloxide::Bot,
    state: &SharedState,  // Додати параметр state
    chat_id: ChatId,
    user_id: Uuid
) -> Result<()> {
    let day_of_week = Utc::now().weekday().num_days_from_monday();
    let checkin = CheckInGenerator::generate_checkin(user_id, day_of_week);

    // ДОДАТИ: Зберегти в сесії
    {
        let mut sessions = state.checkin_sessions.write().await;
        sessions.insert(chat_id.0, checkin.clone());
    }

    // ... решта коду
}
```

#### Б) Оновити виклик в `handle_private:85`:
```rust
if text.starts_with("/checkin") {
    start_daily_checkin(bot, &state, msg.chat.id, user.id).await?;  // Додати &state
    return Ok(());
}
```

#### В) Оновити `handle_callback` для використання сесій (lines 248-324):
```rust
if data.starts_with("ans_") {
    let parts: Vec<&str> = data.split('_').collect();
    if parts.len() == 3 {
        let question_id: i32 = parts[1].parse().unwrap_or(0);
        let value: i16 = parts[2].parse().unwrap_or(0);

        if let Some(msg) = &callback.message {
            let telegram_id = msg.chat.id().0;

            // ВИКОРИСТАТИ сесію замість генерації нового чекіну
            let checkin = {
                let sessions = state.checkin_sessions.read().await;
                sessions.get(&telegram_id).cloned()
            };

            let Some(checkin) = checkin else {
                bot.answer_callback_query(&callback.id)
                    .text("❌ Сесія чекіну завершена. Натисни /checkin щоб почати")
                    .await?;
                return Ok(());
            };

            if let Ok(Some(user)) = db::find_user_by_telegram(&state.pool, telegram_id).await {
                // Знайти питання за ID (тепер гарантовано правильне)
                if let Some(question) = checkin.questions.iter().find(|q| q.id == question_id) {
                    db::insert_checkin_answer(
                        &state.pool,
                        user.id,
                        question_id,
                        &question.qtype,
                        value
                    ).await?;

                    bot.answer_callback_query(&callback.id)
                        .text(format!("✅ Відповідь збережена: {}/10", value))
                        .await?;

                    bot.delete_message(msg.chat.id(), msg.id).await.ok();

                    // Правильний next_index
                    let current_index = checkin.questions.iter()
                        .position(|q| q.id == question_id)
                        .unwrap_or(0);
                    let next_index = current_index + 1;

                    if next_index < checkin.questions.len() {
                        send_checkin_question(bot, msg.chat.id(), &checkin, next_index).await?;
                    } else {
                        // Чекін завершено - ВИДАЛИТИ з сесії
                        {
                            let mut sessions = state.checkin_sessions.write().await;
                            sessions.remove(&telegram_id);
                        }

                        bot.send_message(msg.chat.id(), "✅ *Чекін завершено!*...")
                            .parse_mode(teloxide::types::ParseMode::Markdown)
                            .await?;

                        // Перевірка метрик...
                        // ... решта коду
                    }
                }
            }
        }
    }
}
```

---

### 3. 🟡 Відсутність автоматичних міграцій (IMPORTANT)

**Проблема:**
Міграції створені, але не запускаються автоматично при деплої на Railway.

**Файли:**
- `migrations/01_init_schema.sql`
- `migrations/02_seed_users.sql`
- `migrations/03_checkin_answers.sql`

**Рішення:**

#### Варіант А: SQLx migrations (Рекомендовано)
```rust
// В main.rs після підключення до БД:
sqlx::migrate!("./migrations")
    .run(&pool)
    .await?;
```

#### Варіант Б: Railway Railway DB
Додати в `railway.toml`:
```toml
[build]
builder = "DOCKERFILE"
dockerfilePath = "Dockerfile"

[deploy]
startCommand = "/usr/local/bin/opslab-mindguard"
healthcheckPath = "/"
healthcheckTimeout = 100
restartPolicyType = "ON_FAILURE"
restartPolicyMaxRetries = 10

[migrations]
runOnStart = true
```

**Необхідні зміни в `Cargo.toml`:**
```toml
[dependencies]
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "postgres", "uuid", "chrono", "json", "migrate"] }
```

---

### 4. 🟡 Відсутність scheduler для автоматичних чекінів (IMPORTANT)

**Проблема:**
Чекіни мають надсилатися автоматично о 10:00 щодня, але scheduler не реалізований.

**Рішення:**

#### Додати залежність:
```toml
[dependencies]
tokio-cron-scheduler = "0.10"
```

#### Додати scheduler в `main.rs`:
```rust
use tokio_cron_scheduler::{JobScheduler, Job};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // ... існуючий код ...

    // Додати scheduler ПЕРЕД запуском сервера
    let scheduler = JobScheduler::new().await?;
    let shared_clone = shared.clone();

    // Щоденні чекіни о 10:00
    scheduler.add(Job::new_async("0 0 10 * * *", move |_uuid, _l| {
        let state = shared_clone.clone();
        Box::pin(async move {
            if let Err(e) = send_daily_checkins_to_all(&state).await {
                tracing::error!("Failed to send daily check-ins: {}", e);
            }
        })
    })?).await?;

    scheduler.start().await?;

    // ... запуск axum server ...
}

async fn send_daily_checkins_to_all(state: &SharedState) -> anyhow::Result<()> {
    let bot_token = std::env::var("TELEGRAM_BOT_TOKEN")?;
    let bot = teloxide::Bot::new(bot_token);

    // Отримати всіх користувачів з telegram_id
    let users = sqlx::query!(
        r#"SELECT telegram_id FROM users WHERE telegram_id IS NOT NULL AND role != 'ADMIN'"#
    )
    .fetch_all(&state.pool)
    .await?;

    for user in users {
        if let Some(telegram_id) = user.telegram_id {
            let chat_id = teloxide::types::ChatId(telegram_id);

            // Отримати user_id
            if let Ok(Some(db_user)) = db::find_user_by_telegram(&state.pool, telegram_id).await {
                if let Err(e) = bot::enhanced_handlers::start_daily_checkin(
                    &bot,
                    state,
                    chat_id,
                    db_user.id
                ).await {
                    tracing::error!("Failed to send check-in to {}: {}", telegram_id, e);
                }
            }

            // Затримка між повідомленнями (Rate limiting)
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    }

    Ok(())
}
```

**ВАЖЛИВО:** `start_daily_checkin` треба зробити `pub` в `enhanced_handlers.rs`:
```rust
pub async fn start_daily_checkin(
    bot: &teloxide::Bot,
    state: &SharedState,
    chat_id: ChatId,
    user_id: Uuid
) -> Result<()> {
```

---

### 5. 🟢 Оптимізація БД індексів (NICE TO HAVE)

**Поточний стан:** Індекси створені, але можна оптимізувати.

**Рекомендовані зміни в `migrations/03_checkin_answers.sql`:**

```sql
-- Додати composite index для швидших aggregate queries
CREATE INDEX IF NOT EXISTS idx_checkin_answers_user_type_date
    ON checkin_answers(user_id, question_type, created_at DESC);

-- Додати partial index тільки для recent data (10 днів)
CREATE INDEX IF NOT EXISTS idx_checkin_answers_recent
    ON checkin_answers(user_id, created_at DESC)
    WHERE created_at >= NOW() - INTERVAL '10 days';
```

---

## 🌐 Railway Deployment Checklist

### Змінні середовища (Railway):

```bash
# Обов'язкові
DATABASE_URL=postgresql://...         # Надається автоматично Railway
TELEGRAM_BOT_TOKEN=your_bot_token
OPENAI_API_KEY=your_openai_key
APP_ENC_KEY=base64_encoded_32_bytes   # Генерувати: openssl rand -base64 32
SESSION_KEY=base64_encoded_32_bytes   # Або використати APP_ENC_KEY

# Опціональні (для critical alerts)
ADMIN_TELEGRAM_ID=123456789          # Oleg's Telegram ID
JANE_TELEGRAM_ID=987654321           # Jane's Telegram ID
BOT_USERNAME=@mindguard_bot          # Для group mentions

# Конфігурація
BIND_ADDR=0.0.0.0:3000               # Railway PORT буде переприсвоєно автоматично
RUST_LOG=info
```

### Dockerfile перевірка:

✅ **Актуальний стан:** Dockerfile виглядає коректно:
```dockerfile
FROM rust:1.76 as builder
WORKDIR /app

# Cache deps
COPY Cargo.toml Cargo.lock* ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release || true

# Build
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates openssl && rm -rf /var/lib/apt/lists/*
WORKDIR /app

COPY --from=builder /app/target/release/opslab_mindguard /usr/local/bin/opslab-mindguard
COPY static static
COPY index.html index.html
COPY migrations migrations  # ✅ Міграції копіюються

ENV RUST_LOG=info
EXPOSE 3000
CMD ["/usr/local/bin/opslab-mindguard"]
```

### Railway.toml перевірка:

✅ **Актуальний стан:** Railway config виглядає коректно.

**Рекомендація:** Додати restart policy:
```toml
[build]
builder = "DOCKERFILE"
dockerfilePath = "Dockerfile"

[deploy]
startCommand = "/usr/local/bin/opslab-mindguard"
healthcheckPath = "/"
healthcheckTimeout = 100
restartPolicyType = "ON_FAILURE"
restartPolicyMaxRetries = 10
```

---

## 📊 Перевірка коректності метрик

### SQL функція `calculate_user_metrics`:

✅ **Актуальний стан:** Формули виглядають коректно:

- **WHO-5**: `(mood + energy + wellbeing) / 3 × 10` → 0-100 ✅
- **PHQ-9**: `(inv(mood + energy + motivation)) / 3 × 2.7` → 0-27 ✅
- **GAD-7**: `(stress + inv(focus)) / 2 × 2.1` → 0-21 ✅
- **MBI**: `(stress + workload + inv(energy + motivation)) / 4 × 10` → 0-100% ✅

### Rust функція `MetricsCalculator::calculate_metrics`:

⚠️ **MINOR ISSUE:** Rust і SQL версії трохи відрізняються в деталях.

**Рекомендація:** Використовувати тільки SQL версію (вона вже інтегрована в `db::calculate_user_metrics`).

Можна видалити Rust імплементацію в `daily_checkin.rs:254-355` або залишити для тестів.

---

## 🔧 Короткий план виправлень (Пріоритет)

### CRITICAL (Зробити ЗАРАЗ):
1. ✅ Виправлено конфлікт роутів
2. ⚠️ Впровадити session management для чекінів (код вище)
3. 🔴 Додати автозапуск міграцій (SQLx migrate)

### HIGH (Зробити до деплою):
4. 🟡 Додати scheduler для автоматичних чекінів о 10:00
5. 🟡 Зробити `start_daily_checkin` публічним
6. 🟡 Протестувати повний flow чекіну

### MEDIUM (Можна зробити після деплою):
7. 🟢 Оптимізувати БД індекси
8. 🟢 Додати cleanup для старих сесій (>24 години)
9. 🟢 Додати unit tests

---

## 🚀 Готовність до деплою

| Компонент | Статус | Примітки |
|-----------|--------|----------|
| Dockerfile | ✅ Готово | Коректний multi-stage build |
| Railway.toml | ⚠️ Потребує restart policy | Додати `restartPolicyType` |
| Міграції БД | ⚠️ Потребує авто-запуску | Додати `sqlx::migrate!()` |
| Telegram bot | ⚠️ CRITICAL BUG | Session management |
| Web API | ✅ Готово | Routes коректні |
| Check-in system | 🔴 НЕ ГОТОВО | Потрібні виправлення вище |
| Metrics calculation | ✅ Готово | SQL функції працюють |
| Scheduler | 🔴 Відсутній | Треба додати cron |

---

## 📝 Висновок

**Система майже готова**, але є **2 критичні блокери**:

1. **Session management** - без цього чекіни не працюватимуть коректно
2. **Міграції** - без автозапуску БД буде пуста

**Після виправлення цих 2 проблем система буде повністю функціональна та готова до деплою на Railway.**

**Scheduler** можна додати пізніше - поки що користувачі можуть запускати чекіни вручну через `/checkin`.

---

**Створено:** 2026-01-04
**Статус:** Аудит завершено, виявлено критичні проблеми
**Наступний крок:** Імплементувати session management + auto-migrations
