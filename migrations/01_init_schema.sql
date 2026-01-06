-- OpsLab Mindguard core schema
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS pgcrypto;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'user_role') THEN
        CREATE TYPE user_role AS ENUM ('ADMIN', 'FOUNDER', 'EMPLOYEE');
    END IF;
END$$;

CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email TEXT NOT NULL UNIQUE,
    hash TEXT NOT NULL,
    enc_name TEXT NOT NULL,
    telegram_id BIGINT UNIQUE,
    role user_role NOT NULL,
    note TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS questions (
    id INTEGER PRIMARY KEY,
    text TEXT NOT NULL,
    order_index INTEGER NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS answers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    question_id INTEGER NOT NULL REFERENCES questions (id) ON DELETE CASCADE,
    value SMALLINT NOT NULL CHECK (value BETWEEN 0 AND 3),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_answers_user ON answers (user_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_answers_question ON answers (question_id, created_at DESC);

CREATE TABLE IF NOT EXISTS voice_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    enc_transcript TEXT NOT NULL,
    enc_ai_analysis TEXT,
    risk_score SMALLINT NOT NULL DEFAULT 1,
    urgent BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_voice_logs_user ON voice_logs (user_id, created_at DESC);

CREATE TABLE IF NOT EXISTS anonymous_feedback (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    enc_message TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
