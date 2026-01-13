-- Wall toxicity signals (auto detection)
CREATE TABLE IF NOT EXISTS wall_toxic_signals (
    post_id UUID PRIMARY KEY REFERENCES wall_posts(id) ON DELETE CASCADE,
    severity SMALLINT NOT NULL DEFAULT 0,
    flagged BOOLEAN NOT NULL DEFAULT false,
    themes JSONB NOT NULL DEFAULT '[]'::jsonb,
    created_at TIMESTAMPTZ DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_wall_toxic_signals_flagged
    ON wall_toxic_signals(flagged, created_at DESC);
