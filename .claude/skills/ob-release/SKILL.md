---
name: ob-release
description: obsidian_mirror 版本开发驱动。根据目标版本号从 docs/ROADMAP.md 提取开发内容，按八层功能修改顺序（config → error → db → core → handlers → main → state → templates）逐层实现，每层后验证编译，循环回查遗漏，最终更新版本号、ROADMAP、CHANGELOG 并提交 git。
---

# obsidian_mirror 版本开发驱动

## 用途

当用户指定目标版本号时，严格执行以下流程：

1. 从 `docs/ROADMAP.md` 定位目标版本章节，提取全部工作项
2. 按八层功能修改顺序拆解为有序 todo
3. 逐层实施，每层完成后 `cargo build` 验证
4. 全部完成后回查 roadmap 检查遗漏，加入新 todo 继续实施
5. 循环直到全部覆盖，执行收尾

**典型触发方式**：`/ob-release 1.3.1`

---

## 固定项目路径

| 资源 | 路径 |
|------|------|
| Roadmap | `docs/ROADMAP.md` |
| Changelog | `docs/CHANGELOG.md` |
| 审计报告 | `docs/CODEREVIEW_X.Y.md`（X.Y 为大版本，如 `1.3`） |
| 版本号 | `Cargo.toml` → `[package].version` |
| 项目指南 | `.claude/project.md` |

---

## 八层功能修改顺序

新特性必须按此顺序修改文件，**禁止跨层跳跃**（下层稳定后才能修改上层）：

```
层 1  src/config.rs                    — 新配置字段，提供 Default 实现，RON 格式
层 2  src/error.rs                     — AppError 新变体（thiserror），中文描述
层 3  src/<feature>_db.rs              — redb 表定义、数据结构（仅需持久化时）
层 4  src/<feature>.rs                 — 核心业务逻辑（git/markdown/search/sync 等）
层 5  src/<feature>_handlers.rs        — actix-web 路由处理器
层 6  src/main.rs                      — 路由注册、初始化组件、AppState 注入
层 7  src/state.rs                     — 全局共享数据字段（RwLock 包裹）
层 8  templates/ + src/templates.rs    — 新 HTML 页面（Askama 编译期模板）
```

todo 拆解时必须按层分组，同一层内才可并行修改。

---

## 核心流程

### 第一步：定位目标版本

1. 读取 `docs/ROADMAP.md`
2. 定位 `### 🔧 vX.Y.Z` / `### 🔒 vX.Y.Z` / `### ✨ vX.Y.Z` 章节（已完成版本为 `### ✅ vX.Y.Z`）
3. 提取该版本的**工作项**（分类：P0/P1 或功能列表）和**设计要点**
4. 同时读取 `.claude/project.md` 的「核心模块架构速查」和「常见陷阱」章节
5. 若该版本引用了 `docs/CODEREVIEW_X.Y.md`，一并读取对应条目

若找不到目标版本，明确告知并停止，不得虚构需求。

---

### 第二步：拆解为有序 TODO

按八层顺序建立 todo，典型结构：

```
【层 1 config — 如有新配置项】
- [ ] src/config.rs：新字段 + Default 实现
- [ ] config.example.ron：补充示例和注释

【层 2 error — 如有新错误类型】
- [ ] src/error.rs：新 AppError 变体，中文 Display

【层 3 数据层 — 如有新持久化需求】
- [ ] src/<feature>_db.rs：TableDefinition、数据结构体（JSON 序列化）

【层 4 核心逻辑 — 主要实现】
- [ ] src/<feature>.rs：核心功能实现
  注意：spawn_blocking 包裹所有 redb 同步 IO
  注意：异步路径不得跨 .await 持有 Mutex

【层 5 处理器 — HTTP 接口】
- [ ] src/<feature>_handlers.rs：actix-web handler
  注意：路径参数用 percent_encoding 解码（中文路径）

【层 6 注册 — main.rs 接入】
- [ ] src/main.rs：路由注册、组件初始化

【层 7 状态 — 如需全局共享】
- [ ] src/state.rs：AppState 新字段（tokio::sync::RwLock）

【层 8 模板 — 如需新页面】
- [ ] templates/<name>.html：继承 layout.html
- [ ] src/templates.rs：新模板结构体 + 字段

【测试与交付检查清单】
- [ ] 每层：cargo build（零 error，零 warning）
- [ ] 每层：cargo test（全量通过）
- [ ] 新公开 API 均有中文注释

【收尾】
- [ ] cargo build — 零 error，零 warning
- [ ] cargo test — 全量通过
- [ ] cargo clippy — 零 warning
- [ ] docs/CHANGELOG.md 新增 [vX.Y.Z] 条目
- [ ] docs/ROADMAP.md 标记 ✅，填写详情
- [ ] Cargo.toml 版本号 → X.Y.Z
- [ ] README.md / CLAUDE.md / .claude/project.md 同步（如有 API / 配置变化）
- [ ] git 提交
```

---

### 第三步：逐层实施

- 每层开始前说明：修改哪些文件、预期功能变化
- 修改后说明：改了哪些文件、新增了哪些路由/类型/函数
- 每层完成后立即执行 `cargo build`，确认零 error 后再进入下一层

**必须遵守的约束**（来自 `.claude/project.md` 常见陷阱）：

1. **路径分隔符**：`notes` HashMap 的 key 统一使用 `/`；Git diff 返回的路径需 `.replace("\\", "/")`
2. **redb 同步阻塞**：所有 redb IO 必须在 `tokio::task::spawn_blocking` 内执行
3. **路径解码**：handler 中路径参数必须用 `percent_encoding::percent_decode_str` 解码（支持中文）
4. **Askama 模板**：修改 templates/ 后必须重新编译才生效；新增字段需同步修改结构体和 HTML
5. **持久化版本**：修改持久化结构体（`Note`、`SidebarNode` 等）时必须递增 `src/persistence.rs` 中的 `CURRENT_VERSION`
6. **并发写保护**：/sync 端点需通过 `AppState.sync_lock` 防止并发同步（若已实现）
7. **中文注释规范**：所有新增的 `///`、`//!`、`//` 注释必须使用中文

---

### 第四步：回查 Roadmap

所有 todo 完成后，回到 `docs/ROADMAP.md` 原文逐条核查：

- `✅ 已完成` — 代码实现 + 测试 + 中文注释均到位
- `⚠ 部分完成` — 有实现但缺测试或注释
- `❌ 遗漏` — 未实现

对 `⚠` 和 `❌` 项：加入新 todo → 继续实施 → 再次回查。循环直到全部为 `✅`。

---

### 第五步：收尾发布

#### 5.1 验证命令（必须按序执行）

```bash
cargo build          # 必须：零 error，零 warning（新增代码）
cargo test           # 必须：全量通过
cargo clippy         # 必须：零 warning
```

#### 5.2 更新 Cargo.toml 版本号

```toml
[package]
version = "X.Y.Z"   # 递增为目标版本
```

#### 5.3 更新 ROADMAP.md

版本状态变更格式：

```markdown
### ✅ vX.Y.Z (已发布 - YYYY-MM-DD)

**主题**: <一句话主题>

#### 核心功能
- ✅ **[问题编号] 问题标题**
  - 文件：`src/xxx.rs`
  - 修复：<修复说明>

#### 技术改进（如有）
- ✅ <改进说明>

#### 实际交付物
- 修改文件：`src/xxx.rs`（核心修复）
- 修改文件：`src/yyy.rs`（关联修复）

#### 测试结果
- 全量测试：**N/N 通过**
- 新增测试：N 个
```

#### 5.4 更新 CHANGELOG.md

在 `## [Unreleased]` 下方插入：

```markdown
## [vX.Y.Z] — YYYY-MM-DD

一句话摘要。

### Fixed（修复问题）
- [B1] <问题简述>：<修复说明>
- [S1] <问题简述>：<修复说明>

### Added（新增功能，如有）
- <功能简述>

### Changed（行为变更，如有）
- <变更说明>

### 修复统计（如为 review 驱动版本）
- 🔴 P0 修复：N 项
- 🟠 P1 修复：N 项
```

#### 5.5 更新 README.md、CLAUDE.md、.claude/project.md（如有变化）

- **README.md**：更新功能列表、API 说明（如有公开接口变化）
- **CLAUDE.md**：更新依赖版本（如有升级）、HTTP 路由表（如有新路由）
- **.claude/project.md**：更新「当前开发状态」表版本号和模块状态

**不要修改**：架构速查、常见陷阱等已正确的内容。

#### 5.6 git 提交

```bash
git add Cargo.toml \
        docs/ROADMAP.md \
        docs/CHANGELOG.md \
        README.md CLAUDE.md .claude/project.md \
        <所有修改和新增的源码文件>
# 不提交：target/、config.ron、*.db、logs/

git commit -m "$(cat <<'EOF'
feat(vX.Y.Z): <一句话摘要>

<功能/修复要点列表（3-6 条）>

交付物：
- src/<file>.rs（核心实现）
- src/<file>_handlers.rs（HTTP 接口）
- 版本号 X.Y.(Z-1) → X.Y.Z
EOF
)"
```

**提交前检查**：
- `git status` 无意外文件（无 `config.ron`、无 `*.db`、无 `target/`）
- 不提交 `Cargo.lock`（已在 .gitignore）

---

## 强制规则

1. **先读后改**：开始实现前必须读取 `docs/ROADMAP.md` 对应章节 + `.claude/project.md`；若版本引用 CODEREVIEW，必须读取对应条目原文
2. **分层验证**：每层完成后执行 `cargo build`，不积累编译错误
3. **测试是完成标准**：无测试覆盖不得标记为完成
4. **中文注释是完成标准**：新增公开 API 无中文注释，不得标记为完成
5. **不虚报进度**：命令未运行、测试未通过、文件未修改，不得声称已完成
6. **最小修改原则**：只改目标版本需要的内容，不顺手重构无关代码
7. **回查是强制的**：即使 todo 全部完成，也必须对照 roadmap 原文回查至少一次
8. **git 提交是最后一步**：必须在所有验证通过、文档更新完毕后才提交

---

## 输出格式约定

### 阶段 A：规划输出

```markdown
## 目标版本
vX.Y.Z

## 定位文件
- Roadmap：docs/ROADMAP.md（已读）
- Changelog：docs/CHANGELOG.md（已读/将创建）
- 版本源：Cargo.toml（当前：X.Y.(Z-1)）
- CODEREVIEW 引用：docs/CODEREVIEW_X.Y.md（如有）

## Roadmap 摘要
（从 docs/ROADMAP.md 提取的原文要点，P0/P1 分类或功能列表）

## 初始 TODO（按八层顺序）
【层 1 config】...
【层 2 error】...
...
【收尾】...

## 风险 / 待确认
（歧义项、外部依赖、不确定点）
```

### 阶段 B：回查输出

```markdown
## Roadmap 回查 — vX.Y.Z

✅ [B1]：persistence::clear() 遗漏标签表（实现 + 测试）
✅ [S1]：XSS 修复，标题文本 HTML 转义（实现）
⚠ [B4]：增量同步反向链接（代码修复，缺专项测试）
❌ [B2]：/sync 并发保护（未实现）

## 新增 TODO
- [ ] 为 B4 增量同步反向链接补充测试
- [ ] 实现 B2 sync_lock 并发保护
```

### 阶段 C：收尾输出

```markdown
## 收尾报告 — vX.Y.Z

### 验证结果
- cargo build：✅ 零 error
- cargo test：✅ N/N 通过
- cargo clippy：✅ 零 warning

### 更新文件
- Cargo.toml：X.Y.(Z-1) → X.Y.Z
- docs/ROADMAP.md：vX.Y.Z 标记 ✅，详情填写
- docs/CHANGELOG.md：新增 [vX.Y.Z] 条目
- README.md / CLAUDE.md / .claude/project.md：（如有变化）

### git 提交
（提交 hash 和 message 摘要）
```
