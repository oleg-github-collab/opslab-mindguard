ALTER TABLE user_preferences
ADD COLUMN IF NOT EXISTS last_web_checkin_announcement_date DATE;

COMMENT ON COLUMN user_preferences.last_web_checkin_announcement_date IS
'Last date when web check-in rollout announcement was sent to the user';
