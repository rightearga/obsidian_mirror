# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 项目概述

**Obsidian Mirror** 是一个只读 Web 服务器，将 Obsidian 笔记库（Markdown 文件）镜像为可浏览的网站。它通过 Git 克隆/同步包含 `.md` 文件的仓库，处理后以 HTTP 方式提供带侧边栏、搜索、反向链接、标签和关系图谱的网页服务。

## 构建与运行命令

```bash
# 构建
cargo build
cargo build --release

# 运行（需要工作目录下存在 config.ron）
cargo run

# 自定义日志级别运行
RUST_LOG=debug cargo run

# 运行所有测试
cargo test

# 运行单个测试
cargo test <测试名称>

# Docker 构建与运行
docker build -t obsidian_mirror .
docker compose up -d
```

## 配置文件

配置文件使用 RON 格式（`config.ron`），可从 `config.example.ron` 复制：

```ron
(
    repo_url: "http://your-git-server.com/your-repo.git",
    local_path: "./my-note",
    listen_addr: "0.0.0.0:3080",
    workers: 4,
    ignore_patterns: [".obsidian", ".trash"],
    database: (
        index_db_path: "./index.db",
        auth_db_path: "./auth.db",
        share_db_path: "./share.db",
        reading_progress_db_path: "./reading_progress.db",
    ),
    security: (
        auth_enabled: false,
        jwt_secret: "CHANGE_THIS",
        token_lifetime_hours: 24,
        default_admin_username: "admin",
        default_admin_password: "admin",
        force_https_cookie: false,     // v1.4.6：仅 HTTPS 时设为 true
    ),
    sync_interval_minutes: 0,          // v1.4.5：定时同步（0=禁用）
    webhook: (
        enabled: false,
        secret: "CHANGE_THIS",
    ),
    // 多仓库支持（v1.7.4，可选）：repos: [(name:"personal", repo_url:..., local_path:..., ignore_patterns:[...]), ...]
)
```

若 `config.ron` 不存在，服务器将使用默认值：`local_path: "./vault_data"`，`listen_addr: "127.0.0.1:8080"`。

## 架构说明

### 数据流

启动时及 `POST /sync` 触发时，同步管道（`src/sync.rs: perform_sync`）按以下步骤执行：

1. **Git 同步**（`src/git.rs`）——克隆或 `git pull`，返回 `SyncResult::{InitialClone, IncrementalUpdate, NoChange}`
2. **文件扫描**（`src/scanner.rs`）——遍历 `local_path`，收集 `.md` 文件（遵循 `ignore_patterns`）
3. **并行 Markdown 处理**（`src/sync.rs: process_markdown_files`）——使用 Rayon 线程池，对每个文件调用 `MarkdownProcessor::process`
4. **索引构建**（`src/indexer.rs`）——链接索引、反向链接（基于 `Note.outgoing_links` 全量重建）、标签索引、资源文件索引（图片/PDF）
5. **侧边栏重建**（`src/sidebar.rs`）——生成 `SidebarNode` 树形结构
6. **Tantivy 搜索索引更新**（`src/search_engine.rs`）——全文检索 + CJK 中文分词；`IncrementalUpdate` 时调用 `update_documents`（只传变更文件内容），`InitialClone` 时全量重建；`NoChange` + 持久化命中且 Tantivy 有内容时跳过重建
7. **持久化**（`src/persistence.rs`）——将索引以 Git commit hash 为键保存到 `redb`；笔记按 1000 条分批写入，metadata 最后提交；下次启动若 commit 未变则直接恢复

**注：** v1.4.9 起 `Note.content_text` 已移除，原始 Markdown 内容在处理期直接传给 Tantivy 后丢弃，不再驻留内存。搜索索引由 Tantivy 磁盘索引维持。`ProcessedNote = (path, Note, outgoing_links, Option<content>)`，`Option<String>` 为 `Some` 表示新处理，`None` 表示缓存复用。

### 应用状态（`src/state.rs`）

所有内存数据存储在 `Arc<AppState>` 中，各字段使用 `tokio::sync::RwLock` 保护：

- `config: std::sync::RwLock<AppConfig>` — 运行时配置，支持热重载（`config_reload_handler` 写入新值）；读取：`.config.read().unwrap()`，**禁止持有读锁跨越 `.await` 点**（v1.5.0）
- `start_time: Instant` — 应用启动时间，供 `/health` 端点返回真实 `uptime_seconds`（v1.5.0）
- `notes: HashMap<String, Note>` — 相对路径 → `Note`（标题、HTML、标签、目录、反向链接、frontmatter、修改时间、出链列表 `outgoing_links`）；注：v1.4.9 起 `content_text` 已移除，内容不再驻留内存
- `link_index: HashMap<String, String>` — 笔记标题/文件名 → 相对路径
- `backlinks: HashMap<String, Vec<String>>` — 笔记标题 → 链接到它的笔记标题列表
- `tag_index: HashMap<String, Vec<String>>` — 标签名 → 包含该标签的笔记标题列表
- `file_index: HashMap<String, String>` — 文件名 → 完整相对路径（用于图片等资源）
- `sidebar: Vec<SidebarNode>` — 侧边栏树形结构
- `search_engine: Arc<SearchEngine>` — Tantivy 索引实例（内含缓存的 `IndexReader`）
- `share_db`、`reading_progress_db` — 基于 redb 的数据库
- `sync_lock: tokio::sync::Mutex<()>` — 同步互斥锁，防止并发 `/sync` 导致 IndexWriter 冲突
- `insights_cache: TokioRwLock<InsightsCache>` — 笔记洞察缓存（v1.7.3），每次同步后更新；v1.8.4 新增 `most_linked_notes`/`monthly_link_counts`；v1.9.3 新增 `tag_cooccurrence`（标签共现矩阵）、`connectivity`（连通度 Top 10）、`reading_hotmap`（阅读频率热力图）；v1.9.4 新增 `monthly_char_counts`（每月字符数）

v1.7.4 新增 `VaultRegistry`（`src/state.rs`）：多仓库支持，持有所有仓库的 `Arc<AppState>`，scoped routes 通过 `app_data()` 覆盖注入对应 AppState。

### Markdown 处理（`src/markdown.rs`）

`MarkdownProcessor::process` 将原始 `.md` 转换为 `(html, links, tags, frontmatter, toc)`：

- 提取 YAML frontmatter（`---` 块）
- 将 Obsidian `[[wikilinks]]` 转换为 HTML `<a>` 标签（同时收集 `links` 列表，存入 `Note.outgoing_links`）
- 将 `![[image.png]]` 转换为 `<img src="/assets/...">`，将 `![[file.pdf]]` 转换为 `<a href="/assets/...">`
- 从 frontmatter 和行内 `#tag` 语法中提取标签（通过 `src/tags.rs`）
- 使用 `pulldown-cmark`（开启 SIMD）渲染 HTML；标题文本经 `html_escape()` 转义，防止 XSS
- 从标题提取目录（TOC）
- 所有正则表达式通过 `lazy_static!` 预编译，仅在进程启动时编译一次

### HTTP 路由（`src/main.rs`、`src/handlers.rs`）

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/` | 首页（优先 README.md/index.md，否则重定向到第一个笔记） |
| GET | `/doc/{path}` | 按路径或标题渲染笔记 |
| GET | `/tags` | 标签列表页 |
| GET | `/tag/{name}` | 某标签下的笔记列表 |
| GET | `/assets/{filename}` | 提供笔记库中的图片/文件（v1.4.10：canonicalize 路径遍历防护） |
| GET | `/api/search` | Tantivy 搜索（参数：`q`、`sort_by`、`tags`、`folder`、`date_from`、`date_to`、`page`、`per_page`，v1.8.0 新增分页） |
| GET | `/api/graph` | 笔记关系图谱（参数：`note`、`depth` 1–3） |
| GET | `/api/preview` | 笔记预览 HTML 片段 |
| GET | `/api/stats` | 笔记数、标签数、近期更新统计 |
| GET | `/health` | 健康检查 JSON |
| GET | `/metrics` | Prometheus 指标 |
| POST | `/sync` | 触发 Git 同步 |
| GET/POST | `/share/{token}` | 公开分享页面 |
| POST | `/api/share/create` | 创建分享链接 |
| GET | `/api/auth/login` | JWT 登录 |
| GET | `/orphans` | 孤立笔记列表（无出链且无入链） |
| GET | `/random` | 随机跳转到一篇笔记（重定向） |
| GET | `/recent` | 最近更新笔记列表（`?days=` 参数） |
| GET | `/api/titles` | 所有笔记标题/路径/标签（供前端自动补全，v1.5.2 新增 `note_items`） |
| GET | `/api/suggest` | 搜索建议（`?q=`，内存前缀 + FuzzyTermQuery，返回 `[{title,path}]`）(v1.5.2) |
| GET | `/api/sync/events` | SSE 同步进度流（`text/event-stream`，各阶段 JSON 事件）(v1.5.5) |
| GET | `/api/sync/history` | 最近 10 次同步历史记录（v1.5.5） |
| GET | `/admin/users` | 用户管理页面（需 admin）(v1.5.3) |
| GET/POST | `/api/admin/users` | 用户列表/创建（需 admin）(v1.5.3) |
| DELETE/POST | `/api/admin/users/{u}` | 禁用/重置密码（需 admin）(v1.5.3) |
| POST/GET/DELETE | `/api/search/history` | 搜索历史记录（需认证）(v1.5.2) |
| GET | `/api/graph/global` | 全库关系图谱（`?hide_isolated=` 参数） |
| GET | `/graph` | 全局知识图谱专页（全屏，含工具栏、聚类着色，v1.7.0） |
| GET | `/insights` | 笔记洞察 Dashboard（写作趋势/健康度/标签云，v1.7.3） |
| GET | `/api/vaults` | 所有仓库名称列表（v1.7.4 多仓库） |
| ANY | `/r/{name}/...` | 多仓库路由前缀（所有仓库特定路由均可加此前缀，v1.7.4） |
| GET | `/timeline` | 时间线视图（按 frontmatter date/mtime 排列，v1.8.4） |
| GET | `/api/timeline` | 时间线数据 JSON（含 date/tags/mtime，v1.8.4） |
| GET | `/knowledge-map` | 知识地图全屏专页（标签相似度聚类，Canvas 渲染，v1.9.5） |
| GET | `/api/knowledge-map` | 知识地图数据 JSON（笔记 + tags + pagerank，v1.9.5） |
| GET | `/api/graph/path` | 笔记最短路径（BFS，`?from=&to=`，最多 6 跳，v1.9.2） |
| GET | `/feed.xml` | Atom 1.0 订阅（`?tag=`/`?folder=` 过滤，v1.8.2） |
| POST | `/api/export/html` | 静态站点 zip 导出（v1.8.2） |
| GET | `/api/insights/stats` | 洞察统计数据 JSON（InsightsCache，v1.7.3） |
| GET | `/doc/{path}/history` | 笔记提交历史列表（git log --follow，v1.7.2） |
| GET | `/doc/{path}/at/{commit}` | 历史版本快照（git show，Markdown 渲染，v1.7.2） |
| GET | `/doc/{path}/diff/{commit}` | 提交 diff（行级 HTML，XSS 转义，v1.7.2） |
| POST | `/webhook/sync` | Webhook 触发同步（GitHub/GitLab 签名验证，需 webhook.enabled=true） |
| POST | `/api/config/reload` | 配置热重载（需认证，重新读取 config.ron 并触发同步） |

### 模板系统（`src/templates.rs`、`templates/`）

使用 **Askama**（编译期 Jinja2 风格模板），所有模板继承自 `layout.html`。模板结构体定义在 `src/templates.rs` 中，与 `templates/` 目录下的文件一一对应。新增模板字段时需同时修改结构体和 HTML 文件。

### 认证系统（`src/auth.rs`、`src/auth_db.rs`、`src/auth_middleware.rs`）

由 `security.auth_enabled` 控制的可选 JWT 认证：

- `AuthMiddleware` 拦截所有请求；精确匹配公开路径（`/login`、`/api/auth/login`），前缀匹配（`/static/`、`/share/`）
- 认证失败时：`/api/*` 路径返回 `401 + JSON {"error":"未认证"}`；页面路径重定向到 `/login`（v1.4.10）
- v1.5.3：Token 携带 `role` 字段（admin/editor/viewer），中间件注入 `UserRole` 到请求扩展；`/sync` 和 `/api/config/reload` 需要 admin 角色
- Token 通过 Cookie（`auth_token`，含 `Secure` + `SameSite::Lax`）或 `Authorization: Bearer` 头传递
- 用户以 bcrypt 哈希密码存储在 `redb` 中
- 首次启动若数据库为空，自动创建默认管理员账户

### 持久化（`src/persistence.rs`）

使用 **redb**（嵌入式键值存储）。`IndexPersistence` 用 **postcard**（二进制格式）序列化所有内存索引。启动时若已保存的 Git commit hash 与当前 HEAD 匹配，则直接恢复索引（跳过全量重处理）。以下情况会使缓存失效：Git commit 变更、`ignore_patterns` 变更、`CURRENT_VERSION`（当前为 **3**，v1.4.9 升级）升级。

写入策略：笔记按 1000 条/事务分批提交，metadata 最后写入（作为原子完成标记）；中途崩溃时 metadata 未写入，下次启动安全触发全量重建。

### 关键依赖（v1.8.5 更新）

| 依赖 | 版本 | 用途 |
|------|------|------|
| actix-web | 4.13.0 | HTTP 服务器与路由 |
| tokio | 1.52.0 | 异步运行时 |
| tantivy | 0.26.0 | 全文搜索索引（存储于 `{index_db_path.parent()}/.search_index/`） |
| jieba-rs | 0.9.0 | 中文分词器 |
| redb | 4.0.0 | 嵌入式 KV 数据库（持久化索引、认证、分享、阅读进度） |
| askama | 0.15.6 | 编译期 HTML 模板 |
| pulldown-cmark | 0.13.3 | Markdown → HTML 渲染（SIMD 加速） |
| wasm-bindgen | 0.2.118 | Rust ↔ JS 互操作（`crates/wasm/`） |
| rayon | 1.12.0 | 并行笔记处理 |
| serde / serde_json | 1.0.228 / 1.0.149 | 序列化 |
| chrono | 0.4.44 | 日期时间处理 |
| hmac / sha2 | 0.13.0 / 0.11.0 | Webhook HMAC-SHA256 签名验证 |
| zip | 8.5.1 | 静态站点导出 zip 打包 |
| bcrypt | 0.19.0 | 密码哈希 |
| postcard | 1.1.3 | 二进制序列化（持久化） |
