-- One-time login tokens for Telegram web linking
-- Allows users to login from Telegram with a temporary token

CREATE TABLE IF NOT EXISTS telegram_login_tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token VARCHAR(64) NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    used BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Index for fast token lookup (removed expires_at > now() from predicate - not IMMUTABLE)
CREATE INDEX IF NOT EXISTS idx_telegram_login_tokens_token
    ON telegram_login_tokens(token) WHERE used = FALSE;

-- Index for cleanup
CREATE INDEX IF NOT EXISTS idx_telegram_login_tokens_expires
    ON telegram_login_tokens(expires_at) WHERE used = FALSE;

-- Clean up expired tokens periodically
CREATE OR REPLACE FUNCTION cleanup_expired_tokens()
RETURNS void AS $$
BEGIN
    DELETE FROM telegram_login_tokens
    WHERE expires_at < now() OR (used = TRUE AND created_at < now() - INTERVAL '1 day');
END;
$$ LANGUAGE plpgsql;
