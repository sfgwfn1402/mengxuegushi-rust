-- 用户关注关系：follower 关注 followee
CREATE TABLE IF NOT EXISTS user_follows (
    follower_id TEXT NOT NULL,
    followee_id TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (follower_id, followee_id)
);
CREATE INDEX IF NOT EXISTS idx_user_follows_follower ON user_follows (follower_id);
CREATE INDEX IF NOT EXISTS idx_user_follows_followee ON user_follows (followee_id);
