ALTER TABLE poems
ADD COLUMN IF NOT EXISTS audio_version TEXT;

COMMENT ON COLUMN poems.audio_version IS '官方朗读/逐句跟读音频版本号；客户端拼到音频URL query，更新音频后用于绕过本地缓存';

UPDATE poems
SET audio_version = '20260620-real-v3', updated_at = NOW()
WHERE id = 33;
