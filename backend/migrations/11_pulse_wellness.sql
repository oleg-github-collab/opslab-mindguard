-- Pulse rooms + Wellness OS tables

-- ============================================
-- Pulse Rooms (anonymous, moderated)
-- ============================================
CREATE TABLE IF NOT EXISTS pulse_rooms (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    slug TEXT UNIQUE NOT NULL,
    title TEXT NOT NULL,
    description TEXT,
    is_active BOOLEAN NOT NULL DEFAULT true,
    require_moderation BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ DEFAULT now(),
    updated_at TIMESTAMPTZ DEFAULT now()
);

CREATE TABLE IF NOT EXISTS pulse_messages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    room_id UUID NOT NULL REFERENCES pulse_rooms(id) ON DELETE CASCADE,
    user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    enc_content BYTEA NOT NULL,
    is_anonymous BOOLEAN NOT NULL DEFAULT true,
    status TEXT NOT NULL DEFAULT 'PENDING',
    moderated_by UUID REFERENCES users(id),
    moderated_at TIMESTAMPTZ,
    moderation_reason TEXT,
    created_at TIMESTAMPTZ DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_pulse_messages_room_status
    ON pulse_messages(room_id, status, created_at DESC);

-- Seed default rooms
INSERT INTO pulse_rooms (slug, title, description, require_moderation)
VALUES
    ('tempo', 'Темп та навантаження', 'Обговорення темпу, дедлайнів, навантаження', true),
    ('process', 'Процеси та якість', 'Покращення процесів, узгодження, якість', true),
    ('conflict', 'Конфлікти та напруга', 'Складні ситуації, взаємодія, етика', true),
    ('support', 'Підтримка та культура', 'Позитивні історії, підтримка, командний дух', true)
ON CONFLICT (slug) DO NOTHING;

-- ============================================
-- Wellness OS (daily plans + goals)
-- ============================================
CREATE TABLE IF NOT EXISTS user_goal_settings (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    sleep_target SMALLINT DEFAULT 7,
    break_target SMALLINT DEFAULT 3,
    move_target SMALLINT DEFAULT 20,
    notifications_enabled BOOLEAN DEFAULT true,
    updated_at TIMESTAMPTZ DEFAULT now()
);

CREATE TABLE IF NOT EXISTS wellness_plans (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    plan_date DATE NOT NULL,
    items JSONB NOT NULL,
    completed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT now(),
    updated_at TIMESTAMPTZ DEFAULT now(),
    UNIQUE (user_id, plan_date)
);

CREATE INDEX IF NOT EXISTS idx_wellness_plans_user_date
    ON wellness_plans(user_id, plan_date DESC);

ALTER TABLE user_preferences
ADD COLUMN IF NOT EXISTS last_plan_nudge_date DATE;
