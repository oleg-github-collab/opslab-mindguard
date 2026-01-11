-- ============================================
-- Row Level Security (RLS) Policies
-- ============================================
-- This migration enables RLS for data isolation
-- Users can only access their own data

-- Enable RLS on sensitive tables
ALTER TABLE checkin_answers ENABLE ROW LEVEL SECURITY;
ALTER TABLE voice_logs ENABLE ROW LEVEL SECURITY;
ALTER TABLE user_preferences ENABLE ROW LEVEL SECURITY;
ALTER TABLE user_streaks ENABLE ROW LEVEL SECURITY;
ALTER TABLE wall_posts ENABLE ROW LEVEL SECURITY;
ALTER TABLE kudos ENABLE ROW LEVEL SECURITY;

-- ============================================
-- CHECKIN_ANSWERS Policies
-- ============================================

-- Users can only view their own check-in answers
CREATE POLICY checkin_answers_select_own
    ON checkin_answers
    FOR SELECT
    USING (user_id = current_setting('app.current_user_id', true)::UUID);

-- Users can only insert their own check-in answers
CREATE POLICY checkin_answers_insert_own
    ON checkin_answers
    FOR INSERT
    WITH CHECK (user_id = current_setting('app.current_user_id', true)::UUID);

-- Admins can view all check-in answers
CREATE POLICY checkin_answers_select_admin
    ON checkin_answers
    FOR SELECT
    USING (
        current_setting('app.current_user_role', true) IN ('ADMIN', 'FOUNDER')
    );

-- ============================================
-- VOICE_LOGS Policies
-- ============================================

CREATE POLICY voice_logs_select_own
    ON voice_logs
    FOR SELECT
    USING (user_id = current_setting('app.current_user_id', true)::UUID);

CREATE POLICY voice_logs_insert_own
    ON voice_logs
    FOR INSERT
    WITH CHECK (user_id = current_setting('app.current_user_id', true)::UUID);

CREATE POLICY voice_logs_select_admin
    ON voice_logs
    FOR SELECT
    USING (
        current_setting('app.current_user_role', true) IN ('ADMIN', 'FOUNDER')
    );

-- ============================================
-- USER_PREFERENCES Policies
-- ============================================

CREATE POLICY user_preferences_all_own
    ON user_preferences
    FOR ALL
    USING (user_id = current_setting('app.current_user_id', true)::UUID)
    WITH CHECK (user_id = current_setting('app.current_user_id', true)::UUID);

-- ============================================
-- USER_STREAKS Policies
-- ============================================

CREATE POLICY user_streaks_select_own
    ON user_streaks
    FOR SELECT
    USING (user_id = current_setting('app.current_user_id', true)::UUID);

CREATE POLICY user_streaks_select_admin
    ON user_streaks
    FOR SELECT
    USING (
        current_setting('app.current_user_role', true) IN ('ADMIN', 'FOUNDER')
    );

-- Streaks are updated by triggers, allow system to write
CREATE POLICY user_streaks_insert_system
    ON user_streaks
    FOR INSERT
    WITH CHECK (true);

CREATE POLICY user_streaks_update_system
    ON user_streaks
    FOR UPDATE
    USING (true);

-- ============================================
-- WALL_POSTS Policies
-- ============================================

-- Users can view all wall posts (it's a shared wall)
CREATE POLICY wall_posts_select_all
    ON wall_posts
    FOR SELECT
    USING (true);

-- Users can only insert their own posts
CREATE POLICY wall_posts_insert_own
    ON wall_posts
    FOR INSERT
    WITH CHECK (user_id = current_setting('app.current_user_id', true)::UUID);

-- Users can only update/delete their own posts
CREATE POLICY wall_posts_update_own
    ON wall_posts
    FOR UPDATE
    USING (user_id = current_setting('app.current_user_id', true)::UUID);

CREATE POLICY wall_posts_delete_own
    ON wall_posts
    FOR DELETE
    USING (user_id = current_setting('app.current_user_id', true)::UUID);

-- Admins can delete any post (moderation)
CREATE POLICY wall_posts_delete_admin
    ON wall_posts
    FOR DELETE
    USING (
        current_setting('app.current_user_role', true) IN ('ADMIN', 'FOUNDER')
    );

-- ============================================
-- KUDOS Policies
-- ============================================

-- Users can view kudos they received or sent
CREATE POLICY kudos_select_involved
    ON kudos
    FOR SELECT
    USING (
        from_user_id = current_setting('app.current_user_id', true)::UUID
        OR to_user_id = current_setting('app.current_user_id', true)::UUID
    );

-- Users can send kudos to others
CREATE POLICY kudos_insert_own
    ON kudos
    FOR INSERT
    WITH CHECK (from_user_id = current_setting('app.current_user_id', true)::UUID);

-- Admins can view all kudos
CREATE POLICY kudos_select_admin
    ON kudos
    FOR SELECT
    USING (
        current_setting('app.current_user_role', true) IN ('ADMIN', 'FOUNDER')
    );

-- ============================================
-- Helper Function: Set User Context
-- ============================================
-- This function should be called at the beginning of each request
-- to set the current user ID and role for RLS policies

CREATE OR REPLACE FUNCTION set_user_context(p_user_id UUID, p_user_role TEXT)
RETURNS void AS $$
BEGIN
    PERFORM set_config('app.current_user_id', p_user_id::TEXT, false);
    PERFORM set_config('app.current_user_role', p_user_role, false);
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

-- ============================================
-- IMPORTANT: Application Integration Required
-- ============================================
-- The application must call set_user_context() at the start of each request:
--
-- await pool.execute(
--     "SELECT set_user_context($1, $2)",
--     &[&user_id, &user_role]
-- ).await?;
--
-- This ensures RLS policies enforce correct access control.
