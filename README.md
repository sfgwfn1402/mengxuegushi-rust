# 萌学古诗 Rust 后端

小程序后端服务，当前是第一版可运行 API 骨架。

## 技术栈

- Rust
- axum
- tokio
- serde
- reqwest
- tracing
- PostgreSQL
- sqlx

## 环境配置

本地开发配置：

```bash
cp .env.local .env
```

生产环境模板：

```bash
cp .env.production.example .env.production
```

> 注意：生产真实数据库密码只放服务器 `/opt/mengxuegushi/.env`，不要提交到仓库。

## 本地运行

```bash
cp .env.local .env
cargo run
```

服务默认监听：`http://127.0.0.1:8080`

默认数据库：PostgreSQL。首次启动会自动执行迁移，并从 `data/poems.seed.json` 写入/更新 100 首初始古诗。

本地可以直接用 Docker Compose 启动 PostgreSQL + API：

```bash
docker compose up --build
```

如果只想本机跑 API，需要先准备 PostgreSQL，并配置：

```env
DATABASE_URL=postgres://mengxuegushi:mengxuegushi@127.0.0.1:5432/mengxuegushi
```

默认音频目录可通过 `AUDIO_DIR` 配置。本地建议指向小程序项目音频目录：

```env
AUDIO_DIR=/path/to/workspace/mengxue-gushi/audios
```

服务会暴露静态音频：

```text
GET /audios/poem-1.mp3
```

如果配置 `PUBLIC_BASE_URL=https://your-domain.com`，数据库写入的 `audio_url` 会变成完整地址，例如：`https://your-domain.com/audios/poem-1.mp3`。

> 说明：项目内置 `.cargo/config.toml` 使用可用的 sparse registry，避免本机全局 cargo 配置里的 tuna git index 更新卡住。

## 已有接口

### 健康检查

```bash
curl http://127.0.0.1:8080/health
```

### 古诗列表

```bash
curl http://127.0.0.1:8080/api/poems
```

支持筛选和分页：

```bash
curl 'http://127.0.0.1:8080/api/poems?page=1&page_size=20'
curl 'http://127.0.0.1:8080/api/poems?level=1'
curl 'http://127.0.0.1:8080/api/poems?difficulty=2'
curl 'http://127.0.0.1:8080/api/poems?season=spring'
curl 'http://127.0.0.1:8080/api/poems?tag=春天'
curl 'http://127.0.0.1:8080/api/poems?keyword=李白'
```

### 古诗详情

```bash
curl http://127.0.0.1:8080/api/poems/1
```

### 微信小程序登录

```bash
curl -X POST http://127.0.0.1:8080/api/auth/wechat-login \
  -H 'Content-Type: application/json' \
  -d '{"code":"wx.login 返回的 code"}'
```

需要在 `.env` 配置：

```env
WECHAT_APP_ID=你的 AppID
WECHAT_APP_SECRET=你的 AppSecret
```

### 开发登录

本地调试小程序时可以先用开发登录，不需要真实微信 code：

```bash
curl -X POST http://127.0.0.1:8080/api/auth/dev-login \
  -H 'Content-Type: application/json' \
  -d '{"openid":"dev-openid-local"}'
```

开发登录由 `.env` 控制：

```env
ENABLE_DEV_LOGIN=true
```

生产环境建议设为：

```env
ENABLE_DEV_LOGIN=false
```

> 注意：现在 token 是开发用临时 token，格式为 `dev-token-{user_id}`。后面接 JWT 后再改成正式登录态。

### 当前用户

```bash
curl http://127.0.0.1:8080/api/me \
  -H 'Authorization: Bearer dev-token-{user_id}'
```

### 学习进度

```bash
curl http://127.0.0.1:8080/api/me/progress \
  -H 'Authorization: Bearer dev-token-{user_id}'

curl -X POST http://127.0.0.1:8080/api/me/progress/1 \
  -H 'Authorization: Bearer dev-token-{user_id}' \
  -H 'Content-Type: application/json' \
  -d '{"learned":true,"read_count_delta":1,"quiz_correct_delta":1}'
```

### 收藏

```bash
curl http://127.0.0.1:8080/api/me/favorites \
  -H 'Authorization: Bearer dev-token-{user_id}'

curl -X POST http://127.0.0.1:8080/api/me/favorites/1 \
  -H 'Authorization: Bearer dev-token-{user_id}'

curl -X DELETE http://127.0.0.1:8080/api/me/favorites/1 \
  -H 'Authorization: Bearer dev-token-{user_id}'
```

## 关联项目

- 小程序前端：`/path/to/workspace/mengxue-gushi`
- Rust 后端：`/path/to/workspace/xiaochengxu/mengxuegushi-rust`

## 环境约定

- 本地开发使用 `.env`，可从 `.env.example` 复制后自行填写。
- 生产环境真实配置只放在服务器环境文件中，例如 `/opt/mengxuegushi/.env`。
- 不要把真实数据库密码、微信 AppSecret、对象存储密钥、服务器 IP/域名等提交到仓库。
- 对外媒体域名、MinIO 地址、Nginx 配置等请在部署环境中配置，仓库文档只保留占位示例。

## 作品发布到发现相关代码

产品逻辑：作品上传后先进入用户自己的“我的作品/我的诗集”；用户在作品页主动点“发布到发现”后，作品才公开并出现在发现页；撤回公开后不再出现在发现页。

### 路由注册

文件：`src/routes/mod.rs`

- `POST /api/artworks/{artwork_id}/submit`：发布诗配画到发现
- `DELETE /api/artworks/{artwork_id}/submit`：撤回诗配画公开
- `POST /api/recitations/{recitation_id}/submit`：发布朗诵到发现
- `DELETE /api/recitations/{recitation_id}/submit`：撤回朗诵公开
- `GET /api/artworks`：发现页诗配画列表
- `GET /api/home/popular-recitations`：发现页/首页朗诵列表

### 诗配画

- 接口层：`src/routes/artworks.rs`
  - `submit_artwork()`：处理发布到发现
  - `withdraw_artwork()`：处理撤回公开
  - `list()`：`mine=false` 时读取发现页公开诗配画
- 数据层：`src/services/artwork_store.rs`
  - `set_submission_status()`：更新作品状态
  - `list_recent()`：发现页诗配画查询，目前只返回 `status = 'public'`
  - `list_mine()`：我的作品列表

### 朗诵

- 接口层：`src/routes/recitations.rs`
  - `submit_recitation()`：处理发布到发现
  - `withdraw_recitation()`：处理撤回公开
- 数据层：`src/services/recitation_store.rs`
  - `set_submission_status()`：更新作品状态
  - 公开朗诵查询需要关注 `status = 'public'` 条件
- 首页/发现聚合：`src/services/home_store.rs`
  - `popular_recitations()`：发现页/首页人气朗诵来源

### 发布无需审核时的后端要求

当前实现：用户点“发布到发现”后，后端 `submit_artwork()` / `submit_recitation()` 会直接把作品状态改为 `public`；发现页查询只返回 `public` 作品。

## 下一步建议

1. 换成正式 JWT 登录态
2. 配置真实微信小程序 `WECHAT_APP_ID` / `WECHAT_APP_SECRET`
3. 部署 PostgreSQL + API 到服务器
4. 接 MinIO/对象存储，统一管理音频 URL
5. 在小程序后台配置正式 HTTPS request 合法域名
