-- Add anonymity flag for wall posts and track reminder send date per user

ALTER TABLE wall_posts
ADD COLUMN IF NOT EXISTS is_anonymous BOOLEAN NOT NULL DEFAULT true;

ALTER TABLE user_preferences
ADD COLUMN IF NOT EXISTS last_reminder_date DATE;

ALTER TABLE user_preferences
ALTER COLUMN timezone SET DEFAULT 'Europe/Kyiv';

UPDATE user_preferences
SET timezone = 'Europe/Kyiv'
WHERE timezone = 'Europe/Kiev'
   OR timezone IS NULL
   OR timezone = '';
