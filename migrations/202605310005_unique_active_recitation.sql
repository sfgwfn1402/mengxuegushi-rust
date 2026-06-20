UPDATE user_recitations r
SET status = 'replaced', updated_at = CURRENT_TIMESTAMP
WHERE status = 'active'
  AND EXISTS (
      SELECT 1
      FROM user_recitations newer
      WHERE newer.user_id = r.user_id
        AND newer.poem_id = r.poem_id
        AND newer.status = 'active'
        AND newer.created_at > r.created_at
  );

CREATE UNIQUE INDEX IF NOT EXISTS uniq_active_recitation_user_poem
ON user_recitations(user_id, poem_id)
WHERE status = 'active';
