# obsidian_mirror v1.5.x 代码审查报告

**审查日期：** 2026-04-14（v1.5.0–v1.5.5 全系列）  
**审查版本：** v1.5.5（commit `f6f2e9b`）  
**审查范围：** `src/` 全部 27 个 `.rs` 文件  
**严重级别：** 🔴 P0 / 🟠 P1 / 🟡 P2 / 🔵 P3 / ⚪ Info  
**状态标记：** ✅ 已修复（本次）/ 🔜 → 版本 / ⏸ 设计性限制，已知接受

---

## 总体评价

**结论：v1.5.x 系列功能丰富（搜索升级、多用户、内嵌语法、SSE 通知），整体代码质量稳步提升；本次审计新增 6 项问题，其中 1 项 P1（sync 失败历史缺失）+ 1 项 P2（SSE 连接泄漏）在本版本修复，其余 4 项为低优先级。**

| 维度 | 评分 | 说明 |
|------|------|------|
| 架构设计   | ★★★★☆ | `RwLock<AppConfig>`、broadcast channel、background_tasks 设计合理 |
| 异步正确性 | ★★★★★ | v1.5.0/1.5.1 修复后 bcrypt/redb 全面异步化；v1.5.5 后台任务句柄存储完善 |
| 安全性     | ★★★★☆ | 多用户角色体系正确；embed 内容为服务端 HTML 无 XSS；SSE 端点需认证保护 |
| 错误处理   | ★★★★☆ | sync 失败历史未记录（B1 修复后升至 ★★★★★） |
| 测试覆盖   | ★★★★☆ | 新增 config_reload 测试；SSE/搜索历史/embed 路径仍缺专项测试 |
| 代码质量   | ★★★★☆ | 搜索历史 key 纳秒冲突风险极低；软删除语义与 HTTP DELETE 方法不符 |

---

## 一、Bug / 正确性（Correctness）

### 🟠 B1 - `sync_history` 只记录成功同步，失败同步未追加记录 ✅ 已修复（本次）

**文件：** `src/sync.rs:542`，`src/handlers.rs:230-236`  
**严重性：** P1

```rust
// sync.rs — SyncRecord 只在 perform_sync 正常完成时追加
// v1.5.5：广播 done 事件并追加同步历史记录
{
    let record = SyncRecord { status: "completed".to_string(), ... };
    let mut history = data.sync_history.write().await;
    history.push_back(record);  // ← 若 perform_sync 中途返回 Err，此处不会执行
}
```

```rust
// handlers.rs:sync_handler — 失败时无 history 记录
match perform_sync(&data).await {
    Ok(_) => HttpResponse::Ok().body("同步成功"),
    Err(e) => {
        error!("同步失败: {:?}", e);
        HttpResponse::InternalServerError().body(format!("同步失败: {}", e))
        // ← 缺失：未将 status="failed" 的 SyncRecord 追加到 sync_history
    }
}
```

Roadmap 规定 `SyncRecord.status` 应有 "completed" / "failed" 两种值，但失败路径（git pull 失败、Markdown 处理 panic 等）不会追加任何记录，`/api/sync/history` 无法反映同步失败情况，`/health` 的 `last_sync_record` 也不能正确显示最近失败。

**修复方案**：在 `sync_handler`、`webhook_sync_handler` 及定时同步任务中，对 `perform_sync` 返回 `Err` 的情况追加 `status="failed"` 的历史记录。

---

## 二、异步与并发（Async）

### 🟡 A1 - SSE 流在 "done" 事件后不关闭，连接持续占用 ✅ 已修复（本次）

**文件：** `src/handlers.rs:1032-1049`  
**严重性：** P2

```rust
// 当前实现：只有 RecvError::Closed（app 关闭）才结束流
let event_stream = stream::unfold(rx, |mut rx| async move {
    match rx.recv().await {
        Ok(event) => {
            // 无论 stage="done" 还是其他，都继续 Some(...)
            Some((Ok::<Bytes, actix_web::Error>(Bytes::from(sse_line)), rx))
        }
        Err(RecvError::Closed) => None,  // ← 只有 sender 关闭时才结束
    }
});
```

同步完成（stage="done"）后，客户端连接仍保持打开状态，等待下一次同步的事件。在长时间运行的服务中，每个访问 `/api/sync/events` 的客户端都会保持连接，直到应用重启。100 个并发客户端 = 100 个挂起的 `async move` 闭包持续等待 broadcast。

**修复方案**：将 unfold 状态改为 `(rx, finished_flag)`；收到 "done" 或 "error" 事件后发送该事件，并在下次调用时返回 `None` 关闭流。

---

### 🟡 A2 - `expand_embeds` 在持有 TokioRwLock 读锁期间递归处理 HTML

**文件：** `src/handlers.rs:497`（调用方 doc_handler）  
**严重性：** P2

```rust
// doc_handler — notes 和 link_index 的读锁在 expand_embeds 期间持续持有
let notes = data.notes.read().await;         // 读锁开始
let link_index = data.link_index.read().await;  // 读锁开始
// ...
let expanded_content = expand_embeds(&note.content_html, &notes, &link_index, 0);
// 递归展开，可能对每个内嵌笔记再次处理 HTML（最多 2 层）
// 读锁直到 notes 和 link_index 变量出作用域才释放
```

读锁本身不阻塞其他读者，但持续持有 TokioRwLock 读锁会阻塞 `/sync`（写锁）更新索引。对于包含多个 `![[]]` 内嵌的大型笔记，expand_embeds 执行时间可长达数毫秒，此间同步操作的写锁必须等待。

**缓解方案**：在持有读锁期间克隆所需数据，然后释放锁再调用 `expand_embeds`。

---

## 三、代码质量（Code Quality）

### 🟡 Q1 - `delete_user_handler` 执行软删除但使用 HTTP DELETE 语义

**文件：** `src/auth_handlers.rs:493-540`  
**严重性：** P2

```rust
// "删除"用户实际只是禁用
pub async fn delete_user_handler(...) -> impl Responder {
    // ...
    let mut user = db.get_user(&t_clone2)?.ok_or_else(...)?;
    user.enabled = false;  // ← 不是真正删除
    db.update_user(&user)?;
    // 响应：{"message": "用户已禁用"}
}
```

HTTP DELETE 语义通常意味着资源被永久移除。当前实现只是将 `enabled = false`，数据仍保留在 auth.db。前端文档应明确说明这是"禁用"而非"删除"，以避免用户误解（如期望被删用户可以重新创建）。

**建议**：要么将路由改为 `POST /api/admin/users/{u}/disable`，要么真正删除用户并添加重新创建的文档说明。接受为已知设计选择。

---

### 🔵 Q2 - `SearchHistoryEntry::db_key()` 使用纳秒时间戳，并发请求可能产生 key 冲突

**文件：** `src/reading_progress_db.rs`（`SearchHistoryEntry::db_key`）  
**严重性：** P3

```rust
pub fn db_key(&self) -> String {
    let timestamp = self.searched_at.duration_since(...).as_nanos();
    format!("{}:{}", self.username, timestamp)
    // 同一用户在同一纳秒并发发送两条搜索请求 → key 相同 → 后一条覆盖前一条
}
```

实际冲突概率极低（需要同一用户在 < 1ns 内发送两条请求），但与 `ReadingHistory::db_key()` 相同模式对比，后者附加了 `note_path` 作为额外区分维度。可考虑添加随机后缀或 UUID 作为 tiebreaker。

---

### 🔵 Q3 - `background_tasks.lock()` 使用 `if let Ok` 静默忽略锁中毒

**文件：** `src/sync.rs`（background_tasks push 段）  
**严重性：** P3

```rust
if let Ok(mut tasks) = data.background_tasks.lock() {
    tasks.retain(|h| !h.is_finished());
    tasks.push(h);
}
// 锁中毒时：句柄丢失，优雅关闭不会等待该任务
```

锁中毒概率极低，但若发生，被忽略的 JoinHandle 不会在关闭时等待，可能导致 Tantivy 写操作被强制中断（数据损坏风险低，Tantivy 有 WAL，但不推荐）。

---

## 四、安全性（Security）

### ⚪ S1 - 旧 JWT Token `role` 字段缺失导致降级为 viewer（文档说明）

**文件：** `src/auth.rs:20-25`  
**严重性：** Info

```rust
#[serde(default = "default_role")]
pub role: String,

fn default_role() -> String { "viewer".to_string() }
```

v1.5.3 前签发的 JWT Token 不含 `role` 字段，反序列化时默认 `viewer`。用户下次登录前的所有请求将以 viewer 权限执行（即使实际是 admin），可能导致 `/sync`、`/api/config/reload` 返回 403。

这是正确的安全降级行为（未知角色给最低权限），但可能导致已登录用户突然无法触发同步。升级至 v1.5.3+ 后建议提示管理员重新登录。

---

### ⚪ S2 - 信息性：分享密码表单 HTML 插入请求路径 token（延续 CODEREVIEW_1.4 S2）

**文件：** `src/share_handlers.rs`  
**严重性：** Info — 不可利用，与前次报告一致，沿袭接受

---

## 修复状态汇总

### v1.5.0–v1.5.1 遗留问题（已修复）

| 编号 | 问题 | 级别 | 状态 |
|------|------|------|------|
| B1 (v1.5.1) | bcrypt verify panic 静默为"密码错误" | 🟠 P1 | ✅ v1.5.1 |
| B2 (v1.5.1) | ShareLink::new() bcrypt 在 async 上下文 | 🟠 P1 | ✅ v1.5.1 |
| B3 (v1.5.1) | save_progress TOCTOU | 🟡 P2 | ⏸ 设计性限制 |
| A1 (v1.5.1) | fire-and-forget JoinHandle | 🟡 P2 | ⏸ 设计性限制 |
| Q1 (v1.5.1) | RwLock unwrap() | 🔵 P3 | 🔜 → v1.6.0 |
| Q2 (v1.5.1) | public_base_url 无格式验证 | 🟡 P2 | 🔜 → v1.6.0 |

### v1.5.2–v1.5.5 新增问题

| 编号 | 问题 | 级别 | 状态 | 修复版本 |
|------|------|------|------|---------|
| B1 | sync_history 只记录成功，失败未追加 | 🟠 P1 | ✅ 已修复（本次） | v1.5.6 |
| A1 | SSE 流在 done 事件后不关闭 | 🟡 P2 | ✅ 已修复（本次） | v1.5.6 |
| A2 | expand_embeds 持锁递归处理 | 🟡 P2 | 🔜 → v1.6.0 | — |
| Q1 | delete_user 软删除与 HTTP DELETE 语义不符 | 🟡 P2 | ⏸ 设计性限制，已知接受 | — |
| Q2 | SearchHistoryEntry key 纳秒冲突 | 🔵 P3 | 🔜 → v1.6.0 | — |
| Q3 | background_tasks 锁中毒静默忽略 | 🔵 P3 | 🔜 → v1.6.0 | — |
| S1 | 旧 JWT 降级为 viewer（文档说明） | ⚪ Info | ⏸ 正确安全行为 | — |

**修复统计（本次审计 v1.5.6）**：  
已修复 **2 项**（B1/A1） / 推迟 **3 项** / 接受为设计限制 **2 项**

---

## 进入 v1.6.0 前必须解决的问题

1. **B1** ✅ — sync history 完整性：已修复，失败同步现在正确记录

---

## 附：CODEREVIEW_1.4 历史问题状态更新

全部 8 项已在 v1.5.0 修复，状态不变。详见 CODEREVIEW_1.4.md。
