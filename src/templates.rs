// Askama 模板定义
use crate::domain::{BreadcrumbItem, FlatNode, TocItem};
use askama::Template;
use crate::git::CommitInfo;

/// 页面模板（用于显示单个笔记）
#[derive(Template)]
#[template(path = "page.html")]
pub struct PageTemplate<'a> {
    pub title: &'a str,
    pub note_title: &'a str,
    pub note_path: &'a str, // 笔记路径（用于收藏功能）
    pub content: &'a str,
    pub sidebar: &'a [FlatNode],
    pub backlinks: &'a [String],
    pub toc: &'a [TocItem],
    pub breadcrumbs: &'a [BreadcrumbItem], // 面包屑导航
}

/// 首页模板（用于空知识库或主页）
#[derive(Template)]
#[template(path = "index.html")]
pub struct IndexTemplate<'a> {
    pub title: &'a str,
    pub sidebar: &'a [FlatNode],
    pub backlinks: &'a [String],
}

/// 标签列表模板（显示所有标签）
#[derive(Template)]
#[template(path = "tags_list.html")]
pub struct TagsListTemplate<'a> {
    pub title: &'a str,
    pub sidebar: &'a [FlatNode],
    pub backlinks: &'a [String],
    pub tags: &'a [(String, usize)], // (标签名, 笔记数量)
}

/// 单个标签笔记列表模板
#[derive(Template)]
#[template(path = "tag_notes.html")]
pub struct TagNotesTemplate<'a> {
    pub title: &'a str,
    pub sidebar: &'a [FlatNode],
    pub backlinks: &'a [String],
    pub tag_name: &'a str,
    pub notes: &'a [(String, String)], // (笔记标题, 路径)
}

/// 分享页面模板
#[derive(Template)]
#[template(path = "share.html")]
pub struct ShareTemplate<'a> {
    pub note_title: &'a str,
    pub content_html: &'a str,
    pub creator: &'a str,
    pub created_at: &'a str,
    pub visit_count: u32,
}

/// 孤立笔记列表模板（无出链且无入链的笔记）
#[derive(Template)]
#[template(path = "orphans.html")]
pub struct OrphansTemplate<'a> {
    pub title: &'a str,
    pub sidebar: &'a [crate::domain::FlatNode],
    pub backlinks: &'a [String],
    /// (笔记标题, 相对路径) 列表
    pub orphans: &'a [(String, String)],
}

/// 最近更新笔记列表模板（按修改时间降序）
#[derive(Template)]
#[template(path = "recent_notes_page.html")]
pub struct RecentNotesPageTemplate<'a> {
    pub title: &'a str,
    pub sidebar: &'a [crate::domain::FlatNode],
    pub backlinks: &'a [String],
    /// (笔记标题, 相对路径, mtime Unix 时间戳) 列表
    pub notes: &'a [(String, String, i64)],
    /// 展示范围（天数）
    pub days: u64,
}

/// 全局知识图谱专页模板（v1.7.0）
///
/// GET /graph 路由的独立全屏图谱页，支持全局图谱与单笔记子图切换。
#[derive(Template)]
#[template(path = "graph_page.html")]
pub struct GraphPageTemplate<'a> {
    pub title: &'a str,
    pub sidebar: &'a [crate::domain::FlatNode],
    pub backlinks: &'a [String],
}

/// 管理员用户管理页面模板（v1.5.3）
#[derive(Template)]
#[template(path = "admin_users.html")]
pub struct AdminUsersTemplate<'a> {
    pub title: &'a str,
    /// 侧边栏节点（layout.html 需要）
    pub sidebar: &'a [crate::domain::FlatNode],
    /// 反向链接（layout.html 需要，管理页传空切片）
    pub backlinks: &'a [String],
    /// (用户名, 角色字符串, 是否启用) 列表
    pub users: &'a [(String, String, bool)],
}

/// 笔记提交历史列表页模板（v1.7.2）
#[derive(Template)]
#[template(path = "history.html")]
pub struct NoteHistoryTemplate<'a> {
    pub title: &'a str,
    pub sidebar: &'a [crate::domain::FlatNode],
    pub backlinks: &'a [String],
    /// 笔记标题（用于页面顶部展示）
    pub note_title: &'a str,
    /// 笔记相对路径（用于跳转链接）
    pub note_path: &'a str,
    /// 提交历史列表（时间降序）
    pub commits: &'a [CommitInfo],
}

/// 历史版本快照页模板（v1.7.2）
#[derive(Template)]
#[template(path = "history_at.html")]
pub struct NoteHistoryAtTemplate<'a> {
    pub title: &'a str,
    pub sidebar: &'a [crate::domain::FlatNode],
    pub backlinks: &'a [String],
    /// 笔记标题
    pub note_title: &'a str,
    /// 笔记相对路径
    pub note_path: &'a str,
    /// 提交元信息（快照所属提交）
    pub commit: &'a CommitInfo,
    /// 已渲染的历史版本 HTML
    pub content_html: &'a str,
    /// 目录
    pub toc: &'a [TocItem],
}

/// 提交 diff 页模板（v1.7.2）
#[derive(Template)]
#[template(path = "history_diff.html")]
pub struct NoteHistoryDiffTemplate<'a> {
    pub title: &'a str,
    pub sidebar: &'a [crate::domain::FlatNode],
    pub backlinks: &'a [String],
    /// 笔记标题
    pub note_title: &'a str,
    /// 笔记相对路径
    pub note_path: &'a str,
    /// 提交元信息
    pub commit: &'a CommitInfo,
    /// 已渲染的 diff HTML（带行颜色标记）
    pub diff_html: &'a str,
}
