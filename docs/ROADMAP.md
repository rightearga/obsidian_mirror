# Obsidian Mirror 开发路线图

> 本文档规划 Obsidian Mirror 的功能演进和版本计划

**当前版本**: v1.4.6 🎉  
**最后更新**: 2026-04-13

---

## 📋 目录

- [版本规划](#版本规划)
- [功能分类](#功能分类)
- [长期愿景](#长期愿景)
- [贡献指南](#贡献指南)

---

## 🎯 版本规划

### 🎉 已实现功能摘要

截至 v1.3.0，Obsidian Mirror 已经实现以下核心功能：

#### 📝 内容处理
- ✅ Markdown 渲染（基于 pulldown-cmark）
- ✅ WikiLinks 支持 `[[笔记]]` 和 `[[笔记|别名]]`
- ✅ 图片和附件支持
- ✅ Frontmatter 元数据解析
- ✅ 代码高亮显示

#### 🔍 搜索与索引
- ✅ 全文搜索（Tantivy 引擎）
- ✅ 中文分词支持（jieba-rs）
- ✅ 搜索结果高亮
- ✅ 搜索历史记录
- ✅ 快捷键支持（Ctrl+K / Cmd+K）
- ✅ 高级搜索过滤（标签、文件夹、日期范围）
- ✅ 搜索结果排序（相关度/修改时间）

#### 🏷️ 标签系统
- ✅ Frontmatter 标签提取
- ✅ 内联标签 `#标签` 识别
- ✅ 标签云可视化
- ✅ 按标签过滤笔记

#### 🗺️ 导航与可视化
- ✅ 侧边栏文件树导航
- ✅ 反向链接显示
- ✅ 面包屑导航
- ✅ 笔记目录（TOC）自动生成
- ✅ 关系图谱（Vis.js，支持 1-3 层深度）

#### 🎨 用户体验
- ✅ 响应式设计（移动端适配）
- ✅ 深色/浅色主题切换
- ✅ 多语言支持（中文/English）
- ✅ 滚动位置记忆
- ✅ 平滑滚动动画
- ✅ 键盘导航支持
- ✅ 最近访问笔记（本地存储）
- ✅ 收藏夹功能（星标收藏）
- ✅ 笔记统计面板（总数、标签、最近更新）
- ✅ 笔记预览（悬浮卡片，前 200 字预览）
- ✅ 设置面板（语言、字体、主题统一管理）
- ✅ 内容自适应布局（宽度自动适配）
- ✅ 移动端 TOC 右侧滑入侧边栏
- ✅ 桌面端 TOC 可收起/展开

#### ⚙️ 系统功能
- ✅ Git 自动同步
- ✅ 增量同步（Git diff 检测）
- ✅ 索引持久化（postcard + redb）
- ✅ 忽略模式配置
- ✅ 高性能并发处理（Actix-web）
- ✅ 后台异步索引构建
- ✅ 健康检查和指标暴露
- ✅ 日志管理和优雅关闭
- ✅ 分享链接生成（带过期时间）
- ✅ 阅读进度跟踪（自动保存和恢复）

---

### ✅ v0.7.1 (已发布 - 2026-01-29)

**主题**: 用户体验优化

- ✅ 界面文本全面中文化
- ✅ 移动端和桌面端侧边栏滚动位置记忆
- ✅ 修复移动端侧边栏交互问题
- ✅ 修复侧边栏多行文本对齐和缩进问题

---

### ✅ v0.8.0 (已发布 - 2026-01-29)

**主题**: 搜索功能 & 标签支持

#### 核心功能

- ✅ **全文搜索**
  - 使用 Tantivy 搜索引擎实现高性能全文索引
  - 在侧边栏添加搜索框
  - 支持标题和内容搜索（中文分词支持）
  - 实时搜索结果预览（300ms 防抖）
  - 高亮显示搜索关键词
  - 快捷键支持 (Ctrl+K / Cmd+K)
  - 搜索历史记录
  - 搜索结果排序（相关度优先）

- ✅ **标签系统**
  - 从 Frontmatter 中提取标签
  - 支持 `#标签` 语法识别
  - 标签页面展示所有标签（`/tags`）
  - 点击标签查看相关笔记列表
  - 标签云可视化（按使用频率显示大小）

- ✅ **性能优化**
  - 搜索索引优化（Tantivy 引擎）
  - 大型笔记库（5000+ 文件）加载优化
  - 后台异步索引构建，不阻塞主线程
  - 批量提交优化（每 1000 条笔记）
  - 50MB 写入缓冲区

#### 技术改进

- ✅ 重构索引构建逻辑，提取为独立模块（`indexer.rs`）
- ✅ 模块化代码结构（`handlers.rs`, `search_engine.rs`, `tags.rs`）

---

### ✅ v0.9.0 (已发布 - 2026-01-29)

**主题**: 可视化增强

#### 核心功能

- ✅ **笔记目录 (TOC)**
  - 自动生成页内目录（从 H1-H6 标题）
  - 点击跳转到对应章节（平滑滚动）
  - 浮动 TOC 支持（右侧固定）
  - 移动端收起/展开（折叠按钮）
  - 滚动时自动高亮当前章节
  - 支持多级嵌套标题

- ✅ **关系图谱**
  - 使用 Vis.js 实现笔记关系可视化
  - 显示当前笔记的直接和间接链接
  - 交互式节点（点击跳转、拖拽移动）
  - 可配置的图谱范围（1-3 层深度选择）
  - 物理引擎动画效果
  - 当前笔记节点高亮显示
  - 箭头表示链接方向

- ✅ **面包屑导航**
  - 显示当前笔记的路径层级
  - 点击跳转到上级路径
  - 移动端友好显示（紧凑布局）
  - 路径分隔符美化

#### 次要改进

- ✅ 笔记统计信息（字数、修改时间显示）
- ✅ 前端代码模块化（独立 JS 文件）
- ✅ CSS 模块化（按功能拆分样式文件）

#### 技术准备

- ✅ 模块化 JavaScript 代码（`static/js/` 目录）
- ✅ 模块化 CSS 代码（`static/css/` 目录）
- ✅ 引入 Vis.js 图谱库

---

### ✅ v0.10.0 (已发布 - 2026-01-29)

**主题**: 关系图谱完善

- ✅ 完善关系图谱交互体验
- ✅ 优化图谱性能和渲染效果
- ✅ 修复边缘情况和错误处理

---

### ✅ v1.0.0 (已发布 - 2026-01-31) 🎉

**主题**: 生产就绪

#### 核心功能

- ✅ **增量同步优化**
  - Git diff 检测变更文件
  - 仅处理修改/新增/删除的文件
  - 智能同步模式（首次克隆/增量更新/无变更）
  - 性能提升 10-100 倍

- ✅ **索引持久化**
  - 使用 postcard + redb 持久化笔记索引
  - Git 提交严格校验
  - 版本兼容性检查
  - 重启恢复速度提升 30-120 倍
  - 修复 Frontmatter 序列化问题

- ✅ **用户认证**
  - JWT 令牌认证
  - bcrypt 密码加密
  - 修改密码功能
  - 用户菜单界面
  - 登录/登出页面

#### 界面优化

- ✅ **侧边栏增强**
  - 默认宽度增加到 320px
  - 可拖动调整大小（200-600px）
  - 宽度记忆功能
  - 移除水平滚动条
  
- ✅ **滚动条样式统一**
  - 侧边栏、正文、目录滚动条统一样式
  - 使用 CSS 变量自动适配 Dark 主题

#### 运维功能

- ✅ **健康检查端点** (`/health`)
  - 返回应用状态、版本、笔记数、运行时长
  - 适配 Docker/Kubernetes

- ✅ **指标暴露** (Prometheus)
  - `/metrics` 端点
  - HTTP 请求计数、延迟直方图
  - 笔记数量、同步操作计数

- ✅ **日志管理**
  - 分级输出到文件（./logs/app.log, ./logs/error.log）
  - 每日自动轮转
  - 控制台 + 文件多目标输出

- ✅ **优雅关闭处理**
  - 捕获 SIGTERM/SIGINT 信号
  - 关闭前保存持久化索引
  - 等待现有请求完成

#### 质量保证

- ✅ **单元测试**
  - 37 个单元测试覆盖核心逻辑
  - Markdown 处理：21 个测试
  - 标签提取：8 个测试
  - 认证系统：4 个测试
  - 错误处理：2 个测试

- ✅ **错误处理完善**
  - 自定义 AppError 类型（8 种错误分类）
  - 统一 Result 类型
  - 错误自动恢复机制
  - 中文错误消息

#### 技术改进

- ✅ 模块化架构（persistence、metrics、error 模块）
- ✅ 性能监控体系
- ✅ 日志追踪体系

---

### ✅ v1.1.0 (已发布 - 2026-01-31) 🎉

**主题**: 功能增强

#### 核心功能

- ✅ **最近访问笔记**
  - 记录最近访问的笔记列表（最多 10 条）
  - 侧边栏快速访问入口
  - 本地存储（localStorage）
  - 相对时间显示
  - 折叠状态记忆

- ✅ **收藏夹功能**
  - 收藏重要笔记（星标图标）
  - 收藏列表管理
  - 本地存储
  - 快速移除功能
  - Toast 提示消息

- ✅ **笔记统计面板**
  - 笔记总数统计
  - 标签数量统计
  - 最近 7 天更新笔记数
  - 3 个统计卡片展示
  - 实时数据更新

#### Bug 修复

- ✅ **标签持久化修复**
  - 修复标签索引持久化缺失
  - 标签数据重启后正确保留
  - 更新持久化模块支持 tag_index

#### 界面优化

- ✅ 侧边栏新增统计、最近访问、收藏夹三个面板
- ✅ 笔记页面添加收藏按钮
- ✅ 更正标签按钮图标
- ✅ 完整的移动端响应式适配

---

### ✅ v1.2.0 (已发布 - 2026-01-31) 🎉

**主题**: 体验优化

#### 核心功能

- ✅ **多语言支持**
  - 中文/English 切换
  - 语言偏好本地存储
  - 全局文本翻译（i18n.js）
  - 自动加载上次选择的语言

- ✅ **笔记预览**
  - 悬浮卡片预览链接内容
  - 显示前 200 字预览
  - 平滑淡入淡出动画
  - 鼠标悬停触发，移开消失

- ✅ **设置面板**
  - 统一的用户偏好管理
  - 语言选择
  - 字体选择（系统默认、衬线、等宽）
  - 字体大小调整（12-24px）
  - 行高调整（1.2-2.0）
  - 重置默认功能

#### 移动端优化

- ✅ **TOC 右侧滑入**
  - 移动端 TOC 改为右侧滑入侧边栏
  - 右上角固定切换按钮（44×44px）
  - 平滑滑入/滑出动画（0.3s ease）
  - 半透明遮罩层，点击关闭
  - 点击 TOC 条目自动关闭并跳转
  - 小屏手机宽度自适应（85vw，最大 300px）

- ✅ **状态栏完善**
  - 移动端显示完整信息（字数、行数、时间）
  - 紧凑布局：字体 11px（768px）、10px（480px）
  - 图标缩小：12px（768px）、11px（480px）
  - 隐藏分隔符节省空间

#### 桌面端优化

- ✅ **TOC 收起/展开**
  - 桌面端 TOC 可一键收起/展开
  - 展开状态记忆（localStorage）
  - 平滑展开/收起动画
  - 保持拖动调整宽度功能

- ✅ **内容自适应布局**
  - 移除固定宽度限制（800px）
  - 内容区自动适配屏幕宽度
  - 响应式边距调整
  - 表格溢出处理（防止与 TOC 重叠）

#### Bug 修复

- ✅ 修复移动端 TOC 侧边栏不显示
- ✅ 修复 ignore_patterns 配置变更不生效
- ✅ 修复表格溢出导致布局问题
- ✅ 添加同步状态调试日志

#### 技术改进

- ✅ 移除设置中内容宽度选项
- ✅ 静态资源版本号管理（防止缓存）
- ✅ CSS 变量统一管理布局参数

---

### ✅ v1.3.0 (已发布 - 2026-02-01) 🎉

**主题**: 高级搜索功能

#### 核心功能

- ✅ **高级搜索过滤**
  - 按标签过滤（支持多个标签，逗号分隔，OR 逻辑）
  - 按文件夹路径精确过滤
  - 按日期范围过滤（基于修改时间）
  - 可折叠的高级过滤面板
  - 实时过滤，输入即刻生效
  - 一键清除所有过滤条件

- ✅ **搜索结果排序**
  - 按相关度排序（默认）
  - 按最新修改时间排序

- ✅ **分享链接生成**: 创建带过期时间的笔记分享链接
  - 支持自定义过期时间（1小时-30天）
  - 分享链接管理界面
  - 访问统计和历史记录
  - redb 持久化存储

- ✅ **阅读进度跟踪**: 自动记录和恢复阅读位置
  - 实时保存滚动位置
  - 刷新页面自动恢复
  - 阅读历史记录
  - redb 持久化存储

#### 后端改进

- ✅ 扩展搜索引擎支持标签和文件夹字段索引
- ✅ 新增 `advanced_search()` 方法支持多条件过滤
- ✅ 使用 Tantivy BooleanQuery 组合查询条件
- ✅ 使用 RangeQuery 实现日期范围过滤
- ✅ 扩展 SearchQuery API 参数（tags, folder, date_from, date_to）
- ✅ 更新索引构建流程包含标签和文件夹信息

#### 前端改进

- ✅ 添加高级过滤面板 UI（+128 行 CSS）
- ✅ 实现过滤交互逻辑（+122 行 JavaScript）
- ✅ 标签输入框、文件夹路径输入、日期范围选择器
- ✅ 响应式设计，支持深色/浅色主题
- ✅ 平滑展开/收起动画

#### 技术亮点

- ✅ 标签字段支持多值索引（一个笔记多个标签）
- ✅ 文件夹路径自动从完整路径提取
- ✅ Unix 时间戳精确匹配
- ✅ 保持向后兼容，简单搜索仍然可用

#### 配置清理

- ✅ 统一数据库配置到 `config.database`
- ✅ 删除 config.ron 中重复的 auth_db_path 配置

---

### ✅ v1.3.1 (已发布 - 2026-04-13)

**主题**: Bug 修复与安全加固

基于 `docs/CODEREVIEW_1.3.md` 代码审查报告，修复全部 P0/P1 级别问题。

#### 核心功能

- ✅ **[B1] 修复 `persistence::clear()` 未清理标签索引**
  - 文件：`src/persistence.rs`
  - 修复：在 `clear()` 补充清空 `TAG_INDEX_TABLE`，消除调用后标签数据残留

- ✅ **[B4] 修复增量同步时反向链接数据丢失**
  - 文件：`src/domain.rs`、`src/sync.rs`、`src/indexer.rs`、`src/persistence.rs`
  - 修复：`Note` 添加 `outgoing_links: Vec<String>` 字段存储出链；`BacklinkBuilder::build` 改为基于全量 `notes.outgoing_links` 重建，不再依赖增量 `temp_links`；`CURRENT_VERSION` 升至 2 强制缓存失效

- ✅ **[S1] 修复 XSS：标题文本未 HTML 转义**
  - 文件：`src/markdown.rs`
  - 修复：添加 `html_escape()` 函数，标题输出前对 `current_heading_text` 进行 `&`, `<`, `>`, `"` 转义

- ✅ **[B2] 修复 `/sync` 端点缺少并发保护**
  - 文件：`src/state.rs`、`src/handlers.rs`
  - 修复：`AppState` 添加 `sync_lock: tokio::sync::Mutex<()>`；`sync_handler` 使用 `try_lock()` 防并发，并发时返回 409 Conflict

#### 实际交付物

- 修改文件：`src/persistence.rs`（B1 标签清理）
- 修改文件：`src/domain.rs`（B4 出链字段）
- 修改文件：`src/sync.rs`（B4 出链存储）
- 修改文件：`src/indexer.rs`（B4 反向链接重建逻辑）
- 修改文件：`src/markdown.rs`（S1 html_escape）
- 修改文件：`src/state.rs`（B2 sync_lock 字段）
- 修改文件：`src/handlers.rs`（B2 并发保护）

#### 测试结果

- 全量测试：**52/52 通过**
- 新增测试：5 个（indexer 3 个、markdown 1 个）

---

### ✅ v1.3.2 (已发布 - 2026-04-13)

**主题**: 安全加固

修复 `CODEREVIEW_1.3.md` 中全部安全类问题（S2/S3/S4）。

#### 核心功能

- ✅ **[S2] Cookie 补充 `Secure` 和 `SameSite` 属性**
  - 文件：`src/auth_handlers.rs`
  - 修复：登录/登出 Cookie 添加 `.secure(true).same_site(SameSite::Lax)`，防止 JWT Token 在 HTTP 连接下明文传输

- ✅ **[S3] 分享链接密码改用 bcrypt 哈希存储**
  - 文件：`src/share_db.rs`、`src/share_handlers.rs`
  - 修复：`password` 字段重命名为 `password_hash`；创建分享时用 `bcrypt::hash` 哈希，验证时用 `bcrypt::verify`；`has_password` 字段同步更新

- ✅ **[S4] 认证中间件路径匹配收紧**
  - 文件：`src/auth_middleware.rs`
  - 修复：`/login`、`/api/auth/login` 改为精确匹配 `==`；`/static/`、`/share/` 保留 `starts_with`

#### 实际交付物

- 修改文件：`src/auth_handlers.rs`（S2 Cookie 安全属性）
- 修改文件：`src/share_db.rs`（S3 密码哈希化）
- 修改文件：`src/share_handlers.rs`（S3 has_password 字段）
- 修改文件：`src/auth_middleware.rs`（S4 路径精确匹配）

#### 测试结果

- 全量测试：**52/52 通过**
- 新增测试：0 个（已有 share_db 测试增强了哈希验证断言）

---

### ✅ v1.3.3 (已发布 - 2026-04-13)

**主题**: 性能深度优化（CODEREVIEW P1–P5）

解决 `CODEREVIEW_1.3.md` 中全部性能问题。

#### 核心功能

- ✅ **[P1] Regex 编译改用 `lazy_static!`**
  - 文件：`src/markdown.rs`（5个正则）、`src/tags.rs`（1个正则）
  - 修复：所有 `Regex::new(...)` 移至 `lazy_static!` 块，进程生命周期内只编译一次
  - 注：`graph.rs` 的正则通过 P5 改用 `outgoing_links` 已消除

- ✅ **[P2] Tantivy `IndexReader` 缓存复用**
  - 文件：`src/search_engine.rs`
  - 修复：`SearchEngine` 持久持有 `reader: IndexReader`，`advanced_search` 直接复用，无需每次初始化

- ✅ **[P3] 搜索索引增量更新**
  - 文件：`src/search_engine.rs`、`src/sync.rs`
  - 修复：新增 `update_documents(changed, deleted)` 方法；增量同步时只更新变更文件，不全量重建

- ✅ **[P4] reading_progress_db 前缀范围查询**
  - 文件：`src/reading_progress_db.rs`
  - 修复：`get_user_progress` / `get_user_history` 改用 redb `range()` 前缀查询，消除全表扫描
  - 注：`share_db.get_user_shares` 因键格式为纯 UUID 暂不适用范围查询，保持原设计

- ✅ **[P5] graph.rs 消除 `content_text` 解析**
  - 文件：`src/graph.rs`
  - 修复：`extract_links_from_note` 改用 `note.outgoing_links` 预计算字段，消除热路径正则解析
  - 注：P5 阶段一（移除 Note.content_text 字段）延至 v1.3.4

#### 实际交付物

- 修改文件：`src/markdown.rs`、`src/tags.rs`（P1 lazy_static）
- 修改文件：`src/search_engine.rs`（P2 reader 缓存 + P3 update_documents）
- 修改文件：`src/sync.rs`（P3 增量搜索更新）
- 修改文件：`src/reading_progress_db.rs`（P4 范围查询）
- 修改文件：`src/graph.rs`（P5 outgoing_links）

#### 测试结果

- 全量测试：**52/52 通过**
- 新增测试：0 个（性能优化无行为变化，现有测试覆盖）

---

### ✅ v1.3.4 (已发布 - 2026-04-13)

**主题**: 代码质量改进 + 测试覆盖补全（CODEREVIEW Q1–Q7）

解决 `CODEREVIEW_1.3.md` 中全部代码质量问题，并大幅提升测试覆盖率。

#### 核心功能

- ✅ **[Q1] 加强 `schema_matches` 字段类型检查**
  - 文件：`src/search_engine.rs`
  - 修复：同时比较字段名和字段类型变体（discriminant），防止 TEXT→STRING 等类型变更时复用错误 schema

- ✅ **[Q2]** — 已在 v1.3.1 完成（注释 use 声明清理）

- ✅ **[Q3] 阅读历史自动清理**
  - 文件：`src/reading_progress_db.rs`
  - 修复：`add_history()` 写入后自动调用 `cleanup_old_history(200)`，防止历史无限增长

- ✅ **[Q5] truncate_html 截断基于纯文本**
  - 文件：`src/handlers.rs`
  - 修复：状态机去除 HTML 标签后再截断，确保 500 字预览为真实可见内容

- ✅ **[Q6] 持久化分批写入**
  - 文件：`src/persistence.rs`
  - 修复：笔记按 1000 条分批提交事务；元数据最后写入（原子完成标记）

- ✅ **[Q7] metrics 指标注册去除 expect**
  - 文件：`src/metrics.rs`
  - 修复：`init_metrics()` 改用 `let _ =` 静默忽略 AlreadyReg 错误，防测试 panic

#### 测试覆盖补全

- ✅ `sync.rs`：4 个 `should_update_note` 测试
- ✅ `graph.rs`：6 个 `generate_graph` BFS 测试（深度/反向链接/孤立节点）
- ✅ `persistence.rs`：4 个往返测试（数据一致性/git hash 不匹配/patterns 变更/clear 失效）
- ✅ `indexer.rs`：已在 v1.3.1 补充（BacklinkBuilder + TagIndexBuilder 场景）

#### 实际交付物

- 修改文件：`src/search_engine.rs`（Q1）
- 修改文件：`src/reading_progress_db.rs`（Q3）
- 修改文件：`src/handlers.rs`（Q5）
- 修改文件：`src/persistence.rs`（Q6 + tests）
- 修改文件：`src/metrics.rs`（Q7）
- 修改文件：`src/sync.rs`（tests）
- 修改文件：`src/graph.rs`（tests）

#### 测试结果

- 全量测试：**66/66 通过**
- 新增测试：14 个（sync 4、graph 6、persistence 4）

---

### ✅ v1.4.0 (已发布 - 2026-04-13)

**主题**: 键盘操作 + 主题定制 + 动画细节

#### 键盘快捷键

- ✅ **页面内导航**
  - `j` / `k` — 平滑向下/上滚动半屏
  - `g g` / `G` — 跳转页面顶部/底部
  - `[` / `]` — 侧边栏中的前/后一篇笔记（按文件树顺序）
  - `b` — 返回上一篇访问过的笔记（浏览历史回退）
- ✅ **功能快捷键**
  - `t` — 切换目录（TOC）面板展开/收起
  - `g` — 打开当前笔记的关系图谱（单击 g 后 450ms 无第二下触发）
  - `?` — 显示快捷键帮助面板（可关闭）
- ✅ **快捷键帮助面板**
  - `?` 键触发，模态弹窗显示所有可用快捷键
  - 分组展示（导航 / 功能）；`Esc` 关闭；`prefers-reduced-motion` 无动画

#### 主题定制

- ✅ **预设配色方案**（在现有深色/浅色基础上叠加）
  - 暖色方案（米黄底色 + 棕色调）
  - 护眼方案（低蓝光、偏绿黄色调）
  - 高对比度方案（增强文字与背景对比度）
- ✅ **自定义主色调**
  - 颜色选择器，影响链接、活跃标签、高亮颜色
  - 偏好写入 `localStorage` 跨会话保留
- ✅ **代码块主题独立选择**
  - 可用方案：Atom One Dark / Atom One Light / GitHub Light / GitHub Dark / Dracula / Monokai
  - 深色模式默认 Atom One Dark，浅色模式默认 Atom One Light（auto 模式）
  - 设置面板统一管理

#### 交互动画

- ✅ **页面切换过渡**
  - 笔记切换时内容区淡入（150ms ease-out）
  - 设置面板可关闭（`data-animations="off"`）；`prefers-reduced-motion` 自动禁用
- ✅ **侧边栏动画优化**
  - 文件夹图标旋转弹性过渡（cubic-bezier 180ms）
  - 搜索结果列表项依次错峰进入（stagger 20ms/条，最多 8 条）
- ✅ **TOC 高亮平滑过渡**
  - 活跃链接颜色/背景使用 CSS transition（150ms ease）

#### 实际交付物

- 新增文件：`static/js/keyboard.js`（键盘快捷键 + 帮助面板逻辑）
- 新增文件：`static/css/keyboard.css`（帮助面板样式）
- 新增文件：`static/css/themes.css`（主题预设 CSS 变量）
- 新增文件：`static/css/animations.css`（淡入 / 错峰 / 旋转动画）
- 修改文件：`static/js/theme.js`（预设 / 强调色 / 代码主题支持）
- 修改文件：`static/js/settings.js`（设置面板扩展：主题预设 / 强调色 / 代码主题 / 动画开关）
- 修改文件：`static/js/search.js`（搜索结果错峰动画延迟注入）
- 修改文件：`static/css/settings.css`（toggle-switch 样式）
- 修改文件：`templates/layout.html`（引入新 CSS/JS、代码主题 CDN 链接）

#### 测试结果

- 全量测试：**66/66 通过**（纯前端变更，后端无改动）
- 新增测试：0 个（前端 JS/CSS 无对应单元测试）

---

### ✅ v1.4.1 (已发布 - 2026-04-13)

**主题**: Obsidian 语法扩展支持

补全 Obsidian 常用的富文本语法，让镜像站内容与客户端高度一致。

#### 数学公式

- ✅ **KaTeX 渲染**（引入前端 KaTeX 库，无需后端依赖）
  - 行内公式：`$E = mc^2$` → 渲染为内联数学公式
  - 块级公式：`$$...$$` → 居中块级显示
  - 后端：`markdown.rs` 将公式包装为 `<span class="math-inline" data-math="...">` / `<div class="math-block" data-math="...">`
  - 前端：`katex-init.js` 读取 `data-math` 属性，KaTeX CDN 渲染
  - 渲染失败时显示原始 LaTeX（优雅降级）

#### Obsidian 标注语法

- ✅ **Callout 块**（`> [!TYPE] Title`）
  - 支持类型：`NOTE` `TIP` `WARNING` `DANGER` `INFO` `SUCCESS` `QUESTION` `QUOTE` `BUG` `TODO` 等 20+ 类型
  - 可折叠：`> [!NOTE]-` 默认收起，`> [!NOTE]+` 默认展开
  - `callout.js` 前端解析，`callout.css` 深/浅色各自配色
- ✅ **高亮语法**
  - `==高亮文本==` → `<mark>高亮文本</mark>`
  - `math.css` 提供深/浅色模式颜色自适应样式

#### 图片体验

- ✅ **灯箱效果**
  - `lightbox.js` 点击图片弹出全屏模态层，`lightbox.css` 毛玻璃遮罩
  - 键盘导航：`←` `→` 切换，`Esc` 关闭；显示 alt 文本说明和图片计数
- ✅ **图片懒加载**
  - `markdown.rs` 后处理为所有 `<img>` 添加 `loading="lazy"`

#### Mermaid 图表扩展

- ✅ **更多图表类型** — `mermaid-init.js` 已配置 sequence/gantt/class/state/er（在之前版本实现）
- ✅ **Mermaid 主题联动** — `theme.js` 已调用 `MermaidManager.switchTheme()`（在之前版本实现）

#### 实际交付物

- 修改文件：`src/markdown.rs`（数学公式包装 + ==高亮== + img lazy + 4个新测试）
- 新增文件：`static/js/katex-init.js`（KaTeX 渲染）
- 新增文件：`static/js/callout.js`（Callout 块解析）
- 新增文件：`static/js/lightbox.js`（图片灯箱）
- 新增文件：`static/css/math.css`（数学公式 + 高亮样式）
- 新增文件：`static/css/callout.css`（Callout 配色）
- 新增文件：`static/css/lightbox.css`（灯箱样式）
- 修改文件：`templates/layout.html`（引入 KaTeX CDN + 新文件）

#### 测试结果

- 全量测试：**70/70 通过**（前 66 + 新增 4）
- 新增测试：4 个（`test_math_block`/`test_math_inline`/`test_highlight_syntax`/`test_image_lazy_loading`）

---

### ✅ v1.4.2 (已发布 - 2026-04-13)

**主题**: 搜索体验升级 + 笔记发现机制

#### 搜索增强

- ✅ **实时搜索自动补全**：`GET /api/titles` 返回标题+标签；`search.js` 前端缓存+前缀过滤，`#` 触发标签补全，sessionStorage TTL 5min
- ✅ **搜索历史管理**：最大条数 10→20，每条历史悬停显示 `×` 单条删除；清空全部按钮保留
- 搜索结果路径+标签+时间显示：延期（搜索 API 返回数据已含，前端展示待 v1.4.3+ 完善）

#### 笔记发现

- ✅ **孤立笔记页面** (`GET /orphans`)：`orphans_handler` + `orphans.html` 模板；`outgoing_links` 为空且无入链
- ✅ **随机笔记** (`GET /random`)：`random_handler` 重定向；侧边栏按钮；快捷键 `r`
- ✅ **最近更新页面** (`GET /recent`)：`recent_page_handler`；`?days=7/30/90`；侧边栏入口

#### 实际交付物

- 修改文件：`src/handlers.rs`（4 个新处理器）
- 修改文件：`src/templates.rs`（OrphansTemplate + RecentNotesPageTemplate）
- 新增文件：`templates/orphans.html`、`templates/recent_notes_page.html`
- 修改文件：`src/main.rs`（路由注册）
- 修改文件：`static/js/search.js`（自动补全 + 历史优化）
- 修改文件：`static/js/keyboard.js`（`r` 快捷键）
- 修改文件：`templates/layout.html`（侧边栏发现面板）
- 修改文件：`static/css/sidebar.css`（discovery-panel 样式）
- 修改文件：`static/css/search.css`（建议/历史删除/列表页样式）

#### 测试结果

- 全量测试：**70/70 通过**（纯前端+路由变更，后端逻辑无新测试需求）

---

### ✅ v1.4.3 (已发布 - 2026-04-13)

**主题**: 全库图谱 + 视觉分组 + 布局多样化

#### 全库图谱

- ✅ **全局图谱视图** (`GET /api/graph/global`)
  - 展示整个笔记库的链接关系；节点 >500 时自动降采样
  - 侧边栏"发现"面板全局图谱按钮；可从任意页面打开
- ✅ **图谱筛选面板**
  - 标签筛选、文件夹筛选、隐藏孤立节点开关、重置

#### 节点视觉增强

- ✅ **节点大小映射**：节点大小 = 10 + min(入度×3, 25)，孤立节点最小尺寸
- ✅ **节点颜色分组**：按第一个标签分配 12 色调色板；右侧图例面板
- 无标签节点使用默认灰色

#### 布局与交互

- ✅ **布局模式切换**：力导向（默认）/ 层级布局（vis.js hierarchical）
- ⚠ **放射布局**：vis.js 无原生支持，延至后续版本（考虑自定义实现）
- ✅ **图谱内搜索**：输入框定位并高亮匹配节点，视口动画移动
- ✅ **图谱导出**：Canvas toDataURL 导出 PNG

#### 实际交付物

- 修改文件：`src/domain.rs`（`GraphNode.tags` 字段）
- 修改文件：`src/graph.rs`（`generate_global_graph` + tags 支持 + 5 个新测试）
- 修改文件：`src/handlers.rs`（`global_graph_handler`）
- 修改文件：`src/main.rs`（路由注册）
- 修改文件：`static/js/graph.js`（全面重写，600+ 行）
- 修改文件：`static/css/graph.css`（模式切换/筛选/图例/搜索样式）
- 修改文件：`templates/page.html`（图谱弹窗移至 layout.html）
- 修改文件：`templates/layout.html`（通用图谱弹窗 + 全局图谱按钮）

#### 测试结果

- 全量测试：**74/74 通过**（+4 graph 测试）

---

### ✅ v1.4.4 (已发布 - 2026-04-13)

**主题**: PWA + 触屏手势 + 无障碍

#### PWA 支持

- ✅ **Service Worker + Web App Manifest**（`manifest.json` + `sw.js`）
  - 可安装为桌面/移动端 App；静态资源缓存优先；离线降级页面
- ✅ **App 图标 & 主题色**：manifest 含 SVG 图标；`meta[name=theme-color]` 随深/浅色切换

#### 移动端手势

- ✅ **侧边栏手势**：左边缘（<30px）右滑 60px+ 打开，侧边栏内左滑关闭
- ✅ **笔记翻页手势**：内容区边界后继续滑动 100px+ 翻页，水平偏移 >60px 取消

#### 无障碍改进

- ✅ **跳过导航链接**：Skip Link `<a href="#main-content">`，键盘 Tab 聚焦时浮现
- ✅ **ARIA 语义标注**：侧边栏树 `role="tree"`；搜索结果 `role="listbox"` + `aria-live`
- ✅ **系统减少动画偏好**：`accessibility.css` 补全 prefers-reduced-motion 覆盖
- ✅ **全局焦点样式**：`:focus-visible` 统一 outline；高对比度媒体查询适配

#### 实际交付物

- 新增文件：`static/manifest.json`（PWA 清单）
- 新增文件：`static/sw.js`（Service Worker）
- 新增文件：`static/js/pwa.js`（SW 注册 + 更新提示）
- 新增文件：`static/js/gestures.js`（触屏手势）
- 新增文件：`static/css/accessibility.css`（无障碍 + focus 样式）
- 修改文件：`static/js/theme.js`（theme-color meta 同步）
- 修改文件：`templates/layout.html`（manifest/meta/skip link/ARIA/新 JS）

#### 测试结果

- 全量测试：**74/74 通过**（纯前端变更，后端无改动）

---

### ✅ v1.4.5 (已发布 - 2026-04-13)

**主题**: 自动同步 + Webhook + 配置增强

#### 自动同步

- ✅ **定时自动同步**：`sync_interval_minutes` 配置；Tokio 定时任务遵守 `sync_lock` 互斥
- ✅ **Webhook 触发同步**（`POST /webhook/sync`）：GitHub HMAC 签名验证 + GitLab 令牌验证；`webhook.enabled/secret` 配置

#### 配置扩展

- ✅ **配置热重载**（`POST /api/config/reload`）：重新读取 config.ron 后触发同步；注 listen_addr/repo_url 需重启
- ✅ **ignore_patterns Glob 语法**：`scanner.rs` 支持 `*`/`**`/`?`（5 个新测试）

#### 监控增强

- ✅ **Prometheus 指标扩展**：`sync_duration_seconds` 直方图 + `sync_last_timestamp_seconds` gauge
- ⚠ **`notes_total_by_tag`**：需 GaugeVec（带标签），延至后续版本
- ✅ **`/health` 端点扩展**：新增 `git_commit`、`sync_status`、`last_sync_at`、`last_sync_duration_ms`

#### 实际交付物

- 修改文件：`src/config.rs`（sync_interval_minutes + WebhookConfig）
- 修改文件：`src/state.rs`（last_sync_at/sync_status/last_sync_duration_ms 原子字段）
- 修改文件：`src/metrics.rs`（sync_duration_seconds + sync_last_timestamp_seconds）
- 修改文件：`src/sync.rs`（记录同步状态和指标）
- 修改文件：`src/scanner.rs`（glob 支持 + 5 个测试）
- 修改文件：`src/handlers.rs`（health 扩展 + webhook + config reload）
- 修改文件：`src/main.rs`（定时任务 + 路由注册）
- 修改文件：`config.example.ron`（新配置项说明）

#### 测试结果

- 全量测试：**79/79 通过**（+5 scanner glob 测试）

---

### ✅ v1.4.6 (已发布 - 2026-04-13)

**主题**: Cookie 动态 Secure + Webhook HMAC + 调试日志清理

#### 安全修复

- ✅ **Cookie Secure 动态判断**
  - 文件：`src/auth_handlers.rs`、`src/config.rs`
  - 问题：v1.3.2 硬编码 `.secure(true)`，HTTP 下（内网/开发）浏览器静默丢弃 Cookie，导致登录失效
  - 修复：`config.ron` 新增 `force_https_cookie: bool`（默认 `false`）；仅在该选项为 `true` 时设置 `Secure` 标志
  - 说明：生产环境通过反向代理（Nginx/Caddy）启用 HTTPS 时，手动将该选项设为 `true`

- ✅ **Webhook HMAC-SHA256 真实实现**
  - 文件：`src/handlers.rs`、`Cargo.toml`
  - 问题：当前 GitHub 签名验证为字符串直接比较，无消息体完整性保证
  - 修复：添加 `hmac = "0.12"` + `sha2 = "0.3"` 依赖，实现标准 HMAC-SHA256 验证；GitLab 令牌保持原有逻辑

#### 代码清理

- ✅ **graph.js / mermaid-init.js 调试日志清理**
  - 文件：`static/js/graph.js`
  - 问题：graph.js 中遗留数十条 `console.log` 调试输出，影响性能和控制台可读性
  - 修复：删除所有非 error 级别的 `console.log`，仅保留 `console.error` 用于异常处理

- ✅ **sync.rs / 其他文件潜在警告确认修复**
  - 编译通过，无新增警告

---

### ✨ v1.4.7 (计划中 - 搜索与性能优化)

**主题**: 重启不重建搜索索引 + share_db 前缀查询 + 搜索结果增强  
**预计发布**: 2026-05 月

#### 搜索性能

- [ ] **重启后跳过搜索索引重建**
  - 文件：`src/sync.rs`、`src/search_engine.rs`
  - 问题：`NoChange` + 持久化命中时，Tantivy 磁盘索引仍然有效，但当前代码仍全量重建，浪费时间
  - 修复：检测「持久化命中且无 Git 变更」场景，直接复用磁盘上的 Tantivy 索引，跳过 `rebuild_index`
  - 依赖：配合 v1.4.9 的 `content_text` 移除（移除后 content 不在 Note 中，无法重建，更需要此机制）

- [ ] **搜索结果显示路径 / 标签 / 修改时间**
  - 文件：`static/js/search.js`、`static/css/search.css`
  - 问题：v1.4.2 延期项，搜索结果卡片目前只显示标题和摘要
  - 修复：在卡片中补充文件相对路径、笔记标签列表（最多 3 个）、最后修改时间

#### 数据库优化

- [ ] **share_db 前缀查询（P4 遗留修复）**
  - 文件：`src/share_db.rs`
  - 问题：`get_user_shares` 全表扫描，用户分享多时性能下降
  - 修复：将主键从 `{token}` 改为 `{creator}:{token}`，配合 `link_to_creator` 辅助表实现 O(1) token 反查
  - 说明：旧有分享数据失效（接受），重启后自动重建

---

### ✨ v1.4.8 (计划中 - 代码质量)

**主题**: clippy 零警告 + 核心模块测试补全  
**预计发布**: 2026-05 月

#### Clippy 全量修复

- [ ] **处理所有约 30 个 clippy 警告**
  - 文件：`src/git.rs`、`src/sync.rs`、`src/search_engine.rs`、`src/handlers.rs`、`src/scanner.rs` 等
  - 涉及类型：`collapsible_if`、`unnecessary_map_or`、`needless_borrows_for_generic_args`、`derivable_impls`、`collapsible_str_replace` 等
  - 目标：`cargo clippy` 零 warning（新旧代码全覆盖）

#### 测试补全

- [ ] **search_engine.rs 单元测试**
  - 覆盖：基本搜索、标签过滤、文件夹过滤、日期范围过滤、`schema_matches` 类型检测
  - 使用 `tempfile::TempDir` 隔离测试环境

- [ ] **handlers.rs 基础集成测试**
  - 覆盖：`/health` 响应结构验证、`/api/titles` 返回格式、`/orphans` 空库场景

---

### ✨ v1.4.9 (计划中 - 架构优化)

**主题**: 移除 `Note.content_text`，内存占用减半  
**预计发布**: 2026-06 月

#### content_text 完整移除（CODEREVIEW P5 阶段一）

- [ ] **从 `Note` 结构体中移除 `content_text` 字段**
  - 文件：`src/domain.rs`
  - 当前状态：每个笔记同时保存原始 Markdown（`content_text`）和渲染 HTML（`content_html`），大型库内存翻倍
  - 修复：直接删除字段，CURRENT_VERSION 升至 3，强制缓存重建

- [ ] **同步管道重构：content 仅在构建期传递**
  - 文件：`src/sync.rs`、`src/indexer.rs`
  - `ProcessedNote` 类型增加 `Option<String>` content 字段（新处理的笔记携带，缓存复用的为 None）
  - 全量同步：content 从处理结果直接传给 SearchEngine，不存入 Note
  - 增量同步：仅更新变更文件，未变更文件依赖 Tantivy 磁盘索引（配合 v1.4.7）

- [ ] **更新 `graph.rs`（已完成，确认无残留依赖）**
  - v1.4.3 已将 `extract_links_from_note` 改用 `outgoing_links`，理论上无 content_text 依赖
  - 验证全量测试通过

- [ ] **文档全面更新**
  - `CLAUDE.md`：更新 Note 结构体说明、同步管道描述
  - `docs/CODEREVIEW_1.3.md`：P5 状态改为 ✅ 完整修复
  - `.claude/project.md`：版本号 + 模块状态

---

## 📦 功能分类

### 🚨 紧急修复（v1.3.1 / v1.3.2）

来自 `docs/CODEREVIEW_1.3.md`，必须优先处理：

1. **[B1] 持久化标签索引清理缺失** (v1.3.1) — Bug，数据一致性问题
2. **[B4] 增量同步反向链接覆盖丢失** (v1.3.1) — Bug，功能不正确
3. **[S1] 标题文本 XSS 漏洞** (v1.3.1) — 安全，影响分享链接访客
4. **[B2] `/sync` 并发保护缺失** (v1.3.1) — Bug，可能导致数据竞争
5. **[S2] Cookie 未设置 Secure 标志** (v1.3.2) — 安全，JWT 明文传输风险
6. **[S3] 分享密码明文存储** (v1.3.2) — 安全，数据库泄露风险
7. **[S4] 认证中间件路径匹配宽松** (v1.3.2) — 安全，低风险但需收紧

### 🔥 高优先级（v1.3.3）

性能问题对大型知识库影响显著：

1. **Regex `lazy_static!` 优化** (v1.3.3) — 每次处理文件重新编译正则
2. **Tantivy Reader 缓存复用** (v1.3.3) — 每次搜索重建 reader
3. **搜索索引增量更新** (v1.3.3) — 增量同步后全量重建索引
4. **交互体验基础** (v1.4.0) — 快捷键、主题配置、动画

### 🎯 中优先级（v1.4.x 系列）

1. **分享/历史查询前缀索引** (v1.3.3 ✅) — 已完成
2. **代码质量改进 Q1-Q7** (v1.3.4 ✅) — 已完成
3. **测试覆盖补全** (v1.3.4 ✅) — 已完成
4. **Obsidian 语法扩展**（math / callout / highlight）(v1.4.1 ✅) — 已完成
5. **搜索建议与笔记发现** (v1.4.2 ✅) — 已完成
6. **关系图谱增强** (v1.4.3 ✅) — 已完成
7. **PWA + 无障碍** (v1.4.4 ✅) — 已完成
8. **运维扩展** (v1.4.5 ✅) — 已完成
9. **Cookie 动态 Secure + Webhook HMAC + 日志清理** (v1.4.6)
10. **搜索性能 + share_db 前缀查询** (v1.4.7)
11. **clippy 零警告 + 测试补全** (v1.4.8)
12. **content_text 移除 + 内存优化** (v1.4.9)

### 💡 低优先级（探索性，v1.5.0+）

这些功能具有创新性，但优先级较低，可能在后续版本考虑：

1. **笔记聚类分析**（v1.5.0+）
   - 基于共享标签或互相链接的笔记自动聚类
   - 知识图谱洞察报告（中心节点、桥接节点、孤岛检测）
2. **`Note.content_text` 完整移除** — **已提前到 v1.4.9**
3. **SSE 实时同步通知**（v1.5.0+）
   - 服务端推送（Server-Sent Events），同步完成后浏览器自动刷新
4. **WASM 加速**（探索中）
   - Markdown 渲染或搜索索引部分迁移至 WebAssembly
5. **多用户权限**（探索中）
   - 细粒度权限：不同用户可访问不同文件夹

### ❌ 不支持的功能

为了保持项目专注于**只读展示**和**高性能浏览**，以下功能**不在**规划范围内：

1. **在线编辑**
   - 原因：Obsidian 客户端已提供完善的编辑体验
   - 替代方案：通过 Git 同步获取编辑更新

2. **多仓库切换**
   - 原因：增加架构复杂度，维护成本高
   - 替代方案：部署多个实例，每个实例对应一个仓库

3. **评论系统**
   - 原因：不符合只读展示定位，增加社交功能复杂度
   - 替代方案：如需讨论，可使用外部工具（如 GitHub Issues）

4. **协作编辑**
   - 原因：需要重大架构调整，与只读定位冲突
   - 替代方案：使用 Obsidian 客户端配合 Git 协作

5. **插件系统**
   - 原因：增加安全风险和维护成本
   - 替代方案：保持核心功能简洁，通过版本迭代添加常用功能

6. **笔记导出**
   - 原因：笔记已存储在 Git 仓库中，可直接访问原始 Markdown 文件
   - 替代方案：使用 Git 克隆获取笔记，或使用 Obsidian 客户端的导出功能

7. **键盘快捷键自定义**
   - 原因：增加配置复杂度，当前快捷键已满足基本需求
   - 替代方案：保持简洁的快捷键设计（Ctrl+K 搜索等）

8. **复杂统计分析**
   - 原因：不属于核心浏览功能，增加计算开销
   - 替代方案：使用外部工具分析 Git 仓库和 Markdown 文件

9. **版本历史查看**
   - 原因：增加实现复杂度，与只读定位偏离
   - 替代方案：使用 Git 客户端或 GitHub/GitLab Web 界面查看历史

10. **笔记快照功能**
    - 原因：笔记已通过 Git 版本控制，无需额外快照机制
    - 替代方案：使用 Git tag 或分支标记重要版本

---

## 🌟 长期愿景

### 2026 年目标

**Q1 (1-3月)**: 功能完善期 ✅ **已完成**
- ✅ 完成 v0.8.0 (搜索和标签)
- ✅ 完成 v0.9.0 (可视化增强)
- ✅ 完成 v0.10.0 (关系图谱完善)
- ✅ 完成 v1.0.0 (生产就绪) 🎉
- ✅ 完成 v1.1.0 (功能增强) 🎉
- ✅ 完成 v1.2.0 (体验优化) 🎉
- ✅ 完成 v1.3.0 (高级搜索) 🎉

**Q2 (4-6月)**: 质量加固期
- ✅ 完成 v1.3.1（紧急 Bug 修复 + XSS 安全修复）🎉
- ✅ 完成 v1.3.2（安全加固：Cookie、分享密码哈希、中间件收紧）🎉
- ✅ 完成 v1.3.3（性能优化：Regex/Tantivy/增量搜索索引/前缀查询/内存优化）🎉
- ✅ 完成 v1.3.4（代码质量 + 测试覆盖补全：Q1/Q3/Q5/Q6/Q7 + sync/graph/persistence 测试）🎉

**Q3 (7-9月)**: 体验提升期
- ✅ 完成 v1.4.0（交互体验：快捷键、主题定制、动画）🎉
- ✅ 完成 v1.4.1（内容渲染：KaTeX 数学公式、Callout、高亮、图片灯箱）🎉
- ✅ 完成 v1.4.2（搜索发现：实时建议、孤立笔记、随机漫游、最近更新）🎉

**Q4 (10-12月)**: 深度功能期
- ✅ 完成 v1.4.3（图谱增强：全库图谱、节点着色、布局模式、搜索导出）🎉
- ✅ 完成 v1.4.4（PWA + 无障碍：离线安装、手势、ARIA）🎉
- ✅ 完成 v1.4.5（运维扩展：自动同步、Webhook、配置热重载、Glob 忽略、指标扩展）🎉
- 完成 v1.4.6（安全修复：Cookie 动态 Secure、Webhook HMAC、graph.js 调试清理）
- 完成 v1.4.7（搜索优化：重启不重建索引、share_db 前缀查询、搜索结果增强）
- 完成 v1.4.8（代码质量：clippy 零警告、search_engine 测试、handlers 集成测试）
- 完成 v1.4.9（架构优化：content_text 移除、内存减半、CURRENT_VERSION 升至 3）

### 核心价值主张

1. **只读展示**: 专注于笔记浏览和展示，不做在线编辑
2. **高性能**: 响应迅速，支持大型笔记库（10000+ 文件）
3. **兼容性好**: 完美支持 Obsidian 语法（WikiLinks、Frontmatter 等）
4. **美观易用**: 现代化 UI，移动端友好，深色模式支持

### 产品定位

**Obsidian Mirror** 是一个**只读的笔记展示工具**，不是 Obsidian 的替代品，而是补充：

- ✅ **查看笔记**: 随时随地通过浏览器访问笔记
- ✅ **分享知识**: 将个人知识库发布为 Web 站点
- ✅ **团队协作**: 团队成员共享只读访问权限
- ❌ **编辑笔记**: 使用 Obsidian 客户端编辑，通过 Git 同步

---

## 🔄 迭代原则

### 开发原则

1. **用户优先**: 功能设计以用户需求为导向
2. **性能第一**: 不能为了功能牺牲性能
3. **渐进增强**: 功能可选，不影响核心体验
4. **保持简单**: 拒绝过度设计

### 版本策略

- **主版本 (X.0.0)**: 重大架构变更或不兼容更新
- **次版本 (0.X.0)**: 新功能和重要改进
- **修订版本 (0.0.X)**: Bug 修复和小优化

### 发布周期

- 功能版本：每 4-6 周
- 修复版本：按需发布
- 安全更新：立即发布

---

## 🛠️ 技术演进

### 短期技术升级

- ✅ **前端工程化** (v0.9.0)
  - 模块化 JavaScript（已实现）
  - 模块化 CSS（已实现）
  - 待完成：构建工具、TypeScript 支持

- ✅ **搜索引擎** (v0.8.0)
  - 已采用 Tantivy 高性能全文索引
  - 支持中文分词（jieba-rs）
  - 后台异步索引构建

- ✅ **增量更新机制** (v1.0.0)
  - Git diff 检测变更（已实现）
  - 索引持久化（已实现）
  - 智能同步模式（已实现）

### 长期技术探索

- **WASM 集成**: Markdown 渲染性能优化（探索中）
- **Server-Sent Events**: 实时同步通知（探索中）
- ✅ **嵌入式数据库**: 已采用 redb 持久化索引
- **边缘计算**: Cloudflare Workers 部署支持（探索中）

### 明确不做的技术方向

- ❌ **在线编辑器**: 不引入富文本编辑器或 Markdown 编辑器
- ❌ **实时协作**: 不实现 CRDT 或 OT 算法
- ❌ **复杂权限**: 不实现细粒度权限控制（文件级、字段级等）
- ❌ **数据库迁移**: 不引入重量级数据库（PostgreSQL、MySQL 等）

---

## 📊 成功指标

### 性能指标

- ✅ 首次加载时间 < 2 秒（已达成：< 1s）
- ✅ 搜索响应时间 < 100ms（已达成：< 5ms P95）
- ✅ 支持 10,000+ 笔记规模（已验证）
- ✅ 内存占用 < 500MB (1000 笔记)（已达成）

### 质量指标

- ✅ 代码覆盖率 > 60%（已达成：核心模块 100%）
- ✅ 关键路径测试覆盖 100%（已达成：37 个测试）
- ✅ Zero 安全漏洞（依赖扫描）
- 用户反馈 Bug 平均修复时间 < 7 天

---

## 💬 反馈渠道

我们重视社区的反馈和建议！

### 功能建议

- 提交 GitHub Issue (功能请求标签)
- 描述使用场景和预期效果
- 说明对你的重要性

### Bug 报告

- 提交 GitHub Issue (Bug 标签)
- 提供复现步骤
- 附上日志和环境信息

### 讨论交流

- GitHub Discussions
- 技术博客评论
- 邮件联系

---

## 📝 变更说明

本路线图会根据以下因素动态调整：

- 用户反馈和需求变化
- 技术可行性评估
- 开发资源情况
- 生态系统演进

**查看历史变更**: 通过 Git 历史查看本文档的修改记录

---

## 🙏 致谢

感谢所有用户的支持和反馈，是你们让 Obsidian Mirror 变得更好！

---

**最后更新**: 2026-04-13（补充 v1.4.6-v1.4.9 规划：安全/性能/质量/架构）  
**维护者**: Obsidian Mirror 开发团队
