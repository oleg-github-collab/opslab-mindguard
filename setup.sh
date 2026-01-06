#!/bin/bash

# ============================================================================
# OpsLab Mindguard Platform - Автоматичне налаштування
# ============================================================================

set -e  # Зупинити при помилках

echo "=============================================="
echo "🧠 OpsLab Mindguard Platform - Setup"
echo "=============================================="
echo ""

# Кольори для виводу
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Функції для логування
log_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

log_error() {
    echo -e "${RED}❌ $1${NC}"
}

log_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

log_info() {
    echo "ℹ️  $1"
}

# ============================================================================
# Крок 1: Перевірка залежностей
# ============================================================================

echo "1️⃣  Перевірка залежностей..."
echo ""

# Перевірка Python
if command -v python3 &> /dev/null; then
    PYTHON_VERSION=$(python3 --version)
    log_success "Python встановлено: $PYTHON_VERSION"
else
    log_error "Python 3 не знайдено! Встановіть Python 3.9+"
    exit 1
fi

# Перевірка PostgreSQL
if command -v psql &> /dev/null; then
    POSTGRES_VERSION=$(psql --version)
    log_success "PostgreSQL встановлено: $POSTGRES_VERSION"
else
    log_warning "PostgreSQL не знайдено!"
    read -p "Встановити PostgreSQL? (y/n) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        if [[ "$OSTYPE" == "darwin"* ]]; then
            brew install postgresql@15
        elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
            sudo apt-get install postgresql-15
        fi
        log_success "PostgreSQL встановлено"
    else
        log_error "PostgreSQL необхідний для роботи системи!"
        exit 1
    fi
fi

# Перевірка pip
if command -v pip3 &> /dev/null; then
    log_success "pip3 встановлено"
else
    log_error "pip3 не знайдено!"
    exit 1
fi

echo ""

# ============================================================================
# Крок 2: Створення бази даних
# ============================================================================

echo "2️⃣  Налаштування бази даних..."
echo ""

DB_NAME="opslab_mindguard"

read -p "Ім'я бази даних [$DB_NAME]: " input_db_name
DB_NAME=${input_db_name:-$DB_NAME}

# Перевірка чи існує БД
if psql -lqt | cut -d \| -f 1 | grep -qw $DB_NAME; then
    log_warning "База даних $DB_NAME вже існує!"
    read -p "Видалити та пересторити? (y/n) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        dropdb $DB_NAME
        log_info "База даних видалена"
    else
        log_info "Використовуємо існуючу базу даних"
    fi
fi

# Створення БД
if ! psql -lqt | cut -d \| -f 1 | grep -qw $DB_NAME; then
    createdb $DB_NAME
    log_success "База даних $DB_NAME створена"
fi

# Запуск schema
log_info "Запуск database schema..."
psql -d $DB_NAME -f backend/database_schema.sql > /dev/null 2>&1
log_success "Database schema встановлена"

echo ""

# ============================================================================
# Крок 3: Backend налаштування
# ============================================================================

echo "3️⃣  Налаштування backend..."
echo ""

cd backend

# Створення віртуального середовища
if [ ! -d "venv" ]; then
    log_info "Створення віртуального середовища..."
    python3 -m venv venv
    log_success "Віртуальне середовище створено"
else
    log_info "Віртуальне середовище вже існує"
fi

# Активація venv
source venv/bin/activate

# Встановлення залежностей
log_info "Встановлення Python залежностей..."
pip install --upgrade pip > /dev/null 2>&1
pip install -r requirements.txt > /dev/null 2>&1
log_success "Залежності встановлені"

# Створення .env файлу
if [ ! -f ".env" ]; then
    log_info "Створення .env файлу..."

    # Генерація SECRET_KEY
    SECRET_KEY=$(openssl rand -hex 32)

    # Запит даних від користувача
    read -p "Telegram Bot Token (отримайте від @BotFather): " TELEGRAM_BOT_TOKEN
    read -p "Telegram Chat ID адміна (Олег): " TELEGRAM_ADMIN_CHAT_ID
    read -p "Telegram Chat ID менеджера (Джейн): " TELEGRAM_JANE_CHAT_ID

    # Створення .env
    cat > .env << EOF
# Database
DATABASE_URL=postgresql://localhost:5432/$DB_NAME
DATABASE_POOL_SIZE=20
DATABASE_MAX_OVERFLOW=10

# Security
SECRET_KEY=$SECRET_KEY
ALGORITHM=HS256
ACCESS_TOKEN_EXPIRE_MINUTES=10080

# Application
APP_NAME=OpsLab Mindguard Platform
APP_VERSION=1.0.0
DEBUG=True
CORS_ORIGINS=http://localhost:3000,http://localhost:8000

# Telegram Bot
TELEGRAM_BOT_TOKEN=$TELEGRAM_BOT_TOKEN
TELEGRAM_ADMIN_CHAT_ID=$TELEGRAM_ADMIN_CHAT_ID
TELEGRAM_JANE_CHAT_ID=$TELEGRAM_JANE_CHAT_ID

# Wall of Complaints
WALL_API_URL=https://opslab-feedback-production.up.railway.app
WALL_API_EMAIL=work.olegkaminskyi@gmail.com
WALL_API_PASSWORD=0000
EOF

    log_success ".env файл створено"
else
    log_info ".env файл вже існує"
fi

cd ..

echo ""

# ============================================================================
# Крок 4: Встановлення дефолтних паролів
# ============================================================================

echo "4️⃣  Встановлення паролів для користувачів..."
echo ""

log_info "Встановлення дефолтного пароля '0000' для всіх користувачів..."

# Генерація bcrypt hash для пароля "0000"
HASH='$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewY5GyYqW7jI3jJi'

psql -d $DB_NAME -c "UPDATE users SET password_hash = '$HASH' WHERE password_hash = '\$2b\$12\$default' OR password_hash LIKE '%placeholder%';" > /dev/null 2>&1

log_success "Паролі встановлені (дефолтний: 0000)"
log_warning "ВАЖЛИВО: Змініть паролі в production!"

echo ""

# ============================================================================
# Крок 5: Опціонально - витягнути дані зі Стіни плачу
# ============================================================================

echo "5️⃣  Витягування даних зі Стіни плачу..."
echo ""

read -p "Витягти дані зі Стіни плачу зараз? (y/n) " -n 1 -r
echo

if [[ $REPLY =~ ^[Yy]$ ]]; then
    cd scraper
    log_info "Запуск scraper..."
    python3 fetch_wall_data.py
    log_success "Дані витягнуті в wall_data_extracted.json"
    cd ..
else
    log_info "Пропущено. Можете запустити пізніше: python scraper/fetch_wall_data.py"
fi

echo ""

# ============================================================================
# Крок 6: Створення запускних скриптів
# ============================================================================

echo "6️⃣  Створення скриптів для запуску..."
echo ""

# Скрипт для запуску backend
cat > start_backend.sh << 'EOF'
#!/bin/bash
cd backend
source venv/bin/activate
uvicorn main:app --reload --host 0.0.0.0 --port 8000
EOF

chmod +x start_backend.sh
log_success "Створено start_backend.sh"

# Скрипт для запуску Telegram бота
cat > start_telegram_bot.sh << 'EOF'
#!/bin/bash
cd backend
source venv/bin/activate
python telegram_bot.py
EOF

chmod +x start_telegram_bot.sh
log_success "Створено start_telegram_bot.sh"

# Скрипт для запуску обох
cat > start_all.sh << 'EOF'
#!/bin/bash
echo "🚀 Запуск OpsLab Mindguard Platform..."
echo ""

# Запуск backend в фоні
./start_backend.sh &
BACKEND_PID=$!
echo "✅ Backend запущено (PID: $BACKEND_PID)"

# Запуск Telegram бота в фоні
./start_telegram_bot.sh &
BOT_PID=$!
echo "✅ Telegram бот запущено (PID: $BOT_PID)"

echo ""
echo "================================================"
echo "🎉 Платформа запущена!"
echo "================================================"
echo ""
echo "📊 API: http://localhost:8000"
echo "📖 Docs: http://localhost:8000/api/docs"
echo "🤖 Telegram бот активний"
echo ""
echo "Для зупинки натисніть Ctrl+C"
echo ""

# Чекаємо на Ctrl+C
trap "kill $BACKEND_PID $BOT_PID; echo ''; echo '🛑 Платформа зупинена'; exit" INT
wait
EOF

chmod +x start_all.sh
log_success "Створено start_all.sh"

echo ""

# ============================================================================
# Фінал
# ============================================================================

echo "=============================================="
echo "✅ Налаштування завершено!"
echo "=============================================="
echo ""
log_success "База даних створена: $DB_NAME"
log_success "Backend налаштовано"
log_success "Telegram бот налаштовано"
log_success "Скрипти для запуску створені"
echo ""
echo "📋 Наступні кроки:"
echo ""
echo "1. Запустити платформу:"
echo "   ./start_all.sh"
echo ""
echo "   Або окремо:"
echo "   ./start_backend.sh      # Backend API"
echo "   ./start_telegram_bot.sh # Telegram бот"
echo ""
echo "2. Відкрити в браузері:"
echo "   http://localhost:8000/api/docs"
echo ""
echo "3. Увійти в систему:"
echo "   Email: work.olegkaminskyi@gmail.com"
echo "   Password: 0000"
echo ""
echo "4. ВАЖЛИВО для production:"
echo "   - Змінити всі паролі!"
echo "   - Увімкнути HTTPS"
echo "   - Налаштувати firewall"
echo ""
echo "📖 Детальна документація: README.md"
echo "📖 Інструкція: IMPLEMENTATION_GUIDE.md"
echo ""
log_success "Готово! 🎉"
echo ""
