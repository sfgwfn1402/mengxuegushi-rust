# 萌学古诗接口设计

基础路径：

```text
/api
```

鉴权接口统一使用：

```http
Authorization: Bearer dev-token-{user_id}
```

错误返回：

```json
{
  "code": "BAD_REQUEST | UNAUTHORIZED | NOT_FOUND | UPSTREAM_ERROR | INTERNAL_ERROR",
  "message": "错误说明"
}
```

## 1. 健康检查

### GET /health

响应：

```json
{
  "status": "ok",
  "service": "mengxuegushi-rust"
}
```

## 2. 登录接口

### POST /api/auth/dev-login

开发登录，仅本地或开发环境使用。

启用条件：

```env
ENABLE_DEV_LOGIN=true
```

请求：

```json
{
  "openid": "dev-openid-local",
  "unionid": null
}
```

响应：

```json
{
  "token": "dev-token-{user_id}",
  "user_id": "uuid",
  "openid": "dev-openid-local",
  "unionid": null
}
```

### POST /api/auth/wechat-login

微信小程序正式登录。

请求：

```json
{
  "code": "wx.login 返回的 code"
}
```

处理流程：

```text
code -> 微信 code2session -> openid/session_key/unionid -> upsert users -> 返回 token
```

响应：

```json
{
  "token": "dev-token-{user_id}",
  "user_id": "uuid",
  "openid": "openid",
  "session_key": "session_key",
  "unionid": null
}
```

## 3. 古诗接口

### GET /api/poems

查询古诗列表。

Query 参数：

| 参数 | 类型 | 必填 | 说明 |
|---|---|---|---|
| page | integer | 否 | 页码，默认 1 |
| page_size | integer | 否 | 每页数量，默认 20 |
| level | integer | 否 | 分级 1/2/3 |
| difficulty | integer | 否 | 难度 1/2/3 |
| season | string | 否 | 季节 spring/summer/autumn/winter/any |
| tag | string | 否 | 标签 |
| keyword | string | 否 | 标题、作者、正文搜索 |

示例：

```text
GET /api/poems?page=1&page_size=100
```

响应：

```json
{
  "items": [
    {
      "id": 1,
      "title": "静夜思111",
      "author": "李白",
      "dynasty": "唐",
      "content": ["床前明月光", "疑是地上霜"],
      "pinyin": "...",
      "translation": "...",
      "story": "...",
      "parent_guide": "...",
      "difficulty": 1,
      "level": 1,
      "tags": ["月亮", "思乡"],
      "season": "any",
      "audio_url": "https://www.example.com/static/audios-id/poem-1.mp3",
      "video_available": false,
      "card_unlocked": true
    }
  ],
  "page": 1,
  "page_size": 100,
  "total": 100
}
```

### GET /api/poems/{id}

查询单首古诗详情。

示例：

```text
GET /api/poems/1
```

响应：单个 poem 对象。

## 4. 当前用户接口

以下接口都需要 Authorization。

### GET /api/me

获取当前用户资料。

响应：

```json
{
  "id": "uuid",
  "openid": "openid",
  "unionid": null,
  "nickname": "小诗童",
  "avatar_url": "https://...",
  "created_at": "2026-05-31T07:00:00Z",
  "updated_at": "2026-05-31T07:00:00Z"
}
```

### POST /api/me

更新当前用户资料。

请求：

```json
{
  "nickname": "小诗童",
  "avatar_url": "https://..."
}
```

响应：更新后的 user 对象。

## 5. 用户统计接口

### GET /api/me/stats

获取个人页/任务页统计。

响应：

```json
{
  "stars": 6,
  "total_days": 1,
  "streak": 1,
  "learned_poem_count": 3,
  "learned_idiom_count": 2,
  "today_checked": true,
  "today_tasks_done": ["learn1", "quiz3"]
}
```

字段说明：

| 字段 | 说明 |
|---|---|
| stars | 星星总数 |
| total_days | 累计打卡天数 |
| streak | 连续打卡天数 |
| learned_poem_count | 已学古诗数量 |
| learned_idiom_count | 已学成语数量 |
| today_checked | 今天是否已打卡 |
| today_tasks_done | 今天已完成任务 ID 列表 |

## 6. 打卡接口

### POST /api/me/checkin

执行今日打卡。重复调用不会重复增加天数。

响应：

```json
{
  "today_checked": true,
  "total_days": 1,
  "streak": 1
}
```

## 7. 每日任务接口

### POST /api/me/tasks

完成每日任务并增加星星。重复完成同一天同一个任务不会重复加星星。

请求：

```json
{
  "task_id": "quiz3",
  "stars": 3
}
```

响应：

```json
{
  "task_id": "quiz3",
  "stars_added": 3,
  "total_stars": 6,
  "completed": true
}
```

## 8. 古诗学习进度接口

### GET /api/me/progress

获取当前用户全部古诗学习进度。

响应：

```json
{
  "items": [
    {
      "poem_id": 1,
      "learned": true,
      "read_count": 2,
      "quiz_correct_count": 3,
      "quiz_wrong_count": 1,
      "last_learned_at": "2026-05-31T07:20:00Z"
    }
  ]
}
```

### POST /api/me/progress/{poem_id}

更新某首古诗学习进度。

请求：

```json
{
  "learned": true,
  "read_count_delta": 1,
  "quiz_correct_delta": 1,
  "quiz_wrong_delta": 0
}
```

响应：更新后的 progress 对象。

## 9. 收藏接口

### GET /api/me/favorites

获取收藏列表。

响应：

```json
{
  "items": [
    {
      "poem_id": 1,
      "created_at": "2026-05-31T07:00:00Z"
    }
  ]
}
```

### POST /api/me/favorites/{poem_id}

收藏古诗。

响应：

```json
{
  "favorited": true
}
```

### DELETE /api/me/favorites/{poem_id}

取消收藏。

响应：

```json
{
  "favorited": false
}
```

## 10. 成语学习进度接口

### GET /api/me/idiom-progress

获取成语学习进度列表。

响应：

```json
{
  "items": [
    {
      "idiom_id": 1,
      "learned": true,
      "read_count": 1,
      "quiz_correct_count": 2,
      "quiz_wrong_count": 0,
      "last_learned_at": "2026-05-31T07:20:00Z"
    }
  ]
}
```

### POST /api/me/idiom-progress

更新成语学习进度。

请求：

```json
{
  "idiom_id": 1,
  "learned": true,
  "read_count_delta": 1,
  "quiz_correct_delta": 1,
  "quiz_wrong_delta": 0
}
```

响应：更新后的 idiom progress 对象。

## 11. 清空用户数据接口

### POST /api/me/clear-data

清空当前用户业务数据。

会清空：

- 古诗学习进度
- 成语学习进度
- 收藏
- 打卡
- 每日任务
- 用户统计

不会删除：

- users 用户账号
- poems 古诗主数据

响应：

```json
{
  "cleared": true
}
```

## 12. 小程序接口调用约定

开发环境：

```js
apiBaseUrl: 'http://192.168.1.230:8080/api'
useDevLogin: true
```

生产环境目标：

```js
apiBaseUrl: 'https://www.example.com/api'
useDevLogin: false
```

生产环境小程序登录：

```text
wx.login -> /api/auth/wechat-login -> 保存 apiToken -> 请求 /api/me/*
```
