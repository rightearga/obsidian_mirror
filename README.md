# Obsidian Mirror

Obsidian 笔记镜像 Web 服务器 - 将你的 Obsidian 笔记库自动同步并以 Web 应用的形式展示。

**当前版本：v1.3.0** 🎉

## 功能特性

### 📝 内容处理
- ✅ **自动同步**：从 Git 仓库自动拉取/克隆 Obsidian 笔记
- ✅ **WikiLinks 支持**：自动解析 Obsidian 的 `[[链接]]` 和 `![[图片]]` 语法
- ✅ **图片和附件**：完整支持图片、PDF 等附件展示
- ✅ **Frontmatter 支持**：解析 YAML 元数据
- ✅ **代码高亮**：使用 Highlight.js 自动高亮代码块

### 🔍 搜索与导航
- ✅ **全文搜索**：Tantivy 引擎，中文分词，关键词高亮
- ✅ **高级搜索**：按标签、文件夹、日期过滤，支持排序（相关度/修改时间）
- ✅ **标签系统**：Frontmatter 和 hashtag 标签，标签云可视化
- ✅ **最近访问**：记录最近访问笔记，快速回到常用页面
- ✅ **收藏夹**：收藏重要笔记，侧边栏快速访问
- ✅ **笔记统计**：笔记总数、标签数量、最近更新统计
- ✅ **侧边栏导航**：文件树结构，可拖动调整大小（200-600px）
- ✅ **反向链接**：显示所有指向当前笔记的链接
- ✅ **关系图谱**：Vis.js 可视化笔记关联，支持 1-3 层深度
- ✅ **笔记目录**：自动生成 TOC，滚动高亮当前章节
- ✅ **面包屑导航**：显示路径层级

### 🎨 用户体验
- ✅ **主题切换**：深色/浅色模式，统一滚动条样式
- ✅ **多语言支持**：中文/English 切换，自动保存偏好
- ✅ **响应式设计**：完整的移动端和桌面端支持
- ✅ **滚动位置记忆**：侧边栏和内容区自动记忆滚动位置
- ✅ **快捷键支持**：Ctrl+K/Cmd+K 快速搜索
- ✅ **内容自适应布局**：内容宽度自动适配屏幕，提升阅读体验
- ✅ **移动端 TOC**：右侧滑入侧边栏，平滑动画，遮罩层交互
- ✅ **桌面端 TOC**：可收起/展开，拖动调整宽度
- ✅ **阅读进度跟踪**：自动记录阅读位置，刷新页面自动恢复
- ✅ **分享链接生成**：生成带过期时间的笔记分享链接

### 🔐 安全与认证
- ✅ **用户认证**：JWT 令牌认证，Cookie 存储
- ✅ **密码管理**：bcrypt 加密，修改密码页面
- ✅ **用户菜单**：显示用户名，登出功能
- ✅ **分享链接**：生成带过期时间的临时访问链接

### ⚡ 性能优化
- ✅ **增量同步**：Git diff 检测，仅处理变更文件（10-100x 提升）
- ✅ **索引持久化**：postcard + redb，重启恢复 < 1s（30-120x 提升）
- ✅ **智能同步**：无变更快速跳过，内存状态检测

### 🛠️ 运维功能
- ✅ **健康检查**：`/health` 端点，适配 Kubernetes
- ✅ **指标暴露**：`/metrics` Prometheus 格式
- ✅ **日志管理**：分级输出（控制台 + 文件），每日轮转
- ✅ **优雅关闭**：捕获信号，保存状态，等待请求完成

### 🧪 质量保证
- ✅ **单元测试**：37 个测试覆盖核心逻辑（100% 通过）
- ✅ **错误处理**：统一错误类型，自动恢复机制

## 技术栈

- **语言**: Rust (Edition 2024)
- **Web 框架**: Actix-web 4.x
- **模板引擎**: Askama
- **Markdown 解析**: pulldown-cmark
- **异步运行时**: Tokio
- **搜索引擎**: Tantivy（倒排索引）
- **中文分词**: jieba-rs
- **图谱可视化**: Vis.js
- **认证**: JWT + bcrypt
- **持久化**: postcard + redb
- **监控**: Prometheus

## 快速开始

### 方式一：Docker 部署（推荐）

**前置要求：**
- Docker
- Docker Compose

**步骤：**

1. 复制配置文件模板
```bash
cp config.example.ron config.ron
```

2. 编辑 `config.ron`，配置你的 Git 仓库地址

3. 启动容器
```bash
docker-compose up -d
```

4. 查看日志
```bash
docker-compose logs -f
```

5. 访问 `http://localhost:3080`

**停止服务：**
```bash
docker-compose down
```

**重新构建：**
```bash
docker-compose up -d --build
```

#### Docker 部署注意事项

**访问私有 Git 仓库：**

如果使用 SSH 方式访问私有仓库，需要挂载 SSH key：

```yaml
# docker-compose.yml
volumes:
  - ~/.ssh:/home/appuser/.ssh:ro
```

如果使用 HTTP 方式，可以配置 Git credentials：

```bash
# 创建 .gitconfig 文件
git config --file .gitconfig credential.helper store
git config --file .gitconfig user.name "Your Name"
git config --file .gitconfig user.email "your@email.com"

# 取消 docker-compose.yml 中相关注释
```

**持久化数据：**

`docker-compose.yml` 默认已配置笔记数据持久化，避免每次重启都重新克隆。

### 方式二：本地编译部署

**前置要求：**
- Rust 1.75+ (`rustup` 推荐)
- Git

### 配置

在项目根目录创建 `config.ron` 文件：

```ron
(
    repo_url: "http://your-git-server.com/your-repo.git",
    local_path: "./my-note",
    listen_addr: "0.0.0.0:3080",
    workers: 4,
    ignore_patterns: [
        "私密文件夹",
        "草稿",
        ".obsidian"
    ],
    persistence_db_path: "./index.db",
    security: (
        auth_enabled: true,
        jwt_secret: "YOUR_RANDOM_SECRET_KEY_HERE",
        token_lifetime_hours: 24,
        auth_db_path: "./auth.db",
        default_admin_username: "admin",
        default_admin_password: "admin123",
    ),
)
```

**配置说明：**
- `repo_url`: Git 仓库地址（支持 http/https，需要无密码访问或配置 Git credentials）
- `local_path`: 本地笔记存储路径
- `listen_addr`: Web 服务器监听地址和端口
- `workers`: 工作线程数（默认为 CPU 核心数）
- `ignore_patterns`: 忽略的文件夹/文件名（不区分大小写）
- `persistence_db_path`: 索引持久化数据库路径（默认 `./index.db`）
- `security`: 认证配置
  - `auth_enabled`: 是否启用用户认证
  - `jwt_secret`: JWT 密钥（务必修改）
  - `token_lifetime_hours`: 令牌有效期（小时）
  - `auth_db_path`: 用户数据库路径
  - `default_admin_username`: 默认管理员用户名
  - `default_admin_password`: 默认管理员密码

### 构建

```bash
# 开发版本
cargo build

# 发布版本（优化编译）
cargo build --release
```

### 运行

```bash
# 开发模式
cargo run

# 或直接运行编译后的二进制
./target/release/obsidian_mirror
```

服务器启动后访问 `http://localhost:3080`

## 使用方法

### 同步笔记

应用启动时会自动执行一次同步。后续可通过以下方式手动同步：

**Web 界面：**
点击侧边栏右上角的"同步"按钮

**命令行：**
```bash
curl -X POST http://localhost:3080/sync
```

### 主题切换

点击侧边栏右上角的主题切换按钮（太阳/月亮图标）在深色和浅色模式之间切换。主题偏好会自动保存在浏览器中。

### 访问笔记

- **首页**: `http://localhost:3080/`
  - 自动尝试显示 README.md
  - 如无 README，重定向到第一个笔记
- **特定笔记**: `http://localhost:3080/doc/文件夹/笔记标题`

### WikiLinks 语法

在 Markdown 中使用：

```markdown
# 基本链接
[[笔记标题]]

# 带显示文字的链接
[[笔记标题|显示文本]]

# 图片（自动识别图片格式）
![[图片.png]]
![[路径/图片.jpg]]

# 带尺寸的图片
![[图片.png|300]]

# 附件链接
[[文件.pdf]]
[[文档.docx|查看文档]]
```

会自动转换为可点击的链接。图片会直接内嵌显示。

### Markdown 图片语法

也支持标准 Markdown 语法：

```markdown
# 相对路径图片
![描述](./images/图片.png)
![](相对路径/文件.jpg)

# 附件链接
[下载文件](./files/文档.pdf)
```

相对路径会自动转换为正确的资源路径。

### Frontmatter 示例

```yaml
---
title: 我的笔记
date: 2024-01-25
tags: [标签1, 标签2]
---

笔记内容...
```

Frontmatter 元数据会被解析但不显示在页面中。

### 代码块

支持语法高亮的代码块：

````markdown
```python
def hello_world():
    print("Hello, World!")
```

```rust
fn main() {
    println!("Hello, World!");
}
```
````

使用 Highlight.js 自动识别和高亮超过 190 种编程语言。

## 项目结构

```
obsidian_mirror/
├── src/
│   ├── main.rs          # 主服务器和路由
│   ├── config.rs        # 配置加载
│   ├── git.rs           # Git 同步逻辑
│   ├── scanner.rs       # 文件扫描器
│   ├── markdown.rs      # Markdown 处理
│   └── domain.rs        # 数据模型
├── templates/           # HTML 模板
│   ├── layout.html      # 基础布局
│   ├── page.html        # 笔记页面
│   └── index.html       # 空白首页
├── static/              # 静态资源
│   └── style.css        # 样式表
├── config.ron           # 配置文件（需手动创建）
├── config.example.ron   # 配置文件模板
├── Dockerfile           # Docker 镜像构建文件
├── docker-compose.yml   # Docker Compose 配置
├── .dockerignore        # Docker 构建忽略文件
├── Cargo.toml           # Rust 依赖配置
└── README.md            # 本文件
```

## 开发

### 前置要求

- Rust 1.75+ (推荐使用 rustup 安装)
- Git 2.0+
- (可选) Docker 和 Docker Compose

### 代码检查

```bash
# 检查编译错误（快速）
cargo check

# 代码格式化
cargo fmt

# Lint 检查（推荐）
cargo clippy

# 运行所有检查
cargo fmt && cargo clippy && cargo check
```

### 开发构建

```bash
# 开发模式（包含调试信息，编译快）
cargo build

# 发布模式（优化编译，性能最佳）
cargo build --release

# 清理构建产物
cargo clean
```

### 日志级别

通过环境变量控制日志级别：

```bash
# 显示所有日志（包括调试信息）
RUST_LOG=debug cargo run

# 仅显示信息级别
RUST_LOG=info cargo run

# 仅显示警告和错误
RUST_LOG=warn cargo run

# 特定模块的日志
RUST_LOG=obsidian_mirror=debug,actix_web=info cargo run
```

### Docker 开发

```bash
# 构建镜像
docker build -t obsidian_mirror .

# 运行容器
docker run -p 3080:3080 -v ./config.ron:/app/config.ron:ro obsidian_mirror

# 使用 docker-compose 开发
docker-compose up --build

# 查看容器日志
docker-compose logs -f obsidian_mirror
```

## API 端点

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/` | 首页（README 或第一个笔记） |
| GET | `/doc/{path}` | 访问指定笔记 |
| GET | `/assets/{path}` | 访问图片、PDF 等附件资源 |
| POST | `/sync` | 手动触发 Git 同步 |
| GET | `/api/search?q=关键词&sort_by=relevance` | 全文搜索（支持高级过滤） |
| GET | `/api/graph?note=笔记&depth=2` | 获取关系图谱数据 |
| GET | `/api/stats` | 获取笔记统计信息 |
| GET | `/tags` | 标签列表 |
| GET | `/tag/{tag}` | 查看指定标签的笔记 |
| GET | `/health` | 健康检查（用于容器编排） |
| GET | `/metrics` | Prometheus 指标 |
| GET | `/static/*` | 静态资源（CSS、JS 等） |

**认证相关端点**（需启用认证）：
| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/login` | 登录页面 |
| POST | `/api/auth/login` | 用户登录 |
| POST | `/api/auth/logout` | 用户登出 |
| GET | `/change-password` | 修改密码页面 |
| POST | `/api/auth/change-password` | 修改密码 |
| GET | `/api/auth/current-user` | 获取当前用户信息 |

**分享链接端点**：
| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/api/share/create` | 创建分享链接 |
| GET | `/share/{token}` | 访问分享的笔记 |
| GET | `/api/share/list` | 获取分享链接列表 |
| DELETE | `/api/share/{token}` | 删除分享链接 |

**阅读进度端点**：
| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/api/reading-progress/save` | 保存阅读进度 |
| GET | `/api/reading-progress/{note_path}` | 获取笔记阅读进度 |
| GET | `/api/reading-progress/history` | 获取阅读历史 |

## 注意事项

### Git 访问配置
- Git 仓库需要配置无密码访问
- **SSH 方式**：配置 SSH key 并添加到 Git 服务器
- **HTTPS 方式**：使用 Git credentials 或 personal access token

### 文件管理
- `local_path` 目录会被 Git 管理，请勿手动修改
- 同步操作会重建所有索引，大型笔记库（1000+ 文件）可能需要几秒钟
- 隐藏文件和文件夹（以 `.` 开头）会自动被忽略

### 图片和附件
- 支持的图片格式：PNG, JPG, JPEG, GIF, SVG, WebP, BMP, ICO
- 附件文件会通过 `/assets/` 路径提供访问
- 支持所有非 `.md` 文件作为附件（PDF, DOCX, ZIP 等）
- 图片路径支持相对路径和文件名直接引用

### 性能优化
- **增量同步**：使用 Git diff 检测变更，仅处理修改的文件
  - 修改 1 个文件：30-60s → 0.5s（**60-120x 提升**）
  - 无变更时立即返回（< 1s）
- **索引持久化**：postcard + redb 持久化笔记索引
  - 首次启动：30-60s → < 1s（**30-60x 提升**）
  - 应用重启：30-60s → < 0.5s（**60-120x 提升**）
- **智能加载**：Git 提交校验，版本兼容性检查
- Worker 线程数建议设置为 CPU 核心数（配置中设为 0 自动检测）

### 安全建议
- 不要在 Git 仓库中提交敏感信息（密码、token 等）
- 使用 `ignore_patterns` 排除私密文件夹
- 生产环境建议配置反向代理（Nginx、Caddy）并启用 HTTPS

## 更新日志

### v1.3.0 (2026-02-01) 🎉
**主题**: 高级搜索功能

**新增功能**：
- ✨ **高级搜索过滤**: 按标签、文件夹、日期范围过滤搜索结果
  - 标签过滤：支持多个标签（逗号分隔），OR 逻辑匹配
  - 文件夹过滤：按文件夹路径精确筛选
  - 日期过滤：按修改时间范围筛选（开始日期-结束日期）
  - 可折叠的高级过滤面板，不占用空间
- ✨ **搜索结果排序**: 按相关度或最新修改时间排序
- ✨ **实时过滤**: 输入即刻生效，无需点击搜索按钮
- ✨ **清除过滤**: 一键清除所有过滤条件
- ✨ **分享链接生成**: 创建带过期时间的笔记分享链接
  - 支持自定义过期时间（1小时-30天）
  - 分享链接管理界面
  - 访问统计和历史记录
- ✨ **阅读进度跟踪**: 自动记录和恢复阅读位置
  - 实时保存滚动位置
  - 刷新页面自动恢复
  - 阅读历史记录

**后端改进**：
- 索引优化：标签和文件夹字段加入搜索索引
- 查询组合：使用 Tantivy BooleanQuery 组合多个条件
- 日期范围查询：RangeQuery 实现时间范围过滤
- API 扩展：新增 tags、folder、date_from、date_to 参数
- 数据库配置统一：集中管理 4 个数据库路径
- 分享链接和阅读进度数据库（redb 持久化）

**前端改进**：
- 高级过滤面板 UI（+128 行 CSS）
- 过滤交互逻辑（+122 行 JavaScript）
- 日期选择器、标签输入框、文件夹路径输入
- 响应式设计，支持深色/浅色主题
- 分享链接管理界面
- 阅读进度自动保存和恢复

**技术亮点**：
- 标签字段支持多值索引
- 文件夹路径自动提取
- Unix 时间戳精确匹配
- 保持向后兼容
- redb 嵌入式数据库轻量高效

### v1.2.0 (2026-01-31) 🎉
**主题**: 体验优化

**新增功能**：
- ✨ **多语言支持**: 中文/English 切换，自动保存语言偏好，全局生效
- ✨ **笔记预览**: 悬浮卡片预览链接内容，快速查看不跳转，前 200 字预览
- ✨ **设置面板**: 统一的用户偏好设置（语言、字体、主题等）

**移动端优化**：
- ✨ **TOC 右侧滑入**: 移动端 TOC 改为右侧滑入侧边栏，操作更直观
- ✨ **状态栏完善**: 移动端显示完整状态信息（字数、行数、时间）
- ✨ 紧凑布局优化：字体、图标、间距调整适配小屏幕

**桌面端优化**：
- ✨ **TOC 收起/展开**: 桌面端 TOC 可一键收起/展开，节省空间
- ✨ **内容自适应**: 移除固定宽度限制，内容区自动适配屏幕宽度
- ✨ 表格溢出处理：防止表格与目录重叠

**用户体验**：
- ✨ 设置页面统一管理用户偏好
- ✨ 移除内容宽度设置项（改为自适应）
- ✨ 同步状态调试日志（排查同步问题）

**Bug 修复**：
- 🐛 修复移动端 TOC 侧边栏不显示问题
- 🐛 修复 ignore_patterns 配置变更不生效
- 🐛 修复表格溢出导致布局问题

### v1.1.0 (2026-01-31)
**主题**: 功能增强

**新增功能**：
- ✨ **最近访问笔记**: localStorage 本地存储，最多 10 条记录，相对时间显示
- ✨ **收藏夹功能**: 星标收藏，侧边栏管理，快速访问重要笔记
- ✨ **笔记统计面板**: 3 个统计卡片（笔记总数、标签数量、最近更新）
- ✨ **标签持久化修复**: 修复标签索引持久化缺失导致的数据丢失问题

**用户体验**：
- ✨ 侧边栏新增统计、最近访问、收藏夹三个面板
- ✨ 笔记页面添加收藏按钮（星标图标）
- ✨ Toast 提示消息（收藏/取消收藏反馈）
- ✨ 完整的移动端响应式适配

**Bug 修复**：
- 🐛 修复标签索引持久化缺失问题
- 🐛 修正标签按钮图标（改为标签图标）

### v1.0.0 (2026-01-31) 🎉
**主题**: 生产就绪

**核心功能**：
- ✨ **增量同步优化**: Git diff 检测变更，仅处理修改的文件，性能提升 10-100 倍
- ✨ **索引持久化**: postcard + redb 持久化笔记索引，重启恢复速度提升 30-120 倍
- ✨ **用户认证**: JWT 令牌认证，bcrypt 密码加密，修改密码功能

**界面优化**：
- ✨ **侧边栏增强**: 默认 320px 宽度，可拖动调整大小（200-600px），宽度记忆
- ✨ **滚动条统一**: 所有滚动区域统一样式，自动适配 Dark 主题

**运维功能**：
- ✨ **健康检查**: `/health` 端点，适配 Docker/Kubernetes
- ✨ **指标暴露**: `/metrics` Prometheus 格式指标
- ✨ **日志管理**: 分级文件输出，每日轮转
- ✨ **优雅关闭**: 捕获信号，保存状态，等待请求完成

**质量保证**：
- ✨ **单元测试**: 37 个测试覆盖核心逻辑（100% 通过）
- ✨ **错误处理**: 自定义 AppError 类型，统一错误处理

### v0.10.0 (2026-01-29)
- ✨ 实现关系图谱可视化（Vis.js）
- ✨ 支持 1-3 层深度关系展示
- ✨ 交互式节点（点击跳转、拖拽移动）
- ✨ 物理引擎动画效果

### v0.9.0 (2026-01-29)
- ✨ 添加笔记目录（TOC）自动生成
- ✨ 浮动 TOC 支持（右侧固定）
- ✨ 滚动高亮当前章节
- ✨ 添加面包屑导航
- ✨ 代码模块化（独立 JS/CSS 文件）

### v0.8.0 (2026-01-29)
- ✨ **全文搜索**: Tantivy 引擎，中文分词，关键词高亮
- ✨ **标签系统**: Frontmatter 和 hashtag 标签，标签云可视化
- ✨ **搜索历史**: 记录搜索历史
- ✨ 快捷键支持（Ctrl+K / Cmd+K）
- ✨ 性能优化：后台异步索引构建

### v0.7.1 (2026-01-29)
- ✨ 界面文本全面中文化
- ✨ 移动端和桌面端侧边栏滚动位置记忆
- 🐛 修复移动端侧边栏交互问题
- 🐛 修复侧边栏多行文本对齐和缩进问题

### v0.7.0
- ✨ 添加图片和附件支持
- ✨ 支持 WikiLinks 图片语法 `![[图片.png]]`
- ✨ 支持 Markdown 图片相对路径
- ✨ 完整的移动端响应式设计

## 常见问题

### Q: 同步失败，提示 Git 错误？
**A:** 检查以下几点：
1. 确认 Git 仓库 URL 正确且可访问
2. 确认已配置 SSH key 或 Git credentials
3. 查看日志：`docker-compose logs -f` 或 `RUST_LOG=debug cargo run`
4. 尝试手动 git clone 测试连接性

### Q: 图片无法显示？
**A:** 检查以下几点：
1. 确认图片文件存在于 Git 仓库中
2. 检查图片路径是否正确（相对于笔记文件）
3. 尝试访问 `http://localhost:3080/assets/你的图片.png` 测试
4. 查看浏览器控制台是否有 404 错误

### Q: 笔记更新后没有同步？
**A:** 
1. 点击"同步"按钮手动触发同步
2. 或调用 API：`curl -X POST http://localhost:3080/sync`
3. 确认 Git 仓库已有新的提交

### Q: Docker 容器无法启动？
**A:** 
1. 检查 `config.ron` 文件是否存在且格式正确
2. 检查端口 3080 是否被占用：`netstat -tlnp | grep 3080`
3. 查看容器日志：`docker-compose logs -f`
4. 尝试重新构建：`docker-compose up -d --build`

### Q: 如何配置反向代理？
**A:** Nginx 配置示例：
```nginx
server {
    listen 80;
    server_name your-domain.com;
    
    location / {
        proxy_pass http://localhost:3080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

### Q: 支持哪些 Obsidian 功能？
**A:** 目前支持：
- ✅ WikiLinks 语法
- ✅ 图片嵌入
- ✅ 附件链接
- ✅ Frontmatter
- ✅ 代码块
- ✅ 反向链接
- ✅ 全文搜索
- ✅ 标签系统
- ✅ 关系图谱
- ✅ 笔记目录 (TOC)
- ✅ 面包屑导航

查看完整规划：[ROADMAP.md](ROADMAP.md)

## 技术架构

### 核心技术
- **Web 框架**: Actix-web - 高性能异步 Web 框架
- **模板引擎**: Askama - 编译时类型安全的模板引擎
- **Markdown**: pulldown-cmark - 快速的 CommonMark 解析器
- **异步运行时**: Tokio - 可靠的异步运行时

### 数据流
1. **启动同步**: Git clone/pull → 扫描 `.md` 文件
2. **处理笔记**: 解析 Frontmatter → 转换 WikiLinks → 生成 HTML
3. **构建索引**: 标题索引、文件索引、反向链接索引
4. **提供服务**: 侧边栏导航 + 笔记内容 + 反向链接

### 内存结构
- `notes: HashMap<Path, Note>` - 笔记内容和元数据
- `link_index: HashMap<Title, Path>` - 标题到路径的映射
- `file_index: HashMap<Filename, Path>` - 文件名到路径的映射
- `backlinks: HashMap<Title, Vec<Title>>` - 反向链接关系
- `sidebar: Vec<SidebarNode>` - 层级目录树

## 未来计划

查看完整的开发路线图：[ROADMAP.md](ROADMAP.md)

### 近期规划

**v1.4.0** - 持续优化
- 性能持续优化
- 用户体验细节完善

查看完整规划：[ROADMAP.md](ROADMAP.md)

### 不支持的功能

本项目专注于**只读展示**和**高性能浏览**，以下功能不在支持范围内：

- ❌ **在线编辑**：请使用 Obsidian 客户端编辑，本项目通过 Git 同步获取更新
- ❌ **多仓库切换**：一个实例对应一个笔记库
- ❌ **评论系统**：专注于笔记展示，不添加社交功能
- ❌ **协作编辑**：不支持多人实时编辑
- ❌ **插件系统**：保持简洁，不引入插件机制
- ❌ **笔记导出**：笔记存储在 Git 仓库中，可直接访问原始 Markdown 文件或使用 Obsidian 导出
- ❌ **键盘快捷键自定义**：保持简洁设计，当前快捷键已满足基本需求
- ❌ **复杂统计分析**：不属于核心浏览功能，可使用外部工具分析
- ❌ **版本历史**：不实现 Git 历史查看功能
- ❌ **笔记快照**：不实现笔记版本快照功能

## 许可

本项目为个人工具，仅供私有使用。

---

**项目地址**: https://github.com/your-username/obsidian_mirror  
**当前版本**: v1.3.0 🎉  
**开发路线图**: [ROADMAP.md](ROADMAP.md)  
**最后更新**: 2026-02-01
