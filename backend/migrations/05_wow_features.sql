-- WOW Features Migration
-- Features: #1, #2, #4, #5, #6, #7, #8, #10, #11, #12, #17
-- Date: 2026-01-04

-- ============================================
-- USER PREFERENCES (for Smart Reminders #2)
-- ============================================
CREATE TABLE user_preferences (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    reminder_hour SMALLINT DEFAULT 10 CHECK (reminder_hour >= 0 AND reminder_hour <= 23),
    reminder_minute SMALLINT DEFAULT 0 CHECK (reminder_minute >= 0 AND reminder_minute <= 59),
    timezone VARCHAR(50) DEFAULT 'Europe/Kiev',
    language VARCHAR(5) DEFAULT 'uk',
    notification_enabled BOOLEAN DEFAULT true,
    updated_at TIMESTAMPTZ DEFAULT now()
);

CREATE INDEX idx_user_preferences_reminder_time ON user_preferences(reminder_hour, reminder_minute);

-- ============================================
-- USER STREAKS (for Weekly Summary #6)
-- ============================================
CREATE TABLE user_streaks (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    current_streak INT DEFAULT 0,
    longest_streak INT DEFAULT 0,
    last_checkin_date DATE,
    total_checkins INT DEFAULT 0,
    milestones_reached JSONB DEFAULT '[]'::jsonb,
    updated_at TIMESTAMPTZ DEFAULT now()
);

CREATE INDEX idx_user_streaks_current ON user_streaks(current_streak DESC);

-- ============================================
-- KUDOS SYSTEM (#17)
-- ============================================
CREATE TABLE kudos (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    from_user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    to_user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    message TEXT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT now(),
    CONSTRAINT kudos_not_self CHECK (from_user_id != to_user_id)
);

CREATE INDEX idx_kudos_to_user ON kudos(to_user_id, created_at DESC);
CREATE INDEX idx_kudos_from_user ON kudos(from_user_id, created_at DESC);
CREATE INDEX idx_kudos_created_at ON kudos(created_at DESC);

-- ============================================
-- TEAM INSIGHTS CACHE (for performance #8)
-- ============================================
CREATE TABLE team_insights_cache (
    id SERIAL PRIMARY KEY,
    insight_type VARCHAR(50) NOT NULL,
    data JSONB NOT NULL,
    generated_at TIMESTAMPTZ DEFAULT now()
);

CREATE INDEX idx_insights_type_date ON team_insights_cache(insight_type, generated_at DESC);

-- ============================================
-- WALL POST CATEGORIZATION (#12)
-- ============================================
-- WOW Feature #12: Wall Posts with AI Categorization
-- ============================================

CREATE TYPE post_category AS ENUM (
    'COMPLAINT',
    'SUGGESTION',
    'CELEBRATION',
    'QUESTION',
    'SUPPORT_NEEDED'
);

-- Create wall_posts table
CREATE TABLE IF NOT EXISTS wall_posts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    enc_content BYTEA NOT NULL,
    category post_category,
    ai_categorized BOOLEAN DEFAULT false,
    created_at TIMESTAMPTZ DEFAULT now(),
    updated_at TIMESTAMPTZ DEFAULT now()
);

CREATE INDEX idx_wall_posts_user ON wall_posts(user_id);
CREATE INDEX idx_wall_posts_category ON wall_posts(category);
CREATE INDEX idx_wall_posts_created ON wall_posts(created_at DESC);

-- ============================================
-- HELPER FUNCTIONS
-- ============================================

-- Function to update user streak
CREATE OR REPLACE FUNCTION update_user_streak(p_user_id UUID, p_checkin_date DATE)
RETURNS void AS $$
DECLARE
    v_last_date DATE;
    v_current_streak INT;
    v_longest_streak INT;
BEGIN
    -- Get or create streak record
    INSERT INTO user_streaks (user_id, last_checkin_date, current_streak, longest_streak, total_checkins)
    VALUES (p_user_id, p_checkin_date, 1, 1, 1)
    ON CONFLICT (user_id) DO NOTHING;

    -- Get current streak info
    SELECT last_checkin_date, current_streak, longest_streak
    INTO v_last_date, v_current_streak, v_longest_streak
    FROM user_streaks
    WHERE user_id = p_user_id;

    -- Update streak logic
    IF v_last_date = p_checkin_date THEN
        -- Same day, no change
        RETURN;
    ELSIF v_last_date = p_checkin_date - INTERVAL '1 day' THEN
        -- Consecutive day, increment streak
        v_current_streak := v_current_streak + 1;
        v_longest_streak := GREATEST(v_longest_streak, v_current_streak);
    ELSE
        -- Streak broken, reset to 1
        v_current_streak := 1;
    END IF;

    -- Update the record
    UPDATE user_streaks
    SET
        last_checkin_date = p_checkin_date,
        current_streak = v_current_streak,
        longest_streak = v_longest_streak,
        total_checkins = total_checkins + 1,
        updated_at = now()
    WHERE user_id = p_user_id;
END;
$$ LANGUAGE plpgsql;

-- Function to get team average metrics (for #10 Anonymous Team Benchmark)
CREATE OR REPLACE FUNCTION get_team_average_metrics(p_days INT DEFAULT 7)
RETURNS TABLE(
    avg_who5 DOUBLE PRECISION,
    avg_phq9 DOUBLE PRECISION,
    avg_gad7 DOUBLE PRECISION,
    avg_burnout DOUBLE PRECISION
) AS $$
BEGIN
    RETURN QUERY
    WITH recent_metrics AS (
        SELECT
            user_id,
            AVG(CASE WHEN question_type = 'mood' THEN value * 20.0 ELSE 0 END) as who5,
            AVG(CASE WHEN question_type IN ('mood', 'sleep', 'concentration') THEN value * 3.0 ELSE 0 END) as phq9,
            AVG(CASE WHEN question_type IN ('anxiety', 'stress') THEN value * 3.0 ELSE 0 END) as gad7,
            AVG(CASE WHEN question_type IN ('energy', 'stress', 'workload') THEN value * 10.0 ELSE 0 END) as burnout
        FROM checkin_answers
        WHERE created_at >= NOW() - (p_days || ' days')::INTERVAL
        GROUP BY user_id
    )
    SELECT
        AVG(who5)::DOUBLE PRECISION,
        AVG(phq9)::DOUBLE PRECISION,
        AVG(gad7)::DOUBLE PRECISION,
        AVG(burnout)::DOUBLE PRECISION
    FROM recent_metrics;
END;
$$ LANGUAGE plpgsql;

-- Trigger to update streak on check-in answer
CREATE OR REPLACE FUNCTION trigger_update_streak()
RETURNS TRIGGER AS $$
BEGIN
    PERFORM update_user_streak(NEW.user_id, DATE(NEW.created_at));
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER checkin_update_streak
    AFTER INSERT ON checkin_answers
    FOR EACH ROW
    EXECUTE FUNCTION trigger_update_streak();

-- ============================================
-- SEED DATA (Initialize preferences for existing users)
-- ============================================
INSERT INTO user_preferences (user_id, reminder_hour, reminder_minute)
SELECT id, 10, 0
FROM users
WHERE id NOT IN (SELECT user_id FROM user_preferences)
ON CONFLICT (user_id) DO NOTHING;

INSERT INTO user_streaks (user_id, current_streak, longest_streak, total_checkins)
SELECT id, 0, 0, 0
FROM users
WHERE id NOT IN (SELECT user_id FROM user_streaks)
ON CONFLICT (user_id) DO NOTHING;

-- ============================================
-- COMMENTS
-- ============================================
COMMENT ON TABLE user_preferences IS 'User-specific settings for smart reminders and notifications (#2)';
COMMENT ON TABLE user_streaks IS 'Tracks daily check-in streaks and milestones (#6)';
COMMENT ON TABLE kudos IS 'Peer recognition system for team appreciation (#17)';
COMMENT ON TABLE team_insights_cache IS 'Cached analytics data for performance (#8)';
COMMENT ON TYPE post_category IS 'AI-generated categories for wall posts (#12)';
