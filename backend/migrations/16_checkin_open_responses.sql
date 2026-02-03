-- Store open-ended check-in responses (text or voice)

CREATE TABLE IF NOT EXISTS checkin_open_responses (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    checkin_id TEXT NOT NULL,
    question_id INTEGER NOT NULL,
    question_type VARCHAR(50) NOT NULL,
    response_source TEXT NOT NULL CHECK (response_source IN ('text', 'voice')),
    enc_response TEXT NOT NULL,
    enc_ai_analysis TEXT,
    risk_score SMALLINT,
    urgent BOOLEAN NOT NULL DEFAULT false,
    audio_duration_seconds INTEGER,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_checkin_open_user_date
    ON checkin_open_responses(user_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_checkin_open_type_date
    ON checkin_open_responses(question_type, created_at DESC);

ALTER TABLE checkin_open_responses ENABLE ROW LEVEL SECURITY;

DO $$ BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_policies WHERE tablename = 'checkin_open_responses' AND policyname = 'checkin_open_select_own'
    ) THEN
        CREATE POLICY checkin_open_select_own
            ON checkin_open_responses
            FOR SELECT
            USING (user_id = current_setting('app.current_user_id', true)::UUID);
    END IF;
END $$;

DO $$ BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_policies WHERE tablename = 'checkin_open_responses' AND policyname = 'checkin_open_insert_own'
    ) THEN
        CREATE POLICY checkin_open_insert_own
            ON checkin_open_responses
            FOR INSERT
            WITH CHECK (user_id = current_setting('app.current_user_id', true)::UUID);
    END IF;
END $$;
