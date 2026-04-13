# CHANGELOG

本文件记录 Obsidian Mirror 各版本的变更历史，遵循 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.0.0/) 格式。

## [Unreleased]

---

## [v1.4.7] — 2026-04-13

搜索性能优化 + 搜索结果增强 + share_db 前缀查询。

### Added
- **搜索结果显示路径/标签/修改时间**：`SearchResult` 新增 `tags: Vec<String>` 字段；搜索卡片展示文件夹路径（去文件名）、最多 3 个标签 chip、相对修改时间（"3天前"格式）

### Changed
- **重启跳过搜索索引重建**：持久化命中 + `NoChange` 场景下，若 Tantivy 磁盘索引已有内容则直接复用，不再全量重建；首次空库时仍正常重建
- **share_db 前缀查询**：主表键改为 `{creator}:{token}`；新增 `TOKEN_LOOKUP_TABLE` 反查表实现 O(1) token 查找；`get_user_shares` 改用 `redb::range()` 前缀查询，彻底消除全表扫描；旧有分享链接数据失效（需重新创建）

---

## [v1.4.6] — 2026-04-13

安全修复：Cookie 动态 Secure 配置、Webhook HMAC-SHA256 真实实现、Mermaid 调试日志清理。

### Fixed
- **Cookie Secure 动态判断**：v1.3.2 硬编码 `.secure(true)` 导致 HTTP 环境下（内网/开发）登录 Cookie 被浏览器静默丢弃，认证失效。新增 `security.force_https_cookie: bool`（默认 `false`），仅在显式启用时设置 Secure 标志；HTTP 环境无需修改配置即可正常使用

### Changed
- **Webhook HMAC-SHA256 真实实现**：替换原有的字符串直接比较，使用 `hmac 0.12` + `sha2 0.10` 实现标准 HMAC-SHA256 签名验证（含常数时间比较防时序攻击）；GitLab `X-Gitlab-Token` 令牌验证逻辑不变
- **Mermaid 调试日志清理**：`mermaid-init.js` 删除 13 处 `console.log`/`console.warn` 调试输出，仅保留 3 处 `console.error` 错误日志

---

## [v1.4.5] — 2026-04-13

运维与同步扩展：定时自动同步、Webhook 触发、配置热重载、Glob 忽略模式、Prometheus 指标扩展。

### Added
- **定时自动同步**：新增 `sync_interval_minutes` 配置项；后台 Tokio 定时任务自动触发同步，遵守 `sync_lock` 互斥（与手动触发不冲突）
- **Webhook 触发同步**（`POST /webhook/sync`）：支持 GitHub `X-Hub-Signature-256` 签名验证和 GitLab `X-Gitlab-Token` 令牌验证；通过 `webhook.enabled` + `webhook.secret` 配置启用
- **配置热重载**（`POST /api/config/reload`，需认证）：重新读取 `config.ron` 后触发完整同步；注：`listen_addr` 和 `repo_url` 的变更仍需重启
- **ignore_patterns Glob 语法**：`scanner.rs` 新增 glob 匹配（`*`/`**`/`?`），支持 `*.tmp`、`draft/**`、`20[0-9][0-9]-*` 等模式；原有精确匹配保留
- **Prometheus 指标扩展**：新增 `sync_duration_seconds`（同步耗时直方图）和 `sync_last_timestamp_seconds`（上次同步 Unix 时间戳 gauge）
- **`/health` 端点扩展**：新增字段 `git_commit`、`sync_status`（idle/running/failed）、`last_sync_at`、`last_sync_duration_ms`
- `state.rs` 新增原子字段 `last_sync_at`、`last_sync_duration_ms`、`sync_status`

---

## [v1.4.4] — 2026-04-13

PWA 支持 + 移动端触屏手势 + 无障碍改进。

### Added
- **PWA — Web App Manifest**（`/static/manifest.json`）：支持"添加到主屏幕"安装；包含名称、主题色、快捷方式
- **PWA — Service Worker**（`/static/sw.js`）：静态资源缓存优先策略，网络离线时提供降级页面；新版本检测提示刷新
- **触屏手势**（`static/js/gestures.js`）：
  - 侧边栏：屏幕左边缘（<30px）右滑打开，左滑关闭（60px 阈值）
  - 笔记翻页：内容区滚动到底/顶后继续滑动超 100px 跳转前/后一篇笔记
- **跳过导航链接**（Skip Link）：`<a href="#main-content">跳至主要内容</a>`，键盘 Tab 聚焦时浮现，鼠标不可见
- **ARIA 语义标注**：侧边栏树 `role="tree"` + `aria-label`；搜索结果 `role="listbox"` + `aria-live="polite"`
- **无障碍 CSS**（`accessibility.css`）：全局 `:focus-visible` 样式、高对比度模式适配
- **prefers-reduced-motion 完善**：覆盖灯箱、键盘帮助面板、callout 折叠图标等新动画

### Changed
- `theme.js` 切换主题时同步更新 `<meta name="theme-color">`（深色 `#202020`，浅色 `#6a5acd`）

---

## [v1.4.3] — 2026-04-13

关系图谱全面增强：全库图谱、节点大小/颜色分组、布局切换、图谱内搜索、PNG 导出。

### Added
- **全局图谱视图**（`GET /api/graph/global`）：展示整个笔记库链接关系；节点 >500 时自动降采样（仅保留有链接的节点）；侧边栏"发现"面板新增全局图谱按钮；`openGlobalGraphView()` 可从任意页面调用
- **图谱筛选面板**：标签筛选、文件夹筛选、隐藏孤立节点开关、重置按钮，实时过滤重新渲染
- **节点大小映射**：节点大小与反向链接入度成正比（10–35 范围），多引用笔记视觉上更突出
- **节点颜色分组**：按笔记第一个标签自动分配 12 色调色板；右侧图例面板显示标签-颜色映射
- **布局切换**：力导向（默认）/ 层级（vis.js hierarchical）两种模式可切换
- **图谱内搜索**：在图谱内输入关键词定位并高亮匹配节点，自动移动视口
- **图谱导出**：一键导出当前可见图谱为 PNG 文件

### Changed
- `domain.rs` `GraphNode` 新增 `tags: Vec<String>` 字段（API 响应包含标签信息）
- 图谱模态从 `page.html` 移至 `layout.html`，全站通用；`page.html` 保留局部图谱入口

---

## [v1.4.2] — 2026-04-13

搜索体验升级 + 笔记发现机制：搜索自动补全、单条历史删除、孤立笔记页、随机漫游、最近更新页。

### Added
- **实时搜索自动补全**（`GET /api/titles`）：前端从服务器一次性获取所有笔记标题和标签，输入时实时过滤推荐，sessionStorage 缓存 5 分钟，零额外网络请求
- **孤立笔记页面**（`GET /orphans`）：列出无出链且无入链的笔记，帮助发现未整合的孤立内容；侧边栏"概览"入口
- **随机漫游**（`GET /random`）：随机跳转到一篇笔记；侧边栏按钮；键盘快捷键 `r`
- **最近更新页面**（`GET /recent`）：按修改时间降序列出笔记，支持 `?days=7/30/90` 范围切换；侧边栏入口

### Changed
- 搜索历史管理升级：最大保留条数从 10 增至 20，每条历史项悬停显示 `×` 单条删除按钮
- 侧边栏"概览"页签新增"发现"快捷入口组（随机漫游/孤立笔记/最近更新）

---

## [v1.4.1] — 2026-04-13

内容渲染增强：KaTeX 数学公式、Callout 标注块、==高亮==、图片灯箱与懒加载。

### Added
- **KaTeX 数学公式渲染**：后端 `markdown.rs` 将 `$$...$$`（块级）和 `$...$`（行内）转换为带 `data-math` 属性的 HTML 元素；前端 `katex-init.js` 通过 KaTeX CDN 渲染；渲染失败时显示原始 LaTeX 文本（优雅降级）
- **Callout 标注块**：`callout.js` 前端解析 `> [!TYPE] Title` 语法的 blockquote 并转换为样式化 callout div，支持 20+ 类型（NOTE/TIP/WARNING/DANGER/SUCCESS/QUESTION/QUOTE/BUG/TODO 等），支持 `[!NOTE]-` 默认收起 / `[!NOTE]+` 默认展开；`callout.css` 深/浅色模式各自配色
- **高亮语法**：`markdown.rs` 将 `==高亮文本==` 转换为 `<mark>高亮文本</mark>`，样式在 `math.css` 中
- **图片灯箱**：`lightbox.js` 点击 `.markdown-body img` 弹出全屏模态层，`←`/`→` 键在当前页图片间切换，`Esc` 关闭；`lightbox.css` 毛玻璃遮罩样式
- **图片懒加载**：`markdown.rs` 输出的所有 `<img>` 标签自动添加 `loading="lazy"`，减少首屏图片请求

### Changed
- Mermaid 图表类型扩展（序列图/甘特图/类图/状态图）和主题联动已在之前版本实现，本版本在 ROADMAP 中标记完成

---

## [v1.4.0] — 2026-04-13

交互体验基础：全套键盘快捷键、主题预设与定制、代码块主题选择、动画优化。

### Added
- **键盘快捷键**（`static/js/keyboard.js`）
  - 页面内导航：`j`/`k` 滚动、`g g` 顶部、`G` 底部、`[`/`]` 前后笔记、`b` 返回
  - 功能：`g` 打开图谱（单击，等 450ms 无第二下则触发）、`t` 切换 TOC、`?` 帮助面板
  - 焦点在输入框时自动停用；`prefers-reduced-motion` 时跳过滚动动画
- **键盘快捷键帮助面板**（`?` 触发，`Esc` 关闭；`static/css/keyboard.css`）
- **主题预设**：暖色 / 护眼 / 高对比度三套，叠加于深色/浅色基础上（`static/css/themes.css`）
- **自定义强调色**：设置面板颜色选择器，写入 `--accent-color` CSS 变量，localStorage 持久化
- **代码块主题独立选择**：6 套可选（auto / Atom One Dark / Atom One Light / GitHub Light / GitHub Dark / Dracula / Monokai），跟随深浅色自动或固定选择
- **交互动画**（`static/css/animations.css`）
  - 笔记内容区淡入（150ms fade-in）
  - 侧边栏文件夹图标旋转过渡（cubic-bezier）
  - 搜索结果错峰进入（stagger 20ms/条，最多 8 条）
  - TOC 活跃链接颜色平滑过渡
  - `prefers-reduced-motion` 媒体查询自动禁用所有动画
- 设置面板新增：主题预设按钮组、强调色选择器、代码块主题下拉、动画开关

---

## [v1.3.4] — 2026-04-13

代码质量改进与测试覆盖补全：6 项 CODEREVIEW Q 系列问题全部落地，新增 14 个单元测试。

### Changed
- **[Q1] schema_matches 字段类型检查加强**：`search_engine.rs` `schema_matches` 同时比较字段名和字段类型变体，防止 TEXT→STRING 等类型变更时复用错误 schema
- **[Q3] 阅读历史自动清理**：`reading_progress_db.rs` `add_history()` 写入后自动调用 `cleanup_old_history(200)`，防止历史记录无限增长
- **[Q5] truncate_html 截断逻辑改进**：`handlers.rs` 先通过状态机去除 HTML 标签提取纯文本，再按字符数截断，确保 500 字预览显示真实可见内容
- **[Q6] 持久化分批写入**：`persistence.rs` `save_indexes` 笔记按 1000 条分批提交，元数据最后写入（作为原子完成标记），降低单次事务锁库时长
- **[Q7] metrics 指标注册去除 expect**：`metrics.rs` `init_metrics()` 改用 `let _ =` 静默忽略 AlreadyRegistered 错误，防止测试环境多次初始化时 panic

### Added（测试覆盖补全）
- `sync.rs`：4 个 `should_update_note` 测试（新文件/相同mtime/更旧mtime/路径不存在）
- `graph.rs`：6 个 `generate_graph` 测试（不存在节点/深度1/深度2/反向链接/孤立节点/孤立中心）
- `persistence.rs`：4 个往返测试（数据一致性/git hash 不匹配/ignore_patterns 变更/clear 后失效）

---

## [v1.3.3] — 2026-04-13

性能深度优化：5 项来自 CODEREVIEW_1.3 的性能改进全部落地。

### Changed
- **[P1] Regex lazy_static 优化**：`markdown.rs`（5个）、`tags.rs`（1个）所有 `Regex::new()` 移至 `lazy_static!` 块，进程生命周期内只编译一次，大型笔记库同步速度显著提升
- **[P2] Tantivy IndexReader 缓存复用**：`SearchEngine` 结构体中持久持有 `IndexReader`，`advanced_search` 不再每次创建新 reader，搜索延迟降低
- **[P3] 搜索索引增量更新**：`SearchEngine` 新增 `update_documents` 方法；增量同步（Git diff）时只更新变更文件的 Tantivy 文档，不再全量重建，大型知识库同步速度大幅提升
- **[P4] reading_progress_db 前缀范围查询**：`get_user_progress` / `get_user_history` 改用 redb `range()` 前缀查询，避免全表扫描
- **[P5] graph.rs 消除 content_text 解析**：`extract_links_from_note` 改用 `Note.outgoing_links` 预计算字段（v1.3.1 已建），消除图谱生成时的正则解析开销

---

## [v1.3.2] — 2026-04-13

安全加固：修复 3 项安全类问题（Cookie 安全属性、分享密码哈希存储、中间件路径精确匹配）。

### Fixed
- **[S2] Cookie 补充 `Secure` 和 `SameSite` 属性**：`auth_handlers.rs` 登录/登出 Cookie 添加 `.secure(true).same_site(SameSite::Lax)`，防止 JWT Token 在 HTTP 连接下明文传输
- **[S3] 分享链接密码改用 bcrypt 哈希存储**：`share_db.rs` 将 `password` 字段重命名为 `password_hash`，创建时使用 `bcrypt` 单向哈希，验证时使用 `bcrypt::verify`，防止数据库泄露时密码明文暴露；`share_handlers.rs` 同步更新 `has_password` 字段
- **[S4] 认证中间件路径匹配收紧**：`auth_middleware.rs` 将 `/login`、`/api/auth/login` 从 `starts_with` 改为精确匹配 `==`，防止 `/login-admin` 等路径绕过认证

### 修复统计
- 🟠 P1 修复：3 项（S2、S3、S4）

---

## [v1.3.1] — 2026-04-13

Bug 修复与安全加固：修复 4 个 P0/P1 级问题（标签持久化残留、增量反向链接丢失、标题 XSS、同步并发竞争）。

### Fixed
- **[B1] `persistence::clear()` 未清理标签索引**：`clear()` 方法补充清空 `TAG_INDEX_TABLE`，消除调用后标签数据残留问题
- **[B4] 增量同步反向链接丢失**：在 `Note` 添加 `outgoing_links` 字段，`BacklinkBuilder::build` 改为基于全量笔记出链重建，增量同步不再遗漏未变更笔记的反向链接；`CURRENT_VERSION` 升至 2 强制缓存重建
- **[S1] 标题文本 XSS 漏洞**：`MarkdownProcessor` 添加 `html_escape()` 函数，标题输出前对 pulldown-cmark 解码后的文本字符（`<`、`>`、`&`、`"`）进行 HTML 转义
- **[B2] `/sync` 端点缺少并发保护**：`AppState` 添加 `sync_lock: tokio::sync::Mutex<()>`，`sync_handler` 使用 `try_lock()` 防止并发同步（返回 409 Conflict）

### 修复统计
- 🔴 P0 修复：2 项（B1、B4）
- 🟠 P1 修复：2 项（S1、B2）

---

## [v1.3.0] — 2026-02-01

高级搜索功能：多条件过滤、搜索排序、分享链接、阅读进度跟踪。

### Added
- **高级搜索过滤**：按标签（多标签 OR 逻辑）、文件夹路径、日期范围过滤，可折叠面板，一键清除
- **搜索结果排序**：支持按相关度（默认）或最新修改时间排序
- **分享链接生成**：创建带过期时间的笔记分享链接，支持自定义过期（1小时-30天）、访问统计、redb 持久化存储
- **阅读进度跟踪**：实时保存滚动位置，刷新自动恢复，记录阅读历史，redb 持久化存储
- 搜索引擎新增标签、文件夹字段索引，扩展 `SearchQuery` API（tags、folder、date_from、date_to）
- `advanced_search()` 方法，使用 Tantivy BooleanQuery + RangeQuery 实现多条件组合查询
- 高级过滤面板 UI（+128 行 CSS）及交互逻辑（+122 行 JavaScript）

### Changed
- 统一数据库配置到 `config.database` 节，删除重复的 `auth_db_path` 顶层配置

---

## [v1.2.0] — 2026-01-31

体验优化：多语言支持、笔记悬浮预览、统一设置面板、移动端与桌面端布局改进。

### Added
- **多语言支持**：中文/English 切换，语言偏好 localStorage 持久化，全局 i18n.js
- **笔记预览**：鼠标悬停触发悬浮卡片，显示前 200 字，平滑淡入淡出动画
- **设置面板**：统一管理语言、字体（系统/衬线/等宽）、字体大小（12-24px）、行高（1.2-2.0），支持重置默认
- **移动端 TOC 右侧滑入**：44×44px 固定切换按钮，0.3s ease 动画，半透明遮罩，85vw 自适应宽度
- **桌面端 TOC 收起/展开**：展开状态 localStorage 记忆，平滑动画，保留宽度拖拽功能
- **内容自适应布局**：移除 800px 固定宽度，响应式边距，表格溢出防止与 TOC 重叠
- 移动端状态栏完整显示（字数、行数、时间），紧凑布局字体和图标缩放

### Fixed
- 移动端 TOC 侧边栏不显示
- `ignore_patterns` 配置变更不生效
- 表格溢出导致布局错位

### Changed
- 移除设置中的内容宽度选项
- 静态资源添加版本号防止缓存
- CSS 变量统一管理布局参数

---

## [v1.1.0] — 2026-01-31

功能增强：最近访问笔记、收藏夹、笔记统计面板，修复标签持久化缺失。

### Added
- **最近访问笔记**：记录最近 10 条访问记录，侧边栏快速入口，相对时间显示，折叠状态记忆
- **收藏夹功能**：星标收藏笔记，收藏列表管理，localStorage 存储，Toast 提示消息
- **笔记统计面板**：笔记总数、标签数量、近 7 天更新数，3 个统计卡片实时展示
- 侧边栏新增统计、最近访问、收藏夹三个面板入口
- 笔记页面收藏按钮

### Fixed
- 标签索引持久化缺失，重启后标签数据丢失（`persistence.rs` 补充 `tag_index` 持久化）

### Changed
- 更正标签按钮图标
- 完善移动端响应式适配

---

## [v1.0.0] — 2026-01-31

生产就绪：增量同步、索引持久化、用户认证、健康检查、Prometheus 指标、日志管理。

### Added
- **增量同步**：Git diff 检测变更文件，仅处理修改/新增/删除文件，性能提升 10-100 倍
- **索引持久化**：postcard + redb 持久化笔记索引，Git commit 严格校验，重启恢复速度提升 30-120 倍
- **用户认证**：JWT 令牌 + bcrypt 密码加密，登录/登出/修改密码页面，用户菜单界面
- **健康检查端点** `GET /health`：返回状态、版本、笔记数、运行时长，适配 Docker/Kubernetes
- **Prometheus 指标** `GET /metrics`：HTTP 请求计数、延迟直方图、笔记数量、同步操作计数
- **日志管理**：分级输出到 `./logs/app.log`（INFO+）和 `./logs/error.log`（ERROR+），每日自动轮转
- **优雅关闭**：捕获 SIGTERM/SIGINT，关闭前保存持久化索引，等待现有请求完成
- 自定义 `AppError` 类型（8 种分类），统一 `Result<T>` 类型，中文错误消息
- 37 个单元测试（Markdown 处理 21 个、标签提取 8 个、认证 4 个、错误处理 2 个）

### Changed
- 侧边栏默认宽度调整为 320px，支持拖动调整（200-600px），宽度 localStorage 记忆
- 统一侧边栏、正文、目录滚动条样式，自动适配深色主题
- 修复 Frontmatter 序列化问题（`serde_yml::Value` → JSON 字符串中转）

---

## [v0.10.0] — 2026-01-29

关系图谱完善：交互体验优化，边缘情况修复。

### Changed
- 完善关系图谱交互体验（节点拖拽、点击跳转）
- 优化图谱物理引擎渲染效果
- 修复图谱边缘情况和错误处理

---

## [v0.9.0] — 2026-01-29

可视化增强：笔记目录（TOC）、关系图谱、面包屑导航。

### Added
- **笔记目录（TOC）**：自动从 H1-H6 提取目录，点击平滑滚动，浮动固定在右侧，滚动时自动高亮当前章节
- **关系图谱**：Vis.js 可视化笔记链接关系，支持 1-3 层深度，交互式节点，物理动画，链接方向箭头
- **面包屑导航**：显示当前笔记路径层级，路径分隔符美化，移动端友好紧凑布局
- 笔记统计信息（字数、修改时间）
- 模块化 JavaScript（`static/js/` 目录）
- 模块化 CSS（`static/css/` 目录）

---

## [v0.8.0] — 2026-01-29

搜索功能 & 标签支持：Tantivy 全文搜索、中文分词、标签系统。

### Added
- **全文搜索**：Tantivy 搜索引擎，侧边栏搜索框，标题和内容搜索，300ms 防抖，关键词高亮，Ctrl+K 快捷键，搜索历史，相关度排序
- **标签系统**：Frontmatter 标签提取，`#标签` 语法识别，`/tags` 标签列表页，`/tag/{name}` 标签笔记列表，标签云可视化（按频率显示大小）
- 中文分词支持（jieba-rs）
- 后台异步索引构建，不阻塞主线程
- 批量提交优化（每 1000 条笔记）、50MB 写入缓冲区，支持 5000+ 文件笔记库

### Changed
- 重构索引构建逻辑为独立模块 `indexer.rs`
- 代码模块化（`handlers.rs`、`search_engine.rs`、`tags.rs`）

---

## [v0.7.1] — 2026-01-29

用户体验优化：界面中文化、侧边栏滚动记忆、移动端交互修复。

### Changed
- 界面文本全面中文化
- 侧边栏滚动位置记忆（移动端和桌面端）

### Fixed
- 移动端侧边栏交互问题
- 侧边栏多行文本对齐和缩进问题
