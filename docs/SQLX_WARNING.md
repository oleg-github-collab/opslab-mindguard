# ⚠️ SQLx offline cache (.sqlx)

## Поточний статус

`.sqlx/` — це offline cache для SQLx query-макросів. Якщо каталог відсутній або застарілий, збірка з `SQLX_OFFLINE=true` **провалиться**.

## Як згенерувати / оновити cache

1. Встановіть `DATABASE_URL` (External URL для Railway або локальна Postgres).
2. Запустіть:

```bash
cargo sqlx prepare
```

3. Закомітьте зміни:

```bash
git add .sqlx
```

## Примітка

- Docker build використовує `SQLX_OFFLINE=true`, тому `.sqlx/` має бути в репозиторії.
- Якщо тимчасово потрібна online перевірка, можна вимкнути `SQLX_OFFLINE` (не рекомендовано для CI).
