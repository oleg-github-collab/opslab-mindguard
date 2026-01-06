# 🚀 OpsLab Mindguard - Інструкція з впровадження

## 📋 Що вже створено

### ✅ Готові компоненти:

1. **Структура БД** (`backend/database_schema.sql`)
   - Таблиці з Row Level Security
   - Автоматичні тригери для розрахунку ризиків
   - Сповіщення для адміністраторів

2. **Backend skeleton** (`backend/`)
   - FastAPI конфігурація
   - Requirements.txt
   - Структура папок

3. **Скрипт витягування даних** (`scraper/fetch_wall_data.py`)
   - Автоматичне витягування даних зі Стіни плачу

4. **Документація** (`ARCHITECTURE.md`)
   - Повний опис системи

---

## 🔧 Покрокова інструкція

### Крок 1: Налаштування бази даних

```bash
# 1. Встановіть PostgreSQL (якщо ще не встановлено)
brew install postgresql@15  # macOS
# або
sudo apt install postgresql-15  # Linux

# 2. Створіть базу даних
createdb opslab_mindguard

# 3. Запустіть schema
psql -d opslab_mindguard -f backend/database_schema.sql

# 4. Перевірте
psql -d opslab_mindguard -c "\dt"  # має показати таблиці
```

### Крок 2: Витягнути дані зі Стіни плачу

```bash
cd scraper
pip install requests
python fetch_wall_data.py

# Дані будуть збережені в wall_data_extracted.json
# Імпортуйте їх в БД вручну або через скрипт
```

### Крок 3: Backend налаштування

```bash
cd backend

# 1. Створіть віртуальне середовище
python3 -m venv venv
source venv/bin/activate  # Linux/Mac
# або
venv\Scripts\activate  # Windows

# 2. Встановіть залежності
pip install -r requirements.txt

# 3. Створіть .env файл
cp .env.example .env
# Відредагуйте .env та додайте свої значення:
nano .env
```

**Приклад .env:**
```env
DATABASE_URL=postgresql://localhost:5432/opslab_mindguard
SECRET_KEY=$(openssl rand -hex 32)
TELEGRAM_BOT_TOKEN=your_bot_token_from_@BotFather
TELEGRAM_ADMIN_CHAT_ID=your_telegram_id
TELEGRAM_JANE_CHAT_ID=jane_telegram_id
```

### Крок 4: Імпорт історичних даних

Створіть скрипт `backend/import_data.py`:

```python
import json
import asyncio
from sqlalchemy import create_engine, text
from config import settings

# Ваш JSON з метриками
DATA = {
  # ... ваш JSON з початкового повідомлення
}

async def import_metrics():
    engine = create_engine(settings.DATABASE_URL)

    with engine.connect() as conn:
        # Імпорт співробітників
        for emp in DATA["employees"]:
            # Знайти user_id
            result = conn.execute(
                text("SELECT id FROM users WHERE email = :email"),
                {"email": emp["email"]}
            )
            user_id = result.scalar()

            if not user_id:
                print(f"Користувач {emp['email']} не знайдений!")
                continue

            # Імпорт історії по місяцях
            for month, metrics in emp["history"].items():
                if metrics["who5"] == 0:  # пропустити пусті місяці
                    continue

                conn.execute(text("""
                    INSERT INTO mental_health_metrics (
                        user_id, assessment_date, month, year,
                        who5_score, phq9_score, gad7_score, mbi_score,
                        sleep_duration, sleep_quality, work_life_balance, stress_level
                    ) VALUES (
                        :user_id, :date, :month, :year,
                        :who5, :phq9, :gad7, :mbi,
                        :sleep_dur, :sleep_qual, :wlb, :stress
                    )
                    ON CONFLICT (user_id, month, year) DO UPDATE SET
                        who5_score = EXCLUDED.who5_score,
                        phq9_score = EXCLUDED.phq9_score,
                        updated_at = NOW()
                """), {
                    "user_id": user_id,
                    "date": f"2025-{month_to_num(month)}-01",
                    "month": month,
                    "year": 2025,
                    "who5": metrics["who5"],
                    "phq9": metrics["phq9"],
                    "gad7": metrics["gad7"],
                    "mbi": metrics["mbi"],
                    "sleep_dur": metrics["sleepDuration"],
                    "sleep_qual": metrics["sleepQuality"],
                    "wlb": metrics["workLifeBalance"],
                    "stress": metrics["stressLevel"]
                })

        conn.commit()
        print("✅ Дані імпортовано!")

def month_to_num(month):
    months = {
        "august": "08", "september": "09", "october": "10",
        "november": "11", "december": "12"
    }
    return months.get(month, "01")

if __name__ == "__main__":
    asyncio.run(import_metrics())
```

Запустіть:
```bash
python import_data.py
```

### Крок 5: Запуск Backend

```bash
# Запустіть FastAPI сервер
uvicorn main:app --reload --host 0.0.0.0 --port 8000

# Перевірте:
# http://localhost:8000/health
# http://localhost:8000/api/docs  # Swagger UI
```

### Крок 6: Frontend інтеграція

Оновіть `index.html` для роботи з API замість статичних даних:

```javascript
// Замість:
const data = { ... };

// Використовуйте:
const API_BASE = "http://localhost:8000/api";
let currentUser = null;

async function login(email, password) {
    const response = await fetch(`${API_BASE}/auth/login`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ email, password })
    });

    const data = await response.json();
    currentUser = data.user;
    localStorage.setItem('token', data.access_token);

    // Завантажити дані згідно з роллю
    loadUserData();
}

async function loadUserData() {
    const token = localStorage.getItem('token');

    if (currentUser.role === 'admin' || currentUser.role === 'manager') {
        // Завантажити всі дані
        const response = await fetch(`${API_BASE}/metrics/team/all`, {
            headers: { 'Authorization': `Bearer ${token}` }
        });
        const data = await response.json();
        renderDashboard(data);
    } else {
        // Завантажити лише свої дані
        const response = await fetch(`${API_BASE}/metrics/my`, {
            headers: { 'Authorization': `Bearer ${token}` }
        });
        const data = await response.json();
        renderPersonalView(data);
    }
}
```

### Крок 7: Telegram Bot

Створіть `backend/telegram_bot.py`:

```python
import asyncio
from aiogram import Bot, Dispatcher, types
from aiogram.filters import Command
from config import settings
import logging

logging.basicConfig(level=logging.INFO)
bot = Bot(token=settings.TELEGRAM_BOT_TOKEN)
dp = Dispatcher()

@dp.message(Command("start"))
async def cmd_start(message: types.Message):
    await message.answer(
        "👋 OpsLab Mindguard Bot\n"
        "Я буду надсилати вам сповіщення про критичні метрики та нові пости."
    )

async def send_critical_alert(user_name, metrics):
    """Надсилає алерт про критичні метрики"""
    message = (
        f"🚨 КРИТИЧНО: {user_name}\n\n"
        f"WHO-5: {metrics['who5']}\n"
        f"PHQ-9: {metrics['phq9']}\n"
        f"GAD-7: {metrics['gad7']}\n"
        f"MBI: {metrics['mbi']}%\n\n"
        f"Термінова дія необхідна!"
    )

    # Надіслати Олегу
    await bot.send_message(settings.TELEGRAM_ADMIN_CHAT_ID, message)

    # Надіслати Джейн
    await bot.send_message(settings.TELEGRAM_JANE_CHAT_ID, message)

async def send_wall_post_notification(post):
    """Надсилає сповіщення про новий пост"""
    author = post.get("author_name", "Анонімний користувач")
    message = (
        f"📝 Новий запис на Стіні плачу\n\n"
        f"Автор: {author}\n"
        f"Категорія: {post['category']}\n\n"
        f"{post['content'][:200]}..."
    )

    await bot.send_message(settings.TELEGRAM_ADMIN_CHAT_ID, message)
    await bot.send_message(settings.TELEGRAM_JANE_CHAT_ID, message)

async def send_weekly_reminder():
    """Щотижневе нагадування (П'ятниця)"""
    message = (
        "🗣️ Привіт, команда!\n\n"
        "Не забудьте поділитися своїми думками на Стіні плачу цього тижня.\n"
        "Ваш відгук важливий!"
    )

    # Надіслати всій команді (отримати chat_id з БД)
    # await bot.send_message(chat_id, message)

async def main():
    await dp.start_polling(bot)

if __name__ == "__main__":
    asyncio.run(main())
```

Запустіть бот:
```bash
python telegram_bot.py
```

### Крок 8: Cron job для щотижневих нагадувань

```bash
# Відкрийте crontab
crontab -e

# Додайте (П'ятниця о 10:00):
0 10 * * 5 cd /path/to/backend && /path/to/venv/bin/python -c "from telegram_bot import send_weekly_reminder; import asyncio; asyncio.run(send_weekly_reminder())"
```

---

## 🎨 Frontend покращення

### Додайте авторизацію

```html
<!-- Додайте на початок index.html -->
<div id="login-screen" class="page">
  <div class="card" style="max-width: 400px; margin: 100px auto;">
    <h2>Вхід в OpsLab Mindguard</h2>
    <form id="login-form">
      <input type="email" id="email" placeholder="Email" required>
      <input type="password" id="password" placeholder="Пароль" required>
      <button type="submit">Увійти</button>
    </form>
    <p class="muted">Використовуйте код: 0000 для першого входу</p>
  </div>
</div>

<div id="app-screen" style="display: none;">
  <!-- Весь існуючий контент -->
</div>

<script>
document.getElementById('login-form').addEventListener('submit', async (e) => {
  e.preventDefault();

  const email = document.getElementById('email').value;
  const password = document.getElementById('password').value;

  try {
    const response = await fetch('http://localhost:8000/api/auth/login', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ email, password })
    });

    if (!response.ok) throw new Error('Невірний email або пароль');

    const data = await response.json();
    localStorage.setItem('token', data.access_token);
    localStorage.setItem('user', JSON.stringify(data.user));

    document.getElementById('login-screen').style.display = 'none';
    document.getElementById('app-screen').style.display = 'block';

    initApp(data.user);
  } catch (error) {
    alert(error.message);
  }
});
</script>
```

---

## 🧪 Тестування

### 1. Перевірка ізоляції даних

```bash
# Увійдіть як співробітник
curl -X POST http://localhost:8000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email": "kateryna.petukhova@opslab.uk", "password": "0000"}'

# Отримайте token
export TOKEN="..."

# Спробуйте отримати дані (має повернути ЛИШЕ свої):
curl http://localhost:8000/api/metrics/my \
  -H "Authorization: Bearer $TOKEN"

# Спробуйте отримати чужі дані (має відмовити):
curl http://localhost:8000/api/metrics/team/all \
  -H "Authorization: Bearer $TOKEN"
# Очікувана відповідь: 403 Forbidden
```

### 2. Перевірка адміністратора

```bash
# Увійдіть як Олег
curl -X POST http://localhost:8000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email": "work.olegkaminskyi@gmail.com", "password": "admin_password"}'

export TOKEN="..."

# Отримайте ВСІ дані (має працювати):
curl http://localhost:8000/api/metrics/team/all \
  -H "Authorization: Bearer $TOKEN"

# Перевірте що Олег НЕ в аналітиці:
# Його даних не має бути в відповіді
```

---

## 🚀 Deployment

### Option 1: Railway

```bash
# 1. Встановіть Railway CLI
npm i -g @railway/cli

# 2. Login
railway login

# 3. Ініціалізуйте проект
railway init

# 4. Додайте PostgreSQL
railway add postgresql

# 5. Встановіть змінні середовища
railway variables set SECRET_KEY=$(openssl rand -hex 32)
railway variables set TELEGRAM_BOT_TOKEN=your_token

# 6. Deploy
railway up
```

### Option 2: Docker

Створіть `Dockerfile`:

```dockerfile
FROM python:3.11-slim

WORKDIR /app

COPY backend/requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

COPY backend/ .

CMD ["uvicorn", "main:app", "--host", "0.0.0.0", "--port", "8000"]
```

```bash
docker build -t opslab-mindguard .
docker run -p 8000:8000 --env-file .env opslab-mindguard
```

---

## ✅ Чеклист готовності

- [ ] БД створена та schema запущена
- [ ] Дані зі Стіни плачу витягнуті
- [ ] Backend запущений та доступний
- [ ] Історичні дані імпортовані
- [ ] Автентифікація працює
- [ ] Ізоляція даних перевірена
- [ ] Telegram бот налаштований
- [ ] Frontend інтегрований з API
- [ ] Адмін панель працює
- [ ] Cron job для нагадувань налаштований
- [ ] Деплой виконаний

---

## 📞 Підтримка

При виникненні проблем:

1. Перевірте логи: `tail -f backend/logs/app.log`
2. Перевірте БД: `psql -d opslab_mindguard -c "SELECT * FROM users;"`
3. Перевірте API: http://localhost:8000/api/docs

**Контакти:**
- Олег Камінський: work.olegkaminskyi@gmail.com
