# 🚀 Railway Setup Guide

## Credentials готові!

Всі необхідні credentials згенеровані та збережені локально в файлі:
```
RAILWAY_ENV_VARS_PRIVATE.txt
```

**ВАЖЛИВО:** Цей файл містить секретні ключі та НЕ закомічений на GitHub з міркувань безпеки.

---

## Railway Environment Variables

Додайте ці змінні в Railway Dashboard → Variables:

### Security Keys (GENERATED)
```bash
APP_ENC_KEY=<see RAILWAY_ENV_VARS_PRIVATE.txt>
SESSION_KEY=<see RAILWAY_ENV_VARS_PRIVATE.txt>
```

### Telegram Bot
```bash
TELEGRAM_BOT_TOKEN=<see RAILWAY_ENV_VARS_PRIVATE.txt>
BOT_USERNAME=mindguard_bot
```

### OpenAI
```bash
OPENAI_API_KEY=<see RAILWAY_ENV_VARS_PRIVATE.txt>
```

### Production Flags
```bash
PRODUCTION=true
SQLX_OFFLINE=true
RUST_LOG=info
```

### Optional
```bash
ADMIN_TELEGRAM_ID=123456789  # Your Telegram user ID
```

**DATABASE_URL** - Railway надасть автоматично при додаванні PostgreSQL database.

---

## Quick Setup via Railway CLI

```bash
# Login
railway login

# Link to project
railway link

# Set all variables (values from RAILWAY_ENV_VARS_PRIVATE.txt)
railway variables set APP_ENC_KEY="..."
railway variables set SESSION_KEY="..."
railway variables set TELEGRAM_BOT_TOKEN="..."
railway variables set BOT_USERNAME="mindguard_bot"
railway variables set OPENAI_API_KEY="..."
railway variables set PRODUCTION="true"
railway variables set SQLX_OFFLINE="true"
railway variables set RUST_LOG="info"
```

---

## Next Steps

1. ✅ Create Railway project
2. ✅ Add PostgreSQL database
3. ✅ Copy env vars from `RAILWAY_ENV_VARS_PRIVATE.txt`
4. ⏳ Get external DATABASE_URL for sqlx-data.json generation
5. ⏳ Generate real sqlx-data.json
6. ⏳ Deploy

See [DATABASE_URL_NOTE.md](DATABASE_URL_NOTE.md) for important info about internal vs external URLs.
