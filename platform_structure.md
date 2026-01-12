# OpsLab Mindguard Platform - Повна Структура

## Архітектура

### 1. Головна Платформа
- **URL:** backend-production-e745.up.railway.app
- **Назва:** OpsLab Mindguard
- **Мова:** Українська

### 2. Розділи Платформи

#### A. MINDGUARD (Моніторинг Здоров'я)
**Призначення:** Щоденні чекіни через Telegram бот + щомісячна аналітика

**Компоненти:**
- Dashboard з метриками (WHO-5, PHQ-9, GAD-7, MBI)
- Місячні тренди (серпень - грудень 2025)
- Heatmap команди
- Інтеграція з Telegram ботом

**Дані:**
- Витягнуті з teampulse-mindguard-production.up.railway.app
- Метрики: wellbeing, depression, anxiety, burnout, sleep, stress
- 5 місяців історії (8-12/2025)

**Telegram Bot:**
- Щоденні опитування (/checkin)
- Голосові повідомлення
- Автоматичні нагадування
- Commands: /help, /checkin, /status, /weblogin, /wall, /settime

#### B. СТІНА ПЛАЧУ (Wall of Tears)
**Призначення:** Анонімний фідбек співробітників про роботу

**Компоненти:**
- Список постів з фільтрацією
- Місячна фільтрація
- Категорії (Complaint, Celebration, Support Needed, Suggestion, Question)
- Сентимент аналіз
- Теги та emotional intensity

**Дані:**
- Витягнуті з opslab-feedback-production.up.railway.app
- 6 постів з грудня 2025
- AI-generated summaries та tags
- Тематика: hiring, burnout, vacation policy, team growth

### 3. Дизайн

**Стиль:** Neobrutal
- Bold borders (3-4px black)
- Bright shadows (8px 8px 0)
- Яскраві кольори (#FF6B35, #FFB347, #00D9FF)
- Space Grotesk font

**Кольорова схема:**
- Primary: #FF6B35 (помаранчевий)
- Secondary: #FFB347 (жовтий)
- Accent: #00D9FF (блакитний)
- Success: #00F5A0 (зелений)
- Warning: #FFB800 (помаранчево-жовтий)
- Danger: #FF4B4B (червоний)

### 4. Навігація

```
[Logo: 🧠 OpsLab Mindguard] [Mindguard] [Стіна Плачу] [Вийти]
```

### 5. API Endpoints

**Auth:**
- POST /auth/login - Login with email + 4-digit code
- POST /auth/logout - Logout

**Dashboard (Mindguard):**
- GET /dashboard/me - Current user
- GET /dashboard/user/:id - User metrics
- GET /dashboard/user/:id/history - Monthly history

**Admin (Mindguard):**
- GET /admin/heatmap - Team heatmap

**Feedback (Wall of Tears):**
- GET /feedback/stats - All posts with sentiment/tags
- GET /feedback/stats/available-months - Available months
- POST /feedback/wall - Create new post
- GET /feedback/wall - Get all posts

**Telegram:**
- POST /telegram/webhook - Telegram webhook

### 6. Deployment

**Environment Variables:**
- DATABASE_URL
- ENCRYPTION_KEY
- SESSION_SECRET
- TELEGRAM_BOT_TOKEN
- OPENAI_API_KEY
- BASE_URL

**Railway Services:**
- PostgreSQL database
- Web service (Rust backend)

### 7. Data Migration

**Wall Posts to Import:**
1. Hiring challenges (negative, management)
2. Workload/burnout (mixed, workload)
3. Vacation policy (negative, management)
4. Team growth celebration (positive, team)
5. Personal growth feedback (positive, team)
6. Team development feedback (positive, team)

**Mindguard Metrics (5 months):**
- Well-being Index: 65.5
- Depression: 8.25
- Anxiety: 7.375
- Burnout: 44.8125%
- Sleep: 6.325h, quality 4.25/10
- Work-Life Balance: 4.875/10
- Stress: 23.875
- At Risk: 4 users
- Critical: 1 user
