-- 动态发布时的 IP 属地（省级），best-effort 解析
ALTER TABLE moments ADD COLUMN IF NOT EXISTS location TEXT;
