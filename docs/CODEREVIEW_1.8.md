# obsidian_mirror v1.8.x 代码审查报告

**审查日期：** 2026-04-15  
**审查版本：** v1.8.6（commit `4f05cd0`）  
**审查范围：** v1.8.0–v1.8.6 全系列新增/修改文件（`src/` 全部 `.rs` 文件 + `static/sw.js` + `crates/wasm/src/lib.rs`）  
**严重级别：** 🔴 P0 / 🟠 P1 / 🟡 P2 / 🔵 P3 / ⚪ Info  
**状态标记：** ✅ 已修复（本次）/ 🔜 → 版本 / ⏸ 设计性限制，已知接受

---

## 总体评价

**结论：v1.8.x 系列交付了规模化性能优化、导出发布、PWA 离线完善、可视化增强、依赖升级和性能回归修复，代码整体质量高；发现 1 项 P2 错误处理问题和 2 项 P3 文档问题，本次全部修复。无 P0/P1 问题。**

| 维度 | 评分 | 说明 |
|------|------|------|
| 架构设计   | ★★★★★ | mtime_cache/InsightsCache 设计合理；WASM M4 及时回退体现良好工程判断 |
| 异步正确性 | ★★★★★ | background_tasks/mtime_cache 填充正确；SW SYNC_COMPLETE 安全广播 |
| 安全性     | ★★★★★ | Atom feed XML 转义完整；export content_html 为自内容导出（已知接受）|
| 错误处理   | ★★★★☆ | zip.write_all 静默丢弃（B1 修复后升至★5）|
| 测试覆盖   | ★★★★★ | 分页 ×4、commit hash ×2、diff XSS ×1、most_linked ×2 等专项测试完善 |
| 代码质量   | ★★★★☆ | WASM/SW 注释版本号过期（Q1/Q2 修复后升至★5）|

---

## 一、Bug / 正确性（Correctness）

### 🟡 B1 - export_html_handler zip.write_all 错误被静默丢弃 ✅ 已修复（本次）

**文件：** `src/handlers.rs:1765`  
**严重性：** P2

```rust
// 当前：write_all 失败时静默忽略
let _ = zip.write_all(html.as_bytes());
```

使用 `let _` 丢弃 `write_all` 的 `Result`，若写入失败（如内存压力、zip 内部错误）将产生空/截断条目而不报告任何错误，导致下载的 zip 文件含损坏条目但 HTTP 状态码仍为 200。

**修复方案**：改为 `if let Err(e) = zip.write_all(...) { error!(...); }` 至少记录警告日志，让用户可排查问题。

---

## 二、代码质量（Code Quality）

### 🔵 Q1 - WASM 模块版本注释未随 v1.8.6 更新 ✅ 已修复（本次）

**文件：** `crates/wasm/src/lib.rs:1`  
**严重性：** P3

```rust
//! obsidian_mirror WebAssembly 模块（v1.8.0）  // ← 当前为 v1.8.6
```

v1.8.6 回退了 M4 评分方案（重要的 WASM 逻辑变更），版本注释应同步更新。

---

### ⚪ Q2 - Service Worker 版本注释确认正确（信息性）

**文件：** `static/sw.js:2`

```js
// Service Worker — Obsidian Mirror v1.8.3   // ← 正确
// v1.6.2 新增：WASM 离线搜索（历史变更注释）
```

第 2 行已正确标注 `v1.8.3`，第 4 行的 `v1.6.2` 为历史变更注释（非版本号声明）。审计时误判，实际无需修改。✅

---

## 三、信息性观察（Info）

### ⚪ I1 - build_static_note_html：content_html 原样插入 ZIP 导出（信息性）

**文件：** `src/handlers.rs:1726`  
**严重性：** Info

```rust
content = note.content_html,  // 原样插入自包含 HTML
```

静态站点导出将 `content_html` 原样插入，与 `feed.xml` 的 CDATA 处理方式一致。由于导出物是用户自己的笔记内容（自内容，self-XSS），属于已知设计选择，与历次审计接受的 `graph.js` tooltip 等场景一致。接受为设计限制。

### ⚪ I2 - SW SYNC_COMPLETE 广播安全性确认（信息性）

**文件：** `static/sw.js:121`  
**严重性：** Info

```js
if (url.pathname === '/sync' && request.method === 'POST') {
    fetch(request).then(response => {
        if (response.ok) {   // ← 仅 2xx 响应时广播
            clients.forEach(c => c.postMessage({ type: 'SYNC_COMPLETE' }));
        }
    });
}
```

`SYNC_COMPLETE` 仅在 `response.ok`（HTTP 2xx）时广播。`POST /sync` 在 `auth_enabled=true` 时要求 admin 角色，未认证请求返回 401（非 ok），不触发广播。✅ 安全设计正确。

### ⚪ I3 - Atom feed 安全性确认（信息性）

**文件：** `src/handlers.rs:1643-1656`  
**严重性：** Info

```rust
xml_escape(&note.title),   // ← title 正确 XML 转义
note.content_html,          // ← 置于 CDATA 块，无需转义
```

`<title>` 字段使用 `xml_escape()`；`<content>` 使用 `<![CDATA[...]]>` 包裹原始 HTML。CDATA 块允许包含任意字符（含 `<`、`>`、`&`），RSS 阅读器按 HTML 渲染，属标准做法。✅ XML 注入风险已正确防护。

---

## 四、安全性确认（v1.8.x 新功能）

| 功能 | 审计结论 |
|------|---------|
| Atom feed 生成 | ✅ title xml_escape；content CDATA；link URL 编码 |
| 静态站点 ZIP 导出 | ✅ 自内容导出（已认证用户下载自己的笔记）；nav 标题 xml_escape |
| timeline_api_handler frontmatter date | ✅ `.len().min(10)` 防 OOB；返回 JSON 字符串，无 HTML 注入 |
| PWA SYNC_COMPLETE 广播 | ✅ 仅 response.ok 时触发；认证保护覆盖 |
| 搜索分页 page/per_page | ✅ clamp(1,100) + min(10_000)；已在 v1.8.1 B1 修复 |
| export_html zip 错误 | ✅ zip.finish() 错误有处理；write_all B1 本次修复 |

---

## 修复状态汇总

| 编号 | 问题 | 级别 | 状态 | 修复版本 |
|------|------|------|------|---------|
| B1 | export_html zip.write_all 静默忽略 | 🟡 P2 | ✅ 已修复（本次） | v1.8.7 |
| Q1 | WASM 模块版本注释过期 | 🔵 P3 | ✅ 已修复（本次） | v1.8.7 |
| Q2 | sw.js 版本注释 | ⚪ Info | ⏸ 审计确认已正确，无需修改 | — |
| I1 | build_static_note_html 自内容插入 | ⚪ Info | ⏸ 自内容导出，历次一致接受 | — |
| I2 | SYNC_COMPLETE 广播安全性 | ⚪ Info | ⏸ 认证保护有效，设计正确 | — |
| I3 | Atom feed XML 处理 | ⚪ Info | ⏸ 设计正确，无需修改 | — |

**修复统计（本次审计 v1.8.7）**：  
已修复 **2 项**（B1/Q1） / 确认正确 **1 项**（Q2）/ 接受为设计限制 **3 项**

---

## 进入 v1.9.x 前必须解决的问题

无 P0/P1 问题。v1.8.x 系列代码质量良好，可进入 v1.9.x 开发。
