CREATE TABLE IF NOT EXISTS events (
    id BIGSERIAL PRIMARY KEY,
    event_name TEXT NOT NULL,
    user_id TEXT,
    page TEXT,
    props JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_events_name_time ON events (event_name, created_at);
CREATE INDEX IF NOT EXISTS idx_events_time ON events (created_at);
CREATE INDEX IF NOT EXISTS idx_events_user_time ON events (user_id, created_at);
