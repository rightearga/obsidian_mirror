# 审计收尾 — v{{major_version}}.x

## 审计发现汇总

- 🔴 P0 Critical：{{p0_count}} 项（本次修复 {{p0_fixed}} 项）
- 🟠 P1 High：{{p1_count}} 项（本次修复 {{p1_fixed}} 项，推迟 {{p1_deferred}} 项）
- 🟡 P2 Medium：{{p2_count}} 项
- 🔵 P3 Low：{{p3_count}} 项
- ⚪ Info：{{info_count}} 项

## 历史文件更新

{{history_updates}}

## 验证结果

- cargo build：{{build_result}}
- cargo test：{{test_result}}（{{test_count}} 通过）
- cargo clippy：{{clippy_result}}

## 文档更新

- docs/CODEREVIEW_{{major_version}}.md：新建，{{total_count}} 项问题
- docs/CHANGELOG.md：新增 [v{{new_version}}] 条目
- docs/ROADMAP.md：v{{new_version}} 标记 ✅
- README.md：{{readme_updates}}
- CLAUDE.md：{{claude_updates}}
- .claude/project.md：{{project_updates}}

## 版本变更

{{prev_version}} → {{new_version}}

## git 提交

```
{{commit_hash}} {{commit_message}}
```
