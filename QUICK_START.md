# ⚡ ШВИДКИЙ СТАРТ - OpsLab Mindguard на Railway

## 🎯 3 команди до деплою

### Крок 1: Логін в Railway
```bash
cd "/Users/olehkaminskyi/Desktop/Платформа OpsLab Mindguard"
railway login
```

### Крок 2: Підключення до проекту
```bash
railway link
```
Обери проект зі списку або створи новий.

### Крок 3: Встановити змінні та деплоїти

Скопіюй і вставте всі команди одразу (замінивши YOUR_* на реальні значення):

```bash
# Встановити всі змінні
railway variables --set APP_ENC_KEY="QSCi5HDSFq691xbRmGYQpqJupG4kRJf9s8968tAbDvQ=" && \
railway variables --set SESSION_KEY="8TwaOtZBTGGxUlsy+v0+5JvWTIkOaLUtZpH4MaFfhkM=" && \
railway variables --set TELEGRAM_BOT_TOKEN="YOUR_BOT_TOKEN" && \
railway variables --set BOT_USERNAME="YOUR_BOT_USERNAME" && \
railway variables --set ADMIN_TELEGRAM_ID="YOUR_TELEGRAM_ID" && \
railway variables --set JANE_TELEGRAM_ID="JANE_TELEGRAM_ID" && \
railway variables --set OPENAI_API_KEY="YOUR_OPENAI_KEY" && \
railway variables --set BIND_ADDR="0.0.0.0:3000" && \
railway variables --set RUST_LOG="info" && \
railway variables --set SQLX_OFFLINE="true" && \
railway variables --set PRODUCTION="true"

# Перевірити
railway variables

# Деплоїти
railway up
```

---

## 🔑 Де взяти ключі

| Змінна | Як отримати |
|--------|-------------|
| `TELEGRAM_BOT_TOKEN` | [@BotFather](https://t.me/botfather) → `/newbot` або `/token` |
| `BOT_USERNAME` | Ім'я бота без @ |
| `ADMIN_TELEGRAM_ID` | [@userinfobot](https://t.me/userinfobot) → відправ будь-яке повідомлення |
| `JANE_TELEGRAM_ID` | Jane відправляє повідомлення [@userinfobot](https://t.me/userinfobot) |
| `OPENAI_API_KEY` | [platform.openai.com/api-keys](https://platform.openai.com/api-keys) → Create new key |

---

## ✅ Що вже готово

- ✅ Всі 34 compilation errors виправлені
- ✅ Код запушений на GitHub
- ✅ SQLX cache згенерований
- ✅ Encryption keys згенеровані
- ✅ Dockerfile готовий

---

## 📊 Після деплою

```bash
# Дивитись логи
railway logs

# Відкрити dashboard
railway open

# Статус
railway status
```

---

**Готово до запуску! 🚀**

Детальна інструкція: [RAILWAY_DEPLOYMENT.md](RAILWAY_DEPLOYMENT.md)
