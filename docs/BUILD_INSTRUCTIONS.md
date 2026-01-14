# Build Instructions - OpsLab Mindguard

## Критичні кроки перед деплоєм

### 1. Згенерувати Cargo.lock

```bash
cd "/Users/olehkaminskyi/Desktop/Платформа OpsLab Mindguard"
cargo build
```

Це створить `Cargo.lock` з закріпленими версіями залежностей.

**IMPORTANT**: Додати Cargo.lock в git:
```bash
git add Cargo.lock
git commit -m "Add Cargo.lock for deterministic builds"
```

---

### 2. SQLX Offline Mode

SQLx використовує макроси `query!()` які перевіряють SQL під час компіляції.
Це вимагає підключення до БД при збірці, що неможливо в Docker/Railway.

**Рішення**: SQLx Offline Mode

#### Крок 1: Встановити sqlx-cli

```bash
cargo install sqlx-cli --no-default-features --features postgres
```

#### Крок 2: Запустити міграції локально

```bash
# Set DATABASE_URL
export DATABASE_URL="postgresql://postgres:password@localhost:5432/mindguard"

# Run migrations
sqlx migrate run

# Verify
psql $DATABASE_URL -c "\dt"
```

#### Крок 3: Згенерувати .sqlx

```bash
# This creates .sqlx/ directory with query metadata
cargo sqlx prepare

# Check what was generated
ls -la .sqlx/
cat .sqlx/query-*.json | head -20
```

Це створить файл `.sqlx/query-<hash>.json` для кожного макросу `sqlx::query!()`.

#### Крок 4: Додати в git

```bash
git add .sqlx/
git commit -m "Add SQLx offline query data"
```

#### Крок 5: Перевірити offline build

```bash
# Set offline mode
export SQLX_OFFLINE=true

# Clean build to test
cargo clean
cargo build --release

# Should succeed without DATABASE_URL
```

---

### 3. Dockerfile Updates

Переконайтеся що Dockerfile встановлює `SQLX_OFFLINE=true`:

```dockerfile
# Build stage
FROM rust:1.75 as builder

WORKDIR /app

# Copy dependency manifests
COPY Cargo.toml Cargo.lock ./

# Copy SQLx offline data (CRITICAL!)
COPY .sqlx ./.sqlx

# Set offline mode
ENV SQLX_OFFLINE=true

# Copy source
COPY src ./src
COPY migrations ./migrations

# Build
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# ... rest of dockerfile
```

---

### 4. Railway Configuration

В Railway environment variables додати:

```bash
DATABASE_URL=<automatically_set_by_railway>
TELEGRAM_BOT_TOKEN=<your_token>
SESSION_KEY_BASE64=<generate_with_openssl>
OPENAI_API_KEY=<your_key>

# IMPORTANT: Enable offline mode
SQLX_OFFLINE=true

# Production detection (optional, Railway sets RAILWAY_ENVIRONMENT)
PRODUCTION=true
```

---

## Повний workflow для деплою

```bash
# 1. Generate Cargo.lock
cargo build

# 2. Run migrations locally
export DATABASE_URL="postgresql://localhost/mindguard"
sqlx migrate run

# 3. Generate SQLx metadata
cargo sqlx prepare

# 4. Verify offline build works
export SQLX_OFFLINE=true
cargo clean && cargo build --release

# 5. Commit everything
git add Cargo.lock .sqlx/
git commit -m "Prepare for production deployment

- Add Cargo.lock for deterministic dependencies
- Add SQLx offline metadata for database-less builds
- Enable SQLX_OFFLINE in .env.example"

# 6. Push to Railway
git push origin main
```

---

## Troubleshooting

### Error: "sqlx::query! requires DATABASE_URL"

**Cause**: SQLX_OFFLINE не встановлений або відсутній `.sqlx/` directory

**Fix**:
```bash
cargo sqlx prepare
export SQLX_OFFLINE=true
cargo build
```

### Error: "migrations failed"

**Cause**: Database schema не відповідає queries

**Fix**: Перегенерувати metadata:
```bash
cargo sqlx prepare --check  # Check for issues
cargo sqlx prepare          # Regenerate
```

### Error: "Cargo.lock conflict"

**Cause**: Різні версії залежностей

**Fix**:
```bash
cargo update
cargo build
git add Cargo.lock
git commit -m "Update dependencies"
```

---

## Continuous Integration (Optional)

### GitHub Actions workflow

```yaml
name: Build & Deploy

on:
  push:
    branches: [main]

jobs:
  build:
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

      - name: Run migrations
        env:
          DATABASE_URL: postgres://postgres:postgres@localhost/mindguard
        run: |
          sqlx database create
          sqlx migrate run

      - name: Check SQLx metadata is up-to-date
        env:
          DATABASE_URL: postgres://postgres:postgres@localhost/mindguard
        run: cargo sqlx prepare --check

      - name: Build (offline mode)
        env:
          SQLX_OFFLINE: true
        run: cargo build --release

      - name: Run tests
        run: cargo test
```

---

## Checklist перед production deploy

- [ ] `Cargo.lock` існує та в git
- [ ] `.sqlx/` directory існує та в git
- [ ] `SQLX_OFFLINE=true` в .env
- [ ] Всі міграції застосовані
- [ ] `cargo sqlx prepare` виконано
- [ ] `cargo build --release` працює без DATABASE_URL
- [ ] Environment variables налаштовані в Railway
- [ ] Secure cookies enabled (PRODUCTION=true)
- [ ] RLS policies активовані (migration 06)

---

**Готово до деплою після виконання всіх кроків!** ✅
