-- 个人简介（签名），用于「我的」页展示与编辑。
ALTER TABLE users ADD COLUMN IF NOT EXISTS bio TEXT;
