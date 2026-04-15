# obsidian_mirror v1.7.x 代码审查报告

**审查日期：** 2026-04-15  
**审查版本：** v1.7.0（commit `af605ea`）+ `search_engine.rs` 未提交修复  
**审查范围：** v1.7.0 引入的全部新增/修改文件（5 个 `.rs` + 2 个模板文件）  
**严重级别：** 🔴 P0 / 🟠 P1 / 🟡 P2 / 🔵 P3 / ⚪ Info  
**状态标记：** ✅ 已修复（本次）/ 🔜 → 版本 / ⏸ 设计性限制，已知接受

---

## 总体评价

**结论：v1.7.0 全局知识图谱专页 + WASM M4/M5 性能优化代码质量良好；发现 1 项 P1 生产级 Bug（Windows 文件锁）和 1 项 P3 文档问题，本次全部修复。**

| 维度 | 评分 | 说明 |
|------|------|------|
| 架构设计   | ★★★★★ | 图谱专页复用现有 API + layout，M4/M5 改动最小化 |
| 异步正确性 | ★★★★★ | ReloadPolicy::Manual 修复 Windows 文件锁竞争（B1 修复后升至★5）|
| 安全性     | ★★★★★ | /graph 路由正确受认证保护，无白名单遗漏 |
| 错误处理   | ★★★★☆ | search_engine.rs reader.reload() 失败时降级为 warn（不 panic）|
| 测试覆盖   | ★★★★★ | 新增 4 个 WASM 测试（M4 ×2、M5 ×2），全量 136 个测试通过 |
| 代码质量   | ★★★★☆ | WASM 模块注释版本号未随 v1.7.0 更新（Q1 修复后升至★5）|

---

## 一、Bug / 正确性（Correctness）

### 🟠 B1 - ReloadPolicy::OnCommitWithDelay 在 Windows 上产生文件锁冲突 ✅ 已修复（本次）

**文件：** `src/search_engine.rs:104`  
**严重性：** P1

```rust
// v1.7.0 之前（存在竞争窗口）：
let reader = index
    .reader_builder()
    .reload_policy(ReloadPolicy::OnCommitWithDelay)  // ← 启动后台 reload 线程
    .try_into()?;
```

`OnCommitWithDelay` 在每次 `commit()` 后自动启动后台线程，延迟数百毫秒后重新打开段文件（`.term`、`.idx`）。在 Windows 上，文件锁是**强制性的（Mandatory Lock）**，而非 Unix 的建议性锁：

- Writer 段合并阶段需要写新文件、删旧文件
- 后台 reload 线程在 commit 延迟窗口内与 writer 竞争相同文件的读写锁
- 时序竞争 → `PermissionDenied (os error 5)`

**错误日志特征：**
```
ERROR   └─ 重建搜索索引失败: Failed to open file for write:
    'IoError { io_error: Os { code: 5, kind: PermissionDenied, message: "拒绝访问。" },
     filepath: "2d8c36c07adf4b6785f2da7219d0401a.term" }'
```

**修复方案**：改用 `ReloadPolicy::Manual`，消灭后台线程；`rebuild_index` 和 `update_documents` 在 `commit()` 完成后显式调用 `self.reader.reload()`，保证写入完全结束后再刷新读视图。

---

## 二、代码质量（Code Quality）

### 🔵 Q1 - WASM crate 模块版本号未随 v1.7.0 更新 ✅ 已修复（本次）

**文件：** `crates/wasm/src/lib.rs:1`  
**严重性：** P3

```rust
//! obsidian_mirror WebAssembly 模块（v1.6.5）  // ← v1.7.0 引入 M4/M5 后未更新
```

v1.7.0 新增了 M4（评分阶段 Bitset）和 M5（Barnes-Hut θ 自适应），但模块顶部注释仍为 v1.6.5。

---

## 三、信息性观察（Info）

### ⚪ I1 - graph_page.html vis.js tooltip innerHTML（信息性）

**文件：** `templates/graph_page.html`  
**严重性：** Info

```js
title: node.label + (node.tags?.length ? '\n标签: ' + node.tags.join(', ') : ''),
```

vis.js 将 `title` 属性内容以 `innerHTML` 注入 tooltip DOM，若笔记标题或标签含 `<script>` 等 HTML 特殊字符，理论上可通过 tooltip 触发 XSS。

与 `static/js/graph.js`（v1.4.3 起存在）的相同模式一致，历次审计均接受为设计性限制（自用笔记内容自我 XSS，不构成跨用户威胁）。接受为已知设计选择。

---

## 四、安全性确认（Security）

| 检查项 | 结论 |
|--------|------|
| `GET /graph` 路由认证 | ✅ 未加入 auth 白名单，`auth_enabled=true` 时正确要求认证 |
| `/graph` 页面加载 vis.js | ✅ `layout.html` 全局加载 vis-network，无依赖缺失 |
| M4 u8 位移溢出 | ✅ query_tokens ≤ 8，bit i ∈ {0..7}，`1u8 << 7 = 128` 合法 |
| M5 iterations=0/1 边界 | ✅ iterations=0 循环不执行；iterations=1 时 warmup_iters=1，单次迭代使用 θ_early，正确 |
| GraphPageTemplate 注入 | ✅ title="知识图谱" 静态字符串；sidebar 来自服务端已处理数据 |

---

## 修复状态汇总

| 编号 | 问题 | 级别 | 状态 | 修复版本 |
|------|------|------|------|---------|
| B1 | ReloadPolicy::OnCommitWithDelay 产生 Windows 文件锁 PermissionDenied | 🟠 P1 | ✅ 已修复（本次） | v1.7.1 |
| Q1 | WASM crate 模块版本号未更新为 v1.7.0 | 🔵 P3 | ✅ 已修复（本次） | v1.7.1 |
| I1 | vis.js tooltip innerHTML 自内容 XSS | ⚪ Info | ⏸ 与 graph.js 已有行为一致 | — |

**修复统计（本次审计 v1.7.1）**：  
已修复 **2 项**（B1/Q1） / 接受为设计限制 **1 项**

---

## 进入 v1.7.2 前必须解决的问题

无 P0 问题。v1.7.x 系列内 P1 问题（B1）已在 v1.7.1 修复。

---

## v1.7.0 新功能安全性确认

| 功能 | 审计结论 |
|------|---------|
| GET /graph 全屏图谱专页 | ✅ 认证保护正确；静态模板内容无 XSS 风险 |
| 图谱聚类着色 | ✅ 纯 JS 内存计算，无 IO 或 RPC 风险 |
| WASM M4 评分 Bitset | ✅ query_tokens ≤ 8，u8 位移边界安全 |
| WASM M5 θ 自适应 | ✅ 边界条件（iterations=0/1）验证通过 |
| ReloadPolicy::Manual | ✅ 消灭后台线程，修复 Windows 文件锁竞争 |
