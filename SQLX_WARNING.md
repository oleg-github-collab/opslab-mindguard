# ⚠️ sqlx-data.json - Placeholder Warning

## Поточний статус

Файл `sqlx-data.json` зараз містить **мінімальний placeholder**, тому що для генерації справжнього файлу потрібна підключена PostgreSQL база даних.

```json
{
  "db": "PostgreSQL",
  "query_data": []
}
```

## Чому це може викликати проблеми

Railway build з `SQLX_OFFLINE=true` **може провалитися** через відсутність metadata для sqlx::query! макросів.

## Рішення

### Варіант 1: Згенеруйте після створення Railway Postgres (Рекомендовано)

1. Створіть проект на Railway
2. Додайте PostgreSQL database
3. Скопіюйте DATABASE_URL з Railway
4. Згенеруйте локально:

```bash
# Встановіть DATABASE_URL з Railway
export DATABASE_URL="postgresql://postgres:XXX@containers-us-west-YYY.railway.app:5432/railway"

# Згенеруйте sqlx-data.json
cargo install sqlx-cli --no-default-features --features postgres
sqlx database create
sqlx migrate run
cargo sqlx prepare --merged

# Закомітьте
git add sqlx-data.json
git commit -m "Add real sqlx-data.json from Railway Postgres"
git push origin main
```

### Варіант 2: Використайте локальну Postgres

```bash
# Встановіть Postgres локально
brew install postgresql@15
brew services start postgresql@15

# Створіть БД
createdb mindguard_dev

# Встановіть DATABASE_URL
export DATABASE_URL="postgresql://localhost/mindguard_dev"

# Запустіть скрипт
./GENERATE_LOCKFILE.sh

# Закомітьте
git add sqlx-data.json
git commit -m "Add real sqlx-data.json"
git push origin main
```

### Варіант 3: Вимкніть SQLX_OFFLINE (Не рекомендовано)

Це змусить Railway підключатися до БД під час збірки.

У Dockerfile:
```diff
- ENV SQLX_OFFLINE=true
+ # ENV SQLX_OFFLINE=true
```

**Недолік:** Збірка залежить від доступності БД, повільніша, не детермінована.

## Що відбувається зараз

- ✅ `Cargo.lock` - згенеровано (100KB)
- ⚠️ `sqlx-data.json` - placeholder (потрібна справжня БД)
- ✅ Всі інші файли готові

## Наступний крок

Коли отримаєте Railway DATABASE_URL, згенеруйте справжній `sqlx-data.json` та закомітьте його.

Інакше Railway build провалиться з помилкою типу:
```
error: no such file or directory: sqlx-data.json or query data is incomplete
```
