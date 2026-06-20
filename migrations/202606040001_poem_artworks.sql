CREATE TABLE IF NOT EXISTS poem_artworks (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    poem_id INTEGER NOT NULL REFERENCES poems(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    description TEXT,
    image_url TEXT NOT NULL,
    object_path TEXT NOT NULL,
    like_count INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'active',
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_poem_artworks_poem_rank
ON poem_artworks(poem_id, status, like_count DESC, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_poem_artworks_user
ON poem_artworks(user_id, created_at DESC);

CREATE TABLE IF NOT EXISTS poem_artwork_likes (
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    artwork_id TEXT NOT NULL REFERENCES poem_artworks(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY(user_id, artwork_id)
);
