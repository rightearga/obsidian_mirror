# 收尾报告 — {{version}}

## 验证结果

- cargo build：{{build_result}}
- cargo test：{{test_result}}（{{test_count}} 通过）
- cargo clippy：{{clippy_result}}

## 修改文件清单

{{changed_files}}

## 更新文档

- Cargo.toml：{{prev_version}} → {{version}}
- docs/ROADMAP.md：{{version}} 标记 ✅，详情填写
- docs/CHANGELOG.md：新增 [{{version}}] 条目
- {{other_docs}}

## git 提交

```
{{commit_hash}} {{commit_message}}
```
