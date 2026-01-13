-- Onboarding state for reminders and bot notifications
ALTER TABLE user_preferences
ADD COLUMN IF NOT EXISTS onboarding_completed BOOLEAN NOT NULL DEFAULT false;

ALTER TABLE user_preferences
ADD COLUMN IF NOT EXISTS onboarding_completed_at TIMESTAMPTZ;
