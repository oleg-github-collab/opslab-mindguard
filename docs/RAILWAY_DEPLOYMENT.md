# 🚀 OpsLab Mindguard - Railway Deployment Guide

## ✅ Готовий до деплою!

Всі 34 compilation errors виправлені. Проект компілюється успішно в offline режимі.

---

## 📋 Кроки для деплою

### 1️⃣ Автентифікація Railway CLI

```bash
cd "/Users/olehkaminskyi/Desktop/Платформа OpsLab Mindguard"
railway login
```

Це відкриє браузер для авторизації.

---

### 2️⃣ Підключення до проекту Railway

```bash
railway link
```

Обери свій існуючий проект `opslab-mindguard` зі списку.

---

### 3️⃣ Налаштування змінних оточення

#### **Спосіб 1: Використати скрипт (базові значення)**

```bash
chmod +x setup-railway-vars.sh
./setup-railway-vars.sh
```

Потім оновити реальні значення через Railway dashboard або CLI.

#### **Спосіб 2: Встановити вручну через CLI (рекомендовано)**

```bash
# 🔐 Encryption Keys (вже згенеровані)
railway variables --set APP_ENC_KEY="QSCi5HDSFq691xbRmGYQpqJupG4kRJf9s8968tAbDvQ="
railway variables --set SESSION_KEY="8TwaOtZBTGGxUlsy+v0+5JvWTIkOaLUtZpH4MaFfhkM="

# 🤖 Telegram Bot (ЗАМІНІТЬ НА РЕАЛЬНІ ЗНАЧЕННЯ!)
railway variables --set TELEGRAM_BOT_TOKEN="YOUR_TOKEN_FROM_BOTFATHER"
railway variables --set BOT_USERNAME="your_bot_username"
railway variables --set ADMIN_TELEGRAM_ID="123456789"  # Твій Telegram ID
railway variables --set JANE_TELEGRAM_ID="987654321"   # Jane's Telegram ID

# 🧠 OpenAI API (ЗАМІНІТЬ НА РЕАЛЬНИЙ КЛЮЧ!)
railway variables --set OPENAI_API_KEY="sk-your-real-openai-key"

# ⚙️ Server Configuration
railway variables --set BIND_ADDR="0.0.0.0:3000"
railway variables --set RUST_LOG="info"
railway variables --set SQLX_OFFLINE="true"
railway variables --set PRODUCTION="true"
```

**📌 Важливо:** DATABASE_URL автоматично встановлюється Railway при підключенні Postgres сервісу.

---

### 4️⃣ Перевірка змінних

```bash
railway variables
```

Переконайся що всі 11 змінних встановлені правильно.

---

### 5️⃣ Deploy!

```bash
railway up
```

Або якщо хочеш деплоїти через GitHub (краще для CI/CD):

```bash
# Railway автоматично деплоїть з GitHub після push
git push origin main
```

Railway побачить новий коміт та автоматично запустить білд.

---

## 🔑 Як отримати необхідні ключі

### Telegram Bot Token
1. Відкрий [@BotFather](https://t.me/botfather) в Telegram
2. Відправ `/newbot` або `/token` для існуючого бота
3. Скопіюй токен формату: `1234567890:ABCdefGHIjklMNOpqrsTUVwxyz`

### Bot Username
- Ім'я бота без `@`, наприклад: `mindguard_bot`

### Admin Telegram ID
1. Відправ повідомлення [@userinfobot](https://t.me/userinfobot)
2. Скопіюй свій ID (число)

### Jane Telegram ID
- Попроси Jane відправити повідомлення [@userinfobot](https://t.me/userinfobot)

### OpenAI API Key
1. Зайди на [platform.openai.com/api-keys](https://platform.openai.com/api-keys)
2. Створи новий ключ
3. Скопіюй ключ формату: `sk-proj-...`

---

## 📊 Перевірка DATABASE_URL

Railway автоматично встановлює `DATABASE_URL` коли ти додаєш Postgres сервіс до проекту.

Перевір що він встановлений:

```bash
railway variables | grep DATABASE_URL
```

Повинен бути формату:
```
DATABASE_URL=postgresql://postgres:password@hostname.railway.app:5432/railway
```

Якщо його немає:
1. Зайди в Railway Dashboard
2. Додай "New Service" → "Database" → "PostgreSQL"
3. Railway автоматично створить змінну

---

## 🏗️ Архітектура деплою на Railway

Railway створить 2 сервіси:

1. **PostgreSQL Database**
   - Автоматично надає `DATABASE_URL`
   - Backup та моніторинг вбудовані

2. **Web Service (Rust)**
   - Білдиться з Dockerfile
   - Використовує SQLX offline mode
   - Автоматичний HTTPS

---

## 🔍 Моніторинг після деплою

```bash
# Дивитись логи в реальному часі
railway logs

# Статус деплою
railway status

# Відкрити проект в браузері
railway open
```

---

## 🐛 Troubleshooting

### Білд падає з SQLX помилкою
- ✅ **Вирішено:** SQLX_OFFLINE=true та .sqlx cache вже згенерований

### Connection refused до PostgreSQL
- Перевір що PostgreSQL сервіс доданий до проекту
- Перевір що DATABASE_URL встановлений Railway

### Bot не відповідає
1. Перевір TELEGRAM_BOT_TOKEN
2. Перевір логи: `railway logs`
3. Перевір що вебхук не встановлений в іншому місці:
   ```bash
   curl https://api.telegram.org/bot<YOUR_TOKEN>/getWebhookInfo
   ```

---

## 📦 Що вже зроблено

✅ Всі 34 compilation errors виправлені
✅ SQLX offline cache згенерований (.sqlx/*.json)
✅ Dockerfile оптимізований для Railway
✅ Cargo.lock закомічений
✅ Код запушений на GitHub
✅ Encryption keys згенеровані

---

## 🎯 Наступні кроки після деплою

1. **Налаштувати Telegram бота:**
   - Встановити опис: `/setdescription`
   - Встановити команди: `/setcommands`
   - Додати бота в груповий чат

2. **Створити першого юзера:**
   ```bash
   # Через Railway shell
   railway run bash
   # В контейнері запустити seed скрипт або створити через API
   ```

3. **Тестування:**
   - Відправити `/start` боту в особистих повідомленнях
   - Перевірити voice message транскрипцію
   - Перевірити daily check-in

---

## 📞 Підтримка

Якщо щось не працює:
1. Перевір логи: `railway logs`
2. Перевір змінні: `railway variables`
3. Перевір статус: `railway status`

---

**Готовий до запуску! 🚀**

Всі compilation errors виправлені. Проект компілюється успішно.
Залишилось тільки:
1. `railway login`
2. `railway link`
3. Встановити змінні
4. `railway up`
