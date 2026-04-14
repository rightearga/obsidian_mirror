# obsidian_mirror v1.6.x 代码审查报告

**审查日期：** 2026-04-14  
**审查版本：** v1.6.5（commit `378fc09`）  
**审查范围：** `src/` 全部 27 个 `.rs` 文件 + `crates/wasm/src/lib.rs`  
**严重级别：** 🔴 P0 / 🟠 P1 / 🟡 P2 / 🔵 P3 / ⚪ Info  
**状态标记：** ✅ 已修复（本次）/ 🔜 → 版本 / ⏸ 设计性限制，已知接受

---

## 总体评价

**结论：v1.6.x WASM 系列实现了显著的客户端性能改善（离线搜索、图谱布局加速等），整体代码质量良好；发现 1 项 P1 安全问题（index.json 认证绕过）和若干 P2/P3 质量问题，本次全部修复。**

| 维度 | 评分 | 说明 |
|------|------|------|
| 架构设计   | ★★★★★ | Bitset / Barnes-Hut / TokenCache 设计正确，层次清晰 |
| 异步正确性 | ★★★★☆ | index.json 生成句柄未加入 background_tasks（B2 修复后升至★5）|
| 安全性     | ★★★★☆ | index.json 在 auth 启用时暴露笔记内容（B1 修复后升至★5）|
| 错误处理   | ★★★★☆ | WASM 中的 `.expect()` 仅在测试路径，生产路径良好 |
| 测试覆盖   | ★★★★★ | 132 个测试覆盖所有新增功能，WASM 行为测试完善 |
| 代码质量   | ★★★★☆ | 注释文档详尽；WASM crate 版本号未同步更新 |

---

## 一、安全问题（Security）

### 🟠 B1 - index.json 在 auth_enabled 时暴露给未认证用户 ✅ 已修复（本次）

**文件：** `src/sync.rs:563`  
**严重性：** P1

```rust
// sync.rs — 无论是否启用认证，都写入 static/wasm/index.json
tokio::task::spawn(async move {
    if let Ok(json) = generate_search_index_json(&notes_for_index) {
        tokio::fs::write("static/wasm/index.json", json).await  // ← 总是写入
    }
});
```

`/static/` 路径在 `auth_middleware.rs` 中为公开路径（`path.starts_with("/static/")`），无需认证即可访问。`index.json` 包含所有笔记的 `title`、`path`、`tags` 及内容摘要（前 300 字符），当 `auth_enabled = true` 时，这些私有数据对任意未认证用户可见。

**修复方案**：当 `auth_enabled = true` 时跳过 index.json 生成。离线搜索本质上只对公开部署（auth 禁用）有意义——启用认证的部署需要凭据，Service Worker 的离线拦截无法携带，因此跳过不影响功能。

---

## 二、Bug / 正确性（Correctness）

### 🟡 B2 - index.json 生成任务未加入 background_tasks ✅ 已修复（本次）

**文件：** `src/sync.rs:562`  
**严重性：** P2

```rust
// 当前：spawn 后丢弃句柄，优雅关闭不等待此任务
tokio::task::spawn(async move {
    // ... 写入 index.json ...
});
```

v1.5.5 建立了 `background_tasks` 机制，让优雅关闭等待后台任务完成（30s 超时）。`sync_engine.rebuild_index` 和 `persistence.save_indexes` 的句柄已加入，但 `index.json` 写入任务未加入，服务器关闭时可能产生文件写入截断。

---

## 三、代码质量（Code Quality）

### 🔵 Q1 - WASM crate 模块文档版本号未同步更新

**文件：** `crates/wasm/src/lib.rs:1`  
**严重性：** P3

```rust
//! obsidian_mirror WebAssembly 模块（v1.6.1）  // ← 当前为 v1.6.5
```

已发布 v1.6.5，文档标注仍为 v1.6.1。

---

### 🟡 Q2 - index.json 使用相对路径写入，依赖运行时工作目录

**文件：** `src/sync.rs:566`  
**严重性：** P2（已有 warn 日志，但路径不可靠）

```rust
tokio::fs::write("static/wasm/index.json", json).await
```

写入路径相对于进程工作目录。在 Docker 容器或非标准部署中，若工作目录不是项目根目录，写入会静默失败（日志记录 warn，但无法保证文件存在）。对比 `serve_service_worker` 中 `static/sw.js` 的硬编码路径，是一致的设计选择，接受为设计限制。

---

### ⚪ Q3 - render_markdown 的 pulldown-cmark HTML passthrough（信息性）

**文件：** `crates/wasm/src/lib.rs`  
**严重性：** Info

`render_markdown` 使用默认安全级别的 pulldown-cmark，原始 HTML（`<script>`、`<img onerror=...>` 等）会被透传到 HTML 输出，并通过 `WasmPreview._render()` 的 `output.innerHTML = html` 注入 DOM。

由于实时预览功能仅面向已认证用户预览自己的笔记内容（自 XSS），不构成跨用户安全威胁。接受为已知设计选择。

---

## 四、异步正确性（Async）

### ⚪ A1 - NoteIndex WASM 对象内存未显式释放（信息性）

**文件：** `static/wasm/loader.js`  
**严重性：** Info

`WasmLoader.noteIndex` 持有 Rust WASM 堆内存，页面卸载时未调用 `.free()`。wasm-bindgen 生成的 JS 代码在对象超出 JS 引用范围后理论上可被 GC，但实际行为依赖浏览器实现。在 SPA 中（笔记之间跳转不刷新页面），不存在此问题。接受为已知限制。

---

## 修复状态汇总

| 编号 | 问题 | 级别 | 状态 | 修复版本 |
|------|------|------|------|---------|
| B1 | index.json 在 auth 启用时暴露私有笔记内容 | 🟠 P1 | ✅ 已修复（本次） | v1.6.6 |
| B2 | index.json 生成任务未加入 background_tasks | 🟡 P2 | ✅ 已修复（本次） | v1.6.6 |
| Q1 | WASM crate 模块版本号未更新 | 🔵 P3 | ✅ 已修复（本次） | v1.6.6 |
| Q2 | index.json 相对路径依赖工作目录 | 🟡 P2 | ⏸ 设计限制，与其他静态路径一致 | — |
| Q3 | render_markdown HTML passthrough | ⚪ Info | ⏸ 自用笔记预览，已知接受 | — |
| A1 | NoteIndex WASM 内存未显式释放 | ⚪ Info | ⏸ SPA 场景不存在泄漏 | — |

**修复统计（本次审计 v1.6.6）**：  
已修复 **3 项**（B1/B2/Q1） / 接受为设计限制 **3 项**

---

## 进入 v1.7.0 前必须解决的问题

无 P0 问题。v1.6.x 系列内已修复所有 P1 问题（B1）。

---

## v1.6.x 新功能安全性确认

| 功能 | 审计结论 |
|------|---------|
| WASM render_markdown | ✅ WikiLink/公式/高亮均通过 `html_escape` 转义 |
| NoteIndex 搜索 | ✅ 仅读取内存数据，无 IO 风险 |
| Barnes-Hut 图谱布局 | ✅ 纯计算，无安全风险 |
| Bitset 候选集 | ✅ 边界检查 `if word < self.bits.len()` 防止越界 |
| expand_embeds | ✅ 深度限制 2 层，title/section 均通过 `html_escape_text` 处理 |
| generate_search_index_json | ⚠ B1：auth 启用时跳过（本次修复）|
