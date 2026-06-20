CREATE TABLE IF NOT EXISTS user_recitations (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    poem_id INTEGER NOT NULL REFERENCES poems(id) ON DELETE CASCADE,
    audio_url TEXT NOT NULL,
    object_path TEXT NOT NULL,
    duration_seconds INTEGER,
    status TEXT NOT NULL DEFAULT 'active',
    like_count INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_user_recitations_poem_rank
ON user_recitations(poem_id, status, like_count DESC, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_user_recitations_user
ON user_recitations(user_id, created_at DESC);

CREATE TABLE IF NOT EXISTS user_recitation_likes (
    id TEXT PRIMARY KEY,
    recitation_id TEXT NOT NULL REFERENCES user_recitations(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(recitation_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_user_recitation_likes_user
ON user_recitation_likes(user_id, created_at DESC);
