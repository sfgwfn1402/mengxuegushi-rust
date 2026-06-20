# 萌学古诗数据库设计

数据库：PostgreSQL

## 1. ER 概览

```text
users
  ├── user_poem_progress
  ├── favorites
  ├── user_idiom_progress
  ├── user_checkins
  ├── user_daily_tasks
  └── user_stats

poems
  ├── user_poem_progress
  └── favorites
```

## 2. users 用户表

用途：保存微信用户身份和可选资料。

| 字段 | 类型 | 说明 |
|---|---|---|
| id | TEXT PK | 后端用户 ID，UUID 字符串 |
| openid | TEXT UNIQUE NOT NULL | 微信 openid |
| unionid | TEXT | 微信 unionid，可空 |
| nickname | TEXT | 用户昵称，可选 |
| avatar_url | TEXT | 用户头像，可选 |
| created_at | TIMESTAMPTZ | 创建时间 |
| updated_at | TIMESTAMPTZ | 更新时间 |

## 3. poems 古诗表

用途：保存 100 首古诗内容和音频地址。

| 字段 | 类型 | 说明 |
|---|---|---|
| id | INTEGER PK | 古诗 ID |
| title | TEXT | 标题 |
| author | TEXT | 作者 |
| dynasty | TEXT | 朝代 |
| content_json | TEXT | 古诗正文，当前为字符串 |
| pinyin | TEXT | 拼音 |
| translation | TEXT | 翻译 |
| story | TEXT | 故事 |
| parent_guide | TEXT | 家长讲解 |
| difficulty | INTEGER | 难度 1/2/3 |
| level | INTEGER | 分级 1/2/3 |
| tags_json | TEXT | 标签 JSON |
| season | TEXT | 季节 spring/summer/autumn/winter/any |
| audio_url | TEXT | 音频 URL，生产指向 MinIO |
| video_available | BOOLEAN | 是否有视频 |
| card_unlocked | BOOLEAN | 卡片是否解锁 |
| annotated_content_json | TEXT | 注音内容 JSON |
| created_at | TIMESTAMPTZ | 创建时间 |
| updated_at | TIMESTAMPTZ | 更新时间 |

索引：

```sql
idx_poems_level(level)
```

## 4. user_poem_progress 古诗学习进度表

用途：记录用户对每首古诗的学习状态。

| 字段 | 类型 | 说明 |
|---|---|---|
| id | TEXT PK | UUID |
| user_id | TEXT FK | 用户 ID |
| poem_id | INTEGER FK | 古诗 ID |
| learned | BOOLEAN | 是否已学会 |
| read_count | INTEGER | 阅读/打开次数 |
| quiz_correct_count | INTEGER | 答题正确次数 |
| quiz_wrong_count | INTEGER | 答题错误次数 |
| last_learned_at | TIMESTAMPTZ | 最近学会时间 |
| created_at | TIMESTAMPTZ | 创建时间 |
| updated_at | TIMESTAMPTZ | 更新时间 |

唯一约束：

```sql
UNIQUE(user_id, poem_id)
```

## 5. favorites 收藏表

用途：记录用户收藏的古诗。

| 字段 | 类型 | 说明 |
|---|---|---|
| id | TEXT PK | UUID |
| user_id | TEXT FK | 用户 ID |
| poem_id | INTEGER FK | 古诗 ID |
| created_at | TIMESTAMPTZ | 收藏时间 |

唯一约束：

```sql
UNIQUE(user_id, poem_id)
```

## 6. user_idiom_progress 成语学习进度表

用途：记录用户成语学习数据。

| 字段 | 类型 | 说明 |
|---|---|---|
| id | TEXT PK | UUID |
| user_id | TEXT FK | 用户 ID |
| idiom_id | INTEGER | 成语 ID |
| learned | BOOLEAN | 是否已学会 |
| read_count | INTEGER | 阅读次数 |
| quiz_correct_count | INTEGER | 答题正确次数 |
| quiz_wrong_count | INTEGER | 答题错误次数 |
| last_learned_at | TIMESTAMPTZ | 最近学会时间 |
| created_at | TIMESTAMPTZ | 创建时间 |
| updated_at | TIMESTAMPTZ | 更新时间 |

唯一约束：

```sql
UNIQUE(user_id, idiom_id)
```

## 7. user_checkins 打卡表

用途：记录用户每日打卡。

| 字段 | 类型 | 说明 |
|---|---|---|
| id | TEXT PK | UUID |
| user_id | TEXT FK | 用户 ID |
| checkin_date | DATE | 打卡日期 |
| created_at | TIMESTAMPTZ | 创建时间 |

唯一约束：

```sql
UNIQUE(user_id, checkin_date)
```

## 8. user_daily_tasks 每日任务表

用途：记录用户每日任务完成情况。

| 字段 | 类型 | 说明 |
|---|---|---|
| id | TEXT PK | UUID |
| user_id | TEXT FK | 用户 ID |
| task_date | DATE | 任务日期 |
| task_id | TEXT | 任务 ID，例如 learn1/quiz3/review3/share |
| stars | INTEGER | 奖励星星数 |
| created_at | TIMESTAMPTZ | 创建时间 |

唯一约束：

```sql
UNIQUE(user_id, task_date, task_id)
```

## 9. user_stats 用户统计表

用途：保存用户聚合统计。

| 字段 | 类型 | 说明 |
|---|---|---|
| user_id | TEXT PK/FK | 用户 ID |
| stars | INTEGER | 星星总数 |
| total_days | INTEGER | 累计打卡天数 |
| streak | INTEGER | 连续打卡天数 |
| updated_at | TIMESTAMPTZ | 更新时间 |

## 10. 迁移文件

```text
migrations/202605310001_init.sql
migrations/202605310002_expand_poems.sql
migrations/202605310003_user_activity.sql
```

## 11. 数据初始化策略

古诗 seed 文件：

```text
data/poems.seed.json
```

策略：

```text
如果 poems 表为空，则插入 100 首诗。
如果 poems 表已有数据，则跳过 seed。
```

原因：数据库是主数据源，不能用 seed 覆盖运营修改。

## 12. 生产数据注意事项

- PostgreSQL 只监听 `127.0.0.1:5432`
- 不开放公网 5432
- 生产密码只放 `/opt/mengxuegushi/.env`
- 不提交 `.env.production`
- 需要定期备份 PostgreSQL

建议备份命令：

```bash
pg_dump "$DATABASE_URL" > backup-$(date +%Y%m%d%H%M%S).sql
```
