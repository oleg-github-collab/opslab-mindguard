# 🚨 Потрібна локальна генерація артефактів

## Проблема
Cargo та Rust toolchain недоступні в поточному середовищі Claude Code.

## Рішення: Виконайте локально

### Варіант 1: Автоматичний скрипт (рекомендовано)

```bash
# 1. Переконайтеся, що встановлено Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 2. Створіть .env файл з DATABASE_URL
cp .env.example .env
# Відредагуйте .env і встановіть DATABASE_URL

# 3. Запустіть скрипт
./GENERATE_LOCKFILE.sh
```

Скрипт автоматично:
- ✅ Згенерує `Cargo.lock`
- ✅ Запустить міграції
- ✅ Згенерує `.sqlx`
- ✅ Перевірить offline збірку

### Варіант 2: Ручні команди

```bash
# 1. Cargo.lock
cargo generate-lockfile

# 2. Встановіть sqlx-cli (якщо немає)
cargo install sqlx-cli --no-default-features --features postgres

# 3. Міграції (потрібен DATABASE_URL в .env)
export DATABASE_URL="postgresql://user:password@localhost/mindguard"
sqlx database create
sqlx migrate run

# 4. SQLx metadata
cargo sqlx prepare

# 5. Перевірка
export SQLX_OFFLINE=true
cargo check
```

### Після генерації

```bash
# Видаліть placeholder файли
rm -f Cargo.lock.PLACEHOLDER

# Перевірте, що файли створені
ls -lh Cargo.lock .sqlx

# Закомітьте
git add Cargo.lock .sqlx
git commit -m "Add production build artifacts"
git push origin main
```

---

## Альтернатива: Генерація на CI/CD

Якщо у вас є GitHub Actions або інший CI:

```yaml
# .github/workflows/prepare.yml
name: Prepare Build Artifacts
on:
  push:
    branches: [main]

jobs:
  prepare:
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres:15
        env:
          POSTGRES_PASSWORD: postgres
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5

    steps:
      - uses: actions/checkout@v3

      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Install sqlx-cli
        run: cargo install sqlx-cli --no-default-features --features postgres

      - name: Generate artifacts
        env:
          DATABASE_URL: postgresql://postgres:postgres@localhost/test
        run: |
          sqlx database create
          sqlx migrate run
          cargo generate-lockfile
          cargo sqlx prepare

      - name: Commit artifacts
        run: |
          git config user.name github-actions
          git config user.email github-actions@github.com
          git add Cargo.lock .sqlx
          git commit -m "Auto-generate build artifacts" || exit 0
          git push
```

---

## Після деплою на Railway

Railway автоматично:
1. Виявить `Dockerfile`
2. Побачить `SQLX_OFFLINE=true`
3. Використає `Cargo.lock` та `.sqlx`
4. Зберe без підключення до БД
5. Задеплоїть на HTTPS

**Environment variables на Railway:**
- `DATABASE_URL` - Railway Postgres надає автоматично
- `APP_ENC_KEY` - згенеруйте: `openssl rand -base64 32`
- `SESSION_KEY` - згенеруйте: `openssl rand -base64 32`
- `TELEGRAM_BOT_TOKEN` - від BotFather
- `OPENAI_API_KEY` - ваш ключ
- `PRODUCTION=true`
- `SQLX_OFFLINE=true`

Детальніше: [PRODUCTION_DEPLOY.md](PRODUCTION_DEPLOY.md)
