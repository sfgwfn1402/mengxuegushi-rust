## 15.1 2026-07-26 给 poems 表加 custom_lines_json 字段（咏鹅 4 句对齐）

### 背景

详情页《诗内容逐行显示》走 `poem.lines`（iOS `AppCore/Sources/AppCore/Poem.swift` 的
`lines` extension），逻辑是：

1. 优先用 `customLines`（解码后端下发的 `custom_lines` 字段）
2. 否则按 `，。！？；` 标点切 `content`

咏鹅 (`poems.id = 9`) `content = "鹅，鹅，鹅，曲项向天歌。白毛浮绿水，红掌拨清波。"`
按 `，` 切出 6 句：`["鹅，", "鹅，", "鹅，", "曲项向天歌。", "白毛浮绿水，", "红掌拨清波。"]`。
但官方朗读音频（`audios-id/poem-9.mp3`，13.2s）和 `poem-line-audios.json` 都已经是
4 句合并版（"鹅，鹅，鹅，" 一段 2.688s），详情页高亮永远对不上音频。

### 改动

1. **Migration `migrations/202607260001_poem_custom_lines.sql`**
   ```sql
   ALTER TABLE poems ADD COLUMN IF NOT EXISTS custom_lines_json TEXT NOT NULL DEFAULT '[]';
   ```
   - 字段语义：JSON 数组，每元素是一行原文（含标点）
   - `DEFAULT '[]'`：诗没自定义时客户端走默认标点切
   - 与 iOS `Poem.swift` 中 `customLines` 字段（解码 `custom_lines`）对应

2. **Rust `src/models/poem.rs`**：`Poem` struct 加字段
   ```rust
   #[serde(default)]
   pub custom_lines: Option<Vec<String>>,
   ```
   `#[serde(default)]` 兼容 seed JSON 缺字段

3. **Rust `src/services/poem_store.rs`**：
   - `build_list_query` SELECT 加 `custom_lines_json`
   - `select_poem_sql!` 宏 SELECT 加 `custom_lines_json`
   - `row_to_poem` 解析：`"[]"` → `None`（避免下发空数组让客户端走默认切），其他 → `Some(vec)`
   - `insert_poem` 不动（依赖列 `DEFAULT '[]'`，且 `Poem::custom_lines` 缺字段时 `#[serde(default)]` 兜底）

4. **SQL UPDATE 咏鹅写 4 句**：
   ```sql
   UPDATE poems
   SET custom_lines_json = '["鹅，鹅，鹅，","曲项向天歌。","白毛浮绿水，","红掌拨清波。"]'
   WHERE id = 9;
   ```

5. **iOS `AppCore/Sources/AppCore/Poem.swift`**：保持 `Poem.lines` 优先用 `customLines`

## 15.2 2026-07-26 iOS 1.1.0 (build 2) 第二次发布 + 全链路修复

### 背景

v1.0 (build 1) 2026-07-18 审核通过，174 国（不含大陆）已上架销售。
07-20 App 备案通过拿到 `京ICP备2026033440号-3A`。
本批改动：7 个 commit，覆盖 24 个文件 + 4 个新文件。

### 改动一览

| Commit | 主题 | 文件数 |
|---|---|---|
| `abead78` | fix(audio): 修整诗/跟读音频 + NSURLCache 30 天缓存命中 | 7 |
| `ecfe142` | feat(poem): 后端下发 custom_lines + audio_version | 3 |
| `101220a` | fix(subscription): 验单上报去重表改为成功后才写 | 3 |
| `e013869` | feat(admin + analytics): 数据看板 + 远程事件统计 | 3 |
| `e4f92ef` | feat(debug): 设备本地 DebugLog | 2 |
| `e01431b` | chore: 学习/闯关/创作/首页/图缓存 细节 | 9 |
| `fd65a0f` | chore(release): bump to 1.1.0 (build 2) | 1 |
| `51daab9` | chore(release): Info.plist 同步到 1.1.0 (2) | 1 |

### 关键修复详解

#### 1. NSURLCache 30 天缓存命中老响应（abead78）

**根因**：iOS `URLSession.shared.data(from:)` 默认 `.useProtocolCachePolicy`，严格遵守 HTTP
`Cache-Control: max-age=2592000`（nginx `expires 30d`）。云端 mc cp 覆盖音频后，
iOS 端仍返回磁盘层 30 天老响应。

**修法**（`MengxueGushi/Media/AudioCache.swift`）：
```swift
var req = URLRequest(url: remote,
                    cachePolicy: .reloadIgnoringLocalAndRemoteCacheData,
                    timeoutInterval: 15)
req.setValue("MengxueGushi/1.0 iOS", forHTTPHeaderField: "User-Agent")
let (data, _) = try await URLSession.shared.data(for: req)
```

**未踩坑**：
- `init()` 跑 `URLCache.shared.removeCachedResponse` 不可靠（iOS API 行为）
- `AudioDiskCache` 命中本地时仍走本地（`if FileManager.default.fileExists`），不破坏 cache
- 覆盖装 binary 后必须显式杀进程让 init() 跑（iOS 14+ 不一定杀进程）

#### 2. 整诗/跟读音频异常（abead78）

**咏鹅 (id=9)**：原版 22.1s 含引导语（"咏鹅，骆宾王，唐代，鹅、鹅鹅，曲项向天歌..."），
跟读第 1 句 1.77s（FunASR 字符 end + 0.4s 字尾音缓冲）。
**游子吟 (id=8)**：原版 12.1s，6 句（含"慈母手中线..."），`poem-line-audios.json` key 8
url 写 `poem-74-line-X.mp3`（命名历史遗留，实际云端 `poem-74` 才是游子吟分句）。

**`poem-line-audios.json` 4 句 start/end 累计时间戳**（按整诗版真实时长算）：

| 诗 | id | 整诗时长 | 4 句 start/end |
|---|---|---|---|
| 咏鹅 | 9 | 22.1s | 鹅鹅鹅: 1.69-3.39 / 曲项向天歌: 3.39-4.91 / 白毛浮绿水: 4.91-6.43 / 红掌拨清波: 6.43-7.85（用宽区间，end=下一句 start）|
| 游子吟 | 8 | 12.1s | 6 句累计 |

**跟读模式**：用 `ctrl.lineIndex` 走第几句，json start/end 不影响（独立 mp3 拼接）

#### 3. 后端下发 custom_lines + audio_version（ecfe142）

`Poem` struct 加 2 字段：
- `audioVersion: String?` → 解码 `audio_version` → 拼进 URL query `?v=<version>` 绕过本地缓存
- `customLines: [String]?` → 解码 `custom_lines` → 优先于 `content` 标点切

`poems-snapshot.json`：75 首诗全量加 `audio_version: "20260726-init"` + `custom_lines: null`
（仅咏鹅后端填了真值）

**`Poem.lines` 优先级**：
```swift
var lines: [String] {
    if let custom = customLines, !custom.isEmpty { return custom }  // 后端下发
    // 标点切 fallback
    for ch in content {
        cur.append(ch)
        if ch == "，" || ch == "。" || ch == "！" || ch == "？" || ch == "；" {
            out.append(cur); cur = ""
        }
    }
}
```

#### 4. 验单上报去重表改为成功后才写（101220a）

**原 bug**：游客购买/弱网失败时去重表已写，下次再不上报 → 永久漏报。
**修法**：去重表只在上报成功（后端 200）后才写。

#### 5. 数据看板 + 远程事件（e013869）

后端 `GET /api/admin/analytics` 已就绪，iOS 端：
- `AdminAnalyticsView`：DAU 柱状图 + 事件分布 + 热门诗 Top10 + 最近错误
- `AnalyticsService`：10 条/15s flush + 退后台 flush + 失败回队 + 50 条上限

#### 6. DebugLog 设备本地日志（e4f92ef）

`Documents/mengxue-debug.log`，冷启清空。远程拉 log 命令：
```bash
xcrun devicectl device copy from \
  --device AC91E147-1802-5A0E-AAAA-1A5A6B437A65 \
  --domain-type appDataContainer \
  --domain-identifier com.duwei.mengxuegushi \
  --source Documents/mengxue-debug.log
```

### 上线流程

**iOS 端**（archive + upload 一次跑通，2026-07-26 22:09-22:19）：

```bash
# 1. 改 version（project.yml source-of-truth）
MARKETING_VERSION: "1.0" → "1.1.0"
CURRENT_PROJECT_VERSION: "1" → "2"

# 2. 同步 Info.plist 字面量
CFBundleShortVersionString: "1.0" → "1.1.0"
CFBundleVersion: "1" → "2"

# 3. 重新生成 xcodeproj
xcodegen generate

# 4. Archive（命令行 archive 必须显式传 DEVELOPMENT_TEAM，Xcode GUI 默认注入）
xcodebuild \
  -project MengxueGushi.xcodeproj \
  -scheme MengxueGushi \
  -configuration Release \
  -destination 'generic/platform=iOS' \
  -archivePath build/MengxueGushi-1.1.0.xcarchive \
  DEVELOPMENT_TEAM=82KWBKH76T \
  archive

# 5. ExportArchive + Upload（一次到位）
cat > /tmp/ExportOptions.plist <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0">
<dict>
  <key>destination</key><string>upload</string>
  <key>method</key><string>app-store</string>
  <key>signingStyle</key><string>automatic</string>
  <key>teamID</key><string>82KWBKH76T</string>
  <key>uploadSymbols</key><true/>
</dict>
</plist>
EOF

xcodebuild -exportArchive \
  -archivePath build/MengxueGushi-1.1.0.xcarchive \
  -exportPath build/1.1.0 \
  -exportOptionsPlist /tmp/ExportOptions.plist
```

**App Store Connect**（用户在浏览器操作）：

1. https://appstoreconnect.apple.com/apps/6789952342/distribution/ios/version/inflight
2. 选 1.1.0 版本 → 选刚上传的 build（处理中转圈，等几分钟变绿）
3. 填"此版本的新增功能"（中文 + 英文）
4. 提交审核

### Release Notes（ASC "新增功能" 字段）

**中文**：
```
v1.1.0 更新：

【修复】
• 修复多首古诗整诗/跟读音频异常（重新拼接朗诵引导语与字尾音，告别断尾）
• 修复本地缓存导致音频无法更新到最新版本的问题
• 订阅验单上报优化，弱网/游客购买不再永久漏报

【新增】
• 管理员数据看板：DAU 曲线、事件分布、热门诗 Top 10、最近错误
• 远程使用事件统计（启动、页面、跟读、背诵等行为汇总）

【优化】
• 创作、闯关、详情页若干细节调整
• 启动稳定性与崩溃日志采集

祝小朋友读诗愉快 ✨
```

**英文**：
```
What's New in v1.1.0:

Fixes:
• Fixed audio playback issues for several classic poems — restored
  reciter intro and trailing sounds, no more truncated line endings
• Fixed stale local audio cache preventing users from hearing
  freshly updated recordings
• Improved subscription receipt reporting so weak-network and
  guest purchases no longer drop silently

New:
• Admin analytics dashboard — DAU curve, event distribution,
  top 10 popular poems, and recent error feed
• Remote usage analytics (launches, page views, follow-read,
  recite, etc.)

Improvements:
• Polish across Create, Quest, and Detail screens
• Hardened app startup and on-device error logging

Happy reading!
```

### 销售范围

继续 174 国（不含大陆），备案号 `京ICP备2026033440号-3A` 已下来但单独
再做一次提审加回大陆（避免 1.1.0 一次过承担过大风险）。

### 回滚方案

如 1.1.0 审核被拒：
- 不删 ASC build，等修问题重新提交
- 用户端不受影响（174 国老用户仍跑 v1.0 (1)）
- 走 ASC 标准"拒绝后重提"流程

如线上发现严重 bug：
- 在 ASC 把版本下架（手动 → 暂停销售）
- hotfix 出 1.1.1 (3) 重提

### 待办（用户）

- [ ] 浏览器进 ASC 选 1.1.0 (2) build + 填 changelog + 提交审核
- [ ] 监控 1-3 天审核结果
- [ ] 审核通过后：手动发布版本（按首次流程"控制上线时间"）
- [ ] 同步加大陆：单独再做一次提审（加备案号 + 勾中国大陆）
