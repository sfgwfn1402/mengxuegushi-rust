# 萌学古诗 Rust 后端概要设计

## 1. 项目目标

萌学古诗 Rust 后端为微信小程序提供统一的业务 API，负责古诗内容、用户登录、学习进度、收藏、打卡、每日任务、成语学习进度、音频地址等核心能力。

目标是把小程序中的业务主数据从本地缓存/写死数据迁移到服务端数据库，保证：

- 用户换设备后学习数据不丢失
- 古诗内容可在数据库中维护
- 收藏、进度、打卡、星星等数据可长期保存
- 后续可扩展管理后台、会员、AI 讲解、推荐学习等功能

## 2. 总体架构

```text
微信小程序
  |
  | HTTPS / HTTP API
  v
Rust API 服务 axum
  |
  | SQLx
  v
PostgreSQL

Rust API / 小程序
  |
  | 音频 URL
  v
MinIO 对象存储
```

## 3. 技术栈

| 层级 | 技术 |
|---|---|
| 服务端语言 | Rust |
| Web 框架 | axum |
| 异步运行时 | tokio |
| 数据库 | PostgreSQL |
| 数据库访问 | sqlx |
| HTTP 客户端 | reqwest |
| 日志 | tracing / tracing-subscriber |
| 静态文件服务 | tower-http ServeDir |
| 对象存储 | MinIO |
| 进程管理 | systemd |
| 反向代理 | Nginx |

## 4. 运行环境

### 本地环境

- Mac 本地 PostgreSQL
- 本地 Rust 服务监听 `8080`
- 小程序 dev 环境请求 Mac 局域网 IP，例如：

```text
http://192.168.1.230:8080/api
```

### 生产环境

- 腾讯云 Ubuntu Server 24.04
- PostgreSQL 16，只监听 `127.0.0.1:5432`
- Rust 服务监听 `0.0.0.0:8080`
- MinIO 监听 `9000/9001`
- Nginx 负责公网入口和 HTTPS

## 5. 核心模块

| 模块 | 说明 |
|---|---|
| auth | 微信登录、开发登录 |
| poems | 古诗列表、筛选、搜索、详情 |
| me | 当前用户、用户画像、学习数据 |
| progress | 古诗学习进度 |
| favorites | 古诗收藏 |
| activity | 打卡、每日任务、星星、统计 |
| idiom progress | 成语学习进度 |
| audio | 音频 URL，生产环境指向 MinIO |

## 6. 数据原则

当前设计原则：

```text
数据库是主数据源。
小程序不再使用本地古诗备份作为业务数据来源。
后端不可用时，小程序显示“服务维护中”。
```

本地缓存只允许保存：

- 登录 token
- 用户基本登录信息
- 页面跳转临时参数

不再用本地缓存保存主业务数据，例如：

- 星星
- 打卡
- 学习进度
- 收藏
- 每日任务

## 7. 部署现状

生产服务器已完成：

- PostgreSQL 安装和初始化
- 生产数据库 `mengxuegushi`
- Rust API systemd 服务 `mengxuegushi.service`
- MinIO 已存在并有音频对象
- 100 首古诗已导入 PostgreSQL
- 音频 URL 已切到 MinIO

仍需完善：

- Nginx + HTTPS 公网访问稳定性
- 小程序正式生产域名配置
- 微信 AppSecret 泄露后建议重置
