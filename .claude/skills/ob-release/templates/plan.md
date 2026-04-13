# obsidian_mirror 版本开发规划

## 目标版本
{{version}}

## 定位文件
- Roadmap：docs/ROADMAP.md
- Changelog：docs/CHANGELOG.md
- 版本源：Cargo.toml [package].version（当前：{{current_version}}）
- CODEREVIEW 引用：{{codereview_ref}}

## Roadmap 摘要
{{roadmap_summary}}

## 初始 TODO（按八层功能修改顺序）

{{#if has_config}}
【层 1 config】
- [ ] src/config.rs：{{config_todo}}
- [ ] config.example.ron：补充示例和注释
{{/if}}

{{#if has_error}}
【层 2 error】
- [ ] src/error.rs：{{error_todo}}
{{/if}}

{{#if has_db}}
【层 3 数据层】
- [ ] src/{{feature}}_db.rs：{{db_todo}}
{{/if}}

{{#if has_core}}
【层 4 核心逻辑】
- [ ] src/{{feature}}.rs：{{core_todo}}
{{/if}}

{{#if has_handlers}}
【层 5 处理器】
- [ ] src/{{feature}}_handlers.rs：{{handlers_todo}}
{{/if}}

{{#if has_main}}
【层 6 注册】
- [ ] src/main.rs：{{main_todo}}
{{/if}}

{{#if has_state}}
【层 7 状态】
- [ ] src/state.rs：{{state_todo}}
{{/if}}

{{#if has_templates}}
【层 8 模板】
- [ ] templates/{{template_name}}.html：继承 layout.html
- [ ] src/templates.rs：新模板结构体
{{/if}}

【测试与交付检查清单】
- [ ] 每层：cargo build（零 error，零 warning）
- [ ] 每层：cargo test（全量通过）
- [ ] 新公开 API 均有中文注释

【收尾】
- [ ] cargo build — 零 error，零 warning
- [ ] cargo test — 全量通过
- [ ] cargo clippy — 零 warning
- [ ] docs/CHANGELOG.md 新增 [{{version}}] 条目
- [ ] docs/ROADMAP.md 标记 ✅，填写详情（发布日期、交付物、测试结果）
- [ ] Cargo.toml 版本号 → {{version}}
- [ ] README.md / CLAUDE.md / .claude/project.md 同步（如有 API / 配置变化）
- [ ] git 提交

## 风险 / 待确认
{{risks_or_questions}}

## 下一步行动
{{next_action}}
