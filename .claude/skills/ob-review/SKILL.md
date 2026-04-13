---
name: ob-review
description: obsidian_mirror 代码审计驱动。审计整个项目的 src/ 全部 .rs 文件，写入 docs/CODEREVIEW_X.Y.md，更新历史 CODEREVIEW 文件状态，修复本次必须解决的问题，版本号末位加一，更新 CHANGELOG、ROADMAP、README/CLAUDE.md，最后提交 git。
---

# obsidian_mirror 代码审计驱动

## 用途

在一个版本（vX.Y 系列）开发完毕后，系统性地审计整个项目代码质量，输出审计报告，修复必须修复的问题，并以 patch 版本号形式发布修复结果。

**典型触发时机**：完成 v1.3.0 → 执行 `/ob-review` → 产出 `CODEREVIEW_1.3.md` + 修复补丁 → 版本升为 v1.3.1

---

## 固定项目路径

| 资源 | 路径 |
|------|------|
| 审计报告目录 | `docs/` |
| 报告命名格式 | `docs/CODEREVIEW_X.Y.md`（X.Y 为当前大版本，如 `1.3`） |
| 历史报告 | `docs/CODEREVIEW_1.2.md` 等 |
| Changelog | `docs/CHANGELOG.md` |
| Roadmap | `docs/ROADMAP.md` |
| 版本源 | `Cargo.toml` → `[package].version` |

---

## 版本号规则

- **当前版本**：从 `Cargo.toml` 读取，如 `1.3.0`
- **大版本号**（X.Y）：取前两段，如 `1.3`
- **报告文件名**：`docs/CODEREVIEW_1.3.md`
- **修复后版本**：末位加一，如 `1.3.0 → 1.3.1`

---

## 核心流程

### 第一步：读取上下文

按顺序读取以下文件，建立项目状态基线：

1. `Cargo.toml` — 当前版本号及所有依赖版本
2. `docs/ROADMAP.md` — 当前版本已实现特性列表及计划修复项（若有 `🔧 vX.Y.Z` 章节）
3. `docs/CHANGELOG.md` — 近期变更历史（如文件存在）
4. `.claude/project.md` — 项目架构速查、常见陷阱
5. 所有历史 `docs/CODEREVIEW_X.Y.md` — 了解遗留问题状态

---

### 第二步：系统性代码审计

**按模块顺序审计**（从基础层到业务层）：

```
src/config.rs            ← 配置层，最底层
src/error.rs             ← 错误类型层
src/domain.rs            ← 数据结构层
src/state.rs             ← 应用状态层
src/git.rs               ← Git 客户端层
src/scanner.rs           ← 文件扫描层
src/markdown.rs          ← Markdown 处理层（重点审计：XSS、编码）
src/tags.rs              ← 标签解析层
src/indexer.rs           ← 索引构建层
src/sidebar.rs           ← 侧边栏构建层
src/graph.rs             ← 图谱生成层
src/search_engine.rs     ← 搜索引擎层
src/persistence.rs       ← 持久化层（重点审计：版本兼容性、数据完整性）
src/sync.rs              ← 同步管道层（重点审计：并发、增量正确性）
src/auth.rs              ← JWT 认证层
src/auth_db.rs           ← 用户数据库层
src/auth_middleware.rs   ← 认证中间件层（重点审计：路径匹配、绕过）
src/auth_handlers.rs     ← 认证处理器层（重点审计：Cookie 安全）
src/share_db.rs          ← 分享链接数据库层（重点审计：密码存储）
src/share_handlers.rs    ← 分享处理器层
src/reading_progress_db.rs    ← 阅读进度数据库层
src/reading_progress_handlers.rs ← 阅读进度处理器层
src/handlers.rs          ← 通用处理器层
src/metrics.rs           ← 指标采集层
src/templates.rs         ← 模板定义层
src/lib.rs / src/main.rs ← 应用入口层
```

**每个模块审计维度**：

#### A. 安全性（Security）

- **XSS**：用户内容或笔记文本插入 HTML 时是否转义（标题、路径、标签、frontmatter 中的字符串）
- **路径遍历**：`assets_handler`、`doc_handler` 等接受路径参数的接口是否存在目录穿越风险
- **认证绕过**：`auth_middleware` 公开路径白名单是否过于宽松（`starts_with` vs 精确匹配）
- **Cookie 安全**：JWT Token Cookie 是否设置 `Secure`、`HttpOnly`、`SameSite`
- **密码存储**：分享链接密码、用户密码是否使用哈希存储（明文 vs bcrypt）
- **JWT 验证**：token 过期、签名错误的错误路径是否完整
- **CORS / CSRF**：跨域请求是否有保护（如 POST 接口）

#### B. 异步正确性

- **redb 阻塞**：所有 redb IO 是否在 `tokio::task::spawn_blocking` 内（`AuthDatabase`、`ShareDatabase`、`ReadingProgressDatabase`、`IndexPersistence`）
- **锁跨 await**：`RwLock`/`Mutex` guard 是否跨 `.await` 持有（`tokio::sync` 锁可以，`std::sync` 锁不能）
- **并发同步**：`POST /sync` 是否有并发保护（`sync_lock`），防止多请求同时触发 Tantivy IndexWriter 冲突
- **spawn_blocking JoinError**：`spawn_blocking` 返回的 `JoinHandle` 是否处理了 panic 传播
- **后台任务泄漏**：`tokio::spawn` 启动的后台任务是否在应用关闭时正确等待

#### C. 正确性

- **增量同步反向链接**：`BacklinkBuilder::build` 在增量模式下是否正确合并已有链接（而非全量覆盖）
- **持久化完整性**：`IndexPersistence::clear()` 是否清理了所有表（包括 `TAG_INDEX_TABLE`）
- **路径统一性**：Git diff 返回的路径是否统一转换为 `/` 分隔（Windows 下尤其关键）
- **搜索索引一致性**：同步流程中搜索索引重建是否与内存 `notes` 保持一致
- **分享链接过期**：`expires_at` 判断逻辑是否正确（时区、比较方向）
- **阅读进度键格式**：`{username}:{note_path}` 格式是否在路径含 `:` 时产生歧义

#### D. 错误处理质量

- **`unwrap()`/`expect()`**：生产路径（非测试、非初始化）中是否存在 `unwrap`（可能 panic）
- **错误上下文**：`anyhow::Context` / `thiserror` 是否携带足够信息（文件路径、操作类型）
- **HTTP 错误响应**：API 接口是否在所有错误路径返回结构化 JSON（而非纯文本）
- **日志级别**：`warn!`/`error!` 是否滥用（频繁触发的正常情况不应用 warn）

#### E. 测试覆盖

- **核心逻辑测试**：`markdown.rs`、`tags.rs`、`sidebar.rs`、`graph.rs`、`indexer.rs` 是否有内联测试
- **错误路径测试**：`error.rs` 中的 Display 实现、From 转换是否有覆盖
- **边界条件**：空笔记库、空标签、无效路径、超长路径
- **并发测试**：多请求并发读取 AppState 是否有验证

#### F. 新版本专项（本次引入的变化）

- 对照 `docs/ROADMAP.md` 本版本章节，逐条验证实现是否完整
- 对照 `.claude/project.md` 「常见陷阱」，验证是否踩坑
- 新增路由是否在认证中间件白名单中配置正确（公开 vs 需要认证）

---

### 第三步：分级汇总，写入 CODEREVIEW 报告

**报告文件**：`docs/CODEREVIEW_X.Y.md`

**严重级别**：

| 符号 | 级别 | 定义 |
|------|------|------|
| 🔴 | P0 Critical | 数据损坏、XSS/安全漏洞、生产 panic 风险，必须在本次 patch 修复 |
| 🟠 | P1 High | 语义错误、潜在 panic、异步竞争、功能缺失，应在本次或下版本修复 |
| 🟡 | P2 Medium | 代码质量、错误处理不完善、边界行为未定义，计划修复 |
| 🔵 | P3 Low | 命名/重构建议、冗余代码、注释缺失，方便时修复 |
| ⚪ | Info | 架构观察、未来改进方向，无需立即行动 |

**状态标记**：

| 符号 | 含义 |
|------|------|
| ✅ 已修复（本次）| 本次审计中直接修复 |
| 🔜 → vX.Y.Z | 推迟至指定版本修复 |
| ⏸ 设计性限制，已知接受 | 有意为之，暂不修复 |

**报告结构**（使用 `templates/report.md`）：

```markdown
# obsidian_mirror vX.Y.x 代码审查报告

**审查日期：** YYYY-MM-DD
**审查版本：** vX.Y.Z（commit `hash`）
**审查范围：** `src/` 全部 N 个 `.rs` 文件
**严重级别：** 🔴 P0 / 🟠 P1 / 🟡 P2 / 🔵 P3 / ⚪ Info
**状态标记：** ✅ 已修复（本次）/ 🔜 → 版本 / ⏸ 设计性限制，已知接受

---

## 总体评价

**结论：一句话总结。**

| 维度 | 评分 | 说明 |
|------|------|------|
| 架构设计   | ★★★★☆ | ... |
| 异步正确性 | ★★★★☆ | ... |
| 安全性     | ★★★☆☆ | ... |
| 错误处理   | ★★★★☆ | ... |
| 测试覆盖   | ★★★☆☆ | ... |
| 代码质量   | ★★★★☆ | ... |

---

## 一、安全问题（Security）

### 🔴 S1 - <标题> <状态>

**文件：** `src/xxx.rs:N`
**严重性：** P0

...

---

## 二、Bug / 正确性（Correctness）

### 🔴 B1 - <标题> <状态>

**文件：** `src/xxx.rs:N`
**严重性：** P0

...

---

## 三、异步与并发（Async）

...

---

## 四、错误处理（Error Handling）

...

---

## 五、性能（Performance）

...

---

## 六、测试覆盖（Testing）

...

---

## 七、代码质量（Code Quality）

...

---

## 修复状态汇总

| 编号 | 问题 | 级别 | 状态 | 修复版本 |
|------|------|------|------|---------|
| S1 | <标题> | 🔴 P0 | <状态> | vX.Y.Z |
| B1 | <标题> | 🔴 P0 | <状态> | vX.Y.Z |

**修复统计（本次审计）**：已修复 **N 项** / 推迟 **N 项** / 接受为设计限制 **N 项**

---

## 进入下一大版本前必须解决的问题

> 以下问题必须在 vX.Y.x 系列内解决，不得带入 vX.(Y+1).0。

1. **S1** ✅/🔜 — <标题>：<说明>
```

---

### 第四步：更新历史 CODEREVIEW 文件

对每一个历史 `docs/CODEREVIEW_X.Y.md`：

1. 找出其中标记为 `🔜 → 当前版本以前` 且本次已解决的条目
2. 将状态从 `🔜` 更新为 `✅ 已修复（vX.Y.Z）`
3. 在修复状态汇总表中同步更新
4. 若本次引入的功能解决了历史遗留问题，同样标注

**更新原则**：只改状态标记，不修改历史问题的描述内容。

---

### 第五步：修复本次必须解决的问题

**本次必须修复**：报告中所有 🔴 P0 条目。

**强烈建议修复**：明确标为「本次修复」的 🟠 P1 条目。

修复流程：

1. 按模块依赖顺序修复（底层 → 上层，参考八层功能修改顺序）
2. 每修复一项，在报告中将状态改为 `✅ 已修复（本次）`
3. 修复后执行 `cargo build`，确认无新编译错误
4. 有针对性地补充测试（如修复 XSS，补充含 `<script>` 的标题测试）

---

### 第六步：验证与收尾

#### 6.1 运行验证

```bash
cargo build      # 必须：零 error，零 warning（新增代码）
cargo test       # 必须：全量通过
cargo clippy     # 必须：零 warning
```

#### 6.2 更新 Cargo.toml 版本号

```toml
[package]
version = "X.Y.(Z+1)"   # 末位加一
```

#### 6.3 更新 CHANGELOG.md

在 `## [Unreleased]` 下方插入（使用 `templates/changelog_entry.md`）：

```markdown
## [vX.Y.(Z+1)] — YYYY-MM-DD

代码审计修复版本（CODEREVIEW_X.Y）。

### Fixed
- [S1] <问题简述>：<修复说明>
- [B1] <问题简述>：<修复说明>

### Changed（如有）
- <重构或行为调整>

### 审计统计
- 🔴 P0 修复：N 项
- 🟠 P1 修复：N 项（推迟 N 项）
- 发现问题总计：N 项
```

#### 6.4 更新 ROADMAP.md

在 ROADMAP 中为本次 patch 添加已发布记录，并更新对应计划版本的状态：

```markdown
### ✅ vX.Y.(Z+1) (已发布 - YYYY-MM-DD)

**主题**: 代码审计修复（CODEREVIEW_X.Y）

#### 修复内容
- ✅ **[S1] <标题>**：<修复说明>
- ✅ **[B1] <标题>**：<修复说明>

#### 测试结果
- 全量测试：**N/N 通过**
- 新增测试：N 个
```

#### 6.5 更新 README.md、CLAUDE.md、.claude/project.md

- **README.md**：检查功能列表是否与当前实现一致；更新代码示例中已变更的 API（如有）
- **CLAUDE.md**：更新 HTTP 路由表（如有新路由）；更新依赖版本（如有升级）
- **.claude/project.md**：更新「当前开发状态」表版本号和模块状态；同步「依赖版本速查」

**不要修改**：架构速查、常见陷阱等已正确的内容。

#### 6.6 git 提交

```bash
git add docs/CODEREVIEW_X.Y.md \
        docs/CODEREVIEW_*.md \
        docs/CHANGELOG.md \
        docs/ROADMAP.md \
        README.md CLAUDE.md .claude/project.md \
        Cargo.toml \
        <所有修复涉及的源码文件>

git commit -m "$(cat <<'EOF'
review(vX.Y): 代码审计 + 修复 CODEREVIEW_X.Y

审计范围：src/ 全部 N 个 .rs 文件，共发现 P0/P1/P2/P3/Info 项问题。

本次修复（P0）：
- [S1] <简述>
- [B1] <简述>

本次修复（P1）：
- [B2] <简述>

推迟至后续版本：
- [P1] → vX.Y.W：<简述>

版本号：X.Y.Z → X.Y.(Z+1)
文档：CHANGELOG.md / ROADMAP.md / .claude/project.md 已同步更新
EOF
)"
```

---

## 强制规则

1. **先全量阅读再下结论**：每个主要源文件必须读取，不得仅凭文件名猜测问题
2. **问题要有文件和行号**：每个审计条目必须给出 `src/xxx.rs:N` 定位，并附代码片段
3. **P0 必须修复**：报告中的 🔴 P0 条目必须在本次 patch 中修复，无例外
4. **历史文件必须同步更新**：已解决的历史问题不得保持 `🔜` 状态
5. **修复后必须验证**：`cargo build` + `cargo test` + `cargo clippy` 全部通过后才能进行收尾步骤
6. **版本号末位加一**：审计修复专用 patch bump，不得与功能版本合并
7. **CHANGELOG.md 必须更新**：在 `## [Unreleased]` 下方插入新条目，格式遵循 Keep a Changelog（`docs/CHANGELOG.md`）
8. **README/CLAUDE.md/.claude/project.md 必须更新**：检查并修正与当前实现不符的描述
9. **不降低已有评分**：若本版本未引入新问题，维度评分不得低于上个版本

---

## 输出格式约定

### 阶段 A：审计开始

```markdown
## 审计启动 — vX.Y.x

当前版本：X.Y.Z
报告输出：docs/CODEREVIEW_X.Y.md
审计范围：src/ 全部 N 个 .rs 文件

正在读取历史审计：
- docs/CODEREVIEW_1.2.md（已读）

开始逐模块审计...
```

### 阶段 B：发现问题时

```markdown
### 发现 [S-1]：<标题>

文件：`src/xxx.rs`，第 N 行
严重级别：🔴 P0
类别：安全性 / XSS / 异步正确性 / ...

```rust
// 问题代码片段
```

问题：...
建议：...
本次处理：✅ 立即修复 / 🔜 推迟至 vX.Y.W
```

### 阶段 C：收尾报告

```markdown
## 审计收尾 — vX.Y.x

### 审计发现汇总
- 🔴 P0：N 项（本次修复 N 项）
- 🟠 P1：N 项（本次修复 N 项，推迟 N 项）
- 🟡 P2：N 项
- 🔵 P3：N 项
- ⚪ Info：N 项

### 历史文件更新
- CODEREVIEW_1.2.md：更新 N 项状态

### 验证结果
- cargo build：✅ 零 error
- cargo test：✅（N/N 通过）
- cargo clippy：✅ 零 warning

### 文档更新
- CHANGELOG.md：新增 [vX.Y.(Z+1)] 条目
- ROADMAP.md：vX.Y.(Z+1) 标记 ✅
- README.md：更新 N 处
- CLAUDE.md：更新依赖版本 / 路由表
- .claude/project.md：更新版本状态表

### 版本变更
X.Y.Z → X.Y.(Z+1)

### git 提交
（提交 hash）
```
