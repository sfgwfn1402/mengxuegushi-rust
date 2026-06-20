# 萌学古诗 Rust 后端生产部署文档

## 1. 生产环境信息

| 项目 | 值 |
|---|---|
| 服务器 | 腾讯云 Ubuntu 24.04 |
| 公网 IP | example.com |
| 内网 IP | 10.0.0.10 |
| 用户 | ubuntu |
| 域名 | example.com / www.example.com |
| 后端目录 | /opt/mengxuegushi |
| 代码目录 | /opt/mengxuegushi/app |
| 二进制 | /opt/mengxuegushi/mengxuegushi-rust |
| 环境文件 | /opt/mengxuegushi/.env |
| systemd 服务 | mengxuegushi.service |
| PostgreSQL | 127.0.0.1:5432 |
| Rust API | 0.0.0.0:8080 |
| MinIO API | 9000 |
| MinIO Console | 9001 |

## 2. 目录结构

```text
/opt/mengxuegushi/
  app/                         # Rust 源代码
  mengxuegushi-rust             # 生产二进制
  .env                          # 生产环境变量，不能提交
  audios/                       # 本地音频 fallback 目录
```

MinIO 数据：

```text
/opt/minio/data/mengxue-gushi/
  audios/
  audios-id/
```

## 3. PostgreSQL 部署

### 3.1 安装

```bash
sudo apt update
sudo apt install -y postgresql postgresql-contrib
```

### 3.2 创建数据库和用户

```bash
sudo -u postgres psql
```

示例 SQL：

```sql
CREATE USER mengxuegushi WITH PASSWORD '强密码';
CREATE DATABASE mengxuegushi OWNER mengxuegushi;
GRANT ALL PRIVILEGES ON DATABASE mengxuegushi TO mengxuegushi;
```

### 3.3 安全要求

PostgreSQL 只监听本机：

```text
127.0.0.1:5432
```

不要开放公网 5432。

如需从 Mac 连接生产库，使用 SSH 隧道：

```bash
ssh -L 15432:127.0.0.1:5432 deploy@example.com
```

然后本地连接：

```text
127.0.0.1:15432
```

## 4. 生产环境变量

文件：

```text
/opt/mengxuegushi/.env
```

示例：

```env
PORT=8080
DATABASE_URL=postgres://mengxuegushi:数据库密码@127.0.0.1:5432/mengxuegushi
AUDIO_DIR=/opt/mengxuegushi/audios
PUBLIC_BASE_URL=https://www.example.com
ENABLE_DEV_LOGIN=false
WECHAT_APP_ID=小程序 AppID
WECHAT_APP_SECRET=小程序 AppSecret
RUST_LOG=info,tower_http=debug
```

权限：

```bash
sudo chmod 600 /opt/mengxuegushi/.env
```

注意：

- 不要把真实数据库密码提交 git
- 不要把 AppSecret 打印到日志或聊天
- AppSecret 如果泄露，应在微信公众平台重置

## 5. 构建和发布

> 标准流程：本地同步源码到 `/opt/mengxuegushi/app` → 服务器 release 编译 → 备份旧二进制 → 停止服务 → 原子替换二进制 → 启动服务 → 验证接口。
>
> 不建议长期把源码同步到 `/tmp` 目录发布；生产源码目录统一使用 `/opt/mengxuegushi/app`，避免服务器上出现多个版本来源。

### 5.1 发布前本地检查

在本地仓库先确认代码能编译，并提交变更：

```bash
cd /path/to/workspace/xiaochengxu/mengxuegushi-rust
cargo check
git status --short
git add <changed-files>
git commit -m "描述本次后端变更"
```

如只是紧急热修，也至少执行 `cargo check` 并记录改动文件。

### 5.2 同步代码到生产源码目录

从本地同步到服务器标准代码目录：

```bash
rsync -az --delete \
  --exclude target \
  --exclude .git \
  --exclude .env \
  --exclude .env.production \
  /path/to/workspace/xiaochengxu/mengxuegushi-rust/ \
  deploy@example.com:/opt/mengxuegushi/app/
```

注意：

- `--delete` 会删除服务器代码目录中本地不存在的文件，所以不要在 `/opt/mengxuegushi/app` 手工放临时文件。
- `.env` 不同步，生产环境变量只保留在 `/opt/mengxuegushi/.env`。
- 不要直接覆盖 `/opt/mengxuegushi/mengxuegushi-rust` 正在运行的二进制，否则可能出现 `Text file busy`。

### 5.3 服务器编译

```bash
ssh deploy@example.com
cd /opt/mengxuegushi/app
. "$HOME/.cargo/env"
cargo build --release
```

编译产物：

```text
/opt/mengxuegushi/app/target/release/mengxuegushi-rust
```

### 5.4 备份、停服、替换、启动

推荐使用 `.new` 临时文件 + `mv` 替换，避免直接写正在运行的二进制。

```bash
cd /opt/mengxuegushi/app

# 1) 备份当前生产二进制，便于回滚
sudo cp /opt/mengxuegushi/mengxuegushi-rust \
  /opt/mengxuegushi/mengxuegushi-rust.bak.$(date +%Y%m%d%H%M%S)

# 2) 先安装到临时文件
sudo install -m 0755 target/release/mengxuegushi-rust \
  /opt/mengxuegushi/mengxuegushi-rust.new

# 3) 停止服务后再替换，避免 Text file busy
sudo systemctl stop mengxuegushi
sudo mv /opt/mengxuegushi/mengxuegushi-rust.new \
  /opt/mengxuegushi/mengxuegushi-rust

# 4) 启动服务并查看状态
sudo systemctl start mengxuegushi
sudo systemctl status mengxuegushi --no-pager
```

如果启动失败，立刻查看日志：

```bash
sudo journalctl -u mengxuegushi -n 200 --no-pager
```

### 5.5 一条命令发布模板

确认本地已 `cargo check` 通过后，也可以使用下面的串联命令发布：

```bash
rsync -az --delete \
  --exclude target \
  --exclude .git \
  --exclude .env \
  --exclude .env.production \
  /path/to/workspace/xiaochengxu/mengxuegushi-rust/ \
  deploy@example.com:/opt/mengxuegushi/app/ && \
ssh deploy@example.com 'set -e
cd /opt/mengxuegushi/app
. "$HOME/.cargo/env"
cargo build --release
sudo cp /opt/mengxuegushi/mengxuegushi-rust /opt/mengxuegushi/mengxuegushi-rust.bak.$(date +%Y%m%d%H%M%S)
sudo install -m 0755 target/release/mengxuegushi-rust /opt/mengxuegushi/mengxuegushi-rust.new
sudo systemctl stop mengxuegushi
sudo mv /opt/mengxuegushi/mengxuegushi-rust.new /opt/mengxuegushi/mengxuegushi-rust
sudo systemctl start mengxuegushi
sudo systemctl status mengxuegushi --no-pager
'
```

### 5.6 发布后验证

至少验证：

```bash
# 基础接口
curl http://127.0.0.1:8080/api/poems/1

# 登录接口，生产环境如 ENABLE_DEV_LOGIN=false，则不要用 dev-login 验证
curl -sS -X POST http://127.0.0.1:8080/api/auth/dev-login \
  -H 'Content-Type: application/json' \
  -d '{"openid":"deploy-test-openid"}'
```

如需要验证需要登录的接口，先取得 token：

```bash
TOKEN=$(curl -sS -X POST http://127.0.0.1:8080/api/auth/dev-login \
  -H 'Content-Type: application/json' \
  -d '{"openid":"deploy-test-openid"}' \
  | python3 -c 'import sys,json; print(json.load(sys.stdin)["token"])')

curl http://127.0.0.1:8080/api/me/stats \
  -H "Authorization: Bearer $TOKEN"
```

首页相关接口示例：

```bash
curl http://127.0.0.1:8080/api/home/today-poem \
  -H "Authorization: Bearer $TOKEN"

curl http://127.0.0.1:8080/api/home/continue-learning \
  -H "Authorization: Bearer $TOKEN"
```

如果生产已关闭 `ENABLE_DEV_LOGIN`，则需要通过小程序 `wx.login` 或临时测试 token 方式验证登录态接口；不要为了测试长期打开 dev-login。

## 6. systemd 服务

文件：

```text
/etc/systemd/system/mengxuegushi.service
```

示例：

```ini
[Unit]
Description=Mengxue Gushi Rust API
After=network.target postgresql.service

[Service]
Type=simple
User=ubuntu
WorkingDirectory=/opt/mengxuegushi
EnvironmentFile=/opt/mengxuegushi/.env
ExecStart=/opt/mengxuegushi/mengxuegushi-rust
Restart=always
RestartSec=3

[Install]
WantedBy=multi-user.target
```

启用：

```bash
sudo systemctl daemon-reload
sudo systemctl enable mengxuegushi
sudo systemctl restart mengxuegushi
```

查看状态：

```bash
sudo systemctl status mengxuegushi --no-pager
sudo journalctl -u mengxuegushi -n 100 --no-pager
```

## 7. Nginx 配置

目标：

```text
https://www.example.com/api/*          -> Rust API 127.0.0.1:8080/api/*
https://www.example.com/health         -> Rust API 127.0.0.1:8080/health

# 公开标准资源：Nginx 直接反代 MinIO，避免每次经过 Rust
https://www.example.com/audios/*       -> MinIO bucket mengxue-gushi/audios-id/*
https://www.example.com/line-audios/*  -> MinIO bucket mengxue-gushi/line-audios/*
https://www.example.com/images/*       -> MinIO bucket mengxue-gushi/images-id/*
https://www.example.com/static/*       -> MinIO bucket mengxue-gushi/*

# 用户生成内容：继续经过 Rust，保留鉴权/状态/审核/删除等业务逻辑
https://www.example.com/api/recitations/{id}/audio -> Rust -> MinIO recitations/*
https://www.example.com/recitations/*              -> Rust -> MinIO recitations/*（兼容旧媒体路径）
https://www.example.com/avatars/*                  -> Rust -> MinIO avatars/*
https://www.example.com/artworks/*                 -> Rust -> MinIO artworks/*
```

原则：公开教材音频/图片走“高速路”（Nginx -> MinIO）；用户朗诵等需要权限和状态控制的资源走“业务后端”（Nginx -> Rust -> MinIO）。不要把 MinIO 对象改成本地 `alias`，资源仍以 MinIO 为准。

示例配置：

```nginx
server {
    listen 443 ssl;
    server_name www.example.com;

    client_max_body_size 100m;

    location /api/ {
        proxy_pass http://127.0.0.1:8080/api/;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }

    location /health {
        proxy_pass http://127.0.0.1:8080/health;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }

    # 公开标准资源：Nginx 直接反代 MinIO，不经过 Rust。
    location /images/ {
        proxy_pass http://127.0.0.1:9000/mengxue-gushi/images-id/;
        proxy_http_version 1.1;
        proxy_set_header Connection "";
        proxy_set_header Host 127.0.0.1:9000;
        proxy_connect_timeout 10s;
        proxy_send_timeout 60s;
        proxy_read_timeout 60s;
        proxy_buffering on;
        expires 30d;
        add_header Cache-Control "public, max-age=2592000" always;
    }

    location /audios/ {
        proxy_pass http://127.0.0.1:9000/mengxue-gushi/audios-id/;
        proxy_http_version 1.1;
        proxy_set_header Connection "";
        proxy_set_header Host 127.0.0.1:9000;
        proxy_connect_timeout 10s;
        proxy_send_timeout 60s;
        proxy_read_timeout 60s;
        proxy_buffering on;
        expires 30d;
        add_header Cache-Control "public, max-age=2592000" always;
    }

    location /line-audios/ {
        proxy_pass http://127.0.0.1:9000/mengxue-gushi/line-audios/;
        proxy_http_version 1.1;
        proxy_set_header Connection "";
        proxy_set_header Host 127.0.0.1:9000;
        proxy_connect_timeout 10s;
        proxy_send_timeout 60s;
        proxy_read_timeout 60s;
        proxy_buffering on;
        expires 30d;
        add_header Cache-Control "public, max-age=2592000" always;
    }

    # 用户生成朗诵兼容路径：继续走 Rust，保留业务逻辑。
    location /recitations/ {
        proxy_pass http://127.0.0.1:8080/recitations/;
        proxy_http_version 1.1;
        proxy_set_header Connection "";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }

    location /static/ {
        proxy_pass http://127.0.0.1:9000/mengxue-gushi/;
        proxy_http_version 1.1;
        proxy_set_header Connection "";
        proxy_set_header Host 127.0.0.1:9000;
    }

    location / {
        return 200 'Mengxue Gushi API OK\n';
        add_header Content-Type text/plain;
    }

    ssl_certificate /etc/letsencrypt/live/www.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/www.example.com/privkey.pem;
    include /etc/letsencrypt/options-ssl-nginx.conf;
    ssl_dhparam /etc/letsencrypt/ssl-dhparams.pem;
}

server {
    listen 80;
    server_name www.example.com example.com;
    return 301 https://www.example.com$request_uri;
}
```

检查并 reload：

```bash
sudo nginx -t
sudo systemctl reload nginx
```

## 8. HTTPS 证书

使用 certbot：

```bash
sudo apt install -y certbot python3-certbot-nginx
sudo certbot --nginx -d www.example.com -d example.com
```

证书自动续期：

```bash
systemctl list-timers | grep certbot
sudo certbot renew --dry-run
```

当前注意事项：

- 服务器本机 HTTPS 测试正常
- 公网 HTTPS 曾出现 TLS reset，需要检查云安全组/防火墙/443 入口

## 9. MinIO 音频

生产音频 bucket：

```text
mengxue-gushi
```

标准资源对象路径：

```text
audios-id/poem-{id}.mp3              # 整首诗朗读
line-audios/poem-{id}-line-{n}.mp3   # 逐句跟读标准音频
images-id/poem-{id}.jpg              # 诗词图片
```

小程序/数据库面向公网使用统一域名路径，不暴露 MinIO 端口：

```text
https://www.example.com/audios/poem-{id}.mp3
https://www.example.com/line-audios/poem-{id}-line-{n}.mp3
https://www.example.com/images/poem-{id}.jpg
```

这些公开标准资源由 Nginx 直接反代 MinIO：

```text
/audios/      -> http://127.0.0.1:9000/mengxue-gushi/audios-id/
/line-audios/ -> http://127.0.0.1:9000/mengxue-gushi/line-audios/
/images/      -> http://127.0.0.1:9000/mengxue-gushi/images-id/
```

用户生成朗诵不要改成直连公开 MinIO：

```text
/api/recitations/{id}/audio -> Rust -> MinIO recitations/*
```

原因：用户朗诵涉及作品状态、审核、删除、权限、本人作品等业务逻辑，需要 Rust 控制。

## 10. 验证命令

### 10.1 Rust 直连验证

健康检查路径是 `/health`，不带 `/api` 前缀；业务接口才使用 `/api/*`。

```bash
curl http://127.0.0.1:8080/health
curl http://127.0.0.1:8080/api/poems/1
```

### 10.2 Nginx HTTP 验证

```bash
curl http://example.com/health
curl http://example.com/api/poems/1
```

### 10.3 Nginx HTTPS 验证

```bash
curl https://www.example.com/health
curl https://www.example.com/api/poems/1
curl -I https://www.example.com/audios/poem-1.mp3
curl -I https://www.example.com/line-audios/poem-3-line-2.mp3
curl -I https://www.example.com/images/poem-1.jpg
```

### 10.4 服务日志

```bash
sudo journalctl -u mengxuegushi -n 100 --no-pager
sudo tail -100 /var/log/nginx/error.log
sudo tail -100 /var/log/nginx/access.log
```

## 11. 小程序生产配置

目标配置：

```js
prod: {
  apiBaseUrl: 'https://www.example.com/api',
  useBackendPoems: true,
  useDevLogin: false
}
```

微信小程序后台需要配置合法域名：

```text
request 合法域名：https://www.example.com
downloadFile 合法域名：https://www.example.com
```

不能使用：

- HTTP
- IP 地址
- 自签名证书

## 12. 回滚方案

如果新版本异常：

1. 查看 systemd 日志：

```bash
sudo journalctl -u mengxuegushi -n 200 --no-pager
```

2. 回滚旧二进制：

```bash
sudo cp /opt/mengxuegushi/mengxuegushi-rust.bak /opt/mengxuegushi/mengxuegushi-rust
sudo systemctl restart mengxuegushi
```

3. 若 Nginx 配置异常，恢复备份：

```bash
sudo cp /etc/nginx/sites-available/minio.conf.bak.YYYYMMDDHHMMSS /etc/nginx/sites-available/minio.conf
sudo nginx -t
sudo systemctl reload nginx
```

## 13. 备份方案

### PostgreSQL 备份

```bash
set -a
. /opt/mengxuegushi/.env
set +a
pg_dump "$DATABASE_URL" > /opt/mengxuegushi/backup-$(date +%Y%m%d%H%M%S).sql
```

### MinIO 音频备份

```bash
sudo tar -czf /opt/mengxuegushi/minio-audios-$(date +%Y%m%d%H%M%S).tar.gz \
  /opt/minio/data/mengxue-gushi/audios-id
```

## 14. 安全要求

- PostgreSQL 不暴露公网
- `.env` 权限 600
- `ENABLE_DEV_LOGIN=false` 用于生产
- AppSecret 泄露后必须重置
- 不在日志中打印密码、AppSecret、数据库连接串
- 后续建议接入 JWT 和 token 过期时间

## 14. 音频识别与逐字时间轴工具（FunASR）

用于检查古诗朗读音频是否与诗文匹配，并生成可供小程序“逐字高亮/跟读”使用的时间轴数据。这个工具适合放在离线数据处理流程里运行，不建议作为线上 API 每次请求实时调用。

### 14.1 使用场景

- 验证 `poem-*.mp3` 是否读的是对应古诗。
- 识别真实朗读音频文本，发现错读、漏读、音频不匹配的问题。
- 生成每个字/词的大致起止时间，用于前端播放时同步高亮。
- 批量处理 `audios-id/` 或本地临时音频目录中的古诗音频。

### 14.2 本地准备

建议单独建 Python 虚拟环境，避免污染 Rust 后端环境：

```bash
python3 -m venv /tmp/funasr-venv
source /tmp/funasr-venv/bin/activate
python -m pip install --upgrade pip
pip install funasr modelscope torch torchaudio
```

如果本机安装了 `ffmpeg`，音频兼容性会更好：

```bash
# macOS
brew install ffmpeg

# Ubuntu
sudo apt update
sudo apt install -y ffmpeg
```

> 备注：没有 `ffmpeg` 时 FunASR/torchaudio 也可能正常读取 mp3，但遇到格式兼容问题时优先安装 `ffmpeg`。

### 14.3 单条音频验证

示例：识别《登鹳雀楼》的本地音频。

```bash
source /tmp/funasr-venv/bin/activate
python - <<'PY'
import json
from funasr import AutoModel

model = AutoModel(
    model="paraformer-zh",
    vad_model="fsmn-vad",
    punc_model="ct-punc",
)

res = model.generate(
    input="/path/to/workspace/tmp/poem-audio-candidates/登鹳雀楼-王之涣.mp3",
    batch_size_s=300,
)

print(json.dumps(res, ensure_ascii=False, indent=2))
PY
```

常见输出会包含：

```json
{
  "text": "登鹳雀楼唐王之涣白日依山尽黄河入海流欲穷千里目更上一层楼",
  "raw_text": "登 鹳 雀 楼 唐 王 之 涣 白 日 依 山 尽 黄 河 入 海 流 欲 穷 千 里 目 更 上 一 层 楼",
  "timestamp": [[0, 240], [240, 480]]
}
```

字段说明：

| 字段 | 说明 |
|---|---|
| `text` | 识别后的连续文本，可用于和数据库中的标题、作者、正文做匹配校验 |
| `raw_text` | 带空格的原始识别文本，便于观察逐字/分词结果 |
| `timestamp` | 每个字/词片段的起止时间，单位通常为毫秒，可转换为前端播放高亮时间轴 |

### 14.4 批量处理建议

批量处理时建议写成独立脚本，输出 JSON manifest，再由 Rust 后端或数据库迁移脚本导入。

推荐 manifest 结构：

```json
[
  {
    "poem_id": 1,
    "audio_file": "poem-1.mp3",
    "recognized_text": "静夜思唐李白床前明月光疑是地上霜举头望明月低头思故乡",
    "raw_text": "静 夜 思 唐 李 白 ...",
    "timestamps": [[0, 240], [240, 480]],
    "matched": true,
    "notes": "识别文本与标题/作者/正文匹配"
  }
]
```

处理流程建议：

1. 先用 3-5 条音频抽样验证模型、音频格式和时间轴输出是否正常。
2. 批量扫描 `poem-{id}.mp3`。
3. 将识别文本规范化：去空格、去标点、统一繁简/异体字（如有需要）。
4. 与数据库中的 `title + dynasty + author + content` 或 `title + author + content` 做相似度匹配。
5. `matched=false` 的音频单独人工复查，不要直接入库。
6. 入库前保留原始 JSON，方便后续追溯。

### 14.5 和 Rust 后端的关系

Rust 后端仍只负责：

- 提供古诗 API。
- 暴露音频/静态资源 URL。
- 从数据库读取音频时间轴字段并返回给小程序。

FunASR 工具只作为离线数据生产工具使用：

```text
音频文件 -> FunASR 识别/时间轴 -> manifest JSON -> 人工/脚本校验 -> 写入数据库 -> Rust API 返回给小程序
```

如果后续要支持逐字高亮，可以在数据库中增加类似字段：

```sql
ALTER TABLE poems ADD COLUMN audio_timeline JSONB;
```

其中 `audio_timeline` 保存前端需要的精简时间轴，例如：

```json
[
  { "text": "白", "start_ms": 8950, "end_ms": 9190 },
  { "text": "日", "start_ms": 9190, "end_ms": 9430 }
]
```

### 14.6 注意事项

- FunASR 模型首次运行会下载模型文件，耗时较长，建议在开发机或专门的数据处理机上执行。
- 不要在生产 API 请求链路里实时跑识别，CPU/内存开销和响应时间都不可控。
- 识别结果不是 100% 准确，尤其是作者、朝代、停顿和背景音乐较重的音频，必须做匹配校验。
- `timestamp` 和 `raw_text` 的粒度可能不是严格逐字，入库前需要按前端展示需求做一次规整。
