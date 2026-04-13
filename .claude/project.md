# obsidian_mirror 项目指南

## 文档与版本管理

- Roadmap：`docs/ROADMAP.md`
- 版本号：`Cargo.toml` 的 `[package].version`
- 发布检查：`cargo fmt` → `cargo test` → `cargo clippy`
- 代码注释规范：**所有注释必须使用中文**（包括 `///`、`//!`、`//`）
- 配置示例文件：`config.example.ron`（`config.ron` 已被 `.gitignore` 排除，含敏感信息）

---

## 当前开发状态

**当前版本：1.4.10**

| 模块 | 状态 | 说明 |
|------|------|------|
| Markdown 处理 | ✅ 稳定 | WikiLink、图片嵌入、frontmatter、TOC；标题文本已 HTML 转义（S1 修复） |
| 全文搜索 | ✅ 稳定 | Tantivy + jieba 中文分词，支持标签/文件夹/日期过滤 |
| Git 同步 | ✅ 稳定 | 增量同步（diff 驱动）；并发保护（B2 修复）；增量反向链接修复（B4） |
| 持久化索引 | ✅ 稳定 | redb + postcard（CURRENT_VERSION=2）；clear() 已补全标签清理（B1 修复） |
| 认证系统 | ✅ 稳定 | JWT + bcrypt；Cookie 含 Secure + SameSite::Lax（S2 修复）；路径精确匹配（S4 修复） |
| 分享链接 | ✅ 稳定 | UUID token，支持密码/次数/过期限制；密码 bcrypt 哈希存储（S3 修复） |
| 阅读进度 | ✅ 稳定 | 滚动位置记忆 + 阅读历史 |
| 关系图谱 | ✅ 稳定 | Vis.js 渲染，支持 1-3 层深度 |
| Prometheus 指标 | ✅ 稳定 | `/metrics` 端点，含请求数、笔记数、延迟直方图 |

---

## 新增功能的完整流程

每个新功能按以下顺序修改，**禁止跨层跳跃**：

```
1. 配置层   src/config.rs              — 添加配置字段（RON 格式，提供 Default）
2. 错误层   src/error.rs               — 添加 AppError 变体
3. 数据层   src/<feature>_db.rs        — 定义 redb 表和数据结构（如需持久化）
4. 核心层   src/<feature>.rs           — 实现业务逻辑
5. 处理层   src/<feature>_handlers.rs  — 实现 actix-web 路由处理器
6. 注册层   src/main.rs                — 注册路由、初始化组件
7. 状态层   src/state.rs               — 如需全局共享，添加到 AppState
8. 模板层   templates/ + src/templates.rs — 如需新页面
```

---

## 核心模块架构速查

### 同步管道（`src/sync.rs`）

```
POST /sync 或启动时
  ↓
perform_sync(Arc<AppState>)
  ├── 步骤 0: 尝试从 redb 加载持久化索引（内存为空时）
  ├── 步骤 1: GitClient::sync()  →  SyncResult
  │            InitialClone / IncrementalUpdate / NoChange
  ├── 步骤 2: VaultScanner::scan()  →  Vec<PathBuf>（.md 文件）
  ├── 步骤 3: FileIndexBuilder::build()  →  文件名 → 路径（图片等资源）
  ├── 步骤 4: process_markdown_files()  →  Rayon 并行，MarkdownProcessor::process
  ├── 步骤 5: IndexUpdater + BacklinkBuilder + TagIndexBuilder  →  更新 AppState
  ├── 步骤 6: build_sidebar()  →  SidebarNode 树
  ├── 步骤 7: search_engine.rebuild_index()  →  后台线程
  └── 步骤 8: IndexPersistence::save_indexes()  →  后台线程
```

**增量更新逻辑**：`IncrementalUpdate` 时只处理 Git diff 中变更/新增的 `.md` 文件，未变更文件直接复用内存中的 `Note`（通过 mtime 比对）。

### AppState 字段用途

| 字段 | 类型 | 用途 |
|------|------|------|
| `notes` | `RwLock<HashMap<String, Note>>` | 相对路径 → Note，核心数据 |
| `link_index` | `RwLock<HashMap<String, String>>` | 标题/文件名 → 相对路径，用于 WikiLink 解析 |
| `backlinks` | `RwLock<HashMap<String, Vec<String>>>` | 标题 → 引用它的笔记标题列表 |
| `tag_index` | `RwLock<HashMap<String, Vec<String>>>` | 标签名 → 笔记标题列表 |
| `file_index` | `RwLock<HashMap<String, String>>` | 文件名 → 相对路径（图片/PDF 等资源，每次启动重建，不持久化） |
| `sidebar` | `RwLock<Vec<SidebarNode>>` | 侧边栏树，每次同步后重建 |
| `search_engine` | `Arc<SearchEngine>` | Tantivy 索引，线程安全 |
| `share_db` | `Arc<ShareDatabase>` | 分享链接 redb 数据库 |
| `reading_progress_db` | `Arc<ReadingProgressDatabase>` | 阅读进度 redb 数据库 |

### Markdown 处理（`src/markdown.rs`）

`MarkdownProcessor::process(content)` 返回 `(html, links, tags, frontmatter, toc)`，处理顺序：

1. 提取 YAML frontmatter（`---` 块），解析为 `serde_yml::Value`
2. 预处理 `![[文件]]` → `<img>` 或 `<a>`（图片/非图片分支）
3. 预处理 `[[笔记]]` / `[[笔记|别名]]` → `<a href="/doc/...">` 并收集链接列表
4. pulldown-cmark 渲染（SIMD 加速）
5. 从 frontmatter 和行内 `#tag` 提取标签（`src/tags.rs`）
6. 从 headings 生成 TOC（`TocItem { level, text, id }`）

**路径编码**：资源路径使用 `percent_encoding::NON_ALPHANUMERIC` 编码，确保中文文件名正确传递。

### 认证流程（`src/auth_middleware.rs`）

```
HTTP 请求
  ↓
AuthMiddleware（auth_enabled=true 时生效）
  ├── 公开路径白名单：/login, /static/, /share/, /health, /metrics
  │    └── 直接放行
  └── 其他路径
       ├── 读取 Cookie: jwt_token 或 Header: Authorization: Bearer <token>
       ├── JwtManager::verify_token()
       │    ├── 有效 → 注入 username 到请求扩展，放行
       │    └── 无效 → API 路径返回 401 JSON，页面路径重定向 /login
       └── 无 token → 同上
```

### 持久化策略（`src/persistence.rs`）

- 数据库：redb，序列化格式：postcard（二进制，比 JSON 更紧凑）
- 缓存键：当前 Git commit hash + ignore_patterns 组合
- 持久化内容：`notes`、`link_index`、`backlinks`、`tag_index`、`sidebar`
- **不持久化**：`file_index`（每次启动快速扫描重建）、`search_engine`（Tantivy 自管理磁盘索引）
- 失效条件：Git commit 变更、`ignore_patterns` 变更、`CURRENT_VERSION` 常量升级

### redb 数据库文件

| 文件 | 管理模块 | 用途 |
|------|----------|------|
| `index.db` | `src/persistence.rs` | 笔记索引持久化缓存 |
| `auth.db` | `src/auth_db.rs` | 用户账户（用户名 + bcrypt 密码） |
| `share.db` | `src/share_db.rs` | 分享链接（token → ShareLink JSON） |
| `reading_progress.db` | `src/reading_progress_db.rs` | 阅读进度 + 历史（key 格式：`{username}:{note_path}`） |

---

## 模板系统注意事项

Askama 编译期模板：修改 `templates/*.html` 后必须重新编译才生效（无热重载）。新增模板字段需同步修改：
1. `src/templates.rs` — 结构体字段
2. `templates/<name>.html` — 模板变量引用
3. 对应的 handler（`src/handlers.rs` 或 `src/<feature>_handlers.rs`）

所有页面模板继承自 `templates/layout.html`。

---

## 测试

```bash
# 运行所有测试
cargo test

# 运行单个测试（支持模块路径过滤）
cargo test test_error_display
cargo test markdown::tests
```

现有测试集中在 `src/error.rs`（错误类型测试）。新增功能建议在模块末尾的 `#[cfg(test)] mod tests` 中覆盖：正常路径、边界情况、错误路径。

涉及文件 I/O 的测试使用 `tempfile::NamedTempFile` 或 `tempfile::TempDir` 隔离，避免测试间干扰。

---

## 依赖版本速查

| 依赖 | 版本 | 用途 |
|------|------|------|
| actix-web | 4.12.1 | HTTP 服务器与路由 |
| tokio | 1.49.0 | 异步运行时 |
| tantivy | 0.25.0 | 全文搜索引擎 |
| jieba-rs | 0.8.1 | 中文分词 |
| redb | 3.1.0 | 嵌入式键值数据库 |
| postcard | 1.1.3 | 二进制序列化（持久化） |
| pulldown-cmark | 0.13.0 | Markdown 渲染（SIMD） |
| askama | 0.15.4 | 编译期 HTML 模板 |
| rayon | 1.11 | 并行处理 |
| jsonwebtoken | 9.3.0 | JWT 生成与验证 |
| bcrypt | 0.18.0 | 密码哈希 |
| prometheus | 0.14.0 | 指标采集 |
| ron | 0.12.0 | 配置文件格式 |
| serde + serde_json | 1.0 | 序列化 |
| tracing + tracing-subscriber | 0.1 / 0.3 | 结构化日志 |

---

## 常见陷阱

1. **redb 同步阻塞**：redb 所有 IO 操作必须在 `tokio::task::spawn_blocking` 中执行，直接在 async 上下文调用会阻塞 tokio 线程池。

2. **路径分隔符**：`notes` HashMap 的 key 统一使用 `/` 作为路径分隔符。Git diff 在 Windows 下可能返回 `\`，需调用 `.replace("\\", "/")` 统一化，否则增量更新时找不到对应笔记。

3. **Askama 热重载**：模板修改后必须重新 `cargo build`，否则运行中的服务不会反映变化。

4. **搜索索引位置**：Tantivy 索引存储在 `{local_path}/.search_index/`，不在项目目录内，`.gitignore` 无需特殊处理。

5. **持久化版本升级**：修改 `Note`、`SidebarNode` 等持久化结构体时，必须同步递增 `src/persistence.rs` 中的 `CURRENT_VERSION` 常量，否则旧格式数据会导致反序列化失败（postcard 不向后兼容）。

6. **并发读写**：`AppState` 所有字段使用 `tokio::sync::RwLock`，sync 期间会短暂持有写锁。HTTP 请求只需读锁，通常不受影响；但 sync 耗时过长时会造成读请求等待。

7. **默认管理员账户**：`auth_enabled: true` 时，若 `auth.db` 为空，自动以 `config.security.default_admin_username/password` 创建账户。生产环境务必修改默认密码，并将 `jwt_secret` 替换为随机字符串（建议 `openssl rand -base64 32`）。
