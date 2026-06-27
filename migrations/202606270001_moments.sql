-- 亲子广场动态（晒娃生活）
CREATE TABLE IF NOT EXISTS moments (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    content TEXT NOT NULL DEFAULT '',
    image_url TEXT NOT NULL,
    object_path TEXT NOT NULL,
    like_count INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'submitted', -- submitted / public / rejected / deleted
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_moments_status_time ON moments (status, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_moments_user ON moments (user_id, created_at DESC);

CREATE TABLE IF NOT EXISTS moment_likes (
    moment_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (moment_id, user_id)
);
