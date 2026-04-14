# CHANGELOG

本文件记录 Obsidian Mirror 各版本的变更历史，遵循 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.0.0/) 格式。

> v1.4.10 及之前的所有历史条目已归档至 [CHANGELOG-ARCHIVE.md](./CHANGELOG-ARCHIVE.md)。

## [Unreleased]

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
