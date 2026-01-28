-- Add check-in frequency preference for web-based check-ins

ALTER TABLE user_preferences
    ADD COLUMN IF NOT EXISTS checkin_frequency VARCHAR(20) DEFAULT 'daily';

UPDATE user_preferences
SET checkin_frequency = 'daily'
WHERE checkin_frequency IS NULL;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'user_preferences_checkin_frequency_valid'
    ) THEN
        ALTER TABLE user_preferences
            ADD CONSTRAINT user_preferences_checkin_frequency_valid
            CHECK (checkin_frequency IN ('daily', 'every_3_days', 'weekly'));
    END IF;
END$$;

COMMENT ON COLUMN user_preferences.checkin_frequency IS
    'Check-in cadence: daily (2-3 q), every_3_days (10 q), weekly (full test)';
