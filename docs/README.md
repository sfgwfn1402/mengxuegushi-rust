# 萌学古诗 Rust 后端文档

本文档目录包含 Rust 后端的设计和部署说明。

## 文档列表

| 文档 | 说明 |
|---|---|
| [01-overview-design.md](./01-overview-design.md) | 概要设计：目标、架构、技术栈、模块划分 |
| [02-detailed-design.md](./02-detailed-design.md) | 详细设计：代码结构、启动流程、鉴权、错误处理 |
| [03-database-design.md](./03-database-design.md) | 数据库设计：表结构、关系、迁移、数据初始化策略 |
| [04-api-design.md](./04-api-design.md) | 接口设计：登录、古诗、用户、进度、收藏、打卡、任务 |
| [05-production-deployment.md](./05-production-deployment.md) | 生产部署：PostgreSQL、systemd、Nginx、HTTPS、MinIO、验证、回滚、FunASR 音频识别与时间轴 |

## 当前设计原则

```text
数据库是主数据源。
小程序不再使用本地古诗备份。
后端不可用时，小程序显示“服务维护中”。
```

## 生产服务

| 项目 | 值 |
|---|---|
| systemd 服务 | mengxuegushi.service |
| 生产目录 | /opt/mengxuegushi |
| 生产二进制 | /opt/mengxuegushi/mengxuegushi-rust |
| 生产环境变量 | /opt/mengxuegushi/.env |
| 生产 API 临时地址 | http://example.com:8080 |
| HTTPS 目标地址 | https://www.example.com/api |

## 注意

- 不要提交 `.env.production` 或真实密钥
- 生产 PostgreSQL 不开放公网
- 小程序上线必须使用 HTTPS 合法域名
