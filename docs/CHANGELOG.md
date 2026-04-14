# CHANGELOG

本文件记录 Obsidian Mirror 各版本的变更历史，遵循 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.0.0/) 格式。

> v1.4.10 及之前的所有历史条目已归档至 [CHANGELOG-ARCHIVE.md](./CHANGELOG-ARCHIVE.md)。

## [Unreleased]

---

## [v1.5.5] — 2026-04-14

实时通知与运维增强：SSE 同步进度、优雅关闭、同步历史记录。

### Added
- **`GET /api/sync/events`** — Server-Sent Events 端点：同步期间通过 `tokio::sync::broadcast` 实时推送 `{stage, progress, message}` 事件（git/scan/markdown/index/search/persist/done）
- **`GET /api/sync/history`** — 返回最近 10 次同步历史记录 `{started_at, finished_at, notes_count, status, duration_ms}`
- **`SyncProgressEvent` / `SyncRecord`** 结构体（`sync.rs`），供 SSE 和历史记录使用
- **`AppState` 新增字段**（`state.rs`）：`sync_progress_tx`（broadcast Sender）、`sync_history`（VecDeque，TokioRwLock）、`background_tasks`（Mutex<Vec<JoinHandle>>）

### Changed
- **`/health` 响应新增 `last_sync_record` 字段**：最近一次同步记录（`null` 表示从未同步）
- **优雅关闭增强**（`main.rs`）：HTTP 服务器停止后，等待所有后台任务（Tantivy 重建、redb 持久化）完成，超时上限 30 秒后强制退出
- `perform_sync` 将搜索索引重建和持久化任务的 `JoinHandle` 存入 `AppState.background_tasks`，过期句柄自动清理

---

## [v1.5.4] — 2026-04-14

Obsidian 语法完整支持：笔记内嵌、脚注、Mermaid 主题注入、Callout 折叠动画。

### Added
- **笔记内嵌**（`![[笔记.md]]` / `![[笔记.md#章节]]`）：
  - `markdown.rs` 将内嵌语法编译为占位符 div（`class="note-embed-placeholder"`）
  - `handlers.rs` 新增 `expand_embeds()` 函数，在 `doc_handler` 响应时展开为原生 `<details>/<summary>` 折叠块，深度限制 2 层防止循环内嵌
  - `static/css/embed.css`：内嵌块 CSS（深/浅色适配）
- **脚注支持** `[^1]`：开启 pulldown-cmark `ENABLE_FOOTNOTES`，自动生成双向跳转锚点
- **Mermaid 主题注入**（`mermaid-init.js`）：渲染/重渲染前自动注入 `%%{init: {"theme": "..."}}%%`，确保图表跟随全局主题切换；若图表已有 init 指令则跳过
- **Callout 折叠 CSS 动画**（`callout.css`）：`display:none` 改为 `max-height` + `opacity` transition，展开/折叠有 0.3s 平滑过渡
- **Callout 折叠状态持久化**（`callout.js`）：新增 `saveCalloutState` / `restoreCalloutStates` 函数，用 `localStorage` 记录每个 Callout 在当前页面的折叠状态，刷新后自动恢复

---

## [v1.5.3] — 2026-04-14

多用户与权限管理：三级角色体系、管理员 API、/admin/users 页面。

### Added
- **`UserRole` 枚举**（`auth_db.rs`）：`admin` / `editor` / `viewer` 三级角色，`#[serde(default)]` 旧数据自动升级为 admin
- **`User.role` 字段**：`User` 结构体新增角色字段，JWT Claims 同步携带 `role`
- **认证中间件注入 `UserRole`**：验证 Token 后将角色写入请求扩展，handler 通过 `req.extensions().get::<UserRole>()` 获取
- **管理员用户管理 API**（均需 admin 角色）：
  - `GET /api/admin/users` — 用户列表（JSON）
  - `POST /api/admin/users` — 创建用户（指定角色）
  - `DELETE /api/admin/users/{username}` — 禁用用户
  - `POST /api/admin/users/{username}/reset-password` — 重置密码
- **`GET /admin/users`** — 管理员用户管理页面（Askama 模板，含角色徽章和操作按钮）
- **`GET /api/share/list?all=true`** — 管理员查看全部用户分享链接（普通用户只能看自己的）
- **`ShareDatabase::list_all_shares()`** 方法

### Changed
- **`POST /sync` 和 `POST /api/config/reload` 现在需要 admin 角色**（auth 未启用时全放行）
- `JwtManager::generate_token` 签名新增 `role: &str` 参数

---

## [v1.5.2] — 2026-04-14

搜索体验全面升级：模糊建议、路径上下文、`<mark>` 高亮摘要、搜索历史持久化。

### Added
- **`GET /api/suggest?q=`** 搜索建议端点：内存前缀匹配优先 + Tantivy `FuzzyTermQuery`（编辑距离 ≤1）补充，合并去重后返回 `[{title, path}]`
- **`/api/titles` 增加 `note_items` 字段**：`[{title, path}]` 列表，兼容保留原有 `titles` 字符串数组
- **搜索历史 API**（存入 `reading_progress_db` 复用，新表 `search_history`）：
  - `POST /api/search/history` — 记录搜索词（需认证，每用户保留最近 50 条）
  - `GET /api/search/history?limit=` — 获取历史（默认 20 条）
  - `DELETE /api/search/history` — 清空历史
- **`SearchHistoryEntry` 数据结构**（`reading_progress_db.rs`）+ `SEARCH_HISTORY_TABLE` redb 表
- **`SearchEngine::fuzzy_suggest()`** 方法：基于 `FuzzyTermQuery` 返回标题模糊建议

### Changed
- **搜索摘要 `<mark>` 高亮**：`generate_snippet` 重构，在命中上下文中用 `<mark>…</mark>` 包裹关键词，前端无需额外处理即可渲染高亮效果；新增 `highlight_terms` 辅助函数

---

## [v1.5.1] — 2026-04-14

代码审计修复版本（CODEREVIEW_1.5）。

### Fixed
- **[B1] bcrypt verify spawn_blocking panic 静默为密码错误**：`auth_handlers.rs` 的 `login_handler` 和 `change_password_handler` 中，`spawn_blocking` 线程 panic 时 `.unwrap_or(Ok(false))` 会静默返回"密码错误"而不记录错误日志。改为 `.unwrap_or_else(|e| { error!(...); Err(...) })`，panic 时正确返回 500
- **[B2] `ShareLink::new()` bcrypt hash 在 async 上下文直接执行**：`create_share_handler` 直接调用 `ShareLink::new()`，其内部的 `bcrypt::hash`（~100-300ms CPU）阻塞 Tokio worker 线程。将 `ShareLink::new()` 和 `db.create_share()` 合并到同一个 `spawn_blocking` 闭包中；`ShareLink::new` 文档注释补充调用方要求。`share_db.rs` 的 `.expect()` 替换为 `unwrap_or_else` + 错误日志

### 审计统计
- 🟠 P1 修复：2 项（B1/B2）
- 发现问题总计：7 项（含接受 3 项、推迟 2 项）

---

## [v1.5.0] — 2026-04-14

架构加固：清偿 CODEREVIEW_1.4 全部推迟项，消灭运行时潜在 panic 和阻塞隐患。

### Changed
- **[A1] redb IO 全面移入 spawn_blocking**：`auth_handlers`、`share_handlers`、`reading_progress_handlers` 中所有 redb 同步 IO（`begin_read/write`、`commit`）以及 bcrypt 密码计算均通过 `tokio::task::spawn_blocking` 执行，避免阻塞 Tokio 线程池工作线程
- **[B2] AppConfig 热重载真正生效**：`AppState.config` 改为 `std::sync::RwLock<AppConfig>`；`config_reload_handler` 现在实际写入新配置，之后触发同步时使用最新配置
- **[B3] /health `uptime_seconds` 修复**：新增 `AppState.start_time: Instant`，返回真实运行时长而非 Unix 时间戳
- **[E1] Rayon mutex 中毒优雅恢复**：`sync.rs` 中 `results.into_inner().unwrap()` 改为 `.unwrap_or_else(|e| e.into_inner())`，防止 Rayon worker panic 时主线程连带 panic
- **[E2] 模板渲染错误返回 JSON**：所有 `InternalServerError().body(format!("Template error..."))` 统一改为 `json!({"error": "..."})` 格式，与其他 API 响应一致
- **[Q2] 分享 URL scheme 改用 X-Forwarded-Proto**：新增 `AppConfig.public_base_url: Option<String>` 配置项；分享链接生成优先使用该字段，其次读取 `X-Forwarded-Proto` header，最后 fallback 到 Host header 判断
- **[Q3] Git commit 读取函数合并**：`handlers.rs::read_local_git_commit`、`sync.rs::get_current_git_commit`、`main.rs::get_git_commit` 三处重复实现移除，统一使用 `GitClient::get_current_commit`（公开化）

### Added
- **[T3] config_reload_handler 集成测试**：验证未认证调用返回 401
- `AppConfig.public_base_url` 配置项（`config.example.ron` 已补充注释）

---
