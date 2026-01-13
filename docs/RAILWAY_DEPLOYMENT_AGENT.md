# OpsLab Mindguard — Railway Deployment (Agent Runbook)

Цей гайд призначений для агента, який розгортатиме платформу на Railway через Dockerfile.

## 1) Перед початком

Підготуйте:
- Доступ до репозиторію та потрібної гілки.
- Railway акаунт + права на проект.
- PostgreSQL сервіс у Railway (або плануєте додати).
- Telegram Bot Token + username.
- OpenAI API key.
- Encryption keys: `APP_ENC_KEY`, `SESSION_KEY` (base64).
- Публічний домен Railway (потрібен для `APP_BASE_URL` та webhook).

## 2) Створення проекту у Railway

1. Railway Dashboard → New Project → Deploy from GitHub.
2. Оберіть репозиторій з OpsLab Mindguard.
3. У Service Settings встановіть Root Directory = `backend`.
   - Це важливо, бо `backend/railway.toml` і `backend/Dockerfile` лежать у цій папці.

## 3) Додайте PostgreSQL

1. New Service → Database → PostgreSQL.
2. Railway автоматично створить змінну `DATABASE_URL`.

## 4) Environment Variables

Додайте змінні в Railway Dashboard → Variables:

Обов’язкові:
- `DATABASE_URL` (Railway додає автоматично)
- `APP_ENC_KEY` (base64)
- `SESSION_KEY` (base64)
- `TELEGRAM_BOT_TOKEN`
- `BOT_USERNAME` (без `@`)
- `OPENAI_API_KEY`
- `APP_BASE_URL` або `PUBLIC_BASE_URL` (наприклад `https://<service>.up.railway.app`)
- `PRODUCTION=true`
- `SQLX_OFFLINE=true`
- `RUST_LOG=info`

Опціональні (алерти):
- `ADMIN_TELEGRAM_ID` або `TELEGRAM_ADMIN_CHAT_ID`
- `JANE_TELEGRAM_ID` або `TELEGRAM_JANE_CHAT_ID`

Примітки:
- Railway сам встановлює `PORT`. `BIND_ADDR` не обов’язковий.
- `APP_BASE_URL` потрібен для посилань у Telegram (login / wall).

## 5) Деплой

Варіант A (GitHub):
- `git push` у вибрану гілку → Railway автоматично стартує build.

Варіант B (Railway CLI):
```bash
railway login
railway link
railway up
```

## 6) Telegram Webhook

Після деплою встановіть webhook на Railway URL:
```bash
curl "https://api.telegram.org/bot<TELEGRAM_BOT_TOKEN>/setWebhook?url=<APP_BASE_URL>/telegram/webhook"
```

Перевірка:
```bash
curl "https://api.telegram.org/bot<TELEGRAM_BOT_TOKEN>/getWebhookInfo"
```

## 7) Групові чати (важливо)

- Додайте бота в групу.
- У BotFather залиште Privacy Mode = ON (рекомендовано).
- Бот відповідає у групі тільки на:
  - `/mindguard ...`
  - @mention
  - reply на повідомлення бота
- Це гарантує, що в групі не видаються конфіденційні дані.

## 8) Пост‑деплой чек‑лист

1. `GET /health` повертає `200 OK`.
2. Web UI відкривається з браузера.
3. Вхід по email + PIN працює.
4. `/checkin`, `/status`, `/wall`, `/plan`, `/goals`, `/pulse`, `/insight` працюють.
5. Стіна плачу приймає анонімні і публічні пости.
6. Голосові опитування транскрибуються (OpenAI key).
7. Адмін панель показує heatmap + action cards.
8. Webhook активний і бот відповідає.

## 9) Міграції

Міграції запускаються автоматично при старті сервера:
- `sqlx::migrate!("./migrations")`

Якщо міграції падають — перевірте `DATABASE_URL` і права доступу.

## 10) Troubleshooting

Проблема: Bot мовчить
- Перевірте `TELEGRAM_BOT_TOKEN`
- Перевірте webhook (`getWebhookInfo`)
- Перевірте логи Railway

Проблема: Немає посилань у боті
- Перевірте `APP_BASE_URL` / `PUBLIC_BASE_URL`

Проблема: Білд SQLX
- Переконайтесь, що `SQLX_OFFLINE=true` і `.sqlx/` є в репо

Проблема: 500 на старті
- Перевірте, що `DATABASE_URL`, `APP_ENC_KEY`, `SESSION_KEY` встановлені
