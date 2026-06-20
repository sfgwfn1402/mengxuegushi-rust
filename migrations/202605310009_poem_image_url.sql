ALTER TABLE poems ADD COLUMN IF NOT EXISTS image_url TEXT;

COMMENT ON COLUMN poems.image_url IS '古诗插画图片URL，生产环境指向MinIO images-id/poem-{id}.jpg';
