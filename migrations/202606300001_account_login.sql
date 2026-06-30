-- 手机号 / 邮箱 + 密码登录所需字段。
-- 复用现有 users 表与 dev-token-{id} 鉴权：account 用户的 openid 合成为
-- "acct:phone:<手机号>" 或 "acct:email:<邮箱>"，以满足 openid NOT NULL UNIQUE。

ALTER TABLE users ADD COLUMN IF NOT EXISTS phone TEXT;
ALTER TABLE users ADD COLUMN IF NOT EXISTS email TEXT;
ALTER TABLE users ADD COLUMN IF NOT EXISTS password_hash TEXT;

-- 部分唯一索引：允许多个 NULL，但非空手机号/邮箱必须唯一。
CREATE UNIQUE INDEX IF NOT EXISTS users_phone_key ON users (phone) WHERE phone IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS users_email_key ON users (email) WHERE email IS NOT NULL;
