-- 每个用户对各类互动消息（赞/粉丝/评论）的“已读时间”，用于顶部徽章显示未读数。
CREATE TABLE IF NOT EXISTS message_read_marks (
    user_id  TEXT        NOT NULL,
    kind     TEXT        NOT NULL,   -- like | follow | comment
    read_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, kind)
);
