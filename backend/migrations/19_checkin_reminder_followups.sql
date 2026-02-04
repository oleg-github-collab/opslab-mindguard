-- Add reminder stage tracking for follow-up notifications

ALTER TABLE user_preferences
ADD COLUMN IF NOT EXISTS last_reminder_stage SMALLINT;

UPDATE user_preferences
SET last_reminder_stage = 0
WHERE last_reminder_date IS NOT NULL
  AND last_reminder_stage IS NULL;

COMMENT ON COLUMN user_preferences.last_reminder_stage IS
    'Reminder stage sent for last_reminder_date (0 = initial, 1+ = follow-ups)';
