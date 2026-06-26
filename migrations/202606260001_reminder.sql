-- 学习提醒订阅：一次性订阅攒额度，每条额度发一次提醒
ALTER TABLE users ADD COLUMN IF NOT EXISTS reminder_credits INTEGER NOT NULL DEFAULT 0;
ALTER TABLE users ADD COLUMN IF NOT EXISTS last_reminded_at DATE;

COMMENT ON COLUMN users.reminder_credits IS '学习提醒订阅额度（一次性订阅，每条额度可发一次）';
COMMENT ON COLUMN users.last_reminded_at IS '最近一次发送学习提醒的日期，用于同日去重';
