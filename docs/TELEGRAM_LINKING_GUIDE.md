# 🔗 Автоматичне зв'язування Telegram - Інструкція

## ✅ Реалізовано!

Система автоматичного зв'язування Telegram ID через PIN-код повністю реалізована!

---

## 📋 Що додано:

### 1. Database Migration
**Файл:** `migrations/04_telegram_pins.sql`
- Таблиця `telegram_pins` для зберігання PIN-кодів
- PIN дійсний 5 хвилин
- Автоматична очистка старих PIN
- Indexes для швидкого пошуку

### 2. Database Functions
**Файл:** `src/db/mod.rs` (lines 231-340)
- `generate_telegram_pin()` - генерує 4-digit PIN
- `verify_and_link_telegram()` - перевіряє PIN і зв'язує Telegram ID
- `get_active_pin()` - отримує активний PIN для відображення

### 3. Bot Handler
**Файл:** `src/bot/enhanced_handlers.rs`
- `/start PIN` - команда для зв'язування
- `handle_pin_verification()` - обробка PIN-коду
- Персоналізоване привітання після успішного зв'язування

### 4. Web API Endpoints
**Файл:** `src/web/telegram.rs` (NEW)
- `POST /telegram/generate-pin` - генерує новий PIN
- `GET /telegram/status` - перевіряє статус підключення

---

## 🎯 User Flow (автоматичний)

### Крок 1: Користувач логінується на web

```
User → Web Platform
├─> Email: veronika.kukharchuk@opslab.uk
├─> Password: 4582
└─> ✅ Logged in
```

### Крок 2: Dashboard показує статус Telegram

```javascript
// Frontend викликає API:
GET /telegram/status

// Response:
{
  "connected": false,
  "telegram_id": null,
  "active_pin": null
}
```

### Крок 3: Користувач клікає "Підключити Telegram"

```javascript
// Frontend викликає:
POST /telegram/generate-pin

// Response:
{
  "pin_code": "1234",
  "expires_in_seconds": 300
}

// Dashboard показує:
┌────────────────────────────────────┐
│  ⚠️ Telegram не підключено         │
├────────────────────────────────────┤
│  Ваш PIN-код: 1234                 │
│  Дійсний: 5 хвилин                 │
│                                    │
│  Напишіть боту:                    │
│  @opslab_mindguard_bot             │
│                                    │
│  Команда: /start 1234              │
│                                    │
│  [Згенерувати новий PIN]           │
└────────────────────────────────────┘
```

### Крок 4: Користувач пише боту

```
User → Telegram → @opslab_mindguard_bot
└─> /start 1234
```

### Крок 5: Бот обробляє PIN

```rust
// Bot handler:
handle_pin_verification(bot, state, chat_id, telegram_id, "1234")

// 1. Перевіряє PIN в БД:
SELECT user_id FROM telegram_pins
WHERE pin_code = '1234'
AND used = false
AND expires_at > NOW()

// ✅ Знайдено! user_id = uuid3 (Вероніка)

// 2. Зв'язує Telegram ID:
UPDATE users
SET telegram_id = 123456789
WHERE id = uuid3

// 3. Марує PIN як використаний:
UPDATE telegram_pins
SET used = true, used_at = NOW()
WHERE pin_code = '1234'
```

### Крок 6: Бот відправляє підтвердження

```
БОТ → User (Telegram):

✅ Вітаємо, Вероніка Кухарчук!

Telegram успішно підключено до вашого акаунту!

🎉 Тепер ви будете отримувати:
• Щоденні чекіни о 10:00 AM
• Критичні сповіщення
• Можливість відправляти голосові для AI аналізу

*Доступні команди:*
/checkin - Пройти чекін зараз
/status - Переглянути свої метрики
/wall - Стіна плачу
/help - Допомога

Побачимось завтра о 10:00! 👋
```

### Крок 7: Dashboard оновлюється

```javascript
// Frontend знову викликає:
GET /telegram/status

// Response:
{
  "connected": true,
  "telegram_id": 123456789,
  "active_pin": null
}

// Dashboard показує:
┌────────────────────────────────────┐
│  ✅ Telegram підключено            │
├────────────────────────────────────┤
│  Telegram ID: 123456789            │
│  Username: @veronika_k             │
│                                    │
│  📅 Наступний чекін: завтра 10:00  │
│                                    │
│  [Відключити Telegram]             │
└────────────────────────────────────┘
```

---

## 🔒 Безпека

### PIN-код характеристики:
- ✅ 4 цифри (1000-9999)
- ✅ Дійсний 5 хвилин
- ✅ Одноразовий (автоматично деактивується)
- ✅ Можна згенерувати новий в будь-який момент
- ✅ Старі PIN автоматично інвалідуються

### Захист від зловживань:
- ✅ Кожен користувач може підключити тільки 1 Telegram
- ✅ PIN зберігається в БД, не в коді
- ✅ Після використання PIN марується як `used`
- ✅ Expired PINs автоматично очищаються

---

## 📱 Frontend Integration

### HTML приклад для Dashboard:

```html
<div id="telegram-status">
  <!-- Якщо не підключено -->
  <div class="telegram-not-connected" style="display: none;">
    <h3>⚠️ Telegram не підключено</h3>
    <p>Для отримання щоденних чекінів підключіть Telegram:</p>

    <div class="pin-display" id="pin-display" style="display: none;">
      <h2>PIN-код: <span id="pin-code">1234</span></h2>
      <p>Дійсний: <span id="pin-timer">5:00</span></p>
      <p>Напишіть боту <a href="https://t.me/opslab_mindguard_bot" target="_blank">@opslab_mindguard_bot</a></p>
      <code>/start <span id="pin-code-cmd">1234</span></code>
    </div>

    <button onclick="generatePin()">Згенерувати PIN</button>
  </div>

  <!-- Якщо підключено -->
  <div class="telegram-connected" style="display: none;">
    <h3>✅ Telegram підключено</h3>
    <p>ID: <span id="telegram-id"></span></p>
    <p>📅 Наступний чекін: завтра о 10:00</p>
  </div>
</div>

<script>
async function checkTelegramStatus() {
  const response = await fetch('/telegram/status', {
    headers: { 'Authorization': 'Bearer ' + localStorage.getItem('token') }
  });
  const data = await response.json();

  if (data.connected) {
    document.querySelector('.telegram-connected').style.display = 'block';
    document.querySelector('.telegram-not-connected').style.display = 'none';
    document.getElementById('telegram-id').textContent = data.telegram_id;
  } else {
    document.querySelector('.telegram-not-connected').style.display = 'block';
    document.querySelector('.telegram-connected').style.display = 'none';

    if (data.active_pin) {
      showPin(data.active_pin);
    }
  }
}

async function generatePin() {
  const response = await fetch('/telegram/generate-pin', {
    method: 'POST',
    headers: { 'Authorization': 'Bearer ' + localStorage.getItem('token') }
  });
  const data = await response.json();

  showPin(data.pin_code);
  startTimer(data.expires_in_seconds);
}

function showPin(pin) {
  document.getElementById('pin-display').style.display = 'block';
  document.getElementById('pin-code').textContent = pin;
  document.getElementById('pin-code-cmd').textContent = pin;
}

function startTimer(seconds) {
  let remaining = seconds;
  const timerEl = document.getElementById('pin-timer');

  const interval = setInterval(() => {
    const mins = Math.floor(remaining / 60);
    const secs = remaining % 60;
    timerEl.textContent = `${mins}:${secs.toString().padStart(2, '0')}`;

    if (--remaining < 0) {
      clearInterval(interval);
      timerEl.textContent = 'Прострочено';
      timerEl.style.color = 'red';
    }
  }, 1000);
}

// Check status on page load
checkTelegramStatus();

// Refresh status every 5 seconds (to detect when user links Telegram)
setInterval(checkTelegramStatus, 5000);
</script>
```

---

## 🧪 Testing Flow

### Test 1: Генерація PIN

```bash
# Login first
curl -X POST http://localhost:3000/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"veronika.kukharchuk@opslab.uk","password":"4582"}'

# Response: {"token":"eyJ..."}

# Generate PIN
curl -X POST http://localhost:3000/telegram/generate-pin \
  -H "Authorization: Bearer eyJ..."

# Response:
# {
#   "pin_code": "1234",
#   "expires_in_seconds": 300
# }
```

### Test 2: Зв'язування через бота

```
1. Напишіть боту в Telegram: /start 1234
2. Бот відповість: "✅ Вітаємо, Вероніка Кухарчук! Telegram успішно підключено..."
```

### Test 3: Перевірка статусу

```bash
curl http://localhost:3000/telegram/status \
  -H "Authorization: Bearer eyJ..."

# Before linking:
# {
#   "connected": false,
#   "telegram_id": null,
#   "active_pin": "1234"
# }

# After linking:
# {
#   "connected": true,
#   "telegram_id": 123456789,
#   "active_pin": null
# }
```

---

## ✅ Переваги цієї системи

1. **Безпечно** - PIN одноразовий і короткочасний
2. **Просто** - 3 кроки для користувача
3. **Автоматично** - не потрібно вручну вводити Telegram ID
4. **User-friendly** - зрозумілі інструкції
5. **Надійно** - перевірка на backend
6. **Масштабовано** - працює для будь-якої кількості користувачів

---

## 📊 Очікуваний результат

### Після деплою:

1. **Олег** логінується → генерує PIN → підключає Telegram
2. **Jane** логінується → генерує PIN → підключає Telegram
3. **Вероніка** логінується → генерує PIN → підключає Telegram
4. ... (всі 9 користувачів)

### Наступного дня о 10:00 AM:

```
Scheduler відправляє чекіни всім 9 користувачам автоматично! 🎉
```

---

## 🚀 Ready to Deploy!

Всі файли створені:
- ✅ Database migration
- ✅ Database functions
- ✅ Bot handler
- ✅ Web API endpoints
- ✅ Documentation

**Наступний крок:** Deploy на Railway і протестувати flow!

---

**Створено:** 2026-01-04
**Статус:** ✅ Готово до використання
