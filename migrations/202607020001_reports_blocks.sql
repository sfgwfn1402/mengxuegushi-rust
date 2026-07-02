-- 内容举报 + 用户拉黑（App Store 对 UGC 应用的强制要求）。

CREATE TABLE IF NOT EXISTS content_reports (
    id TEXT PRIMARY KEY,
    reporter_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    target_kind TEXT NOT NULL,          -- moment | comment | artwork | recitation | user
    target_id TEXT NOT NULL,
    reason TEXT,
    status TEXT NOT NULL DEFAULT 'pending',  -- pending | reviewed | dismissed
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_content_reports_status ON content_reports (status, created_at DESC);

CREATE TABLE IF NOT EXISTS user_blocks (
    blocker_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    blocked_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (blocker_id, blocked_id)
);
CREATE INDEX IF NOT EXISTS idx_user_blocks_blocker ON user_blocks (blocker_id);
