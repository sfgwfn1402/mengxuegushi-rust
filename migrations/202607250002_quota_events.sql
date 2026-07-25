-- 每日限额事件流水：服务器口径的「今日新学/今日创作」计数依据。
-- 客户端闸口改调 /api/me/quota/*，重装 App / 多端不再丢计数。
CREATE TABLE IF NOT EXISTS quota_events (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    kind TEXT NOT NULL,               -- start_poem | creation
    ref_id INT,                       -- start_poem 时为 poem_id
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_quota_events_user_kind_day ON quota_events(user_id, kind, created_at);
