# 萌学古诗 Rust 后端详细设计

## 1. 代码结构

```text
src/
  main.rs                 # 程序入口，加载配置、初始化数据库、启动 axum
  config.rs               # 环境变量配置
  error.rs                # 统一错误类型和响应
  models/
    auth.rs               # 登录请求/响应模型
    poem.rs               # 古诗模型和列表查询模型
    user.rs               # 用户、古诗进度、收藏响应模型
    activity.rs           # 打卡、任务、统计、成语进度模型
    profile.rs            # 用户资料更新模型
  routes/
    mod.rs                # API 路由注册
    health.rs             # 健康检查
    auth.rs               # 登录接口
    poems.rs              # 古诗接口
    me.rs                 # 当前用户及用户相关接口
  services/
    poem_store.rs         # 古诗数据访问和 seed
    user_store.rs         # 用户、古诗进度、收藏数据访问
    activity_store.rs     # 打卡、任务、星星、成语进度数据访问
    wechat.rs             # 微信 code2session 调用
```

## 2. 启动流程

```text
main()
  -> dotenvy::dotenv()
  -> AppConfig::from_env()
  -> init_database()
      -> 创建 PostgreSQL 连接池
      -> 执行 migrations
      -> seed_default_poems()
  -> 创建 AppState
  -> 注册路由
  -> 注册 /audios 静态文件服务
  -> 监听 0.0.0.0:PORT
```

## 3. 配置模型

配置来自环境变量：

| 变量 | 说明 |
|---|---|
| PORT | API 监听端口，默认 8080 |
| DATABASE_URL | PostgreSQL 连接串 |
| AUDIO_DIR | 本地音频目录，作为 fallback 静态服务 |
| PUBLIC_BASE_URL | 对外访问根地址，用于 seed 生成完整 URL |
| ENABLE_DEV_LOGIN | 是否启用开发登录 |
| WECHAT_APP_ID | 微信小程序 AppID |
| WECHAT_APP_SECRET | 微信小程序 AppSecret |
| RUST_LOG | 日志级别 |

## 4. AppState

```rust
pub struct AppState {
    pub config: AppConfig,
    pub http_client: reqwest::Client,
    pub db: sqlx::PgPool,
}
```

用途：

- 共享配置
- 共享 HTTP 客户端
- 共享 PostgreSQL 连接池

## 5. 登录设计

### 5.1 开发登录

接口：

```text
POST /api/auth/dev-login
```

仅当：

```env
ENABLE_DEV_LOGIN=true
```

时可用。

用途：本地开发免微信登录。

返回 token：

```text
dev-token-{user_id}
```

### 5.2 微信登录

接口：

```text
POST /api/auth/wechat-login
```

流程：

```text
小程序 wx.login 得到 code
  -> 后端调用微信 code2session
  -> 得到 openid/unionid（session_key 只在服务端处理，禁止返回给小程序）
  -> users 表 upsert 用户
  -> 返回 token
```

当前 token 仍为开发形式：

```text
dev-token-{user_id}
```

后续建议替换成 JWT。

## 6. 鉴权设计

所有 `/api/me/*` 接口需要：

```http
Authorization: Bearer dev-token-{user_id}
```

鉴权逻辑：

```text
读取 Authorization
  -> 校验 Bearer
  -> 提取 user_id
  -> 查询 users 表
  -> 不存在则返回 401
```

## 7. 古诗数据设计

古诗数据来源：

```text
data/poems.seed.json
```

seed 原则：

```text
只在 poems 表为空时初始化。
数据库已有数据时跳过 seed。
```

这样数据库中的运营修改不会被服务重启覆盖。

## 8. 用户活动设计

用户活动包括：

- 古诗学习进度
- 成语学习进度
- 收藏
- 打卡
- 每日任务
- 星星
- 连续打卡天数

均存 PostgreSQL。

## 9. 错误设计

统一错误结构：

```json
{
  "code": "BAD_REQUEST | UNAUTHORIZED | NOT_FOUND | UPSTREAM_ERROR | INTERNAL_ERROR",
  "message": "..."
}
```

HTTP 状态码：

| 错误 | 状态码 |
|---|---|
| BadRequest | 400 |
| Unauthorized | 401 |
| NotFound | 404 |
| Upstream | 502 |
| Internal | 500 |

## 10. 前端降级策略

当前产品策略：

```text
不再使用本地古诗备份作为业务数据 fallback。
后端不可用时，小程序显示“服务维护中”。
```

保留本地 storage 的内容仅限：

- `apiToken`
- `apiUser`
- `devOpenid`
- `warehouseDefaultCategory`
- `warehouseSearchKeyword`

## 11. 后续优化点

- token 改 JWT
- refresh token
- 微信 session_key 不返回给小程序；如后续确需使用，只能服务端加密存储或不落库
- 管理后台
- 音频上传管理
- 用户数据导出
- 接入对象存储 SDK，生成签名 URL 或统一 CDN URL
