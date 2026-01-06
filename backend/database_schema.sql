-- ============================================================================
-- OpsLab Mindguard Platform - Database Schema
-- Ізоляція даних на рівні БД з Row Level Security (PostgreSQL)
-- ============================================================================

-- Створення таблиць

-- Таблиця користувачів з рольовою моделлю
CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    employee_code VARCHAR(10) UNIQUE,
    name VARCHAR(255) NOT NULL,
    role VARCHAR(50) NOT NULL DEFAULT 'employee',  -- 'admin', 'manager', 'employee'
    is_active BOOLEAN DEFAULT TRUE,
    is_excluded_from_analytics BOOLEAN DEFAULT FALSE, -- для адміна Олега
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW(),
    last_login TIMESTAMP,

    CONSTRAINT role_check CHECK (role IN ('admin', 'manager', 'employee'))
);

-- Індекси для швидкого пошуку
CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_employee_code ON users(employee_code);
CREATE INDEX idx_users_role ON users(role);


-- Таблиця метрик ментального здоров'я
CREATE TABLE mental_health_metrics (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    assessment_date DATE NOT NULL,
    month VARCHAR(20) NOT NULL, -- 'january', 'february', etc.
    year INTEGER NOT NULL,

    -- WHO-5 Well-Being Index (0-100)
    who5_score INTEGER CHECK (who5_score >= 0 AND who5_score <= 100),

    -- PHQ-9 Depression Scale (0-27)
    phq9_score INTEGER CHECK (phq9_score >= 0 AND phq9_score <= 27),

    -- GAD-7 Anxiety Scale (0-21)
    gad7_score INTEGER CHECK (gad7_score >= 0 AND gad7_score <= 21),

    -- MBI Burnout Index (0-100%)
    mbi_score NUMERIC(5,2) CHECK (mbi_score >= 0 AND mbi_score <= 100),

    -- Додаткові метрики
    sleep_duration NUMERIC(3,1) CHECK (sleep_duration >= 0 AND sleep_duration <= 24),
    sleep_quality INTEGER CHECK (sleep_quality >= 0 AND sleep_quality <= 10),
    work_life_balance INTEGER CHECK (work_life_balance >= 0 AND work_life_balance <= 10),
    stress_level INTEGER CHECK (stress_level >= 0 AND stress_level <= 40),

    -- Рівень ризику (автоматично розраховується)
    risk_level VARCHAR(20),
    notes TEXT,

    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW(),

    CONSTRAINT unique_user_month_year UNIQUE (user_id, month, year),
    CONSTRAINT risk_level_check CHECK (risk_level IN ('low', 'medium', 'high', 'critical'))
);

-- Індекси
CREATE INDEX idx_metrics_user_id ON mental_health_metrics(user_id);
CREATE INDEX idx_metrics_date ON mental_health_metrics(assessment_date);
CREATE INDEX idx_metrics_risk_level ON mental_health_metrics(risk_level);


-- Таблиця Стіни плачу (анонімний зворотний зв'язок)
CREATE TABLE wall_posts (
    id SERIAL PRIMARY KEY,
    user_id INTEGER REFERENCES users(id) ON DELETE SET NULL, -- може бути NULL для анонімності
    content TEXT NOT NULL,
    is_anonymous BOOLEAN DEFAULT TRUE,
    category VARCHAR(50), -- 'complaint', 'suggestion', 'praise', 'concern'
    sentiment VARCHAR(20), -- 'positive', 'neutral', 'negative'
    status VARCHAR(20) DEFAULT 'open', -- 'open', 'acknowledged', 'resolved'

    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW(),

    CONSTRAINT category_check CHECK (category IN ('complaint', 'suggestion', 'praise', 'concern', 'other')),
    CONSTRAINT sentiment_check CHECK (sentiment IN ('positive', 'neutral', 'negative')),
    CONSTRAINT status_check CHECK (status IN ('open', 'acknowledged', 'resolved', 'archived'))
);

-- Індекси
CREATE INDEX idx_wall_posts_user_id ON wall_posts(user_id);
CREATE INDEX idx_wall_posts_created_at ON wall_posts(created_at);
CREATE INDEX idx_wall_posts_status ON wall_posts(status);
CREATE INDEX idx_wall_posts_category ON wall_posts(category);


-- Таблиця коментарів до постів на Стіні плачу
CREATE TABLE wall_comments (
    id SERIAL PRIMARY KEY,
    post_id INTEGER NOT NULL REFERENCES wall_posts(id) ON DELETE CASCADE,
    user_id INTEGER REFERENCES users(id) ON DELETE SET NULL,
    content TEXT NOT NULL,
    is_admin_reply BOOLEAN DEFAULT FALSE,

    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW()
);

-- Індекси
CREATE INDEX idx_wall_comments_post_id ON wall_comments(post_id);
CREATE INDEX idx_wall_comments_user_id ON wall_comments(user_id);


-- Таблиця сповіщень для адміністраторів
CREATE TABLE admin_notifications (
    id SERIAL PRIMARY KEY,
    user_id INTEGER REFERENCES users(id) ON DELETE CASCADE,
    type VARCHAR(50) NOT NULL, -- 'critical_metric', 'new_wall_post', 'weekly_reminder'
    title VARCHAR(255) NOT NULL,
    message TEXT NOT NULL,
    related_entity_type VARCHAR(50), -- 'metric', 'wall_post', 'user'
    related_entity_id INTEGER,
    is_read BOOLEAN DEFAULT FALSE,
    is_sent_to_telegram BOOLEAN DEFAULT FALSE,

    created_at TIMESTAMP DEFAULT NOW(),
    read_at TIMESTAMP,

    CONSTRAINT notification_type_check CHECK (type IN ('critical_metric', 'new_wall_post', 'weekly_reminder', 'system'))
);

-- Індекси
CREATE INDEX idx_notifications_user_id ON admin_notifications(user_id);
CREATE INDEX idx_notifications_is_read ON admin_notifications(is_read);
CREATE INDEX idx_notifications_created_at ON admin_notifications(created_at);


-- Таблиця сесій для автентифікації
CREATE TABLE user_sessions (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    session_token VARCHAR(255) UNIQUE NOT NULL,
    ip_address VARCHAR(45),
    user_agent TEXT,
    expires_at TIMESTAMP NOT NULL,
    created_at TIMESTAMP DEFAULT NOW(),

    CONSTRAINT session_active CHECK (expires_at > NOW())
);

-- Індекси
CREATE INDEX idx_sessions_token ON user_sessions(session_token);
CREATE INDEX idx_sessions_user_id ON user_sessions(user_id);
CREATE INDEX idx_sessions_expires_at ON user_sessions(expires_at);


-- ============================================================================
-- Row Level Security (RLS) для ізоляції даних
-- ============================================================================

-- Увімкнути RLS для таблиці метрик
ALTER TABLE mental_health_metrics ENABLE ROW LEVEL SECURITY;

-- Політика: співробітники бачать лише свої дані
CREATE POLICY employee_view_own_metrics ON mental_health_metrics
    FOR SELECT
    USING (
        user_id = current_setting('app.current_user_id')::INTEGER
        OR
        EXISTS (
            SELECT 1 FROM users
            WHERE id = current_setting('app.current_user_id')::INTEGER
            AND role IN ('admin', 'manager')
        )
    );

-- Політика: співробітники можуть оновлювати лише свої дані
CREATE POLICY employee_update_own_metrics ON mental_health_metrics
    FOR UPDATE
    USING (user_id = current_setting('app.current_user_id')::INTEGER);

-- Політика: співробітники можуть створювати лише свої записи
CREATE POLICY employee_insert_own_metrics ON mental_health_metrics
    FOR INSERT
    WITH CHECK (user_id = current_setting('app.current_user_id')::INTEGER);


-- Увімкнути RLS для Стіни плачу
ALTER TABLE wall_posts ENABLE ROW LEVEL SECURITY;

-- Політика: всі можуть читати пости
CREATE POLICY all_view_wall_posts ON wall_posts
    FOR SELECT
    USING (TRUE);

-- Політика: користувачі можуть створювати пости
CREATE POLICY users_create_wall_posts ON wall_posts
    FOR INSERT
    WITH CHECK (
        EXISTS (
            SELECT 1 FROM users
            WHERE id = current_setting('app.current_user_id')::INTEGER
            AND is_active = TRUE
        )
    );

-- Політика: тільки автор або адмін може оновлювати/видаляти
CREATE POLICY owner_or_admin_update_wall_posts ON wall_posts
    FOR UPDATE
    USING (
        user_id = current_setting('app.current_user_id')::INTEGER
        OR
        EXISTS (
            SELECT 1 FROM users
            WHERE id = current_setting('app.current_user_id')::INTEGER
            AND role = 'admin'
        )
    );


-- ============================================================================
-- Функції для автоматичного розрахунку ризиків
-- ============================================================================

CREATE OR REPLACE FUNCTION calculate_risk_level(
    p_who5 INTEGER,
    p_phq9 INTEGER,
    p_gad7 INTEGER,
    p_mbi NUMERIC
) RETURNS VARCHAR AS $$
BEGIN
    -- Критичний рівень
    IF p_who5 < 50 OR p_phq9 >= 15 OR p_gad7 >= 15 OR p_mbi >= 70 THEN
        RETURN 'critical';
    END IF;

    -- Високий рівень
    IF p_who5 < 60 OR p_phq9 >= 10 OR p_gad7 >= 10 OR p_mbi >= 50 THEN
        RETURN 'high';
    END IF;

    -- Середній рівень
    IF p_who5 < 70 OR p_phq9 >= 5 OR p_gad7 >= 5 OR p_mbi >= 35 THEN
        RETURN 'medium';
    END IF;

    -- Низький рівень
    RETURN 'low';
END;
$$ LANGUAGE plpgsql;


-- Тригер для автоматичного розрахунку risk_level
CREATE OR REPLACE FUNCTION update_risk_level()
RETURNS TRIGGER AS $$
BEGIN
    NEW.risk_level := calculate_risk_level(
        NEW.who5_score,
        NEW.phq9_score,
        NEW.gad7_score,
        NEW.mbi_score
    );

    NEW.updated_at := NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_update_risk_level
BEFORE INSERT OR UPDATE ON mental_health_metrics
FOR EACH ROW
EXECUTE FUNCTION update_risk_level();


-- ============================================================================
-- Функція для створення сповіщення при критичних метриках
-- ============================================================================

CREATE OR REPLACE FUNCTION notify_admins_on_critical_metrics()
RETURNS TRIGGER AS $$
DECLARE
    admin_rec RECORD;
    employee_name VARCHAR(255);
BEGIN
    -- Перевірка чи метрики критичні
    IF NEW.risk_level = 'critical' THEN
        SELECT name INTO employee_name FROM users WHERE id = NEW.user_id;

        -- Створюємо сповіщення для всіх адмінів та менеджерів
        FOR admin_rec IN
            SELECT id FROM users WHERE role IN ('admin', 'manager') AND is_active = TRUE
        LOOP
            INSERT INTO admin_notifications (
                user_id,
                type,
                title,
                message,
                related_entity_type,
                related_entity_id
            ) VALUES (
                admin_rec.id,
                'critical_metric',
                'КРИТИЧНІ МЕТРИКИ: ' || employee_name,
                format(
                    'У співробітника %s виявлено критичні показники: WHO-5=%s, PHQ-9=%s, GAD-7=%s, MBI=%s%%',
                    employee_name,
                    NEW.who5_score,
                    NEW.phq9_score,
                    NEW.gad7_score,
                    NEW.mbi_score
                ),
                'metric',
                NEW.id
            );
        END LOOP;
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_notify_admins_on_critical
AFTER INSERT OR UPDATE ON mental_health_metrics
FOR EACH ROW
WHEN (NEW.risk_level = 'critical')
EXECUTE FUNCTION notify_admins_on_critical_metrics();


-- ============================================================================
-- Функція для сповіщення адмінів про нові пости на Стіні плачу
-- ============================================================================

CREATE OR REPLACE FUNCTION notify_admins_on_new_wall_post()
RETURNS TRIGGER AS $$
DECLARE
    admin_rec RECORD;
    author_name VARCHAR(255);
BEGIN
    -- Отримуємо ім'я автора (якщо не анонімно)
    IF NEW.is_anonymous THEN
        author_name := 'Анонімний користувач';
    ELSE
        SELECT name INTO author_name FROM users WHERE id = NEW.user_id;
    END IF;

    -- Створюємо сповіщення для Олега та Джейн
    FOR admin_rec IN
        SELECT id FROM users
        WHERE role IN ('admin', 'manager')
        AND email IN ('work.olegkaminskyi@gmail.com', 'jane.davydyuk@opslab.uk')
        AND is_active = TRUE
    LOOP
        INSERT INTO admin_notifications (
            user_id,
            type,
            title,
            message,
            related_entity_type,
            related_entity_id
        ) VALUES (
            admin_rec.id,
            'new_wall_post',
            'Новий запис на Стіні плачу',
            format(
                'Користувач %s додав новий запис (категорія: %s): %s',
                author_name,
                NEW.category,
                LEFT(NEW.content, 200)
            ),
            'wall_post',
            NEW.id
        );
    END LOOP;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_notify_admins_on_new_wall_post
AFTER INSERT ON wall_posts
FOR EACH ROW
EXECUTE FUNCTION notify_admins_on_new_wall_post();


-- ============================================================================
-- Початкові дані
-- ============================================================================

-- Створення адміністраторів
INSERT INTO users (email, password_hash, name, role, is_excluded_from_analytics, employee_code) VALUES
('work.olegkaminskyi@gmail.com', '$2b$12$placeholder_hash_1', 'Олег Камінський', 'admin', TRUE, NULL),
('jane.davydyuk@opslab.uk', '$2b$12$placeholder_hash_2', 'Джейн Давидюк', 'manager', FALSE, '4747');

-- Створення співробітників (з даних JSON)
INSERT INTO users (email, password_hash, name, role, employee_code) VALUES
('kateryna.petukhova@opslab.uk', '$2b$12$default', 'Катерина Петухова', 'employee', '1122'),
('ivanna.sakalo@opslab.uk', '$2b$12$default', 'Іванна Сакало', 'employee', '6738'),
('mykhailo.ivashchuk@opslab.uk', '$2b$12$default', 'Михайло Іващук', 'employee', '9267'),
('oksana.klinchaian@opslab.uk', '$2b$12$default', 'Оксана Клінчаян', 'employee', '8463'),
('iryna.miachkova@opslab.uk', '$2b$12$default', 'Ірина М\'ячкова', 'employee', '3814'),
('veronika.kukharchuk@opslab.uk', '$2b$12$default', 'Вероніка Кухарчук', 'employee', '4582'),
('mariya.vasylyk@opslab.uk', '$2b$12$default', 'Марія Василик', 'employee', '1425');

-- ПРИМІТКА: Реальні паролі будуть встановлені через API при ініціалізації системи
