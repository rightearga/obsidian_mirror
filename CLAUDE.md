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
    ),
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
6. **Tantivy 搜索索引更新**（`src/search_engine.rs`）——全文检索 + CJK 中文分词；`IncrementalUpdate` 时调用 `update_documents` 增量更新，`InitialClone` 时全量重建
7. **持久化**（`src/persistence.rs`）——将索引以 Git commit hash 为键保存到 `redb`；笔记按 1000 条分批写入，metadata 最后提交；下次启动若 commit 未变则直接恢复

### 应用状态（`src/state.rs`）

所有内存数据存储在 `Arc<AppState>` 中，各字段使用 `tokio::sync::RwLock` 保护：

- `notes: HashMap<String, Note>` — 相对路径 → `Note`（标题、HTML、原文、标签、目录、反向链接、frontmatter、修改时间、**出链列表 `outgoing_links`**）
- `link_index: HashMap<String, String>` — 笔记标题/文件名 → 相对路径
- `backlinks: HashMap<String, Vec<String>>` — 笔记标题 → 链接到它的笔记标题列表
- `tag_index: HashMap<String, Vec<String>>` — 标签名 → 包含该标签的笔记标题列表
- `file_index: HashMap<String, String>` — 文件名 → 完整相对路径（用于图片等资源）
- `sidebar: Vec<SidebarNode>` — 侧边栏树形结构
- `search_engine: Arc<SearchEngine>` — Tantivy 索引实例（内含缓存的 `IndexReader`）
- `share_db`、`reading_progress_db` — 基于 redb 的数据库
- `sync_lock: tokio::sync::Mutex<()>` — 同步互斥锁，防止并发 `/sync` 导致 IndexWriter 冲突

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
| GET | `/assets/{filename}` | 提供笔记库中的图片/文件 |
| GET | `/api/search` | Tantivy 搜索（参数：`q`、`sort_by`、`tags`、`folder`、`date_from`、`date_to`） |
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
| GET | `/api/titles` | 所有笔记标题和标签（供前端自动补全） |
| GET | `/api/graph/global` | 全库关系图谱（`?hide_isolated=` 参数） |
| POST | `/webhook/sync` | Webhook 触发同步（GitHub/GitLab 签名验证，需 webhook.enabled=true） |
| POST | `/api/config/reload` | 配置热重载（需认证，重新读取 config.ron 并触发同步） |

### 模板系统（`src/templates.rs`、`templates/`）

使用 **Askama**（编译期 Jinja2 风格模板），所有模板继承自 `layout.html`。模板结构体定义在 `src/templates.rs` 中，与 `templates/` 目录下的文件一一对应。新增模板字段时需同时修改结构体和 HTML 文件。

### 认证系统（`src/auth.rs`、`src/auth_db.rs`、`src/auth_middleware.rs`）

由 `security.auth_enabled` 控制的可选 JWT 认证：

- `AuthMiddleware` 拦截所有请求；精确匹配公开路径（`/login`、`/api/auth/login`），前缀匹配（`/static/`、`/share/`）
- Token 通过 Cookie（`auth_token`，含 `Secure` + `SameSite::Lax`）或 `Authorization: Bearer` 头传递
- 用户以 bcrypt 哈希密码存储在 `redb` 中
- 首次启动若数据库为空，自动创建默认管理员账户

### 持久化（`src/persistence.rs`）

使用 **redb**（嵌入式键值存储）。`IndexPersistence` 用 **postcard**（二进制格式）序列化所有内存索引。启动时若已保存的 Git commit hash 与当前 HEAD 匹配，则直接恢复索引（跳过全量重处理）。以下情况会使缓存失效：Git commit 变更、`ignore_patterns` 变更、`CURRENT_VERSION`（当前为 **2**）升级。

写入策略：笔记按 1000 条/事务分批提交，metadata 最后写入（作为原子完成标记）；中途崩溃时 metadata 未写入，下次启动安全触发全量重建。

### 关键依赖

- **actix-web 4** — HTTP 服务器与路由
- **tantivy** — 全文搜索索引（存储于 `{local_path}/.search_index/`）
- **jieba-rs** — 中文分词器
- **redb** — 嵌入式数据库，用于持久化索引、认证、分享链接、阅读进度
- **askama** — 编译期 HTML 模板
- **pulldown-cmark** — Markdown → HTML 渲染
- **rayon** — 并行笔记处理
- **ron** — 配置文件格式
