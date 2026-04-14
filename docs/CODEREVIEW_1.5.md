# obsidian_mirror v1.5.x 代码审查报告

**审查日期：** 2026-04-14  
**审查版本：** v1.5.0（commit `8a74135`）  
**审查范围：** `src/` 全部 27 个 `.rs` 文件  
**严重级别：** 🔴 P0 / 🟠 P1 / 🟡 P2 / 🔵 P3 / ⚪ Info  
**状态标记：** ✅ 已修复（本次）/ 🔜 → 版本 / ⏸ 设计性限制，已知接受

---

## 总体评价

**结论：v1.5.0 架构加固效果显著，redb spawn_blocking 覆盖基本完整；但 A1 修复存在两处遗漏——bcrypt verify panic 静默为"密码错误"，以及分享链接创建时 bcrypt hash 仍在 async 上下文直接调用，需本次修复。**

| 维度 | 评分 | 说明 |
|------|------|------|
| 架构设计   | ★★★★☆ | `RwLock<AppConfig>` 设计正确；所有 config 读取均未持锁跨 await |
| 异步正确性 | ★★★★☆ | auth/share/progress 的 redb IO 已全面异步化；bcrypt 有两处遗漏（B1/B2 修复后升至 ★★★★★） |
| 安全性     | ★★★★☆ | 与 v1.4.x 审计无新增安全问题；路径遍历、XSS、JWT 均已稳定 |
| 错误处理   | ★★★★☆ | spawn_blocking JoinError 大部分已处理；B1 修复后全面覆盖 |
| 测试覆盖   | ★★★☆☆ | 新增 T3 测试；spawn_blocking 路径和 bcrypt 路径无专项测试 |
| 代码质量   | ★★★★☆ | config RwLock 使用规范；Q4 遗留密码表单 token 问题仍在 |

---

## 一、安全问题（Security）

### ⚪ S1 - 信息性：分享密码表单 HTML 插入请求路径 token

**文件：** `src/share_handlers.rs:330`  
**严重性：** Info（不可利用）

```rust
// token 来自请求路径，非 DB 验证值
let password_form = format!(r#"...fetch('/share/{}', ...)..."#, token);
```

此分支仅在 `get_share(&token)` 成功（token 匹配 DB 中的 UUID）后才进入，实际不存在 XSS 注入风险。沿袭 CODEREVIEW_1.4 S2 观察：防御性最佳实践应使用 `share_link.token`（DB 验证值）。

---

## 二、Bug / 正确性（Correctness）

### 🟠 B1 - bcrypt verify spawn_blocking panic 被静默为"密码错误" ✅ 已修复（本次）

**文件：** `src/auth_handlers.rs:93`，`src/auth_handlers.rs:230`  
**严重性：** P1

```rust
// login_handler:93 — spawn_blocking panic → Ok(false) → 登录失败但无 500 无日志
let is_valid = tokio::task::spawn_blocking(move || {
    crate::auth::PasswordManager::verify_password(&pwd, &hash)
})
.await
.unwrap_or(Ok(false));  // ← JoinError 被静默为"密码错误"

// change_password_handler:230 — 同样模式
let verify_result = ...spawn_blocking(...).await.unwrap_or(Ok(false));
```

当 `spawn_blocking` 内的 bcrypt verify 线程 panic 时，`JoinHandle::await` 返回 `Err(JoinError)`。`.unwrap_or(Ok(false))` 将此错误转为 `Ok(false)`，用户看到"用户名或密码错误"，而非 500，且**无任何错误日志**。其他所有 `spawn_blocking` 调用均使用 `unwrap_or_else(|e| Err(...))` 模式，唯此两处例外。

**修复方案**：改用 `unwrap_or_else(|e| { error!(...); Err(anyhow!(...)) })`。

---

### 🟠 B2 - `ShareLink::new()` 的 bcrypt hash 在 async 上下文直接调用 ✅ 已修复（本次）

**文件：** `src/share_handlers.rs:121`，`src/share_db.rs:69-73`  
**严重性：** P1

```rust
// create_share_handler:121 — async context 直接调用 ShareLink::new()
let share_link = ShareLink::new(
    body.note_path.clone(), username.clone(),
    expires_in, body.password.clone(), body.max_visits,
);

// share_db.rs:69-73 — ShareLink::new 内部调用 bcrypt::hash（~100-300ms CPU）
let password_hash = password.map(|p| {
    bcrypt::hash(p, PASSWORD_BCRYPT_COST)
        .expect("bcrypt 哈希失败（不应发生）")  // ← 可 panic + 阻塞 async 线程
});
```

当分享链接设有密码时，`bcrypt::hash` 在 Tokio worker 线程上运行 100-300ms，阻塞其他 async 任务。A1 修复将 `db.create_share()` 移入 spawn_blocking，但未包含 `ShareLink::new()` 本身。

此外 `.expect(...)` 在 bcrypt 极少情况下失败时会 panic 整个请求。

**修复方案**：将 `ShareLink::new()` 和 `db.create_share()` 合并到同一个 `spawn_blocking` 闭包中执行。

---

### 🟡 B3 - `save_progress_handler` 双步骤存在 TOCTOU 竞争 ⏸ 设计性限制，已知接受

**文件：** `src/reading_progress_handlers.rs:110-151`  
**严重性：** P2

```rust
// 步骤1：get_progress（spawn_blocking）
let mut progress = match tokio::task::spawn_blocking(move || db.get_progress(&uname, &note_path)) ...

// 步骤2：save_progress（另一个 spawn_blocking）
let db2 = Arc::clone(&app_state.reading_progress_db);
if let Err(e) = tokio::task::spawn_blocking(move || db2.save_progress(&progress_clone)) ...
```

两次 spawn_blocking 之间若同一用户并发访问，可能出现"读旧写新"覆盖丢失。但这是已有设计（阅读进度为最终一致性语义），竞争不影响数据安全，只影响进度精确度。接受为已知限制。

---

## 三、异步与并发（Async）

### 🟡 A1 - `update_share`/`update_last_login` fire-and-forget JoinHandle 丢弃

**文件：** `src/share_handlers.rs:341-347`，`src/auth_handlers.rs:103-105`  
**严重性：** P2（可接受的设计选择）

```rust
// fire-and-forget，JoinHandle 被丢弃
tokio::task::spawn_blocking(move || {
    if let Err(e) = db2.update_share(&updated_link) {
        tracing::error!("更新分享链接访问次数失败: {}", e);
    }
});
```

JoinHandle 丢弃后，Tokio 不取消任务（任务继续运行）。服务正常运行时这是"最终一致性"语义，可接受。应用关闭时若 Tokio runtime drop，正在执行的 spawn_blocking 任务会被强制终止，最后一次访问计数可能丢失（TOCTOU）。

此为已知设计限制，accept。

---

## 四、错误处理（Error Handling）

### 🔵 Q1 - `RwLock::read/write().unwrap()` 在锁中毒时会 panic

**文件：** `src/handlers.rs:284,505,885,1004,1012`，`src/sync.rs:43`，`src/auth_handlers.rs:117,165`，`src/main.rs:325`  
**严重性：** P3

`std::sync::RwLock` 锁中毒场景（持有写锁的线程 panic）在 Rust async 程序中极其罕见，但 `.unwrap()` 意味着中毒后所有后续访问均 panic。可使用 `.unwrap_or_else(|e| e.into_inner())` 模式从中毒锁恢复，或接受为设计限制（中毒概率可忽略不计）。

---

## 五、代码质量（Code Quality）

### 🟡 Q2 - `public_base_url` 无格式验证，空字符串或非 URL 值会生成无效分享 URL

**文件：** `src/config.rs:36`，`src/share_handlers.rs:146-147`  
**严重性：** P2

```rust
// 若 public_base_url = Some("") 或 Some("not-a-url")：
format!("{}/share/{}", base_url.trim_end_matches('/'), share_link.token)
// → "/share/{token}"（相对 URL）或 "not-a-url/share/{token}"（无效 URL）
```

应在应用启动时或配置加载时校验 `public_base_url` 格式（须以 `http://` 或 `https://` 开头），或在使用时 fallback 到 Host header 推断。

---

## 修复状态汇总

| 编号 | 问题 | 级别 | 状态 | 修复版本 |
|------|------|------|------|---------|
| B1 | bcrypt verify panic 静默为"密码错误" | 🟠 P1 | ✅ 已修复（本次） | v1.5.1 |
| B2 | ShareLink::new() bcrypt 在 async 上下文直接调用 | 🟠 P1 | ✅ 已修复（本次） | v1.5.1 |
| B3 | save_progress TOCTOU | 🟡 P2 | ⏸ 设计性限制 | — |
| A1 | fire-and-forget JoinHandle | 🟡 P2 | ⏸ 设计性限制 | — |
| Q1 | RwLock unwrap() | 🔵 P3 | 🔜 → v1.6.0 | — |
| Q2 | public_base_url 无格式验证 | 🟡 P2 | 🔜 → v1.5.2 | — |
| S1 | 密码表单使用请求路径 token | ⚪ Info | ⏸ 不可利用，接受 | — |

**修复统计（本次审计 v1.5.1）**：  
已修复 **2 项**（B1/B2） / 推迟 **2 项** / 接受为设计限制 **3 项**

---

## 进入 v1.6.0 前必须解决的问题

无 P0 问题。v1.5.x 系列内已修复所有 P1 问题。

---

## 附：CODEREVIEW_1.4 历史问题状态确认

| 编号 | 状态 | 说明 |
|------|------|------|
| A1（redb spawn_blocking）| ✅ 已修复（v1.5.0）| auth/share/progress 全覆盖；bcrypt 在 v1.5.1 补全 |
| B2（config_reload 热重载）| ✅ 已修复（v1.5.0）| AppConfig → RwLock，写入后触发同步 |
| B3（uptime_seconds 语义）| ✅ 已修复（v1.5.0）| start_time.elapsed() |
| E1（Rayon mutex 中毒）| ✅ 已修复（v1.5.0）| unwrap_or_else |
| E2（模板错误纯文本）| ✅ 已修复（v1.5.0）| JSON 统一 |
| Q2（URL scheme 不可靠）| ✅ 已修复（v1.5.0）| X-Forwarded-Proto + public_base_url |
| Q3（Git commit 重复）| ✅ 已修复（v1.5.0）| 统一到 GitClient |
| T3（config_reload 测试）| ✅ 已修复（v1.5.0）| 401 未认证测试 |
