-- Telegram PIN codes for automatic linking
-- User generates PIN on web, then sends to bot to link Telegram ID

CREATE TABLE IF NOT EXISTS telegram_pins (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    pin_code VARCHAR(4) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL DEFAULT (now() + INTERVAL '5 minutes'),
    used BOOLEAN NOT NULL DEFAULT false,
    used_at TIMESTAMPTZ
);

-- Index for fast PIN lookup
CREATE INDEX IF NOT EXISTS idx_telegram_pins_code
    ON telegram_pins(pin_code)
    WHERE used = false AND expires_at > now();

-- Index for user's active PINs
CREATE INDEX IF NOT EXISTS idx_telegram_pins_user
    ON telegram_pins(user_id, created_at DESC);

-- Cleanup old PINs (run periodically)
CREATE OR REPLACE FUNCTION cleanup_expired_pins()
RETURNS INTEGER AS $$
DECLARE
    deleted_count INTEGER;
BEGIN
    DELETE FROM telegram_pins
    WHERE expires_at < now() - INTERVAL '1 day';

    GET DIAGNOSTICS deleted_count = ROW_COUNT;
    RETURN deleted_count;
END;
$$ LANGUAGE plpgsql;
