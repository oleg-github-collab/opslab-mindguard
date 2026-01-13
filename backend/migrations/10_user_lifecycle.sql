-- User lifecycle management: soft deactivate users while retaining data

ALTER TABLE users
ADD COLUMN IF NOT EXISTS is_active BOOLEAN NOT NULL DEFAULT true;

ALTER TABLE users
ADD COLUMN IF NOT EXISTS offboarded_at TIMESTAMPTZ;

ALTER TABLE users
ADD COLUMN IF NOT EXISTS offboarded_by UUID;

ALTER TABLE users
ADD COLUMN IF NOT EXISTS offboarded_reason TEXT;

CREATE INDEX IF NOT EXISTS idx_users_active ON users(is_active);
