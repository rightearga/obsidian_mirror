# CHANGELOG

本文件记录 Obsidian Mirror 各版本的变更历史，遵循 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.0.0/) 格式。

> v1.4.10 及之前的所有历史条目已归档至 [CHANGELOG-ARCHIVE.md](./CHANGELOG-ARCHIVE.md)。

## [Unreleased]

---

## [v1.8.2] — 2026-04-15

导出与发布：PDF 打印样式、Atom 订阅、静态站点 zip 导出。

### Added
- **PDF 导出**（`static/css/print.css`、`templates/page.html`）：`@media print` 隐藏侧边栏/状态栏等 UI，优化纸张排版；笔记页面"打印"按钮调用 `window.print()`
- **Atom 订阅**（`GET /feed.xml`）：全库最近 50 篇笔记，支持 `?tag=` 和 `?folder=` 过滤；Atom 1.0 格式，`<content type="html">` 包含完整渲染 HTML，CDATA 封装
- **静态站点导出**（`POST /api/export/html`）：将整个 vault 打包为自包含 HTML zip，每篇笔记独立 HTML 文件含内嵌 CSS 和侧边栏导航；包含 `index.html` 笔记列表和 `README.md`；可直接部署到 GitHub Pages / Netlify

### Fixed
- **hmac 0.13 API 适配**（`src/handlers.rs`）：`verify_github_signature` 及相关测试更新为 `SimpleHmac<Sha256>` + `KeyInit::new_from_slice`（依赖升级引入的 API 变更）

### 测试统计
- 服务端：123/123 通过（新增 4 个：xml_escape、note_path_to_html、静态 HTML、rfc3339）

---

## [v1.8.1] — 2026-04-15

代码审计修复版本（CODEREVIEW_1.8）。

### Fixed
- **[B1] `page` 参数无上限，Modified 排序时 usize 溢出风险**（`src/search_engine.rs`）：`page.min(10_000)` 限制页码上限，消除超大 page 值导致 offset 整数溢出的风险
- **[Q1] WASM 模块版本号未更新**（`crates/wasm/src/lib.rs`）：顶部注释从 `v1.7.0` 更新为 `v1.8.0`

### 审计统计
- 🟡 P2 修复：1 项（B1）
- 🔵 P3 修复：1 项（Q1）
- 接受为设计限制：2 项（I1 旧 Firefox 降级、I2 concat 新数组）

---

## [v1.8.0] — 2026-04-15

规模化性能优化：侧边栏虚拟渲染、搜索结果分页、全局图谱渐进式加载。

### Added
- **搜索结果分页**（`src/search_engine.rs`、`src/handlers.rs`）：`/api/search` 新增 `page`（默认 1）和 `per_page`（默认 20，最大 100）参数；响应格式升级为 `{results, total, page, per_page, total_pages}`；新增 `advanced_search_paginated` 方法（`SortBy::Relevance` 使用 Tantivy 原生 offset，`SortBy::Modified` Rust 层排序后分页）
- **搜索"加载更多"**（`static/js/search.js`）：结果底部加"加载更多"按钮（当 total_pages > 1），点击追加下一页结果；显示命中总数
- **全局图谱渐进式加载**（`static/js/graph.js`、`templates/graph_page.html`）：超过 800 节点时先渲染有连接的核心节点，500ms 后追加孤立节点；节点数 ≤ 800 时直接全量加载

### Changed
- **侧边栏虚拟渲染**（`static/css/sidebar.css`、`static/js/init.js`）：`.tree-item` 加 `content-visibility: auto; contain-intrinsic-size: 0 28px`，浏览器自动跳过屏幕外元素的 layout/paint；大型文件树（>500 节点）通过 `requestIdleCallback` 延迟初始化，避免阻塞首屏

### 测试统计
- 服务端：119/119 通过（新增 4 个分页测试）

---

## [v1.7.4] — 2026-04-15

多仓库支持：单实例管理多个 Git 仓库，路由自动按仓库命名空间化。

### Added
- **多仓库配置**（`src/config.rs`）：新增 `RepoConfig` 结构体和 `repos: Vec<RepoConfig>` 字段；`effective_repos()` 方法在单仓库兼容模式下自动从顶级字段构造 "default" 仓库
- **VaultRegistry**（`src/state.rs`）：保存所有仓库的 `AppState`，提供 `primary()`、`get(name)`、`names()` 方法
- **动态 scope 路由**（`src/main.rs`）：为每个仓库构建 `/r/{name}/...` 前缀 scope，通过 actix-web `app_data()` 覆盖机制注入对应 AppState，无需修改任何现有 handler
- **`GET /api/vaults`**（`src/handlers.rs`）：返回所有仓库名称及主仓库标记
- **仓库切换器**（`templates/layout.html`）：多仓库时在侧边栏显示下拉菜单，单仓库时隐藏
- **`config.example.ron`**：新增多仓库配置示例注释

### Changed
- `main()` 改为为每个仓库创建独立的 `AppState`（独立搜索索引、sync_lock、IndexPersistence）
- 优雅关闭时等待所有仓库的后台任务并保存所有仓库的持久化索引
- 每仓库独立定时同步任务（`sync_interval_minutes` 全局生效）

### 测试统计
- 服务端：115/115 通过（新增 4 个 config 多仓库测试）

---

## [v1.7.3] — 2026-04-15

笔记洞察 Dashboard：写作趋势、知识库健康度报告和标签云。

### Added
- **笔记洞察 Dashboard**（`GET /insights`）：月度折线图（SVG）、最近 30 天热力图、断链列表、孤立/超大笔记健康度、标签云（字号与笔记数正比，高频蓝/孤立灰/普通紫）
- **洞察统计 API**（`GET /api/insights/stats`）：返回 `InsightsCache` 完整 JSON，供前端图表渲染
- **`InsightsCache` + `compute_insights`**（`src/insights.rs`）：纯内存计算，`perform_sync` 完成后自动更新；包含断链检测、超大笔记（>5000字且无TOC）、孤立笔记、标签云、月度/日度写作趋势
- **`AppState.insights_cache`**（`src/state.rs`）：`TokioRwLock<InsightsCache>`，零初始值，首次同步后填充

### 测试统计
- 服务端：111/111 通过（新增 9 个 insights 单元测试）

---

## [v1.7.2] — 2026-04-15

Git 版本历史查看：基于 Git log 展示笔记修改历史、历史快照和提交 diff。

### Added
- **Git 历史 API**（`src/git.rs`）：新增 `CommitInfo` 结构体及 3 个异步方法：`get_file_history`（`git log --follow`）、`get_file_at_commit`（`git show`）、`get_file_diff`（`git diff {commit}~1 {commit}`，首次提交自动回退空树 hash）
- **笔记提交历史页**（`GET /doc/{path}/history`）：展示时间降序提交列表，含 hash、作者、日期、说明及跳转操作
- **历史版本快照页**（`GET /doc/{path}/at/{commit}`）：展示指定提交时的笔记渲染内容，顶部显示历史快照标记横幅
- **提交 diff 页**（`GET /doc/{path}/diff/{commit}`）：行级 diff，新增绿色、删除红色，HTML 转义防 XSS
- **commit hash 注入防护**（`is_valid_commit_hash`）：白名单验证（仅十六进制字符，4–64 位），拒绝 `HEAD~1`、`../` 等注入尝试

### 测试统计
- 服务端：102/102 通过（新增 4 个：commit hash 验证 ×2、diff 渲染 ×2）

---

## [v1.7.1] — 2026-04-15

代码审计修复版本（CODEREVIEW_1.7）。

### Fixed
- **[B1] ReloadPolicy::OnCommitWithDelay 在 Windows 上产生文件锁冲突**（`src/search_engine.rs`）：将 `ReloadPolicy::OnCommitWithDelay` 改为 `ReloadPolicy::Manual`，消灭 commit 后自动启动的 reload 后台线程；在 `rebuild_index` 和 `update_documents` 的 `commit()` 后显式调用 `self.reader.reload()`。修复 Windows 强制文件锁（error code 5）导致段合并写入失败的生产 Bug。
- **[Q1] WASM crate 模块版本号未更新**（`crates/wasm/src/lib.rs`）：顶部注释从 `v1.6.5` 更新为 `v1.7.0`。

### 审计统计
- 🟠 P1 修复：1 项（B1）
- 🔵 P3 修复：1 项（Q1）
- 接受为设计限制：1 项（I1 vis.js tooltip，与 graph.js 一致）
- 发现问题总计：2 项

---

## [v1.7.0] — 2026-04-14

全局知识图谱专页 + WASM M4/M5 性能优化。

### Added
- **全局知识图谱专页**（`GET /graph`）：独立全屏图谱页面，支持全局图谱与单笔记子图（`?note=&depth=`）切换，工具栏含搜索高亮、深度选择器、孤立节点开关、聚类着色开关；聚类着色按笔记最多共享标签分组着色
- **WASM M4：CJK 评分阶段 Bitset**（`crates/wasm/src/lib.rs`）：评分热路径改用 `token_to_qidx` HashMap + `u8` 位掩码替换重复 24 次 `HashSet::contains()`，减少 CJK 多 token 场景的热路径字符串哈希开销
- **WASM M5：Barnes-Hut θ 自适应**（`crates/wasm/src/lib.rs`）：前 60% 迭代 θ=1.2（快速收敛），后 40% θ=0.7（精细布局）；预期 500 节点 ~5ms → ~3ms

### 测试统计
- 服务端：98/98 通过
- WASM：38/38 通过（新增 4 个：M4 评分正确性 ×2、M5 θ 自适应 ×2）

---

## [v1.6.6] — 2026-04-14

代码审计修复版本（CODEREVIEW_1.6）。

### Fixed
- **[B1] index.json 在 auth_enabled 时暴露私有笔记内容**（`src/sync.rs`）：当 `auth_enabled = true` 时跳过 `static/wasm/index.json` 的生成。`/static/` 为认证白名单前缀，生成该文件会导致笔记 title/path/tags/内容摘要向未认证用户泄露；禁用认证的公开部署不受影响。
- **[B2] index.json 生成任务未加入 background_tasks**（`src/sync.rs`）：将 `tokio::task::spawn` 返回的 `JoinHandle` 加入 `data.background_tasks`，保证优雅关闭时等待文件写入完成，消除写入截断风险。
- **[Q1] WASM crate 模块版本号未同步**（`crates/wasm/src/lib.rs`）：将模块顶部注释从 `v1.6.1` 更新为 `v1.6.5`。

### 审计统计
- 🟠 P1 修复：1 项（B1）
- 🟡 P2 修复：1 项（B2）
- 🔵 P3 修复：1 项（Q1）
- 接受为设计限制：3 项（Q2 相对路径、Q3 HTML passthrough、A1 WASM 内存释放）

---

## [v1.6.5] — 2026-04-14

NoteIndex 倒排索引位图加速（M3-续）：`HashSet<usize>` → `Bitset` 候选集。

### Changed
- **[M3-续] NoteIndex.search_json Bitset 加速**（`crates/wasm/src/lib.rs`）：
  - 新增 `Bitset` 结构体（`Vec<u64>`，第 i 位代表 note i 是否为候选）和 `BitsetIter` 迭代器
  - 将 posting list 合并从 `HashSet.extend()` 改为 `bits[idx/64] |= 1 << (idx%64)` 位或运算，零堆分配
  - 1000 条笔记仅需 16 个 u64 = 128 字节（完全 L1 缓存友好）
  - 实测改善：ASCII 搜索 239µs → **192µs**（**-19%**）；CJK 搜索在噪声范围内稳定
  - 新增 4 个 Bitset 单元测试（基本置位/空位图/去重/搜索正确性）

---

## [v1.6.4] — 2026-04-14

WASM 性能优化：消灭三个基准测试暴露的性能瓶颈。

### Changed
- **[M1] 消除 highlight_terms 双倍 to_lowercase 分配**：
  - `src/search_engine.rs`：新增 `find_substr_ci` 助手函数（字节级大小写不敏感查找，不分配 `text_lower`），更新 `generate_snippet` 和 `highlight_terms` 使用它
  - `crates/wasm/src/lib.rs`：同步更新 `highlight_term` 和 `find_substr_ci` 函数
  - 实测改善：`search/fulltext_hit` 54.4µs → **42.2µs**（-22%）
- **[M2] `compute_graph_layout` Barnes-Hut 四叉树近似**：
  - n > 100 时自动切换至 O(n log n) Barnes-Hut 算法（θ=0.9），n ≤ 100 仍用精确 O(n²) FR
  - 新增 `QuadTree` enum（Empty / Leaf / Internal），实现 `insert` 和 `repulsion_force`
  - 实测改善：`graph_layout/500` 10.6ms → **4.69ms**（-56%）
- **[M3] NoteIndex CJK 搜索均衡**：
  - query token 按倒排索引稀有度（最小 posting list）排序，截断至最多 8 个（抑制 CJK bigram 泛滥）
  - CJK bigram 标题匹配权重提升至 15（原 10），精准匹配优先
  - `is_cjk_bigram` 改为零分配实现（无 Vec<char> 分配）
  - 预计算 `title_weights` 向量，消除每候选节点的重复计算
  - 实测改善：`note_index_search_cjk` 705µs → **655µs**（-7%）
- 新增 4 个单元测试（find_substr_ci ASCII/CJK、Barnes-Hut 大图、M3 CJK 查询）

---

## [v1.6.3] — 2026-04-14

前端 JS → WASM 替换：图谱布局加速、本地搜索过滤、客户端 TOC 生成。

### Added
- **`compute_graph_layout(nodes, edges, iterations)` WASM 函数**：Fruchterman-Reingold 力导向布局算法；自适应迭代次数（>300节点→15次，>100节点→30次，≤100节点→50次）；返回 `[{id,x,y}]` 坐标数组
- **`filter_notes(notes, tagsFilter, folderFilter, limit)` WASM 函数**：多标签交集（ALL 语义）+ 路径前缀过滤，< 5ms（1000 条笔记）；fallback 到 JS 等价实现
- **`generate_toc_from_html(html)` WASM 函数**：从渲染 HTML 提取 h1-h6 标题生成 TOC，< 1ms（100 个标题）；配合实时预览实时更新
- **6 个新单元测试**：图谱布局（基本/空图）、笔记过滤（标签/路径）、TOC（基本/空）

### Changed
- **`graph.js` 图谱渲染**：节点数 ≥ 50 时自动使用 WASM 静态布局（禁用 Vis.js 物理引擎），< 200ms；节点数 < 50 保持 Vis.js 物理动画
- **`loader.js`**：新增 `WasmLoader.computeGraphLayout()`、`filterNotes()`、`generateToc()` 方法（均含 fallback）

---

## [v1.6.2] — 2026-04-14

PWA 离线搜索：自定义 WASM 全文索引 + Service Worker 拦截 /api/search。

### Added
- **WASM NoteIndex**（`crates/wasm/src/lib.rs`）：
  - n-gram 分词器（ASCII 词切分 + CJK unigram/bigram，支持中文混合搜索）
  - TF 评分（标题 ×10、标签 ×5、内容 ×1）+ 倒排索引加速候选筛选
  - `NoteIndex.loadJson(json)` — 从服务端 index.json 加载索引
  - `NoteIndex.searchJson(query, limit) -> String` — 返回标准 SearchResult JSON
  - 4 个专项单元测试（加载/搜索/CJK/分词）
- **服务端 index.json 生成**（`src/sync.rs`）：
  - 每次同步完成后后台生成 `static/wasm/index.json`（`{title,path,tags,content,mtime}[]` 格式）
  - 内容字段取笔记 HTML 剥离标签后的前 300 字符
- **Service Worker 离线搜索拦截**（`static/sw.js`）：
  - 升级 CACHE_NAME 至 v2，新增 WASM_ASSETS 独立缓存桶（WASM_CACHE_NAME）
  - 拦截 `GET /api/search`：网络可用时正常转发，网络失败时从缓存 index.json 做 JS 文本搜索，返回与在线 API 格式一致的 JSON
  - `X-Offline-Search` 响应头指示使用了离线搜索
- **`WasmLoader.search()` + `WasmLoader.loadIndex()`**（`static/wasm/loader.js`）：
  - WASM 加载完成后自动异步加载 index.json 并初始化 NoteIndex
  - `search(query, limit)` 返回结果数组，NoteIndex 不可用时返回 null（调用方 fallback 到服务端）

---

## [v1.6.1] — 2026-04-14

Markdown 渲染客户端化：pulldown-cmark 编译为 WASM，浏览器端实时预览 < 5ms。

### Added
- **`render_markdown(content: &str) -> String`** WASM 函数（`crates/wasm/src/lib.rs`）：
  - 支持完整 Obsidian 扩展语法：`[[WikiLink]]`、`![[image.png]]`、`==高亮==`、`$数学$`、`$$块级数学$$`
  - 使用 pulldown-cmark（禁用 simd 保证跨浏览器兼容）
  - 8 个专项单元测试（basic/WikiLink/alias/image/highlight/math-inline/math-block/table）
- **`WasmLoader.renderMarkdown(markdown)`** — loader.js 新增方法，WASM 优先渲染，加载失败自动 fallback 到 JS 段落化
- **实时预览面板**（`static/js/wasm-preview.js` + `static/css/wasm-preview.css`）：
  - 当 WASM 模块可用时，在笔记页面插入「实时预览」切换按钮
  - 激活后呈现双栏布局：左侧 Markdown textarea、右侧实时渲染（300ms 防抖）
  - 渲染后自动触发 callout.js / MermaidManager 处理
  - localStorage 记录折叠状态；移动端自动切换垂直布局

### Changed
- `crates/wasm/Cargo.toml`：新增依赖 pulldown-cmark、regex、lazy_static、percent-encoding

---

## [v1.6.0] — 2026-04-14

WASM 基础设施：打通 Rust → WebAssembly 工具链，建立渐进增强的客户端加速管道。

### Added
- **Cargo workspace 拆分**：新增 `crates/wasm/` 子 crate，`wasm-bindgen = "0.2"` 作为依赖；`Cargo.toml` workspace 增加 `crates/wasm` 成员
- **`crates/wasm/src/lib.rs`**：WASM 模块初版，暴露 3 个函数：
  - `wasm_version() -> String` — 版本确认
  - `highlight_term(text, term) -> String` — `<mark>` 高亮（与服务端 `search_engine::highlight_terms` 逻辑一致）
  - `truncate_html(html, max_chars) -> String` — HTML 截断（与服务端 `handlers::truncate_html` 逻辑一致）
- **`static/wasm/loader.js`**：浏览器端 WASM 加载器，支持：
  - `WebAssembly.instantiateStreaming` 异步加载
  - 加载失败自动 fallback 到 JavaScript 等价实现（渐进增强）
  - `performance.now()` 基准比对（`localStorage.debug_wasm=true` 启用日志）
- **`Makefile`**：`make wasm` / `make wasm-dev` / `make server` / `make build` / `make test` 构建命令
- **`Dockerfile`** 更新：添加 WASM 多阶段构建阶段（注释形式，取消注释即可在 Docker 内构建）
- 8 个 WASM crate 单元测试（highlight/truncate 边界条件）

### Changed
- `templates/layout.html`：新增 `<script defer src="/static/wasm/loader.js">` 加载 WASM 模块

---

## [v1.5.6] — 2026-04-14

代码审计修复版本（CODEREVIEW_1.5，v1.5.2–v1.5.5）。

### Fixed
- **[B1] sync_history 只记录成功同步**：`sync_handler` 和 `webhook_sync_handler` 在 `perform_sync` 返回 `Err` 时现在也会追加 `status="failed"` 的 `SyncRecord`，确保 `/api/sync/history` 和 `/health.last_sync_record` 能正确反映同步失败情况
- **[A1] SSE 流在 done 事件后不关闭**：`sync_events_handler` 的 `unfold` 状态改为 `(rx, finished_flag)`；收到 "done"/"error" 阶段事件后发送该事件，下次调用时返回 `None` 关闭流，避免客户端连接在同步完成后永久挂起

### 审计统计
- 🟠 P1 修复：1 项（B1 sync 失败历史缺失）
- 🟡 P2 修复：1 项（A1 SSE 连接泄漏）
- 发现问题总计（v1.5.2–v1.5.5）：8 项（含接受 2 项、推迟 3 项）

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
