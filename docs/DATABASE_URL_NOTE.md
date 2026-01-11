# ⚠️ DATABASE_URL - Important Note

## Internal vs External URL

Railway надає **два різних URL** для PostgreSQL:

### 1. Internal URL (для додатків на Railway)
```
postgresql://postgres:ZYStivuqwNNVTPrNZkfAOWyqylnMDuXX@postgres.railway.internal:5432/railway
```
- ✅ Використовується в Railway Variables
- ✅ Працює всередині Railway network
- ❌ НЕ працює з локальної машини

### 2. External URL (для локального підключення)
```
postgresql://postgres:ZYStivuqwNNVTPrNZkfAOWyqylnMDuXX@roundhouse.proxy.rlwy.net:12345/railway
```
- ✅ Працює з локальної машини
- ✅ Потрібен для запуску `./GENERATE_LOCKFILE.sh`
- ✅ Потрібен для `cargo sqlx prepare`

## Як отримати External URL

1. Відкрийте Railway Dashboard
2. Перейдіть до PostgreSQL database
3. Клацніть **"Connect"** або **"Variables"** tab
4. Знайдіть **"Public Network URL"** або **"External URL"**

Виглядає приблизно так:
```
postgresql://postgres:PASSWORD@roundhouse.proxy.rlwy.net:PORT/railway
```

## Що робити далі

Для генерації `sqlx-data.json` потрібен **External URL**.

**Надайте мені External/Public URL**, і я:
1. Оновлю `.env`
2. Запущу міграції
3. Згенерую `sqlx-data.json`
4. Закомічу на GitHub

Або якщо хочете зробити самі:
```bash
# 1. Замініть DATABASE_URL на external URL
export DATABASE_URL="postgresql://postgres:PASSWORD@roundhouse.proxy.rlwy.net:PORT/railway"

# 2. Запустіть скрипт
./GENERATE_LOCKFILE.sh

# 3. Закомітьте
git add sqlx-data.json
git commit -m "Add real sqlx-data.json from Railway Postgres"
git push origin main
```

## Railway Variables

В Railway Dashboard → Variables використовуйте **Internal URL**:
```
DATABASE_URL=postgresql://postgres:ZYStivuqwNNVTPrNZkfAOWyqylnMDuXX@postgres.railway.internal:5432/railway
```

Це правильно! Railway app зможе підключитися всередині мережі.
