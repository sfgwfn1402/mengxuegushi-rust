-- 动态支持多图（最多6张）：object_paths 存所有图片的 MinIO 对象键
ALTER TABLE moments ADD COLUMN IF NOT EXISTS object_paths JSONB NOT NULL DEFAULT '[]';
-- 把已有单图行回填到 object_paths
UPDATE moments
SET object_paths = jsonb_build_array(object_path)
WHERE object_path IS NOT NULL AND object_path <> '' AND (object_paths IS NULL OR object_paths = '[]');
