# Obsidian Mirror

Obsidian 笔记镜像 Web 服务器 - 将你的 Obsidian 笔记库自动同步并以 Web 应用的形式展示。

**当前版本：v1.8.7** 🎉

> 📋 **[CODEREVIEW 报告](docs/CODEREVIEW_1.8.md)**：v1.8.x 系列代码审计报告

## 功能特性

### 📝 内容处理
- ✅ **自动同步**：从 Git 仓库自动拉取/克隆 Obsidian 笔记
- ✅ **WikiLinks 支持**：自动解析 Obsidian 的 `[[链接]]` 和 `![[图片]]` 语法
- ✅ **图片和附件**：完整支持图片、PDF 等附件展示；图片懒加载 + 灯箱放大
- ✅ **Frontmatter 支持**：解析 YAML 元数据
- ✅ **代码高亮**：Highlight.js（可选 6 套主题：Atom One Dark/Light / GitHub / Dracula / Monokai）
- ✅ **KaTeX 数学公式**：`$...$`（行内）和 `$$...$$`（块级）
- ✅ **Callout 标注块**：`> [!NOTE]`、`> [!WARNING]` 等 20+ 类型，支持折叠
- ✅ **高亮语法**：`==文本==` 渲染为高亮
- ✅ **Mermaid 图表**：流程图、序列图、甘特图、类图、状态图

### 🔍 搜索与导航
- ✅ **全文搜索**：Tantivy 引擎，中文分词，关键词高亮；搜索结果显示路径/标签/修改时间
- ✅ **实时搜索建议**：输入时标题/标签自动补全（`#` 触发标签），sessionStorage 缓存
- ✅ **高级搜索**：按标签、文件夹、日期过滤，支持排序（相关度/修改时间）
- ✅ **标签系统**：Frontmatter 和 hashtag 标签，标签云可视化
- ✅ **笔记发现**：孤立笔记页（`/orphans`）、随机漫游（`/random`）、最近更新（`/recent`）
- ✅ **最近访问**：记录最近访问笔记，快速回到常用页面
- ✅ **收藏夹**：收藏重要笔记，侧边栏快速访问
- ✅ **笔记统计**：笔记总数、标签数量、最近更新统计
- ✅ **侧边栏导航**：文件树结构，可拖动调整大小（200-600px）
- ✅ **反向链接**：显示所有指向当前笔记的链接
- ✅ **关系图谱**：全库图谱（`/graph/global`）+ 局部图谱；节点大小按入度、颜色按标签分组；力导向/层级布局；图谱内搜索；PNG 导出
- ✅ **笔记目录**：自动生成 TOC，滚动高亮当前章节
- ✅ **面包屑导航**：显示路径层级
- ✅ **全局知识图谱专页**：`/graph` 全屏独立页，聚类着色，工具栏，孤立节点开关（v1.7.0）
- ✅ **Git 版本历史**：`/doc/{path}/history` 提交历史列表，`/at/{commit}` 快照，`/diff/{commit}` 行级 diff（v1.7.2）
- ✅ **笔记洞察 Dashboard**：`/insights` 写作趋势、知识库健康度（断链/孤立/超大）、标签云（v1.7.3）
- ✅ **多仓库切换**：导航栏仓库下拉切换器，`/r/{name}/...` 路由前缀（v1.7.4）
- ✅ **搜索结果分页**：`/api/search` 支持 page/per_page，"加载更多"按钮（v1.8.0）
- ✅ **时间线视图**：`/timeline` 按 frontmatter date/mtime 时间轴，月/年折叠，标签过滤（v1.8.4）

### 🎨 用户体验
- ✅ **键盘快捷键**：`j/k` 滚动、`gg/G` 顶底、`[/]` 前后笔记、`b` 返回、`t` 切换 TOC、`g` 图谱、`r` 随机、`?` 帮助面板
- ✅ **主题定制**：深色/浅色基础 + 暖色/护眼/高对比度预设；自定义强调色
- ✅ **多语言支持**：中文/English 切换，自动保存偏好
- ✅ **响应式设计**：完整的移动端和桌面端支持
- ✅ **触屏手势**：侧边栏边缘滑动 + 笔记翻页手势
- ✅ **PWA 支持**：可安装为桌面/移动端 App，静态资源离线缓存
- ✅ **无障碍**：Skip Link、ARIA 语义、`:focus-visible` 焦点样式、`prefers-reduced-motion`
- ✅ **交互动画**：页面淡入、搜索结果错峰进入、侧边栏图标旋转（可关闭）
- ✅ **滚动位置记忆**：侧边栏和内容区自动记忆滚动位置
- ✅ **阅读进度跟踪**：自动记录阅读位置，刷新页面自动恢复
- ✅ **分享链接生成**：生成带过期时间的笔记分享链接
- ✅ **PWA 离线完善**：已访问笔记离线可用，Stale-While-Revalidate，同步完成"有新内容"横幅，网络状态指示器（v1.8.3）

### 🔐 安全与认证
- ✅ **用户认证**：JWT 令牌认证；`force_https_cookie` 配置控制 Cookie Secure 标志
- ✅ **密码管理**：bcrypt 加密，修改密码页面
- ✅ **分享链接**：访问密码 bcrypt 哈希存储（非明文）
- ✅ **认证中间件**：公开路径精确匹配，防止路径前缀绕过认证
- ✅ **多用户管理**：admin/editor/viewer 三级角色，用户管理页（`/admin/users`），管理员可创建/禁用/重置密码（v1.5.3）

### ⚡ 性能优化
- ✅ **增量同步**：Git diff 检测，仅处理变更文件（10-100x 提升）
- ✅ **增量搜索索引**：增量同步时只更新变更文件，不全量重建
- ✅ **重启跳过索引重建**：持久化命中 + Tantivy 有内容时直接复用，无需重建
- ✅ **内存优化**：`content_text` 已移除（v1.4.9），大型库内存占用降低约 40-50%
- ✅ **索引持久化**：postcard + redb，分批写入（1000条/事务），重启恢复 < 1s
- ✅ **Regex 预编译**：`lazy_static!` 全局缓存
- ✅ **搜索 Reader 复用**：Tantivy IndexReader 进程生命周期复用
- ✅ **WASM 加速**：客户端渲染（render_markdown）、离线搜索（NoteIndex）、Barnes-Hut 图谱布局（v1.6.x）；Bitset 优化 CJK 搜索速度（M3/M4/M5）
- ✅ **侧边栏虚拟渲染**：CSS content-visibility + requestIdleCallback，5000+ 文件时首屏性能大幅改善（v1.8.0）

### 🛠️ 运维功能
- ✅ **健康检查**：`/health` 端点（含 git_commit / sync_status / last_sync_at / duration_ms）
- ✅ **指标暴露**：`/metrics` Prometheus 格式（含 sync_duration_seconds / sync_last_timestamp）
- ✅ **定时自动同步**：`sync_interval_minutes` 配置后台定时触发
- ✅ **Webhook 触发**：支持 GitHub/GitLab Push 事件触发同步
- ✅ **配置热重载**：`POST /api/config/reload` 重新读取 config.ron
- ✅ **日志管理**：分级输出（控制台 + 文件），每日轮转
- ✅ **优雅关闭**：捕获信号，等待后台任务（Tantivy 重建、redb 持久化）完成后退出（v1.5.5）
- ✅ **实时同步进度**：SSE 推流（`/api/sync/events`），前端实时进度条；同步历史记录（`/api/sync/history`，v1.5.5）
- ✅ **多仓库支持**：单实例管理多个 Git 仓库，路由前缀 `/r/{name}/...`，仓库切换器（v1.7.4）
- ✅ **RSS/Atom 订阅**：`/feed.xml` 全库订阅，支持 tag/folder 过滤（v1.8.2）
- ✅ **静态站点导出**：`POST /api/export/html` 打包为 zip，可部署到 GitHub Pages（v1.8.2）
- ✅ **PDF 导出**：`@media print` 打印样式，笔记页"打印"按钮（v1.8.2）

### 🧪 质量保证
- ✅ **单元测试**：125 个服务端测试 + 38 个 WASM 测试 = 163 个测试（100% 通过），含 search_engine / handlers 集成测试
- ✅ **clippy 零警告**：`cargo clippy` 全量通过
- ✅ **错误处理**：统一错误类型，自动恢复机制
- ✅ **并发保护**：`/sync` 端点互斥锁，防止并发同步数据竞争

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
    database: (
        index_db_path: "./index.db",
        auth_db_path: "./auth.db",
        share_db_path: "./share.db",
        reading_progress_db_path: "./reading_progress.db",
    ),
    security: (
        auth_enabled: true,
        jwt_secret: "YOUR_RANDOM_SECRET_KEY_HERE",
        token_lifetime_hours: 24,
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
- `database`: 数据库路径配置（均有默认值，可省略）
  - `index_db_path`: 索引持久化数据库（默认 `./index.db`）
  - `auth_db_path`: 用户认证数据库（默认 `./auth.db`）
  - `share_db_path`: 分享链接数据库（默认 `./share.db`）
  - `reading_progress_db_path`: 阅读进度数据库（默认 `./reading_progress.db`）
- `security`: 认证配置
  - `auth_enabled`: 是否启用用户认证
  - `jwt_secret`: JWT 密钥（务必修改，建议 `openssl rand -base64 32` 生成）
  - `token_lifetime_hours`: 令牌有效期（小时，默认 24）
  - `default_admin_username`: 默认管理员用户名（仅首次初始化使用）
  - `default_admin_password`: 默认管理员密码（首次登录后请立即修改）

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
│   ├── main.rs                      # 服务器启动、路由注册、日志初始化
│   ├── lib.rs                       # 模块导出
│   ├── config.rs                    # 配置加载（RON 格式）
│   ├── domain.rs                    # 核心数据结构（Note、SidebarNode 等）
│   ├── state.rs                     # 全局应用状态（AppState）
│   ├── sync.rs                      # 同步管道（Git → 扫描 → 处理 → 索引）
│   ├── git.rs                       # Git 客户端（clone/pull/diff）
│   ├── scanner.rs                   # 文件扫描（遍历 .md 文件）
│   ├── markdown.rs                  # Markdown 处理（WikiLink、图片、TOC）
│   ├── indexer.rs                   # 索引构建（链接、反向链接、标签、文件）
│   ├── search_engine.rs             # Tantivy 全文搜索引擎
│   ├── persistence.rs               # 索引持久化（redb + postcard）
│   ├── handlers.rs                  # 通用 HTTP 处理器
│   ├── auth.rs / auth_db.rs         # JWT 认证 + 用户数据库
│   ├── auth_middleware.rs / auth_handlers.rs  # 认证中间件和接口
│   ├── share_db.rs / share_handlers.rs        # 分享链接
│   ├── reading_progress_db.rs / reading_progress_handlers.rs  # 阅读进度
│   ├── sidebar.rs / graph.rs / tags.rs        # 侧边栏、图谱、标签工具
│   ├── templates.rs                 # Askama 模板结构体定义
│   ├── metrics.rs                   # Prometheus 指标
│   └── error.rs                     # 统一错误类型
├── templates/                       # Askama HTML 模板（编译期渲染）
│   ├── layout.html                  # 基础布局（所有页面继承）
│   ├── page.html                    # 笔记页面
│   ├── index.html                   # 空知识库首页
│   ├── login.html / change_password.html
│   ├── tags_list.html / tag_notes.html
│   └── share.html
├── static/
│   ├── css/                         # 模块化样式（variables、layout、markdown 等）
│   └── js/                          # 模块化脚本（search、graph、toc、i18n 等）
├── docs/
│   ├── ROADMAP.md                   # 版本规划
│   └── CHANGELOG.md                 # 变更历史
├── config.example.ron               # 配置文件模板
├── Dockerfile / docker-compose.yml
└── Cargo.toml
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
| GET | `/orphans` | 孤立笔记列表（无出链且无入链） |
| GET | `/random` | 随机跳转到一篇笔记 |
| GET | `/recent` | 最近更新笔记列表（`?days=` 参数） |
| GET | `/api/suggest` | 搜索建议自动补全（`?q=`，v1.5.2） |
| GET | `/api/sync/events` | SSE 实时同步进度流（`text/event-stream`，v1.5.5） |
| GET | `/api/sync/history` | 最近 10 次同步历史记录（v1.5.5） |
| GET | `/graph` | 全局知识图谱专页（全屏，聚类着色，v1.7.0） |
| GET | `/insights` | 笔记洞察 Dashboard（写作趋势/健康度/标签云，v1.7.3） |
| GET | `/api/insights/stats` | 洞察统计数据 JSON（v1.7.3） |
| GET | `/doc/{path}/history` | 笔记 Git 提交历史（v1.7.2） |
| GET | `/doc/{path}/at/{commit}` | 历史版本快照（v1.7.2） |
| GET | `/doc/{path}/diff/{commit}` | 提交 diff（行级 HTML，v1.7.2） |
| GET | `/api/vaults` | 所有仓库列表（v1.7.4 多仓库） |
| ANY | `/r/{name}/...` | 多仓库路由前缀（v1.7.4） |
| GET | `/api/search?page=1&per_page=20` | 搜索分页（v1.8.0） |
| GET | `/timeline` | 时间线视图（v1.8.4） |
| GET | `/api/timeline` | 时间线数据 JSON（v1.8.4） |
| GET | `/feed.xml` | Atom 1.0 订阅（`?tag=`/`?folder=`，v1.8.2） |
| POST | `/api/export/html` | 静态站点 zip 导出（v1.8.2） |

**认证相关端点**（需启用认证）：
| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/login` | 登录页面 |
| POST | `/api/auth/login` | 用户登录 |
| POST | `/api/auth/logout` | 用户登出 |
| GET | `/change-password` | 修改密码页面 |
| POST | `/api/auth/change-password` | 修改密码 |
| GET | `/api/auth/current-user` | 获取当前用户信息 |

**管理员端点**（需 admin 角色，v1.5.3）：
| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/admin/users` | 用户管理页面 |
| GET/POST | `/api/admin/users` | 用户列表/创建 |
| DELETE/POST | `/api/admin/users/{username}` | 禁用/重置密码 |

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
| POST | `/api/reading/progress` | 保存阅读进度 |
| GET | `/api/reading/progress` | 获取所有笔记进度列表 |
| GET | `/api/reading/progress/{note_path}` | 获取指定笔记阅读进度 |
| DELETE | `/api/reading/progress/{note_path}` | 删除指定笔记阅读进度 |
| POST | `/api/reading/history` | 添加阅读历史记录 |
| GET | `/api/reading/history` | 获取阅读历史列表 |

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

完整的版本变更历史请查看 [docs/CHANGELOG.md](docs/CHANGELOG.md)。

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
- `notes: HashMap<Path, Note>` — 笔记内容和元数据（v1.4.9 起不含原始 Markdown，内存减半）
- `link_index: HashMap<Title, Path>` — 标题到路径的映射
- `backlinks: HashMap<Title, Vec<Title>>` — 反向链接关系（基于 `outgoing_links` 全量构建）
- `file_index: HashMap<Filename, Path>` — 文件名到路径的映射（资源文件）
- `sidebar: Vec<SidebarNode>` — 层级目录树

## 未来计划

查看完整的开发路线图：[ROADMAP.md](ROADMAP.md)

### 已完成（v1.3.x – v1.7.4）

v1.3.x–v1.7.4 系列完成了全面的功能增强、安全加固、性能优化和代码质量提升：
- **v1.3.x**：CODEREVIEW 全部 Bug/安全/性能/质量修复
- **v1.4.0–v1.4.5**：键盘快捷键、主题定制、内容渲染增强（KaTeX/Callout/灯箱）、搜索与发现（`/orphans`/`/random`/`/recent`）、图谱增强（全库图谱）、PWA、运维扩展
- **v1.4.6–v1.4.9**：安全加固（Cookie/HMAC）、性能优化（重启跳过索引重建）、clippy 零警告、移除 content_text（内存减半）
- **v1.5.x**：AppConfig 热重载（RwLock）、redb IO 异步化、多用户 JWT 角色（admin/editor/viewer）、用户管理页、SSE 实时同步进度、同步历史记录、优雅关闭后台任务跟踪
- **v1.6.x**：WASM crate（wasm-bindgen）、浏览器端 render_markdown、PWA 离线搜索（NoteIndex）、Barnes-Hut 图谱布局、filterNotes、TOC 生成、Bitset 优化（M3/M4/M5）、θ 自适应
- **v1.7.x**：全局图谱专页（`/graph`，聚类着色）、笔记洞察 Dashboard（`/insights`，写作趋势/健康度/标签云）、Git 版本历史（`/doc/{path}/history`、快照、diff）、多仓库支持（`/r/{name}/...`，VaultRegistry）
- **v1.8.x**：规模化性能优化（搜索分页/侧边栏虚拟渲染/图谱渐进式）、导出与发布（PDF/RSS/静态站）、PWA 离线完善、可视化增强（时间线/热力图/洞察排行）、依赖升级（tantivy 0.26/redb 4.0 等）、性能回归修复（WASM M4 回退 -90%）

查看完整规划：[ROADMAP.md](ROADMAP.md)

### 不支持的功能

本项目专注于**只读展示**和**高性能浏览**，以下功能不在支持范围内：

- ❌ **在线编辑**：请使用 Obsidian 客户端编辑，本项目通过 Git 同步获取更新
- ❌ **评论系统**：专注于笔记展示，不添加社交功能
- ❌ **协作编辑**：不支持多人实时编辑
- ❌ **插件系统**：保持简洁，不引入插件机制
- ❌ **笔记导出**：笔记存储在 Git 仓库中，可直接访问原始 Markdown 文件或使用 Obsidian 导出
- ❌ **键盘快捷键自定义**：保持简洁设计，当前快捷键已满足基本需求
- ❌ **复杂统计分析**：不属于核心浏览功能，可使用外部工具分析

## 许可

本项目为个人工具，仅供私有使用。

---

**项目地址**: https://github.com/your-username/obsidian_mirror  
**当前版本**: v1.8.7 🎉  
**开发路线图**: [ROADMAP.md](ROADMAP.md)  
**最后更新**: 2026-04-15
