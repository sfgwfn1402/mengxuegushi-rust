-- 动态评论（支持两级：顶层评论 + 楼中楼回复）
CREATE TABLE IF NOT EXISTS moment_comments (
    id TEXT PRIMARY KEY,
    moment_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    parent_id TEXT,                 -- NULL=顶层评论；否则指向所属顶层评论 id
    reply_to_nickname TEXT,         -- 楼中楼回复某人时展示「回复 X」
    content TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'public', -- public / deleted
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_moment_comments_moment ON moment_comments (moment_id, created_at);

-- 动态上的评论数（含回复），冗余计数避免每次 COUNT
ALTER TABLE moments ADD COLUMN IF NOT EXISTS comment_count INTEGER NOT NULL DEFAULT 0;
