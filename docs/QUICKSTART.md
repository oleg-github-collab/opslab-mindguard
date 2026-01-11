# ⚡ OpsLab Mindguard - Швидкий старт

## 🚀 За 5 хвилин

### Крок 1: Автоматичне налаштування
```bash
./setup.sh
```

Скрипт автоматично:
- ✅ Створить базу даних
- ✅ Встановить залежності
- ✅ Налаштує .env
- ✅ Створить запускні скрипти

### Крок 2: Запустити платформу
```bash
./start_all.sh
```

### Крок 3: Відкрити в браузері
```
http://localhost:8000/api/docs
```

### Крок 4: Увійти
```
Email: work.olegkaminskyi@gmail.com
Password: 0000
```

---

## 📁 Структура файлів

```
opslab-mindguard/
├── 🚀 setup.sh                     # Автоматичне налаштування
├── 🚀 start_all.sh                 # Запуск всього
├── 🚀 start_backend.sh             # Тільки backend
├── 🚀 start_telegram_bot.sh        # Тільки бот
├── 📖 README.md                    # Основна документація
├── 📖 IMPLEMENTATION_GUIDE.md      # Детальна інструкція
├── 📖 ARCHITECTURE.md              # Архітектура системи
├── 📖 SUMMARY.md                   # Підсумок
├── 📖 QUICKSTART.md                # Цей файл
├── backend/
│   ├── main.py                     # FastAPI додаток
│   ├── config.py                   # Конфігурація
│   ├── database_schema.sql         # БД з RLS
│   ├── telegram_bot.py             # Telegram бот
│   ├── requirements.txt            # Залежності
│   └── .env.example                # Приклад конфігурації
├── scraper/
│   └── fetch_wall_data.py          # Витягування даних
├── index.html                      # Frontend
└── static/
    └── style.css                   # Neobrutal стилі
```

---

## 🔧 Команди

### Запуск
```bash
./start_all.sh              # Все разом
./start_backend.sh          # Тільки API
./start_telegram_bot.sh     # Тільки бот
```

### База даних
```bash
# Створити БД
createdb opslab_mindguard

# Запустити schema
psql -d opslab_mindguard -f backend/database_schema.sql

# Підключитися до БД
psql -d opslab_mindguard

# Перевірити таблиці
psql -d opslab_mindguard -c "\dt"

# Перевірити користувачів
psql -d opslab_mindguard -c "SELECT email, name, role FROM users;"
```

### Backend
```bash
cd backend
source venv/bin/activate

# Запустити сервер
uvicorn main:app --reload

# Запустити з логами
uvicorn main:app --reload --log-level debug

# Запустити на іншому порту
uvicorn main:app --reload --port 8080
```

### Telegram Bot
```bash
cd backend
source venv/bin/activate
python telegram_bot.py
```

### Витягування даних
```bash
cd scraper
python fetch_wall_data.py
# Результат: wall_data_extracted.json
```

---

## 🧪 Тестування

### Перевірка API
```bash
# Health check
curl http://localhost:8000/health

# Docs
open http://localhost:8000/api/docs
```

### Перевірка авторизації
```bash
# Логін
curl -X POST http://localhost:8000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "work.olegkaminskyi@gmail.com",
    "password": "0000"
  }'

# Отримаєте токен, збережіть його
export TOKEN="eyJ..."

# Перевірити поточного користувача
curl http://localhost:8000/api/auth/me \
  -H "Authorization: Bearer $TOKEN"
```

### Перевірка ізоляції даних
```bash
# Увійдіть як співробітник
curl -X POST http://localhost:8000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "kateryna.petukhova@opslab.uk",
    "password": "0000"
  }'

export EMPLOYEE_TOKEN="..."

# Спробуйте отримати чужі дані (має відмовити 403)
curl http://localhost:8000/api/metrics/team/all \
  -H "Authorization: Bearer $EMPLOYEE_TOKEN"

# Отримати свої дані (має працювати)
curl http://localhost:8000/api/metrics/my \
  -H "Authorization: Bearer $EMPLOYEE_TOKEN"
```

---

## 📊 API Endpoints

### Автентифікація
```
POST   /api/auth/login           # Вхід
POST   /api/auth/logout          # Вихід
GET    /api/auth/me              # Поточний користувач
```

### Метрики
```
GET    /api/metrics/my           # Мої метрики
GET    /api/metrics/team/all     # Всі (admin/manager)
GET    /api/metrics/monthly      # По місяцях
POST   /api/metrics              # Додати
```

### Стіна плачу
```
GET    /api/wall/posts           # Всі пости
POST   /api/wall/posts           # Новий пост
PATCH  /api/wall/posts/{id}      # Оновити
DELETE /api/wall/posts/{id}      # Видалити (admin)
```

### Адміністрування
```
GET    /api/admin/users          # Користувачі
POST   /api/admin/users          # Додати
PATCH  /api/admin/users/{id}     # Редагувати
DELETE /api/admin/users/{id}     # Деактивувати
```

---

## 🔑 Дефолтні користувачі

| Email | Пароль | Роль | Доступ |
|-------|--------|------|--------|
| work.olegkaminskyi@gmail.com | 0000 | admin | Всі дані, НЕ в аналітиці |
| jane.davydyuk@opslab.uk | 0000 | manager | Всі дані, У аналітиці |
| kateryna.petukhova@opslab.uk | 0000 | employee | Лише свої дані |
| ivanna.sakalo@opslab.uk | 0000 | employee | Лише свої дані |
| mykhailo.ivashchuk@opslab.uk | 0000 | employee | Лише свої дані |

⚠️ **ВАЖЛИВО:** Змініть всі паролі перед використанням в production!

---

## 🤖 Telegram Bot

### Налаштування

1. Створіть бота через [@BotFather](https://t.me/BotFather)
2. Отримайте token
3. Отримайте ваш Chat ID через [@userinfobot](https://t.me/userinfobot)
4. Додайте в `backend/.env`:
```env
TELEGRAM_BOT_TOKEN=123456:ABC-DEF...
TELEGRAM_ADMIN_CHAT_ID=12345678
TELEGRAM_JANE_CHAT_ID=87654321
```

### Запуск
```bash
./start_telegram_bot.sh
```

### Перевірка
Напишіть боту `/start` в Telegram

### Cron для нагадувань (П'ятниця 10:00)
```bash
crontab -e

# Додати:
0 10 * * 5 cd /path/to/backend && ./venv/bin/python -c "from telegram_bot import send_weekly_reminder; import asyncio; asyncio.run(send_weekly_reminder())"
```

---

## 🐛 Troubleshooting

### Backend не запускається
```bash
# Перевірте залежності
cd backend
source venv/bin/activate
pip install -r requirements.txt

# Перевірте .env
cat .env

# Перевірте порт
lsof -i :8000  # Має бути вільний
```

### БД не підключається
```bash
# Перевірте чи запущений PostgreSQL
pg_ctl status

# Запустіть якщо потрібно
brew services start postgresql@15  # macOS
sudo service postgresql start       # Linux

# Перевірте БД
psql -l | grep opslab_mindguard
```

### Telegram бот не відповідає
```bash
# Перевірте token
echo $TELEGRAM_BOT_TOKEN

# Перевірте чи бот запущений
ps aux | grep telegram_bot

# Перевірте логи
cd backend
python telegram_bot.py
```

### Frontend не з'єднується з backend
```bash
# Перевірте CORS в backend/.env
CORS_ORIGINS=http://localhost:3000,http://localhost:8000

# Перевірте чи backend запущений
curl http://localhost:8000/health
```

---

## 📖 Детальніше

- **Повна документація**: [README.md](README.md)
- **Покрокова інструкція**: [IMPLEMENTATION_GUIDE.md](IMPLEMENTATION_GUIDE.md)
- **Архітектура**: [ARCHITECTURE.md](ARCHITECTURE.md)
- **Підсумок**: [SUMMARY.md](SUMMARY.md)

---

## 🆘 Підтримка

При проблемах:

1. Перевірте логи: `tail -f backend/logs/app.log`
2. Перевірте БД: `psql -d opslab_mindguard -c "SELECT * FROM users;"`
3. Перевірте API: http://localhost:8000/api/docs

**Контакти:**
- Олег Камінський: work.olegkaminskyi@gmail.com

---

**Made with 🧡 by OpsLab**
