-- 中文注释：方便后续通过 psql、DBeaver、DataGrip 等工具直接理解表和字段含义。

COMMENT ON TABLE users IS '用户表：保存微信用户身份、昵称头像等基础资料';
COMMENT ON COLUMN users.id IS '后端用户ID，UUID字符串';
COMMENT ON COLUMN users.openid IS '微信小程序 openid，同一个小程序内唯一';
COMMENT ON COLUMN users.unionid IS '微信 unionid，用户绑定开放平台后可能存在';
COMMENT ON COLUMN users.nickname IS '用户昵称，用户主动授权或填写后保存';
COMMENT ON COLUMN users.avatar_url IS '用户头像URL，用户主动授权或填写后保存';
COMMENT ON COLUMN users.created_at IS '用户创建时间';
COMMENT ON COLUMN users.updated_at IS '用户资料更新时间';

COMMENT ON TABLE poems IS '古诗主表：保存古诗内容、分级、标签、音频地址和讲解信息';
COMMENT ON COLUMN poems.id IS '古诗ID，业务固定编号';
COMMENT ON COLUMN poems.title IS '古诗标题';
COMMENT ON COLUMN poems.author IS '作者';
COMMENT ON COLUMN poems.dynasty IS '朝代';
COMMENT ON COLUMN poems.content_json IS '古诗正文JSON，通常为诗句数组';
COMMENT ON COLUMN poems.level IS '分级，1/2/3，对应小程序分级学习';
COMMENT ON COLUMN poems.tags_json IS '标签JSON，例如思乡、春天、月亮';
COMMENT ON COLUMN poems.audio_url IS '官方朗读音频URL，生产环境指向MinIO或HTTPS静态地址';
COMMENT ON COLUMN poems.created_at IS '古诗记录创建时间';
COMMENT ON COLUMN poems.updated_at IS '古诗记录更新时间';
COMMENT ON COLUMN poems.pinyin IS '古诗拼音文本';
COMMENT ON COLUMN poems.translation IS '诗意解释/白话翻译';
COMMENT ON COLUMN poems.story IS '古诗背景故事';
COMMENT ON COLUMN poems.parent_guide IS '家长讲解提示';
COMMENT ON COLUMN poems.difficulty IS '难度等级，1/2/3';
COMMENT ON COLUMN poems.season IS '季节分类：spring/summer/autumn/winter/any';
COMMENT ON COLUMN poems.video_available IS '是否有视频内容';
COMMENT ON COLUMN poems.card_unlocked IS '卡片是否默认解锁';
COMMENT ON COLUMN poems.annotated_content_json IS '逐字注音内容JSON';

COMMENT ON TABLE user_poem_progress IS '用户古诗学习进度表：记录每个用户每首诗的学习、阅读和答题情况';
COMMENT ON COLUMN user_poem_progress.id IS '进度记录ID，UUID字符串';
COMMENT ON COLUMN user_poem_progress.user_id IS '用户ID，关联 users.id';
COMMENT ON COLUMN user_poem_progress.poem_id IS '古诗ID，关联 poems.id';
COMMENT ON COLUMN user_poem_progress.learned IS '是否已学会/完成学习';
COMMENT ON COLUMN user_poem_progress.read_count IS '阅读或朗读完成次数';
COMMENT ON COLUMN user_poem_progress.quiz_correct_count IS '答题正确次数';
COMMENT ON COLUMN user_poem_progress.quiz_wrong_count IS '答题错误次数';
COMMENT ON COLUMN user_poem_progress.last_learned_at IS '最近一次标记学会时间';
COMMENT ON COLUMN user_poem_progress.created_at IS '记录创建时间';
COMMENT ON COLUMN user_poem_progress.updated_at IS '记录更新时间';

COMMENT ON TABLE favorites IS '古诗收藏表：记录用户收藏的古诗';
COMMENT ON COLUMN favorites.id IS '收藏记录ID，UUID字符串';
COMMENT ON COLUMN favorites.user_id IS '用户ID，关联 users.id';
COMMENT ON COLUMN favorites.poem_id IS '古诗ID，关联 poems.id';
COMMENT ON COLUMN favorites.created_at IS '收藏时间';

COMMENT ON TABLE user_idiom_progress IS '用户成语学习进度表：记录成语学习、阅读和答题情况';
COMMENT ON COLUMN user_idiom_progress.id IS '成语进度记录ID，UUID字符串';
COMMENT ON COLUMN user_idiom_progress.user_id IS '用户ID，关联 users.id';
COMMENT ON COLUMN user_idiom_progress.idiom_id IS '成语ID，目前对应小程序成语数据中的ID';
COMMENT ON COLUMN user_idiom_progress.learned IS '是否已学会/完成学习';
COMMENT ON COLUMN user_idiom_progress.read_count IS '阅读次数';
COMMENT ON COLUMN user_idiom_progress.quiz_correct_count IS '答题正确次数';
COMMENT ON COLUMN user_idiom_progress.quiz_wrong_count IS '答题错误次数';
COMMENT ON COLUMN user_idiom_progress.last_learned_at IS '最近一次标记学会时间';
COMMENT ON COLUMN user_idiom_progress.created_at IS '记录创建时间';
COMMENT ON COLUMN user_idiom_progress.updated_at IS '记录更新时间';

COMMENT ON TABLE user_checkins IS '用户每日打卡表：每个用户每天最多一条打卡记录';
COMMENT ON COLUMN user_checkins.id IS '打卡记录ID，UUID字符串';
COMMENT ON COLUMN user_checkins.user_id IS '用户ID，关联 users.id';
COMMENT ON COLUMN user_checkins.checkin_date IS '打卡日期';
COMMENT ON COLUMN user_checkins.created_at IS '打卡创建时间';

COMMENT ON TABLE user_daily_tasks IS '用户每日任务表：记录每日任务完成情况和星星奖励';
COMMENT ON COLUMN user_daily_tasks.id IS '任务完成记录ID，UUID字符串';
COMMENT ON COLUMN user_daily_tasks.user_id IS '用户ID，关联 users.id';
COMMENT ON COLUMN user_daily_tasks.task_date IS '任务日期';
COMMENT ON COLUMN user_daily_tasks.task_id IS '任务ID，例如 learn1、quiz3、review3、share';
COMMENT ON COLUMN user_daily_tasks.stars IS '该任务奖励的星星数';
COMMENT ON COLUMN user_daily_tasks.created_at IS '任务完成时间';

COMMENT ON TABLE user_stats IS '用户统计表：保存星星数、累计打卡天数、连续打卡天数等聚合数据';
COMMENT ON COLUMN user_stats.user_id IS '用户ID，关联 users.id，同时也是主键';
COMMENT ON COLUMN user_stats.stars IS '星星总数';
COMMENT ON COLUMN user_stats.total_days IS '累计打卡天数';
COMMENT ON COLUMN user_stats.streak IS '连续打卡天数';
COMMENT ON COLUMN user_stats.updated_at IS '统计更新时间';

COMMENT ON TABLE user_recitations IS '用户朗诵作品表：保存用户上传的古诗朗诵音频、点赞数和展示状态';
COMMENT ON COLUMN user_recitations.id IS '朗诵作品ID，UUID字符串';
COMMENT ON COLUMN user_recitations.user_id IS '上传用户ID，关联 users.id';
COMMENT ON COLUMN user_recitations.poem_id IS '朗诵对应古诗ID，关联 poems.id';
COMMENT ON COLUMN user_recitations.audio_url IS '朗诵音频播放URL；当前播放推荐走Rust代理接口 /api/recitations/{id}/audio';
COMMENT ON COLUMN user_recitations.object_path IS 'MinIO对象路径，例如 recitations/poem-1/xxx.mp3';
COMMENT ON COLUMN user_recitations.duration_seconds IS '朗诵音频时长，单位秒';
COMMENT ON COLUMN user_recitations.status IS '作品状态：active=展示中，replaced=被新作品替换，deleted=用户删除，hidden=后台隐藏';
COMMENT ON COLUMN user_recitations.like_count IS '点赞数缓存，用于榜单排序';
COMMENT ON COLUMN user_recitations.created_at IS '作品上传时间';
COMMENT ON COLUMN user_recitations.updated_at IS '作品更新时间';

COMMENT ON TABLE user_recitation_likes IS '用户朗诵点赞表：记录用户对朗诵作品的点赞，限制同一用户对同一作品只能点赞一次';
COMMENT ON COLUMN user_recitation_likes.id IS '点赞记录ID，UUID字符串';
COMMENT ON COLUMN user_recitation_likes.recitation_id IS '朗诵作品ID，关联 user_recitations.id';
COMMENT ON COLUMN user_recitation_likes.user_id IS '点赞用户ID，关联 users.id';
COMMENT ON COLUMN user_recitation_likes.created_at IS '点赞时间';

COMMENT ON INDEX idx_poems_level IS '索引：按古诗分级查询';
COMMENT ON INDEX idx_progress_user_id IS '索引：按用户查询古诗学习进度';
COMMENT ON INDEX idx_favorites_user_id IS '索引：按用户查询收藏';
COMMENT ON INDEX idx_idiom_progress_user_id IS '索引：按用户查询成语学习进度';
COMMENT ON INDEX idx_checkins_user_date IS '索引：按用户和日期查询打卡';
COMMENT ON INDEX idx_daily_tasks_user_date IS '索引：按用户和日期查询每日任务';
COMMENT ON INDEX idx_user_recitations_poem_rank IS '索引：按古诗查询朗诵榜，支持点赞数和时间排序';
COMMENT ON INDEX idx_user_recitations_user IS '索引：按用户查询朗诵作品';
COMMENT ON INDEX idx_user_recitation_likes_user IS '索引：按用户查询朗诵点赞';
COMMENT ON INDEX uniq_active_recitation_user_poem IS '唯一约束：同一用户同一首诗最多一个active朗诵作品';
