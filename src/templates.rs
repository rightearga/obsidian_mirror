// Askama 模板定义
use crate::domain::{BreadcrumbItem, FlatNode, TocItem};
use askama::Template;

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
