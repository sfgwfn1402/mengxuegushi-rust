CREATE TABLE IF NOT EXISTS poem_themes (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    emoji TEXT NOT NULL DEFAULT '📚',
    description TEXT NOT NULL DEFAULT '',
    sort_order INTEGER NOT NULL DEFAULT 0,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS poem_theme_relations (
    poem_id INTEGER NOT NULL REFERENCES poems(id) ON DELETE CASCADE,
    theme_id TEXT NOT NULL REFERENCES poem_themes(id) ON DELETE CASCADE,
    source TEXT NOT NULL DEFAULT 'seed',
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (poem_id, theme_id)
);

CREATE INDEX IF NOT EXISTS idx_poem_theme_relations_theme
ON poem_theme_relations(theme_id, poem_id);

COMMENT ON TABLE poem_themes IS '古诗主题表：定义可用于首页探索和仓库筛选的主题分类';
COMMENT ON COLUMN poem_themes.id IS '主题ID，英文/拼音短标识，例如 spring、homesick';
COMMENT ON COLUMN poem_themes.name IS '主题中文名称，例如 春天、思乡、送别';
COMMENT ON COLUMN poem_themes.emoji IS '主题展示图标';
COMMENT ON COLUMN poem_themes.description IS '主题说明，用于前端副标题或后台说明';
COMMENT ON COLUMN poem_themes.sort_order IS '主题排序，越小越靠前';
COMMENT ON COLUMN poem_themes.enabled IS '是否启用该主题';
COMMENT ON COLUMN poem_themes.created_at IS '主题创建时间';
COMMENT ON COLUMN poem_themes.updated_at IS '主题更新时间';

COMMENT ON TABLE poem_theme_relations IS '古诗主题关联表：一首诗可属于多个主题，一个主题可包含多首诗';
COMMENT ON COLUMN poem_theme_relations.poem_id IS '古诗ID，关联 poems.id';
COMMENT ON COLUMN poem_theme_relations.theme_id IS '主题ID，关联 poem_themes.id';
COMMENT ON COLUMN poem_theme_relations.source IS '关联来源：seed=初始化规则，manual=人工维护';
COMMENT ON COLUMN poem_theme_relations.created_at IS '关联创建时间';
COMMENT ON INDEX idx_poem_theme_relations_theme IS '索引：按主题查询古诗列表';

INSERT INTO poem_themes (id, name, emoji, description, sort_order) VALUES
('spring', '春天', '🌸', '春风、花草、燕子和万物生长', 10),
('summer', '夏天', '🌿', '夏日、荷花、蝉声和清凉景色', 20),
('autumn', '秋天', '🍁', '秋风、秋夜、落叶和思念', 30),
('winter', '冬天', '❄️', '雪景、寒梅和冬日风光', 40),
('homesick', '思乡', '🌙', '月亮、夜晚、远行和故乡', 50),
('farewell', '送别', '👋', '朋友分别、离别赠言和依依不舍', 60),
('friendship', '友情', '🤝', '朋友情谊、相逢和赠别', 70),
('family', '亲情', '👪', '母爱、亲情和家庭思念', 80),
('children', '儿童', '🧒', '儿童生活、童趣和启蒙', 90),
('animals', '动物', '🐾', '鹅、鸟、燕子等动物意象', 100),
('landscape', '山水风景', '🏞', '山川、田园、自然景色', 110),
('moon_night', '月夜', '🌕', '月亮、夜景、钟声和静夜', 120),
('river_lake', '江河湖海', '🌊', '长江、黄河、西湖、江南水乡', 130),
('mountain', '登高山林', '⛰️', '登楼、登山、山路和山林', 140),
('pastoral', '田园乡村', '🌾', '乡村、农事、牧童和田园生活', 150),
('labor_food', '劳动节约', '🍚', '劳动、粮食、农民和节约', 160),
('war_border', '边塞战争', '🏹', '边塞、战争、保家卫国', 170),
('patriotism', '爱国家国', '🇨🇳', '家国情怀、爱国与历史兴亡', 180),
('philosophy', '人生哲理', '💡', '登高望远、成长、生命和道理', 190),
('virtue', '品格志向', '🪨', '坚强、清白、气节和志向', 200),
('imagination', '浪漫想象', '✨', '夸张想象、神话色彩和浪漫表达', 210),
('festival', '节日习俗', '🏮', '清明等节日与传统习俗', 220),
('travel', '旅行行舟', '⛵', '旅途、行舟、远行和江边所见', 230),
('sadness', '悲悯忧伤', '🕯', '悲伤、同情、贫穷和忧思', 240),
('wine', '饮酒豪情', '🍶', '饮酒、豪情和诗人胸怀', 250),
('west_lake', '西湖', '🌉', '西湖风光和杭州景色', 260),
('waterfall', '瀑布庐山', '💦', '瀑布、庐山和壮丽山水', 270),
('lotus', '荷花', '🪷', '荷花、江南和夏日水景', 280)
ON CONFLICT (id) DO UPDATE SET
    name = EXCLUDED.name,
    emoji = EXCLUDED.emoji,
    description = EXCLUDED.description,
    sort_order = EXCLUDED.sort_order,
    updated_at = CURRENT_TIMESTAMP;
