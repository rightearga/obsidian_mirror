# obsidian_mirror v1.8.x 代码审查报告

**审查日期：** 2026-04-15  
**审查版本：** v1.8.0（commit `dcc516c`）  
**审查范围：** v1.8.0 引入的全部新增/修改文件（2 个 `.rs` + 4 个 `.js` + 1 个 `.css` + 1 个模板）  
**严重级别：** 🔴 P0 / 🟠 P1 / 🟡 P2 / 🔵 P3 / ⚪ Info  
**状态标记：** ✅ 已修复（本次）/ 🔜 → 版本 / ⏸ 设计性限制，已知接受

---

## 总体评价

**结论：v1.8.0 三项规模化性能优化代码质量良好；发现 1 项 P2 越界风险（large page 值溢出）和 1 项 P3 文档问题，本次全部修复。**

| 维度 | 评分 | 说明 |
|------|------|------|
| 架构设计   | ★★★★★ | SearchPage 结构清晰，渐进式加载两阶段设计合理 |
| 异步正确性 | ★★★★★ | requestIdleCallback + 安全检查完善 |
| 安全性     | ★★★★★ | page/per_page 参数均有有效范围限制（B1 修复后升至★5）|
| 错误处理   | ★★★★☆ | search.js 向后兼容 `data.results ?? data` 良好 |
| 测试覆盖   | ★★★★★ | 4 个分页测试覆盖正常/边界/超范围/截断 |
| 代码质量   | ★★★★☆ | WASM 模块注释未随发布版本更新（Q1 修复后升至★5）|

---

## 一、Bug / 正确性（Correctness）

### 🟡 B1 - `page` 参数无上限，Modified 排序时可能触发 usize 溢出 ✅ 已修复（本次）

**文件：** `src/search_engine.rs:496`  
**严重性：** P2

```rust
// 当前：page 只有下限，无上限
let per_page = per_page.clamp(1, 100);
let page     = page.max(1);              // ← 无上限
let offset   = (page - 1) * per_page;   // ← 超大 page 值导致 usize 溢出

// SortBy::Modified 路径：
let fetch = (offset + per_page).max(5000).min(total);
// 若 offset 已溢出，min(total) 可能无法正确截断
```

当 `SortBy::Modified` 且 `page` 值极大时，`(page - 1) * per_page` 在 Debug 模式下 panic，Release 模式下 wrap 产生错误 offset，导致 `TopDocs::with_limit(fetch)` 行为不可预测。

`SortBy::Relevance` 路径使用 Tantivy 原生 `and_offset(offset)`，Tantivy 内部将 offset 与 total 比较后返回空列表，实际安全；Modified 路径的 `min(total)` 也能在 offset 未溢出时正确截断，但依赖 usize 不溢出这一前提。

**修复方案**：加 `page.min(10_000)` 上限——实际笔记库单页 20 条时最多 10000 页即 200000 条，远超任何真实用户场景，既不限制正常使用又消灭溢出风险。

---

## 二、代码质量（Code Quality）

### 🔵 Q1 - WASM crate 模块版本号未随 v1.8.0 更新 ✅ 已修复（本次）

**文件：** `crates/wasm/src/lib.rs:1`  
**严重性：** P3

```rust
//! obsidian_mirror WebAssembly 模块（v1.7.0）  // ← 当前发布为 v1.8.0
```

遵循历次审计惯例（v1.6.6→v1.6.5、v1.7.1→v1.7.0），模块注释应反映当前发布版本。

---

## 三、信息性观察（Info）

### ⚪ I1 - `content-visibility: auto` 在旧版 Firefox 中不完整支持（信息性）

**文件：** `static/css/sidebar.css`  
**严重性：** Info

`content-visibility: auto` 在 Chrome/Edge 85+ 和 Firefox 109+ 上支持完整，旧版浏览器仅作降级（元素正常渲染，无性能优化但也无功能损失）。不影响功能，接受为已知限制。

### ⚪ I2 - `_appendSearchResults` 每次追加创建新数组（信息性）

**文件：** `static/js/search.js`  
**严重性：** Info

```js
currentSearchResults = currentSearchResults.concat(results);
```

`Array.concat` 创建新数组。分页步长 20 条，用户触发"加载更多"次数有限，不构成实际内存压力。接受为已知设计选择。

---

## 修复状态汇总

| 编号 | 问题 | 级别 | 状态 | 修复版本 |
|------|------|------|------|---------|
| B1 | `page` 无上限，Modified 排序 usize 溢出风险 | 🟡 P2 | ✅ 已修复（本次） | v1.8.1 |
| Q1 | WASM 模块版本号未更新 | 🔵 P3 | ✅ 已修复（本次） | v1.8.1 |
| I1 | content-visibility 旧 Firefox 降级 | ⚪ Info | ⏸ 优雅降级，功能不受影响 | — |
| I2 | concat 创建新数组 | ⚪ Info | ⏸ 分页步长小，无实际影响 | — |

**修复统计（本次审计 v1.8.1）**：  
已修复 **2 项**（B1/Q1） / 接受为设计限制 **2 项**

---

## 进入 v1.8.2 前必须解决的问题

无 P0/P1 问题。v1.8.x 系列内 P2 问题（B1）已在 v1.8.1 修复。

---

## v1.8.0 新功能安全性确认

| 功能 | 审计结论 |
|------|---------|
| 搜索分页 `page`/`per_page` | ✅ per_page clamp(1,100)；B1 修复后 page 也有上限 |
| `SearchPage` 序列化 | ✅ 仅含基本类型，无 XSS 风险 |
| requestIdleCallback 延迟初始化 | ✅ timeout:2000 兜底，不阻塞关键路径 |
| 图谱渐进式加载 | ✅ `if (!graphNetwork) return` 防止 use-after-free |
| `content-visibility: auto` | ✅ 仅影响渲染性能，不影响功能或安全 |
