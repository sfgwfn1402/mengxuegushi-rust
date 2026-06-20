CREATE TABLE IF NOT EXISTS parent_feedback (
    id UUID PRIMARY KEY,
    user_id TEXT REFERENCES users(id) ON DELETE SET NULL,
    age TEXT,
    feedback_type TEXT NOT NULL,
    pain_point TEXT,
    suggestion TEXT,
    contact TEXT,
    client_info JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_parent_feedback_created_at ON parent_feedback(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_parent_feedback_type ON parent_feedback(feedback_type);
