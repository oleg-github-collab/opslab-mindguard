# 🚀 Deployment Guide - OpsLab Mindguard

## ✅ Всі критичні виправлення завершені!

Система повністю готова до розгортання на Railway.

---

## 🔧 Що було виправлено

### ✅ CRITICAL FIXES
1. **Session Management** - Чекіни тепер зберігають стан між відповідями
2. **Automatic Migrations** - SQLx автоматично застосовує міграції при старті
3. **Daily Scheduler** - Автоматична розсилка чекінів о 10:00 AM
4. **Session Cleanup** - Автоматичне очищення старих сесій кожну годину
5. **Route Conflict** - Використовується тільки `enhanced_handlers.rs`

### ✅ IMPROVEMENTS
6. **Railway Config** - Додано restart policy
7. **Rate Limiting** - 35ms затримка між повідомленнями (Telegram limits)
8. **Logging** - Детальні логи для моніторингу
9. **Error Handling** - Graceful degradation при помилках

---

## 📋 Pre-Deployment Checklist

### 1. Підготовка Telegram Bot

1. Створіть бота через [@BotFather](https://t.me/BotFather)
   ```
   /newbot
   Назва: OpsLab Mindguard
   Username: opslab_mindguard_bot
   ```

2. Отримайте bot token (зберігайте секретно!)

3. Налаштуйте webhook (після деплою на Railway):
   ```bash
   curl -X POST "https://api.telegram.org/bot<YOUR_BOT_TOKEN>/setWebhook?url=https://<your-railway-url>/telegram/webhook"
   ```

4. Отримайте свій Telegram ID:
   - Напишіть [@userinfobot](https://t.me/userinfobot)
   - Скопіюйте свій ID

### 2. Підготовка OpenAI API Key

1. Створіть акаунт на [platform.openai.com](https://platform.openai.com)
2. Створіть API key в [API Keys](https://platform.openai.com/api-keys)
3. Переконайтесь що є credits на балансі

### 3. Генерація ключів шифрування

```bash
# Згенеруйте ключ шифрування
openssl rand -base64 32

# Збережіть вивід - це буде ваш APP_ENC_KEY та SESSION_KEY
```

---

## 🌐 Railway Deployment Steps

### Step 1: Створити новий проект

1. Зайдіть на [railway.app](https://railway.app)
2. Натисніть "New Project"
3. Виберіть "Deploy from GitHub repo"
4. Підключіть ваш GitHub репозиторій
5. Виберіть цей проект

### Step 2: Додати PostgreSQL

1. В вашому проекті натисніть "New"
2. Виберіть "Database" → "Add PostgreSQL"
3. Railway автоматично створить DATABASE_URL

### Step 3: Налаштувати змінні середовища

В налаштуваннях проекту додайте:

```bash
# ========== ОБОВ'ЯЗКОВІ ==========
TELEGRAM_BOT_TOKEN=<your_bot_token_from_botfather>
OPENAI_API_KEY=<your_openai_api_key>
APP_ENC_KEY=<generated_base64_key>
SESSION_KEY=<same_or_different_base64_key>

# ========== КРИТИЧНІ АЛЕРТИ ==========
ADMIN_TELEGRAM_ID=<oleg_telegram_id>
JANE_TELEGRAM_ID=<jane_telegram_id>
BOT_USERNAME=<your_bot_username>

# ========== ОПЦІОНАЛЬНІ ==========
RUST_LOG=info
```

**ВАЖЛИВО:** `DATABASE_URL` створюється автоматично Railway при додаванні PostgreSQL!

### Step 4: Deploy!

1. Railway автоматично розпочне деплой
2. Дочекайтесь завершення білда (5-10 хвилин)
3. Отримайте публічний URL (Settings → Generate Domain)

### Step 5: Налаштувати Telegram Webhook

```bash
curl -X POST "https://api.telegram.org/bot<YOUR_BOT_TOKEN>/setWebhook?url=https://<your-railway-domain>/telegram/webhook"
```

Перевірте статус:
```bash
curl "https://api.telegram.org/bot<YOUR_BOT_TOKEN>/getWebhookInfo"
```

Очікуваний результат:
```json
{
  "ok": true,
  "result": {
    "url": "https://your-app.railway.app/telegram/webhook",
    "has_custom_certificate": false,
    "pending_update_count": 0
  }
}
```

---

## 🧪 Testing After Deployment

### 1. Перевірка роботи бота

1. Знайдіть свого бота в Telegram
2. Натисніть `/start` - має прийти привітання
3. Спробуйте `/checkin` - має почати чекін
4. Відповідайте на питання - має зберігатись прогрес
5. Завершіть чекін - має показати підсумок

### 2. Перевірка метрик

1. Пройдіть 3-4 чекіни протягом кількох днів
2. Напишіть `/status` - має показати ваші метрики

### 3. Перевірка критичних алертів

1. Відповідайте дуже низькими балами (1-3) протягом тижня
2. Після завершення чекіну має прийти алерт адміну та менеджеру

### 4. Перевірка voice messages

1. Надішліть голосове повідомлення
2. Має прийти транскрипція та AI аналіз

---

## 📊 Monitoring

### Railway Logs

В Railway dashboard:
1. Натисніть на ваш сервіс
2. Перейдіть в "Deployments"
3. Натисніть "View Logs"

### Важливі логи для перевірки:

```
✅ Running database migrations...
✅ Scheduler started - daily check-ins at 10:00 AM, session cleanup hourly
✅ Listening on 0.0.0.0:3000
✅ Starting daily check-in broadcast...
✅ Broadcasting daily check-ins to X users
✅ Daily check-in broadcast finished: X successful, 0 failed
```

### Помилки, які можуть виникнути:

1. **"DATABASE_URL missing"** → Додайте PostgreSQL в Railway
2. **"TELEGRAM_BOT_TOKEN missing"** → Додайте змінну середовища
3. **"Failed to run database migrations"** → Перевірте DATABASE_URL
4. **"Webhook already set"** → Видаліть старий webhook:
   ```bash
   curl -X POST "https://api.telegram.org/bot<TOKEN>/deleteWebhook"
   ```

---

## 🔄 Daily Schedule

Система автоматично виконує:

| Час | Дія | Опис |
|-----|-----|------|
| 10:00 AM | Daily Check-ins | Розсилка чекінів всім користувачам |
| Кожну годину | Session Cleanup | Очищення застарілих сесій |

**Часова зона:** UTC (за замовчуванням)

Якщо потрібна інша часова зона, додайте змінну:
```bash
TZ=Europe/Kiev
```

---

## 👥 User Management

### Додавання нових користувачів

1. Користувач реєструється на вебсайті з email + password
2. Система створює користувача в БД
3. Користувач пише боту в Telegram
4. Бот автоматично зв'язує Telegram ID з email
5. З наступного дня користувач отримує чекіни

### Видалення користувача

```sql
-- Через Railway PostgreSQL:
DELETE FROM users WHERE email = 'user@example.com';
-- Всі дані користувача (voice logs, checkin answers) видаляться автоматично (CASCADE)
```

---

## 🛡️ Security Best Practices

### 1. Захист змінних середовища

- ❌ **НЕ** коммітьте `.env` файл в git
- ✅ Використовуйте Railway environment variables
- ✅ Ротуйте API keys регулярно

### 2. Database Security

- ✅ Row Level Security (RLS) увімкнено
- ✅ Шифрування sensitive даних (AES-256-GCM)
- ✅ Argon2 для паролів

### 3. Rate Limiting

- ✅ 35ms delay між Telegram повідомленнями
- ✅ Telegram має ліміт 30 msg/sec
- ✅ Система автоматично throttle

---

## 📈 Scaling Considerations

### Поточна конфігурація:

- **Max DB connections:** 10 (налаштовується в `main.rs:30`)
- **Memory sessions:** In-memory HashMap (швидко, але не persistent)
- **Scheduler:** Single instance

### Якщо кількість користувачів >100:

1. **Збільшити DB connections:**
   ```rust
   .max_connections(20) // в main.rs
   ```

2. **Redis для sessions (опціонально):**
   - Додати Redis в Railway
   - Замінити HashMap на Redis
   - Персистентні сесії між рестартами

3. **Horizontal scaling:**
   - Railway підтримує auto-scaling
   - Потрібно буде синхронізувати scheduler (leader election)

---

## 🔧 Troubleshooting

### Проблема: Чекіни не надсилаються о 10:00

**Рішення:**
1. Перевірте логи: "Scheduler started"
2. Перевірте часову зону (UTC за замовчуванням)
3. Дочекайтесь наступного дня

### Проблема: Бот не відповідає

**Рішення:**
1. Перевірте webhook: `curl https://api.telegram.org/bot<TOKEN>/getWebhookInfo`
2. Перевірте логи Railway
3. Перевстановіть webhook:
   ```bash
   curl -X POST "https://api.telegram.org/bot<TOKEN>/deleteWebhook"
   curl -X POST "https://api.telegram.org/bot<TOKEN>/setWebhook?url=<URL>/telegram/webhook"
   ```

### Проблема: Міграції не застосовуються

**Рішення:**
1. Перевірте що `migrations/` folder копіюється в Docker (є в Dockerfile:20)
2. Перевірте логи: "Running database migrations..."
3. Вручну застосуйте через Railway PostgreSQL:
   ```bash
   railway run psql < migrations/01_init_schema.sql
   railway run psql < migrations/02_seed_users.sql
   railway run psql < migrations/03_checkin_answers.sql
   ```

### Проблема: Sessions губляться

**Рішення:**
- Це нормально при рестарті сервера (in-memory)
- Користувач просто почне новий чекін
- Для persistent sessions потрібен Redis

---

## 📚 Additional Resources

- [Railway Docs](https://docs.railway.app/)
- [Telegram Bot API](https://core.telegram.org/bots/api)
- [OpenAI API Docs](https://platform.openai.com/docs)
- [SQLx Migrations](https://github.com/launchbadge/sqlx/tree/main/sqlx-cli)

---

## ✅ Final Checklist

Перед production:

- [ ] Telegram Bot створено і token отримано
- [ ] OpenAI API key створено і має credits
- [ ] Ключі шифрування згенеровані
- [ ] Railway проект створено
- [ ] PostgreSQL додано
- [ ] Всі environment variables налаштовані
- [ ] Проект задеплоєно
- [ ] Webhook налаштовано
- [ ] `/start` працює
- [ ] `/checkin` працює
- [ ] `/status` працює
- [ ] Voice messages працюють
- [ ] Daily check-ins о 10:00 працюють (дочекатись наступного дня)

---

**Створено:** 2026-01-04
**Версія:** 1.0.0
**Статус:** ✅ Готово до production

Якщо виникають питання - дивіться [CRITICAL_ISSUES_AND_FIXES.md](CRITICAL_ISSUES_AND_FIXES.md) для технічних деталей.
