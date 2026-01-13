```
OpsLab Mindguard — Railway Deployment (Agent Runbook, MarkdownV2)

1) Перед початком
- Доступ до репозиторію та потрібної гілки
- Railway акаунт + права на проект
- PostgreSQL сервіс у Railway
- Telegram Bot Token + BOT_USERNAME (без @)
- OpenAI API key
- APP_ENC_KEY, SESSION_KEY (base64)
- Публічний домен Railway для APP_BASE_URL

2) Створення проекту у Railway
- Railway Dashboard → New Project → Deploy from GitHub
- Оберіть репозиторій OpsLab Mindguard
- Service Settings → Root Directory = backend

3) Додайте PostgreSQL
- New Service → Database → PostgreSQL
- Railway створить DATABASE_URL автоматично

4) Environment Variables (Railway → Variables)
Обов’язкові:
- DATABASE_URL
- APP_ENC_KEY (base64)
- SESSION_KEY (base64)
- TELEGRAM_BOT_TOKEN
- BOT_USERNAME (без @)
- OPENAI_API_KEY
- APP_BASE_URL або PUBLIC_BASE_URL (https://<service>.up.railway.app)
- PRODUCTION=true
- SQLX_OFFLINE=true
- RUST_LOG=info

Опціональні (алерти):
- ADMIN_TELEGRAM_ID або TELEGRAM_ADMIN_CHAT_ID
- JANE_TELEGRAM_ID або TELEGRAM_JANE_CHAT_ID

5) Деплой
Варіант A:
- git push у вибрану гілку → Railway автоматично запускає build

Варіант B (Railway CLI):
- railway login
- railway link
- railway up

6) Telegram Webhook
- Встановіть webhook:
  https://api.telegram.org/bot<TELEGRAM_BOT_TOKEN>/setWebhook?url=<APP_BASE_URL>/telegram/webhook
- Перевірте:
  https://api.telegram.org/bot<TELEGRAM_BOT_TOKEN>/getWebhookInfo

7) Групові чати (безпека)
- Додайте бота в групу
- Privacy Mode у BotFather: ON (рекомендовано)
- Бот відповідає лише на /mindguard, @mention, reply
- Персональні дані не видаються в групі

8) Привʼязка Telegram (одноразова)
- Користувач пише боту:
  /start email@opslab.uk 1234
- Код доступу видає адміністратор
- Для зміни Telegram ID потрібен адмін-ресет

9) Пост-деплой чек-лист
- GET /health → 200 OK
- Web UI відкривається
- Вхід по email + 4-значний код працює
- /checkin, /status, /wall, /plan, /goals, /pulse, /insight працюють
- OpsLab Feedback відкривається за посиланням
- Голосові повідомлення транскрибуються (OpenAI key)
- Admin панель: users + heatmap + action cards
- Advanced Analytics доступна для Admin/Founder
- Webhook активний, бот відповідає

10) Міграції
- Міграції запускаються при старті сервера
- Якщо падають: перевірте DATABASE_URL і права

11) Troubleshooting
- Bot мовчить: перевірте TELEGRAM_BOT_TOKEN + webhook
- Немає посилань: перевірте APP_BASE_URL / PUBLIC_BASE_URL
- SQLX білд: SQLX_OFFLINE=true і .sqlx/ у репо
- 500 на старті: DATABASE_URL, APP_ENC_KEY, SESSION_KEY
```
