# obsidian_mirror {{major_version}}.x 代码审查报告

**审查日期：** {{date}}
**审查版本：** v{{version}}（commit `{{commit_hash}}`）
**审查范围：** `src/` 全部 {{file_count}} 个 `.rs` 文件
**严重级别：** 🔴 P0 Critical / 🟠 P1 High / 🟡 P2 Medium / 🔵 P3 Low / ⚪ Info
**状态标记：** ✅ 已修复（本次）/ 🔜 → 版本 / ⏸ 设计性限制，已知接受

---

## 总体评价

**结论：{{one_line_verdict}}**

{{version_context}}

| 维度 | 评分 | 说明 |
|------|------|------|
| 架构设计   | {{arch_score}} | {{arch_note}} |
| 异步正确性 | {{async_score}} | {{async_note}} |
| 安全性     | {{security_score}} | {{security_note}} |
| 错误处理   | {{error_score}} | {{error_note}} |
| 测试覆盖   | {{test_score}} | {{test_note}} |
| 代码质量   | {{quality_score}} | {{quality_note}} |

---

## 一、安全问题（Security）

### 🔴 S1 - {{s1_title}} {{s1_status}}

**文件：** `{{s1_file}}`
**严重性：** {{s1_level}}

```rust
{{s1_code}}
```

{{s1_description}}

**修复建议：**
```rust
{{s1_fix}}
```

---

## 二、Bug / 正确性（Correctness）

### 🔴 B1 - {{b1_title}} {{b1_status}}

**文件：** `{{b1_file}}`
**严重性：** {{b1_level}}

```rust
{{b1_code}}
```

{{b1_description}}

**修复建议：** {{b1_suggestion}}

---

## 三、异步与并发（Async）

### {{async1_level}} A1 - {{async1_title}} {{async1_status}}

**文件：** `{{async1_file}}`

{{async1_description}}

**修复建议：** {{async1_suggestion}}

---

## 四、错误处理（Error Handling）

### {{eh1_level}} E1 - {{eh1_title}} {{eh1_status}}

**文件：** `{{eh1_file}}`

{{eh1_description}}

---

## 五、性能（Performance）

### {{p1_level}} P1 - {{p1_title}} {{p1_status}}

**文件：** `{{p1_file}}`

{{p1_description}}

---

## 六、测试覆盖（Testing）

### {{t1_level}} T1 - {{t1_title}} {{t1_status}}

{{t1_description}}

---

## 七、代码质量（Code Quality）

### {{q1_level}} Q1 - {{q1_title}} {{q1_status}}

**文件：** `{{q1_file}}`

{{q1_description}}

---

## 修复状态汇总

| 编号 | 问题 | 级别 | 状态 | 修复版本 |
|------|------|------|------|---------|
| S1 | {{s1_title}} | 🔴 P0 | {{s1_status}} | v{{version}} |
| B1 | {{b1_title}} | 🔴 P0 | {{b1_status}} | v{{version}} |

**修复统计（本次审计）**：已修复 **N 项** / 推迟 **N 项** / 接受为设计限制 **N 项**

---

## 进入下一大版本前必须解决的问题

> 以下问题必须在 v{{major_version}}.x 系列内解决，不得带入 v{{next_major_version}}.0。

1. **S1** {{s1_status}} — {{s1_title}}：{{s1_must_fix_note}}
