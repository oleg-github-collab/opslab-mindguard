# 🚀 Quick Start - Production Deployment

## Потрібно 3 кроки до production

### Крок 1: Згенеруйте build артефакти (локально)

```bash
# Якщо немає Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Створіть .env з DATABASE_URL
cp .env.example .env
# Відредагуйте .env: встановіть DATABASE_URL="postgresql://..."

# Запустіть скрипт (генерує Cargo.lock + sqlx-data.json)
./GENERATE_LOCKFILE.sh
```

**Що робить скрипт:**
1. ✅ Генерує `Cargo.lock` (детермінована збірка)
2. ✅ Запускає міграції (включно з RLS policies)
3. ✅ Генерує `sqlx-data.json` (offline SQLx queries)
4. ✅ Перевіряє, що offline build працює

**Очікуваний вивід:**
```
✓ Cargo.lock - 250KB
✓ sqlx-data.json - 15KB
✓ Offline build works!
```

---

### Крок 2: Закомітьте артефакти

```bash
# Видаліть placeholder файли (якщо є)
rm -f Cargo.lock.PLACEHOLDER sqlx-data.json.PLACEHOLDER

# Додайте згенеровані файли
git add Cargo.lock sqlx-data.json

# Commit (можете скопіювати з COMMIT_MESSAGE.txt)
git commit -F COMMIT_MESSAGE.txt

# Push
git push origin main
```

---

### Крок 3: Deploy на Railway

#### A. Створіть проект
```bash
# Через Railway CLI (якщо встановлено)
railway login
railway init
railway up

# Або через Railway Dashboard:
# https://railway.app → New Project → Deploy from GitHub
```

#### B. Додайте Postgres
```
Railway Dashboard → New → Database → PostgreSQL
```

Railway автоматично встановить `DATABASE_URL`

#### C. Встановіть environment variables

```bash
# Згенеруйте ключі
openssl rand -base64 32  # APP_ENC_KEY
openssl rand -base64 32  # SESSION_KEY

# У Railway Dashboard → Variables:
APP_ENC_KEY=<generated_key_1>
SESSION_KEY=<generated_key_2>
TELEGRAM_BOT_TOKEN=<from_botfather>
OPENAI_API_KEY=<your_key>
PRODUCTION=true
SQLX_OFFLINE=true
BOT_USERNAME=mindguard_bot
ADMIN_TELEGRAM_ID=123456789
RUST_LOG=info
```

#### D. Deploy
```bash
# Railway автоматично:
# 1. Виявить Dockerfile
# 2. Збере з SQLX_OFFLINE=true (без database connection!)
# 3. Запустить міграції при старті
# 4. Задеплоїть на HTTPS
```

**URL:** `https://your-app.up.railway.app`

---

## ✅ Verification

### 1. Перевірте health
```bash
curl https://your-app.up.railway.app/
```

### 2. Тест rate limiting (login)
```bash
# Має заблокувати після 5 спроб
for i in {1..10}; do
  curl -X POST https://your-app.up.railway.app/auth/login \
    -H "Content-Type: application/json" \
    -d '{"email":"test@example.com","code":"wrong"}'
  echo ""
done

# Очікувано: перші 5 - 401, 6+ - 429 Too Many Requests
```

### 3. Тест authentication
```bash
# Спроба створити wall post без токена
curl -X POST https://your-app.up.railway.app/feedback/wall \
  -H "Content-Type: application/json" \
  -d '{"content":"Test"}' \
  -v

# Очікувано: 401 Unauthorized (не 200!)
```

### 4. Перевірте логи
```bash
railway logs --tail

# Шукайте:
# ✅ "Rate limit exceeded for IP: ..."
# ✅ "Unauthorized heatmap access attempt..."
# ✅ "RLS context set: user_id=..." (якщо активували RLS)
```

---

## 🔧 Troubleshooting

### Build fails з "sqlx::query! macro error"
```bash
# Переконайтеся:
✅ SQLX_OFFLINE=true встановлено
✅ sqlx-data.json існує і не порожній
✅ Cargo.lock існує

# Перегенеруйте:
./GENERATE_LOCKFILE.sh
git add Cargo.lock sqlx-data.json
git commit --amend --no-edit
git push --force-with-lease
```

### Migrations fail
```bash
# Railway автоматично запускає міграції
# Якщо падає, перевірте DATABASE_URL:

railway run bash
echo $DATABASE_URL
sqlx migrate run
```

### Rate limiting не працює
```bash
# Переконайтеся, що ConnectInfo працює:
# Railway передає X-Forwarded-For header

# У коді вже є ConnectInfo(addr) extractors
# Перевірте логи: "Rate limit exceeded for IP: X.X.X.X"
```

---

## 📊 Monitoring

### Metrics to watch
- Request latency (rate limiter adds ~1ms)
- 429 rate (Too Many Requests) - нормально для спамерів
- 401 rate (Unauthorized) - нормально для невалідних токенів
- 403 rate (Forbidden) - спроби доступу до admin endpoints

### Railway Dashboard
- CPU/Memory usage
- Request count
- Response times
- Error rate

---

## 🎯 Production Checklist

- [ ] ✅ Cargo.lock згенеровано
- [ ] ✅ sqlx-data.json згенеровано
- [ ] ✅ Environment variables встановлені
- [ ] ✅ Postgres database створена
- [ ] ✅ Міграції пройшли успішно
- [ ] ✅ HTTPS працює (Railway auto-provision)
- [ ] ✅ Rate limiting працює (test /auth/login)
- [ ] ✅ Authentication працює (test /feedback/wall)
- [ ] ✅ Admin protection працює (test /admin/heatmap)
- [ ] ✅ Логи показують security events

**Status: 🟢 Production Ready!**

---

## 📚 Додаткова документація

- [SECURITY_FIXES_SUMMARY.md](SECURITY_FIXES_SUMMARY.md) - Всі виправлення
- [PRODUCTION_DEPLOY.md](PRODUCTION_DEPLOY.md) - Детальний deployment guide
- [BUILD_LOCALLY.md](BUILD_LOCALLY.md) - Альтернативні методи генерації
- [.env.example](.env.example) - Всі environment variables

---

## 🆘 Support

**Проблеми з деплоєм?**
1. Перевірте Railway логи: `railway logs --tail`
2. Перевірте environment variables: Railway Dashboard → Variables
3. Перевірте Postgres: `railway run psql $DATABASE_URL`

**Проблеми з build артефактами?**
1. Див. [BUILD_LOCALLY.md](BUILD_LOCALLY.md)
2. Альтернатива: налаштуйте GitHub Actions (приклад у BUILD_LOCALLY.md)

**Security питання?**
1. Див. [SECURITY_FIXES_SUMMARY.md](SECURITY_FIXES_SUMMARY.md)
2. Перевірте RLS policies: [migrations/06_row_level_security.sql](migrations/06_row_level_security.sql)
