# Obsidian Mirror 开发路线图

> 本文档规划 Obsidian Mirror 的未来版本计划。  
> v1.4.10 及之前的历史版本详情请见 [ROADMAP-ARCHIVE.md](./ROADMAP-ARCHIVE.md)。

**当前版本**: v1.6.6 🎉  
**下一里程碑**: v1.7.0（知识洞察与内容增强，规划中）  
**长期规划**: v1.7.x 知识洞察（图谱聚类 / Git 历史 / 笔记分析 / 多仓库）  
**最后更新**: 2026-04-14

---

## 📋 目录

- [v1.5.x 规划](#v15x-规划2026-q3)
- [v1.6.x 规划](#v16x-规划2026-q4)
- [v1.7.x 规划](#v17x-规划2027-q1)
- [功能分类](#功能分类)
- [长期愿景](#长期愿景)
- [迭代原则](#迭代原则)
- [技术演进](#技术演进)
- [成功指标](#成功指标)
- [反馈渠道](#反馈渠道)

---

## 🚀 v1.5.x 规划（2026 Q3）

> v1.5 系列主题：**架构加固 → 搜索升级 → 多用户 → 内容增强 → 运维强化**
>
> 每个 minor 版本聚焦单一主题，patch 版本用于代码审计修复。

---

### ✅ v1.5.0 (已发布 - 2026-04-14) — 架构加固（技术债清偿）

**主题**：清偿 CODEREVIEW_1.4 全部推迟项，消灭运行时潜在 panic 和阻塞隐患

#### A1：redb blocking IO 全面移入 spawn_blocking

| 模块 | 文件 | 涉及函数 |
|------|------|---------|
| 认证 | `src/auth_handlers.rs` | `login_handler`、`change_password_handler` |
| 分享 | `src/share_handlers.rs` | `create_share_handler`、`access_share_handler`、`list_shares_handler` |
| 阅读进度 | `src/reading_progress_handlers.rs` | `save_progress_handler`、`get_progress_handler` |

- 将所有 `auth_db.*`、`share_db.*`、`reading_progress_db.*` 调用包裹进 `tokio::task::spawn_blocking`
- `AuthDatabase` 已用 `Arc<Database>` 实现 Clone，可直接 move 进闭包；`ShareDatabase`、`ReadingProgressDatabase` 补充 `Arc<Database>` 包裹（当前为裸 `Database`）

#### B2：AppConfig 热重载真正生效

- 将 `AppState.config` 改为 `Arc<RwLock<AppConfig>>`
- 更新所有读取处（约 30 处）改为 `.config.read().await`
- `config_reload_handler` 写入新配置后触发 `perform_sync` 以应用 `ignore_patterns` 等变更
- 说明：`listen_addr`、`repo_url` 仍需重启生效，在接口响应中明示

#### B3：/health `uptime_seconds` 修复

- `AppState` 增加 `start_time: std::time::Instant` 字段
- `/health` 中返回 `start_time.elapsed().as_secs()` 真实运行时长

#### E1：Rayon mutex 中毒时优雅恢复

- `src/sync.rs` 中 `results.into_inner().unwrap()` 改为 `.into_inner().unwrap_or_else(|e| e.into_inner())`

#### E2：模板渲染错误返回结构化 JSON

- 模板 `Err` 路径统一改为 `HttpResponse::InternalServerError().json({"error": "..."})`

#### Q2：分享 URL scheme 改用 X-Forwarded-Proto

- `share_handlers.rs` 优先读取 `X-Forwarded-Proto` header；`config.ron` 可选新增 `public_base_url` 字段

#### Q3：Git commit 读取函数合并

- `handlers.rs` 的 `read_local_git_commit` 与 `sync.rs` 的 `get_current_git_commit` 合并到 `src/git.rs`

#### 测试补全

- `config_reload_handler` 集成测试（T3）
- `AppState.config` 热更新路径测试

---

### ✅ v1.5.1 (已发布 - 2026-04-14) — 代码审计（CODEREVIEW_1.5）

**主题**：对 v1.5.0 进行系统性代码审查，修复新引入问题

- 审计范围：`src/` 全部 `.rs` 文件，重点关注 `RwLock<AppConfig>` 新路径的并发正确性
- 产出：`docs/CODEREVIEW_1.5.md`
- 遵循审计流程（`/ob-review 1.5`）

---

### ✅ v1.5.2 (已发布 - 2026-04-14) — 搜索体验升级

**主题**：让搜索从"能用"变"好用"

#### 模糊 / 拼音搜索

- Tantivy 查询层增加容错模糊匹配（`FuzzyTermQuery`，最多 1 个编辑距离）
- 可选：集成拼音转换（`pinyin` crate），支持 `rust` 匹配 "Rust 编程"

#### 搜索建议自动补全优化

- `/api/titles` 返回数据增加 path 字段，前端补全时可显示路径上下文
- 后端增加 `/api/suggest?q=` 端点，返回 fuzzy 匹配的标题列表（限 10 条）

#### 摘要 snippet 改进

- 改为 Tantivy SnippetGenerator 生成真正的命中上下文摘要
- 摘要中关键词加粗高亮（`<mark>` 标签）

#### 搜索历史持久化

- 新增后端 `/api/search/history`（存入 `reading_progress_db` 复用）
- 支持清空历史

---

### ✅ v1.5.3 (已发布 - 2026-04-14) — 多用户与权限管理

**主题**：从单管理员向真正多用户演进

#### 用户角色

| 角色 | 权限 |
|------|------|
| `admin` | 所有操作（含 /sync、/api/config/reload、用户管理） |
| `editor` | 查看 + 分享链接创建/管理（未来预留编辑权限入口） |
| `viewer` | 只读浏览，无分享、无管理操作 |

- `User` 结构体增加 `role: UserRole` 枚举字段
- 认证中间件注入 role 信息，敏感端点（/sync、/api/config/reload）校验 `admin` 角色
- CURRENT_VERSION 递增（User 结构变更触发数据库重建）

#### 管理员界面

- `GET /admin/users` — 用户列表页（Askama 模板）
- `POST /api/admin/users` — 创建用户（admin only）
- `DELETE /api/admin/users/{username}` — 删除用户（admin only）
- `POST /api/admin/users/{username}/reset-password` — 重置密码

#### 分享链接多用户适配

- `GET /api/share/list` 仅返回当前用户的分享；admin 可查看全部（增加 `?all=true` 参数）

---

### ✅ v1.5.4 (已发布 - 2026-04-14) — Obsidian 语法完整支持

**主题**：内容渲染更忠实于原版 Obsidian 体验

#### 笔记内嵌（Block Embed）

- `![[笔记.md]]` → 内联展示目标笔记的渲染 HTML（可折叠）
- `![[笔记.md#标题]]` → 内联展示指定章节内容
- 实现：handler 层递归渲染，加深度保护（最多 2 层，防止循环嵌入）

#### 脚注支持

- `[^1]` 脚注语法（pulldown-cmark 已支持 `ENABLE_FOOTNOTES`，需在 options 中开启）
- 脚注跳转锚点与反跳回原文的双向链接

#### Mermaid 主题适配

- 根据当前主题动态注入 `%%{init: {"theme": "dark"}}%%` 或默认主题
- 前端 JS 监听主题切换事件，重新渲染 Mermaid 图

#### Callout 折叠动画

- 可折叠 Callout 补充展开/折叠 CSS transition 动画
- 折叠状态存入 localStorage，页面刷新后保持

---

### ✅ v1.5.5 (已发布 - 2026-04-14) — 实时通知与运维增强

**主题**：让运维可观测、优雅、可靠

#### SSE 实时同步进度推送

- `GET /api/sync/events` — Server-Sent Events 端点，同步期间推送进度事件
  ```
  data: {"stage": "git_pull", "progress": 20, "message": "拉取最新提交..."}
  data: {"stage": "markdown", "progress": 60, "notes_processed": 234}
  data: {"stage": "done", "progress": 100, "total_notes": 1024}
  ```
- 前端同步按钮点击后改为实时进度条显示（替换当前的"同步中"转圈）
- 实现：`tokio::sync::broadcast` channel 广播给所有 SSE 订阅者

#### 优雅关闭等待后台任务

- 将 Tantivy 重建、redb 持久化任务的 `JoinHandle` 存入 `AppState`，`main.rs` 在关闭前 `await` 全部完成
- 超时保护：等待上限 30 秒，超时后强制退出并打印警告

#### 同步历史记录

- `AppState` 增加 `sync_history: RwLock<VecDeque<SyncRecord>>`（最近 10 条）
- `SyncRecord { started_at, finished_at, notes_processed, status, error_msg }`
- `/health` 返回最近一次同步记录；新增 `/api/sync/history` 返回全部历史

#### 依赖版本升级

- 定期检查并升级 `actix-web`、`tantivy`、`redb`、`jieba-rs` 等核心依赖
- 回归测试全量通过后发布

---

### ✅ v1.5.6 (已发布 - 2026-04-14) — 代码审计（CODEREVIEW_1.5.x）

**主题**：对 v1.5.2–v1.5.5 引入的新代码进行系统性审查

- 审计重点：SSE 连接泄漏、多用户权限绕过、笔记内嵌递归深度越界
- 产出：`docs/CODEREVIEW_1.5.md`（合并 v1.5.x 全系列审计结果）
- 遵循审计流程（`/ob-review 1.5`）

---

## ⚡ v1.6.x 规划（2026 Q4）

> v1.6 系列主题：**WASM 加速**
>
> 将核心 Rust 逻辑编译为 WebAssembly，在浏览器端运行，分三个递进阶段：
> 基础设施搭建 → Markdown 渲染客户端化 → PWA 离线搜索 → 前端 JS 逻辑替换

---

### ✅ v1.6.0 (已发布 - 2026-04-14) — WASM 基础设施

**主题**：打通 Rust → WASM 工具链，建立可复用的编译与集成管道

#### 工具链

- 引入 `wasm-pack` + `wasm-bindgen` 作为 WASM 编译工具
- Cargo workspace 拆分：新增 `crates/wasm/` 目录，独立编译目标
  ```
  obsidian_mirror/
  ├── src/              ← 服务端（保持不变）
  ├── crates/
  │   └── wasm/         ← WASM 专用 crate（#![no_std] 可选）
  └── static/
      └── wasm/         ← wasm-pack 输出目录（.wasm + JS glue）
  ```
- `Makefile` / `build.rs` 增加 `wasm-pack build` 步骤，输出到 `static/wasm/`
- `Dockerfile` 多阶段构建：WASM 编译阶段 + 服务端编译阶段分离

#### 共享代码提取

- 将 `src/markdown.rs`（纯函数部分）、`src/tags.rs` 提取为与 `std` 弱依赖的库函数，供 WASM crate 复用
- 目标：服务端和 WASM 共享同一份 Markdown + 标签逻辑，保证一致性

#### 加载策略

- 浏览器异步加载 WASM 模块（`WebAssembly.instantiateStreaming`）
- 加载失败时自动 fallback 到原有 JS 或 HTTP 服务端路径（渐进增强，不破坏现有功能）
- 引入 `performance.now()` 基准比对：记录 WASM vs 原路径的实际耗时

---

### ✅ v1.6.1 (已发布 - 2026-04-14) — Markdown 渲染客户端化

**主题**：pulldown-cmark 编译为 WASM，浏览器本地渲染 Markdown，实现实时预览

#### 功能

- `crates/wasm` 暴露 `render_markdown(content: &str) -> String` 函数（返回 HTML）
- 包含完整的 Obsidian 扩展语法处理：WikiLink、Callout、数学公式包裹、高亮
- 服务端保留 `content_html` 字段（首屏 SSR 不变）；WASM 模块作为**实时预览**路径

#### 实时预览增强

- 笔记页面侧边栏新增「实时预览」模式：前端 `<textarea>` 输入 Markdown，右侧实时 WASM 渲染
- 搜索结果卡片悬停预览改为 WASM 客户端渲染（减少 `/api/preview` 请求）

#### 性能目标

| 场景 | 当前（HTTP round-trip） | WASM 目标 |
|------|------------------------|-----------|
| 悬停预览 | ~50 ms（含网络） | < 5 ms（本地渲染） |
| 搜索摘要 | 服务端生成 | 客户端实时截取 |

---

### ✅ v1.6.2 (已发布 - 2026-04-14) — PWA 离线搜索

**主题**：在 Service Worker 中内嵌轻量 WASM 全文索引，断网可搜索

#### 技术选型

| 方案 | WASM 大小 | 支持 CJK | 选用理由 |
|------|-----------|---------|---------|
| Tantivy → WASM | ~5 MB | ✅（已有 jieba） | 太重，不适合浏览器 |
| 自定义倒排索引 | < 200 KB | 手动分词 | **选用**，可控、轻量 |
| MiniSearch（JS） | ~40 KB | 插件扩展 | 备选（纯 JS，无 WASM 优势） |

**选用自定义 Rust 倒排索引**：
- `crates/wasm` 实现轻量 `NoteIndex`：n-gram 分词（支持中文）+ TF 评分
- 同步完成后，服务端后台生成 `index.bin`（postcard 序列化）并写入 `static/wasm/`
- Service Worker 在笔记同步后更新 `index.bin` 缓存

#### 离线搜索流程

```
用户搜索（离线）
  ↓
SW 拦截 /api/search 请求
  ↓
WASM NoteIndex.search(query) → 排序结果
  ↓
返回 JSON 结果（格式与在线 API 一致）
  ↓
前端无感知切换（在线/离线统一 UI）
```

#### 索引生成

- `perform_sync` 完成后，后台生成 `SearchIndexDump`：`{title, path, tags, snippet_text}[]`
- 序列化为 `index.bin`（~100-500 KB 对于 1000 条笔记）
- 通过 `/static/wasm/index.bin` 提供下载，SW 在首次同步后预缓存

---

### ✅ v1.6.3 (已发布 - 2026-04-14) — 前端 JS → WASM 替换

**主题**：将计算密集的前端 JS 逻辑迁移至 WASM，提升大规模笔记库的渲染性能

#### 图谱布局计算（Graph Layout）

- 当前：Vis.js 内置 JavaScript 物理引擎，500+ 节点时 CPU 占用高、动画卡顿
- 目标：用 Rust 实现 Force-Directed 布局算法（Barnes-Hut 加速），编译为 WASM
- WASM 计算布局坐标 `{id, x, y}[]`，传回 JS 调用 Vis.js `setPositions()`（静态渲染，无物理动画）

#### 搜索结果排序与过滤

- 前端缓存全量 `titles + tags`，本地 WASM 做二次过滤（多标签交集、路径前缀匹配）
- 搜索框输入时，WASM 先给出本地建议，服务端异步补充精确结果（双轨并行）

#### 性能目标

| 功能 | JS 当前 | WASM 目标 |
|------|---------|-----------|
| 全局图谱（500 节点）布局 | ~2 s | < 200 ms |
| 标签多选过滤（1000 条） | ~50 ms | < 5 ms |
| TOC 生成（100 个标题） | 服务端+网络 | < 1 ms 本地 |

---

### ✅ v1.6.4 (已发布 - 2026-04-14) — WASM 性能优化（M1/M2/M3）

**主题**：消灭 v1.6.3 基准测试暴露的三个性能瓶颈，全面提升 WASM 模块吞吐

#### M1：`highlight_terms` 避免双倍 `to_lowercase` 分配（遗留自 v1.5.2）

**问题**：`search_engine.rs::generate_snippet` 和 `crates/wasm/src/lib.rs::highlight_terms`
在每次搜索命中时对 `text` 和 `term` 各调用一次 `to_lowercase()`，产生两次堆分配。

```rust
// 当前（双倍分配）
let term_lower = term.to_lowercase();   // 分配 1
let text_lower = text.to_lowercase();   // 分配 2
```

**修复方案**：
- 方案 A（简单）：只做一次 `term.to_lowercase()`，用 `memchr` 做大小写不敏感子串查找
- 方案 B（彻底）：引入 `unicase` crate，零分配大小写不敏感比较

**预期收益**：`search/fulltext_hit` -2~5 µs（当前 54.4 µs → ~50 µs），
消除 v1.5.2 引入的 +10 µs `<mark>` 高亮代价中的分配部分

**文件**：`src/search_engine.rs`，`crates/wasm/src/lib.rs`

---

#### M2：WASM `compute_graph_layout` Barnes-Hut 四叉树近似

**问题**：当前 Fruchterman-Reingold 使用 O(n²) 排斥力计算。500 节点 × 15 迭代
= 1.875M 对运算 ≈ 10.6ms（原生 Release），浏览器端约 20-50ms。

**修复方案**：Barnes-Hut 四叉树近似 O(n log n)：
1. 每轮迭代构建四叉树（O(n log n)）
2. 对每个节点，距离足够远的簇用质心近似（θ 参数控制精度/速度权衡，默认 θ=0.9）
3. 吸引力计算不变（只有连边节点，稀疏图中 O(E) ≪ O(n²)）

**预期收益**：

| 节点数 | 当前 | 优化后目标 |
|--------|------|----------|
| 200 | 3.0ms | ~0.8ms |
| 500 | 10.6ms | ~2ms |
| 1000 | ~40ms | ~5ms |

**文件**：`crates/wasm/src/lib.rs`（新增 `QuadTree` 结构体）

---

#### M3：NoteIndex CJK 搜索速度均衡（拉近与 ASCII 的差距）

**问题**：CJK 搜索（705 µs）比 ASCII（244 µs）慢约 3×。原因：
- CJK 每字生成 unigram + bigram（2 token/字），"编程语言"4 字 → 7 个 token
- ASCII "rust"分词后仅 1 个 token
- token 数多 → 倒排索引查询次数多 → 候选集更大

**修复方案**：
1. **限制查询 token 数上限**：query 超过 8 个 token 时，仅取 TF-IDF 最高的 8 个
2. **位图加速候选交集**：改用 bitset 存储候选 note 集合，替换 `HashSet<usize>`
3. **bigram 权重上调**：精确 bigram 匹配得分提高（+15），减少宽泛 unigram 带来的噪声候选

**预期收益**：CJK 搜索从 705 µs → ~300 µs（接近 ASCII 水平）

**文件**：`crates/wasm/src/lib.rs`

---

### ✅ v1.6.5 (已发布 - 2026-04-14) — NoteIndex 倒排索引位图加速（M3-续）

**主题**：用 bitset（位图）替换 `HashSet<usize>` 候选集，进一步提升 CJK 搜索速度

**背景**：M3（v1.6.4）将 CJK 搜索从 705µs 降至 661µs（-6%）。主要瓶颈已从重复分词（v1.6.3 TokenCache 修复）转移到候选集合并操作（多个 token 的 posting list 合并）。

#### 问题分析

```
当前流程：
1. 对每个 query token 从倒排索引取 Vec<usize>（posting list）
2. 将所有 posting list 插入 HashSet<usize> 候选集
3. 遍历候选集评分

CJK 搜索瓶颈："编程语言" → 7 个 token，7 次 HashSet 扩展操作
HashSet 在随机 usize 插入时有较高的内存分配和 hash 计算开销
```

#### 修复方案：bitset 替换 HashSet

```rust
// 位图：u64 数组，第 i 位代表 note 索引 i
// 对于 1000 条笔记，仅需 1000/64 = 16 个 u64（128 字节，完全 L1 缓存友好）
struct Bitset {
    bits: Vec<u64>,
    len: usize,  // 总 note 数（位图大小）
}

impl Bitset {
    fn set(&mut self, idx: usize) { self.bits[idx / 64] |= 1 << (idx % 64); }
    fn iter_set(&self) -> impl Iterator<Item = usize> + '_ {
        // 遍历所有置位的索引（位扫描）
    }
}
```

合并操作从 `HashSet.insert()` O(1) amortized 变为 `bits[i/64] |= bit` 无分配操作。

**文件**：`crates/wasm/src/lib.rs`（在 `search_json` 中替换候选集类型）

**预期收益**：CJK 搜索 661µs → ~400µs（-40%），接近 ASCII 水平（239µs）

---

### ✅ v1.6.6 (已发布 - 2026-04-14)

**主题**：代码审计修复（CODEREVIEW_1.6）

#### 修复内容
- ✅ **[B1] index.json 在 auth_enabled 时暴露私有笔记内容**：`auth_enabled=true` 时跳过 `static/wasm/index.json` 生成，防止笔记内容经 `/static/` 公开路径泄露
- ✅ **[B2] index.json 生成任务未加入 background_tasks**：将写入任务 `JoinHandle` 加入 `background_tasks`，优雅关闭时等待完成
- ✅ **[Q1] WASM crate 模块版本号未同步**：`crates/wasm/src/lib.rs` 顶部注释从 `v1.6.1` 更新为 `v1.6.5`

#### 设计限制（已知接受）
- ⏸ Q2：`index.json` 使用相对路径，与其他静态文件一致，已有 warn 日志
- ⏸ Q3：`render_markdown` HTML passthrough，仅影响已认证用户自身笔记预览（自 XSS）
- ⏸ A1：`NoteIndex` WASM 内存未显式释放，SPA 场景不存在泄漏

#### 测试结果
- 全量测试：**98/98 通过**
- 新增测试：0 个（审计修复，无新逻辑）

---

## 🔮 v1.7.x 规划（2027 Q1）

> v1.7 系列主题：**知识洞察与内容增强**
>
> 在 v1.6.x WASM 基础设施完善后，深挖笔记数据的价值：强化知识图谱、引入 Git 历史浏览、构建写作洞察 Dashboard，并支持多仓库管理。

---

### ✅ v1.7.0 (已发布 - 2026-04-14) — 全局知识图谱专页 + WASM 性能收尾

**主题**：将关系图谱升级为独立全屏专页，补完 PERFORMANCE-1.6 剩余优化项

#### 全局图谱专页

- 新增路由 `GET /graph`，独立全屏图谱页面（替代当前嵌入在笔记页侧栏的入口）
- 支持全局图谱（`/api/graph/global`）与单笔记子图（`?note={path}&depth=1-3`）切换
- 图谱工具栏：搜索框（高亮目标节点）、深度选择器、孤立节点开关、聚类着色开关
- 聚类着色：按笔记所属最多共享标签分组，不同标签组渲染不同颜色

#### WASM CJK 评分阶段优化（M4，来自 PERFORMANCE-1.6 建议）

**背景**：v1.6.5 Bitset 优化了候选集合并阶段；CJK 搜索剩余瓶颈在评分阶段对每个候选笔记遍历所有 `query_tokens` 进行 `HashSet` 查询。

```
当前（评分阶段）：
  对每个候选笔记 × query_token 数（≤8）查询 HashSet
  HashSet 的随机访问在 token 数多时存在 cache miss

优化方案：
  将 query_token 存入 Bitset（最多 64 token），
  评分时改为 bits & note_bits 的位与运算，一次性得出匹配 token 数
```

**预期收益**：CJK 搜索 ~620µs → ~400µs（接近 ASCII ~200µs）

#### Barnes-Hut θ 自适应（M5，来自 PERFORMANCE-1.6 建议）

**问题**：当前 `θ = 0.9` 固定，前期布局和稳定期精度需求不同。

**修复方案**：
- 前期迭代（iter < total * 0.6）：`θ = 1.2`（快速收敛，允许更多近似）
- 稳定期（iter ≥ total * 0.6）：`θ = 0.7`（精细布局，减少抖动）

**预期收益**：500 节点 ~5ms → ~3ms；布局收敛质量更好

**文件**：`crates/wasm/src/lib.rs`

#### 实际交付物
- 新增文件：`templates/graph_page.html`（全屏图谱专页模板）
- 修改文件：`src/templates.rs`（新增 `GraphPageTemplate`）
- 修改文件：`src/handlers.rs`（新增 `graph_page_handler`）
- 修改文件：`src/main.rs`（注册 `GET /graph` 路由）
- 修改文件：`crates/wasm/src/lib.rs`（M4 评分 Bitset + M5 θ 自适应）

#### 测试结果
- 服务端全量测试：**98/98 通过**
- WASM 全量测试：**38/38 通过**
- 新增测试：4 个（M4 评分正确性 ×2、M5 θ 自适应 ×2）

---

### ✅ v1.7.1 (已发布 - 2026-04-15) — 代码审计（CODEREVIEW_1.7）

**主题**：对 v1.7.0 引入的新代码进行系统性审查

#### 修复内容
- ✅ **[B1] ReloadPolicy::OnCommitWithDelay Windows 文件锁冲突**：改为 `ReloadPolicy::Manual`，commit 后显式 `reader.reload()`，修复生产环境 PermissionDenied (os error 5)
- ✅ **[Q1] WASM 模块注释版本号未更新**：`v1.6.5` → `v1.7.0`

#### 设计限制（已知接受）
- ⏸ I1：vis.js tooltip innerHTML 自内容 XSS，与 graph.js 已有行为一致

#### 测试结果
- 服务端全量测试：**98/98 通过**
- WASM 全量测试：**38/38 通过**
- 新增测试：0 个（审计修复，无新逻辑）

---

### ✅ v1.7.2 (已发布 - 2026-04-15) — Git 版本历史查看

**主题**：基于 Git log 展示笔记的修改历史，让笔记内容演变可追溯

#### 路由

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/doc/{path}/history` | 笔记提交历史列表（commit list） |
| GET | `/doc/{path}/at/{commit}` | 指定提交时的历史版本快照 |
| GET | `/doc/{path}/diff/{commit}` | 与上一提交的 diff（行级 HTML 渲染） |

#### 技术实现

- 使用 `git log --follow --format="%H|%ae|%ai|%s" -- {path}` 读取单文件历史
- `git show {commit}:{path}` 获取历史版本内容，走 `MarkdownProcessor::process` 渲染
- `git diff {commit}~1 {commit} -- {path}` 获取 unified diff，渲染为带颜色的 `<table>`（新增绿色 / 删除红色）
- 历史版本页面复用 `layout.html`，侧边栏保持正常，主区域显示"历史快照"标记

#### 约束

- 只读：不支持回滚或手动提交
- 仅对已在 Git 追踪的文件生效（新建未提交的文件无历史）
- 认证：与 `/doc/{path}` 相同的访问控制

#### 实际交付物
- 修改文件：`src/git.rs`（`CommitInfo` + 3 个历史方法）
- 修改文件：`src/templates.rs`（3 个新模板结构体）
- 修改文件：`src/handlers.rs`（3 个新 handler + `is_valid_commit_hash` + `render_diff_html`）
- 修改文件：`src/main.rs`（3 条新路由，注册在 doc_handler 之前）
- 新增文件：`templates/history.html`、`templates/history_at.html`、`templates/history_diff.html`

#### 测试结果
- 服务端全量测试：**102/102 通过**
- 新增测试：4 个（commit hash 验证 ×2、diff 渲染 XSS 防护 ×2）

---

### 🔧 v1.7.3 — 笔记洞察 Dashboard

**主题**：从笔记库数据中挖掘写作规律与质量问题，提供可视化统计报告

#### 路由

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/insights` | 笔记洞察 Dashboard 主页 |
| GET | `/api/insights/stats` | 完整统计数据 JSON（供图表渲染） |

#### 功能模块

**写作趋势**（基于 frontmatter `date` 或文件 mtime）
- 按月笔记数量折线图（Canvas/SVG，无需引入重量级图表库）
- 最近 30 天活跃度热力图（类 GitHub contribution graph）

**知识库健康度**
- 孤立笔记（已有 `/orphans`，集成进 Dashboard 并加解决建议）
- 断链：WikiLink 指向不存在笔记的链接列表（`[[不存在的笔记]]`）
- 超大笔记：字数 > 5000 且无 TOC 的笔记列表（建议拆分）
- 无标签笔记占比

**标签云**
- 所有标签按笔记数排序，字号与数量正比，点击跳转 `/tag/{name}`
- 高频标签（>50 笔记）与孤立标签（=1 笔记）分色显示

#### 技术约束

- 所有统计在 `perform_sync` 后增量更新，结果缓存在 `AppState`（`RwLock<InsightsCache>`）
- 断链检测：遍历 `notes` 中每个 `outgoing_links`，对照 `link_index` 验证目标存在性
- 无新增数据库表，复用现有内存索引

---

### 🔧 v1.7.4 — 多仓库支持

**主题**：单实例管理多个 Git 仓库，适配团队多项目或个人多知识库场景

#### 配置变更

```ron
// config.ron（新格式，向后兼容：单 repo_url 仍可用）
(
    repos: [
        (
            name: "personal",
            repo_url: "https://git.example.com/notes.git",
            local_path: "./vault_personal",
            ignore_patterns: [".obsidian"],
        ),
        (
            name: "work",
            repo_url: "https://git.example.com/work-notes.git",
            local_path: "./vault_work",
            ignore_patterns: [".obsidian", "drafts/"],
        ),
    ],
    listen_addr: "0.0.0.0:3080",
    // ... 其余全局配置不变
)
```

#### 路由策略

- **多仓库模式**：所有路由加前缀 `/r/{name}/doc/{path}`、`/r/{name}/api/search` 等
- **向后兼容**：配置仅有单仓库时，路由保持现有格式（无前缀）
- 顶部导航栏仓库切换器（下拉菜单），切换后跳转对应仓库首页

#### AppState 变更

- `AppState` 拆分为 `GlobalState`（全局：配置、认证）+ `VaultState`（每仓库：notes、索引、search_engine）
- `Arc<HashMap<String, VaultState>>` 管理多个仓库状态
- 每仓库独立 sync_lock，互不阻塞

#### 约束

- 用户权限全局生效（不做仓库级别独立认证）
- Prometheus 指标加 `repo` label 区分
- `perform_sync` 支持按 repo name 单独触发（`POST /r/{name}/sync`）

---

## 📦 功能分类

### 💡 未来探索方向

以下方向已全部纳入 v1.7.x 规划：

1. **笔记聚类分析** → v1.7.0 图谱聚类着色
2. **版本历史查看** → v1.7.2 Git 版本历史
3. **笔记洞察 Dashboard** → v1.7.3 健康度 + 写作趋势
4. **多仓库切换** → v1.7.4 多仓库支持

### ❌ 不支持的功能

为了保持项目专注于**只读展示**和**高性能浏览**，以下功能**不在**规划范围内：

1. **在线编辑** — 使用 Obsidian 客户端编辑，通过 Git 同步
2. **多仓库切换** — 增加架构复杂度；替代方案：部署多个实例
3. **评论系统** — 不符合只读展示定位；替代方案：GitHub Issues
4. **协作编辑** — 与只读定位冲突；替代方案：Obsidian 客户端 + Git
5. **插件系统** — 增加安全风险和维护成本
6. **笔记导出** — 笔记已在 Git 仓库，可直接访问原始 Markdown
7. **键盘快捷键自定义** — 增加配置复杂度
8. **复杂统计分析** — 不属于核心浏览功能
9. **版本历史编辑器** — 使用 GitHub/GitLab Web 界面代替

---

## 🌟 长期愿景

### 2026–2027 年版本目标

| 季度 | 系列 | 主题 |
|------|------|------|
| Q1–Q2 2026（已完成） | v0.7.1 – v1.4.10 | 功能构建 → 质量加固 → 体验提升 → 架构优化 |
| Q3 2026（已完成） | v1.5.0 – v1.5.6 | 架构加固 → 搜索升级 → 多用户 → 内容增强 → 运维强化 |
| Q4 2026（已完成） | v1.6.0 – v1.6.6 | WASM 加速：客户端渲染 / 离线搜索 / JS 替换 |
| Q1 2027（规划中） | v1.7.0 – v1.7.4 | 知识洞察：图谱聚类 / Git 历史 / Dashboard / 多仓库 |

### 核心价值主张

1. **只读展示**: 专注于笔记浏览和展示，不做在线编辑
2. **高性能**: 响应迅速，支持大型笔记库（10000+ 文件）
3. **兼容性好**: 完美支持 Obsidian 语法（WikiLinks、Frontmatter 等）
4. **美观易用**: 现代化 UI，移动端友好，深色模式支持

### 产品定位

**Obsidian Mirror** 是一个**只读的笔记展示工具**，不是 Obsidian 的替代品，而是补充：

- ✅ **查看笔记**: 随时随地通过浏览器访问笔记
- ✅ **分享知识**: 将个人知识库发布为 Web 站点
- ✅ **团队协作**: 团队成员共享只读访问权限
- ❌ **编辑笔记**: 使用 Obsidian 客户端编辑，通过 Git 同步

---

## 🔄 迭代原则

### 开发原则

1. **用户优先**: 功能设计以用户需求为导向
2. **性能第一**: 不能为了功能牺牲性能
3. **渐进增强**: 功能可选，不影响核心体验
4. **保持简单**: 拒绝过度设计

### 版本策略

- **主版本 (X.0.0)**: 重大架构变更或不兼容更新
- **次版本 (0.X.0)**: 新功能和重要改进
- **修订版本 (0.0.X)**: Bug 修复和小优化

### 发布周期

- 功能版本：每 4-6 周
- 修复版本：按需发布
- 安全更新：立即发布

---

## 🛠️ 技术演进

### 当前技术栈（v1.4.10）

- ✅ **搜索引擎**：Tantivy 高性能全文索引，jieba 中文分词，后台异步索引
- ✅ **持久化**：redb 嵌入式 KV 存储，postcard 二进制序列化，分批事务写入
- ✅ **增量更新**：Git diff 检测，Tantivy 磁盘索引复用，content_text 已移除（内存减半）
- ✅ **前端工程化**：模块化 JS/CSS，PWA（Service Worker + Manifest），触屏手势，无障碍

### 技术演进路径

| 版本 | 技术升级 |
|------|---------|
| v1.5.0 | `AppConfig` → `RwLock<AppConfig>`，redb IO 全面异步化 |
| v1.5.2 | Tantivy FuzzyTermQuery + SnippetGenerator |
| v1.5.3 | 用户角色系统，JWT claims 扩展 |
| v1.6.0 | Cargo workspace + wasm-pack + wasm-bindgen |
| v1.6.1 | pulldown-cmark → WASM（客户端渲染） |
| v1.6.2 | 自定义轻量倒排索引 → WASM（PWA 离线搜索） |
| v1.6.3 | Barnes-Hut Force-Directed 布局 → WASM |
| v1.7.0 | 图谱聚类着色，CJK 评分 Bitset，θ 自适应 |
| v1.7.2 | git2-rs / shell Git log 历史浏览，diff 渲染 |
| v1.7.3 | InsightsCache，断链检测，标签云，写作趋势 |
| v1.7.4 | 多仓库 VaultState，路由前缀，仓库切换器 |

### 明确不做的技术方向

- ❌ **在线编辑器**: 不引入富文本编辑器或 Markdown 编辑器
- ❌ **实时协作**: 不实现 CRDT 或 OT 算法
- ❌ **重量级数据库**: 不引入 PostgreSQL、MySQL 等
- ❌ **细粒度文件级权限**: 多用户仅做角色级别权限（admin/editor/viewer）

---

## 📊 成功指标

### 性能指标

- ✅ 首次加载时间 < 2 秒（已达成：< 1s）
- ✅ 搜索响应时间 < 100ms（已达成：< 5ms P95）
- ✅ 支持 10,000+ 笔记规模（已验证）
- ✅ 内存占用 < 500MB（1000 笔记）（已达成）
- 🎯 WASM 图谱布局（500 节点）< 200ms（v1.6.3 目标）
- 🎯 PWA 离线搜索可用（v1.6.2 目标）

### 质量指标

- ✅ 关键路径测试覆盖 > 80%（当前 97 个测试）
- ✅ cargo clippy 零 warning
- ✅ 安全漏洞 P0 修复周期 < 1 个版本
- 🎯 每个 minor 版本附带代码审计报告（v1.5.1、v1.5.6、v1.6.6、v1.7.1 规划中）

---

## 💬 反馈渠道

### 功能建议

- 提交 GitHub Issue（功能请求标签）
- 描述使用场景和预期效果

### Bug 报告

- 提交 GitHub Issue（Bug 标签）
- 提供复现步骤和日志

### 讨论交流

- GitHub Discussions
- 邮件联系

---

## 📝 变更说明

本路线图会根据以下因素动态调整：

- 用户反馈和需求变化
- 技术可行性评估
- 开发资源情况
- 生态系统演进

**历史版本**：v1.4.10 及之前详情见 [ROADMAP-ARCHIVE.md](./ROADMAP-ARCHIVE.md)  
**变更历史**：通过 Git 历史查看本文档修改记录

---

## 🙏 致谢

感谢所有用户的支持和反馈，是你们让 Obsidian Mirror 变得更好！
