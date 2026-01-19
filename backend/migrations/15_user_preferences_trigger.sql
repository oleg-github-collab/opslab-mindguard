-- Auto-create user_preferences on user creation
-- Date: 2026-01-19

-- Create function to auto-create user preferences
CREATE OR REPLACE FUNCTION create_user_preferences()
RETURNS TRIGGER AS $$
BEGIN
    INSERT INTO user_preferences (user_id, reminder_hour, reminder_minute, timezone, notification_enabled)
    VALUES (NEW.id, 10, 0, 'Europe/Kyiv', true)
    ON CONFLICT (user_id) DO NOTHING;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Create trigger
DROP TRIGGER IF EXISTS trigger_create_user_preferences ON users;
CREATE TRIGGER trigger_create_user_preferences
    AFTER INSERT ON users
    FOR EACH ROW
    EXECUTE FUNCTION create_user_preferences();

-- Backfill existing users without preferences
INSERT INTO user_preferences (user_id, reminder_hour, reminder_minute, timezone, notification_enabled)
SELECT id, 10, 0, 'Europe/Kyiv', true
FROM users
WHERE id NOT IN (SELECT user_id FROM user_preferences)
ON CONFLICT (user_id) DO NOTHING;

COMMENT ON FUNCTION create_user_preferences() IS 'Auto-creates user_preferences record when new user is created';
