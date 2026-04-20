// HTTP 路由处理器
use actix_web::{web, HttpResponse, Responder, get, post};
use actix_files;
use std::sync::Arc;
use serde::Deserialize;
use tracing::error;
use askama::Template;

use crate::state::AppState;
use crate::sync::perform_sync;
use crate::sidebar::{flatten_sidebar, find_first_file};
use crate::templates::{PageTemplate, IndexTemplate, TagsListTemplate, TagNotesTemplate, GraphPageTemplate, NoteHistoryTemplate, NoteHistoryAtTemplate, NoteHistoryDiffTemplate, InsightsTemplate, KnowledgeMapTemplate};
use crate::git::{GitClient, CommitInfo};
use crate::search_engine::SortBy;
use crate::graph::generate_graph;
use crate::domain::BreadcrumbItem;
use crate::markdown::MarkdownProcessor;
use std::collections::HashMap;
use crate::domain::Note;

/// 展开笔记内嵌占位符为可折叠 HTML（v1.5.4）。
///
/// 扫描 HTML 中的 `<div class="note-embed-placeholder" ...></div>`，
/// 查找目标笔记内容并替换为 `<details class="note-embed">` 折叠块。
/// `depth` 参数防止循环内嵌，最多展开 2 层。
pub fn expand_embeds(
    html: &str,
    notes: &HashMap<String, Note>,
    link_index: &HashMap<String, String>,
    depth: u8,
) -> String {
    use regex::Regex;
    use lazy_static::lazy_static;
    lazy_static! {
        static ref EMBED_REGEX: Regex = Regex::new(
            r#"<div class="note-embed-placeholder" data-embed-title="([^"]*)" data-embed-section="([^"]*)"></div>"#
        ).unwrap();
    }

    if depth >= 2 {
        // 深度超限：以说明文字替代，防止无限递归
        return EMBED_REGEX.replace_all(html, |caps: &regex::Captures| {
            let title = percent_encoding::percent_decode_str(&caps[1])
                .decode_utf8()
                .unwrap_or_default()
                .to_string();
            format!(
                r#"<div class="note-embed-depth-limit"><em>⚠️ 内嵌深度超限（最多 2 层）：{}</em></div>"#,
                MarkdownProcessor::html_escape_text(&title)
            )
        }).to_string();
    }

    EMBED_REGEX.replace_all(html, |caps: &regex::Captures| {
        let encoded_title = &caps[1];
        let encoded_section = &caps[2];

        // 解码目标笔记标题/路径
        let target = percent_encoding::percent_decode_str(encoded_title)
            .decode_utf8()
            .unwrap_or_default()
            .to_string();
        let section = percent_encoding::percent_decode_str(encoded_section)
            .decode_utf8()
            .unwrap_or_default()
            .to_string();

        // 去除可能的 .md 扩展名后查找笔记
        let title_without_ext = if target.to_lowercase().ends_with(".md") {
            &target[..target.len() - 3]
        } else {
            &target
        };

        // 查找目标笔记：先 link_index（标题→路径），再直接路径匹配
        let note_key = link_index.get(title_without_ext)
            .or_else(|| link_index.get(&target))
            .cloned()
            .or_else(|| if notes.contains_key(&target) { Some(target.clone()) } else { None })
            .or_else(|| {
                let with_md = format!("{}.md", target);
                if notes.contains_key(&with_md) { Some(with_md) } else { None }
            });

        if let Some(key) = note_key {
            if let Some(note) = notes.get(&key) {
                // 如果有章节锚点，提取目标章节内容（简化：截取到下一个同级标题）
                let content_html = if section.is_empty() {
                    note.content_html.clone()
                } else {
                    // 尝试通过 id 锚点提取章节（简单实现：直接展示全文，前端可通过 #anchor 跳转）
                    note.content_html.clone()
                };

                // 递归展开嵌套内嵌（深度 +1）
                let expanded = expand_embeds(&content_html, notes, link_index, depth + 1);

                let section_hint = if !section.is_empty() {
                    format!(" § {}", MarkdownProcessor::html_escape_text(&section))
                } else {
                    String::new()
                };

                format!(
                    r#"<details class="note-embed" open>
  <summary class="note-embed-title">
    <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="flex-shrink:0">
      <path d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z"/>
      <polyline points="14 2 14 8 20 8"/>
    </svg>
    <a href="/doc/{path}" class="note-embed-link">{title}{section}</a>
  </summary>
  <div class="note-embed-content markdown-body">{content}</div>
</details>"#,
                    path = percent_encoding::utf8_percent_encode(&key, percent_encoding::NON_ALPHANUMERIC),
                    title = MarkdownProcessor::html_escape_text(&note.title),
                    section = section_hint,
                    content = expanded,
                )
            } else {
                format!(
                    r#"<div class="note-embed-missing">⚠️ 笔记内容不可用：{}</div>"#,
                    MarkdownProcessor::html_escape_text(&target)
                )
            }
        } else {
            format!(
                r#"<div class="note-embed-missing">⚠️ 找不到笔记：{}</div>"#,
                MarkdownProcessor::html_escape_text(&target)
            )
        }
    }).to_string()
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: String,
    #[serde(default)]
    pub sort_by: SortBy,  // 排序方式（默认按相关度）
    #[serde(default)]
    pub tags: Option<String>, // 标签过滤（逗号分隔多个标签）
    #[serde(default)]
    pub folder: Option<String>, // 文件夹过滤
    #[serde(default)]
    pub date_from: Option<i64>, // 日期过滤：开始时间（Unix 时间戳秒）
    #[serde(default)]
    pub date_to: Option<i64>,   // 日期过滤：结束时间（Unix 时间戳秒）
    /// 页码，1-based（v1.8.0 分页）
    #[serde(default = "default_page")]
    pub page: usize,
    /// 每页条数，最大 100（v1.8.0 分页）
    #[serde(default = "default_per_page")]
    pub per_page: usize,
}

fn default_page()     -> usize { 1  }
fn default_per_page() -> usize { 20 }

#[derive(Debug, Deserialize)]
pub struct GraphQuery {
    pub note: String,     // 笔记标题
    #[serde(default = "default_depth")]
    pub depth: usize,     // 图谱深度（1-3 层）
}

fn default_depth() -> usize {
    2 // 默认显示 2 层
}

/// 根据笔记路径生成面包屑导航
/// 
/// 例如：路径 "文件夹1/子文件夹/笔记.md" 生成：
/// [
///   { name: "首页", path: Some("/") },
///   { name: "文件夹1", path: None },  // 文件夹暂时不可点击
///   { name: "子文件夹", path: None },
///   { name: "笔记", path: None }      // 当前页面
/// ]
fn generate_breadcrumbs(note_path: &str, note_title: &str) -> Vec<BreadcrumbItem> {
    let mut breadcrumbs = vec![
        BreadcrumbItem {
            name: "首页".to_string(),
            path: Some("/".to_string()),
        }
    ];
    
    // 分割路径，排除文件名（最后一个部分）
    let path_parts: Vec<&str> = note_path.split('/').collect();
    
    // 如果只有文件名（没有子目录），直接添加当前页面
    if path_parts.len() == 1 {
        breadcrumbs.push(BreadcrumbItem {
            name: note_title.to_string(),
            path: None, // 当前页面不可点击
        });
        return breadcrumbs;
    }
    
    // 添加中间的文件夹
    for folder in &path_parts[..path_parts.len() - 1] {
        breadcrumbs.push(BreadcrumbItem {
            name: folder.to_string(),
            path: None, // 文件夹暂时不可点击（未来可以添加文件夹视图）
        });
    }
    
    // 添加当前页面
    breadcrumbs.push(BreadcrumbItem {
        name: note_title.to_string(),
        path: None,
    });
    
    breadcrumbs
}

/// POST /sync - 触发 Git 同步并重新处理所有笔记
///
/// 使用 sync_lock 防止并发同步请求导致 Tantivy IndexWriter 冲突和数据竞争。
/// 若同步已在进行中，返回 409 Conflict。
/// v1.5.3：需要 admin 角色（认证启用时）。
#[post("/sync")]
pub async fn sync_handler(req: actix_web::HttpRequest, data: web::Data<Arc<AppState>>) -> impl Responder {
    // v1.5.3：角色检查——admin 才能触发同步（auth 未启用时无角色注入，全放行）
    use actix_web::HttpMessage;
    use crate::auth_db::UserRole;
    if let Some(role) = req.extensions().get::<UserRole>()
        && !role.is_admin() {
        return HttpResponse::Forbidden().json(serde_json::json!({
            "error": "触发同步需要管理员权限"
        }));
    }
    // 使用 try_lock 防止并发同步：若已有同步在进行，立即返回 409
    let _guard = match data.sync_lock.try_lock() {
        Ok(guard) => guard,
        Err(_) => {
            return HttpResponse::Conflict().body("同步正在进行中，请稍后再试");
        }
    };

    // B1 修复：记录失败同步的历史（成功的记录已由 perform_sync 内部追加）
    let sync_start = std::time::Instant::now();
    let start_ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    match perform_sync(&data).await {
        Ok(_) => HttpResponse::Ok().body("同步成功"),
        Err(e) => {
            error!("同步失败: {:?}", e);
            // 记录失败同步记录（perform_sync 内只记录成功情况）
            let end_ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            let notes_count = data.notes.read().await.len();
            let record = crate::sync::SyncRecord {
                started_at: start_ts,
                finished_at: end_ts,
                notes_count,
                status: "failed".to_string(),
                error_msg: Some(e.to_string()),
                duration_ms: sync_start.elapsed().as_millis() as u64,
            };
            let mut history = data.sync_history.write().await;
            if history.len() >= 10 { history.pop_front(); }
            history.push_back(record);
            HttpResponse::InternalServerError().body(format!("同步失败: {}", e))
        }
    }
}

/// GET /api/search — 搜索笔记，支持分页（v1.8.0）
///
/// 响应格式：
/// ```json
/// {"results":[...],"total":150,"page":1,"per_page":20,"total_pages":8}
/// ```
/// `page` 和 `per_page` 默认值分别为 1 和 20。空查询直接返回空结果页。
#[get("/api/search")]
pub async fn search_handler(
    query: web::Query<SearchQuery>,
    data: web::Data<Arc<AppState>>,
) -> impl Responder {
    use crate::search_engine::SearchPage;

    let search_term = query.q.trim();

    if search_term.is_empty() && query.tags.is_none() && query.folder.is_none() {
        return HttpResponse::Ok().json(SearchPage {
            results: vec![], total: 0, page: 1, per_page: query.per_page, total_pages: 0,
        });
    }

    // 解析标签参数（逗号分隔）
    let tags = query.tags.as_ref().and_then(|t| {
        let tag_list: Vec<String> = t
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if tag_list.is_empty() { None } else { Some(tag_list) }
    });

    // v1.8.0：使用分页搜索
    match data.search_engine.advanced_search_paginated(
        search_term,
        query.page,
        query.per_page,
        query.sort_by,
        tags,
        query.folder.clone(),
        query.date_from,
        query.date_to,
    ) {
        Ok(page) => HttpResponse::Ok().json(page),
        Err(e) => {
            error!("搜索失败: {:?}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("搜索失败: {}", e)
            }))
        }
    }
}

/// GET /api/graph - 获取笔记关系图谱数据
#[get("/api/graph")]
pub async fn graph_handler(
    query: web::Query<GraphQuery>,
    data: web::Data<Arc<AppState>>,
) -> impl Responder {
    let note_title = query.note.trim();
    
    if note_title.is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "笔记标题不能为空"
        }));
    }
    
    // 限制深度范围
    let depth = query.depth.clamp(1, 3);
    
    tracing::info!("🔍 图谱请求: 笔记='{}', 深度={}", note_title, depth);
    
    let notes = data.notes.read().await;
    let link_index = data.link_index.read().await;
    
    tracing::info!("📊 当前笔记总数: {}, 链接索引数: {}", notes.len(), link_index.len());
    
    // 生成图谱数据
    let graph_data = generate_graph(note_title, &notes, &link_index, depth);
    
    tracing::info!("✅ 生成图谱: {} 个节点, {} 条边", graph_data.nodes.len(), graph_data.edges.len());
    
    HttpResponse::Ok().json(graph_data)
}

/// GET / - 首页处理器
/// 
/// 逻辑：
/// 1. 尝试查找 README.md 或 index.md 作为首页
/// 2. 如果没有，重定向到第一个文件
/// 3. 如果都没有，显示空知识库页面
#[get("/")]
pub async fn index_handler(data: web::Data<Arc<AppState>>) -> impl Responder {
    let sidebar = data.sidebar.read().await;
    let notes = data.notes.read().await;
    let flat_sidebar = flatten_sidebar(&sidebar);
    
    // 1. Try to find README.md or index.md in root
    let candidates = ["README.md", "Readme.md", "readme.md", "index.md", "Index.md"];
    for name in candidates {
        if let Some(note) = notes.get(name) {
             let backlinks_map = data.backlinks.read().await;
             let empty_vec = Vec::new();
             let note_backlinks = backlinks_map.get(&note.title).unwrap_or(&empty_vec);
             
             // 生成面包屑导航
             let breadcrumbs = generate_breadcrumbs(&note.path, &note.title);

             let tmpl = PageTemplate {
                title: &note.title,
                note_title: &note.title,
                note_path: &note.path,
                content: &note.content_html,
                sidebar: &flat_sidebar,
                backlinks: note_backlinks,
                toc: &note.toc,
                breadcrumbs: &breadcrumbs,
            };
            return match tmpl.render() {
                Ok(html) => HttpResponse::Ok().content_type("text/html").body(html),
                Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": format!("模板渲染失败: {}", e)})),
            };
        }
    }

    // 2. Redirect to first file
    if let Some(first_node) = find_first_file(&sidebar)
         && let Some(path) = &first_node.path {
             return HttpResponse::Found()
                .append_header(("Location", format!("/doc/{}", path)))
                .finish();
         }
    
    // 3. Render Index Template if empty
    let empty_backlinks: Vec<String> = Vec::new();
    
    let tmpl = IndexTemplate {
        title: "Home",
        sidebar: &flat_sidebar,
        backlinks: &empty_backlinks,
    };

    match tmpl.render() {
        Ok(html) => HttpResponse::Ok().content_type("text/html").body(html),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": format!("模板渲染失败: {}", e)})),
    }
}

/// 验证文件路径位于 base_dir 内，防止路径遍历攻击（`../../etc/passwd` 等）。
///
/// 使用 `canonicalize` 解析符号链接并消解 `..`，再验证结果路径以 base_dir 开头。
/// 仅在文件已存在时调用（canonicalize 要求路径在磁盘上存在）。
fn is_path_within(base_dir: &std::path::Path, target: &std::path::Path) -> bool {
    // canonicalize 解析 .. 并跟随符号链接，返回绝对规范路径
    let Ok(canonical_target) = std::fs::canonicalize(target) else {
        return false;
    };
    let Ok(canonical_base) = std::fs::canonicalize(base_dir) else {
        return false;
    };
    canonical_target.starts_with(&canonical_base)
}

/// GET /assets/{filename} - 静态资源处理器（图片、PDF 等）
///
/// 安全：对所有候选路径执行 `is_path_within` 校验，拒绝 `../../` 类路径遍历请求。
#[get("/assets/{filename:.*}")]
pub async fn assets_handler(
    filename: web::Path<String>,
    data: web::Data<Arc<AppState>>,
) -> actix_web::Result<actix_files::NamedFile> {
    let decoded_filename = match percent_encoding::percent_decode_str(&filename).decode_utf8() {
        Ok(s) => s.to_string(),
        Err(e) => {
            error!("❌ 文件名解码失败: {} - 错误: {:?}", filename, e);
            return Err(actix_web::error::ErrorBadRequest("Invalid UTF-8 in filename"));
        }
    };

    let local_path = data.config.read().unwrap().local_path.clone();

    // 先尝试直接访问（可能是完整路径）
    let direct_path = local_path.join(&decoded_filename);
    if direct_path.exists() && direct_path.is_file() {
        // 路径遍历防护：确保解析后的路径仍在 local_path 内
        if !is_path_within(&local_path, &direct_path) {
            error!("❌ 路径遍历攻击被拒绝: {}", decoded_filename);
            return Err(actix_web::error::ErrorForbidden("Access denied"));
        }
        return actix_files::NamedFile::open(direct_path)
            .map_err(actix_web::error::ErrorInternalServerError);
    }

    // 如果不是完整路径，查找文件索引
    let file_index = data.file_index.read().await;
    if let Some(full_path) = file_index.get(&decoded_filename) {
        let file_path = local_path.join(full_path);
        if file_path.exists() && file_path.is_file() {
            // 文件索引中的路径由系统构建，理论上已在 local_path 内，但仍做防御性检查
            if !is_path_within(&local_path, &file_path) {
                error!("❌ 文件索引路径遍历防护触发: {}", full_path);
                return Err(actix_web::error::ErrorForbidden("Access denied"));
            }
            return actix_files::NamedFile::open(file_path)
                .map_err(actix_web::error::ErrorInternalServerError);
        }
    }

    Err(actix_web::error::ErrorNotFound(format!(
        "File not found: {}",
        decoded_filename
    )))
}

/// GET /doc/{path} - 单个笔记页面处理器
#[get("/doc/{path:.*}")]
pub async fn doc_handler(path: web::Path<String>, data: web::Data<Arc<AppState>>) -> impl Responder {
    let raw_path = path.into_inner();
    
    // 正确解码 UTF-8 路径
    let decoded_path = match percent_encoding::percent_decode_str(&raw_path).decode_utf8() {
        Ok(s) => s.to_string(),
        Err(e) => {
            error!("❌ 路径解码失败: {} - 错误: {:?}", raw_path, e);
            return HttpResponse::BadRequest().body("Invalid UTF-8 in path");
        }
    };

    let notes = data.notes.read().await;
    let link_index = data.link_index.read().await;

    // Try to find the note key
    // 1. Direct match (e.g. "Folder/Note.md")
    // 2. Title match (e.g. "Note") -> Look up path
    
    let note_key = if notes.contains_key(&decoded_path) {
        Some(decoded_path.clone())
    } else if let Some(path) = link_index.get(&decoded_path) {
        Some(path.clone())
    } else {
        // Try appending .md
        let with_md = format!("{}.md", decoded_path);
        if notes.contains_key(&with_md) {
            Some(with_md)
        } else {
            None
        }
    };

    if let Some(key) = note_key {
        if let Some(note) = notes.get(&key) {
            let sidebar = data.sidebar.read().await;
            let flat_sidebar = flatten_sidebar(&sidebar);
            let backlinks_map = data.backlinks.read().await;
            let empty_vec = Vec::new();
            let note_backlinks = backlinks_map.get(&note.title).unwrap_or(&empty_vec);

            // 生成面包屑导航
            let breadcrumbs = generate_breadcrumbs(&note.path, &note.title);

            // v1.5.4：展开笔记内嵌占位符（![[笔记.md]] → 可折叠 HTML 块）
            let expanded_content = expand_embeds(&note.content_html, &notes, &link_index, 0);

            let tmpl = PageTemplate {
                title: &note.title,
                note_title: &note.title,
                note_path: &note.path,
                content: &expanded_content,
                sidebar: &flat_sidebar,
                backlinks: note_backlinks,
                toc: &note.toc,
                breadcrumbs: &breadcrumbs,
            };

            match tmpl.render() {
                Ok(html) => HttpResponse::Ok().content_type("text/html").body(html),
                Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": format!("模板渲染失败: {}", e)})),
            }
        } else {
            HttpResponse::NotFound().body("Note not found in map")
        }
    } else {
        HttpResponse::NotFound().body(format!("Note not found: {}", decoded_path))
    }
}

/// GET /tags - 标签列表页面处理器
#[get("/tags")]
pub async fn tags_list_handler(data: web::Data<Arc<AppState>>) -> impl Responder {
    let sidebar = data.sidebar.read().await;
    let flat_sidebar = flatten_sidebar(&sidebar);
    let tag_index = data.tag_index.read().await;
    
    // 构建标签列表，并计算每个标签的笔记数量
    let mut tags_with_count: Vec<(String, usize)> = tag_index
        .iter()
        .map(|(tag, notes)| (tag.clone(), notes.len()))
        .collect();
    
    // 按笔记数量降序排序
    tags_with_count.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    
    let empty_backlinks: Vec<String> = Vec::new();
    let tmpl = TagsListTemplate {
        title: "标签列表",
        sidebar: &flat_sidebar,
        backlinks: &empty_backlinks,
        tags: &tags_with_count,
    };
    
    match tmpl.render() {
        Ok(html) => HttpResponse::Ok().content_type("text/html").body(html),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": format!("模板渲染失败: {}", e)})),
    }
}

/// GET /tag/{tag_name} - 单个标签笔记列表处理器
#[get("/tag/{tag_name:.*}")]
pub async fn tag_notes_handler(
    tag_name: web::Path<String>,
    data: web::Data<Arc<AppState>>,
) -> impl Responder {
    let decoded_tag = match percent_encoding::percent_decode_str(&tag_name).decode_utf8() {
        Ok(s) => s.to_string(),
        Err(e) => {
            error!("❌ 标签名解码失败: {} - 错误: {:?}", tag_name, e);
            return HttpResponse::BadRequest().body("Invalid UTF-8 in tag name");
        }
    };
    
    let sidebar = data.sidebar.read().await;
    let flat_sidebar = flatten_sidebar(&sidebar);
    let tag_index = data.tag_index.read().await;
    let link_index = data.link_index.read().await;
    
    if let Some(note_titles) = tag_index.get(&decoded_tag) {
        // 构建笔记列表（标题 + 路径）
        let mut notes_info: Vec<(String, String)> = note_titles
            .iter()
            .filter_map(|title| {
                link_index.get(title).map(|path| (title.clone(), path.clone()))
            })
            .collect();
        
        // 按标题排序
        notes_info.sort_by(|a, b| a.0.cmp(&b.0));
        
        let empty_backlinks: Vec<String> = Vec::new();
        let tmpl = TagNotesTemplate {
            title: &format!("标签: {}", decoded_tag),
            sidebar: &flat_sidebar,
            backlinks: &empty_backlinks,
            tag_name: &decoded_tag,
            notes: &notes_info,
        };
        
        match tmpl.render() {
            Ok(html) => HttpResponse::Ok().content_type("text/html").body(html),
            Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": format!("模板渲染失败: {}", e)})),
        }
    } else {
        HttpResponse::NotFound().body(format!("标签未找到: {}", decoded_tag))
    }
}

/// GET /health - 健康检查端点
/// 
/// 返回应用健康状态，用于：
/// - 容器编排（Kubernetes/Docker）健康检查
/// - 负载均衡器探测
/// - 监控系统检查
/// 
/// 返回 JSON 格式：
/// ```json
/// {
///   "status": "healthy",
///   "version": "0.10.0",
///   "notes_count": 334,
///   "uptime_seconds": 1234
/// }
/// ```
/// GET /health — 健康检查端点（扩展版，含同步状态）
#[get("/health")]
pub async fn health_handler(data: web::Data<Arc<AppState>>) -> impl Responder {
    use crate::state::sync_status;
    use serde_json::json;
    use std::sync::atomic::Ordering;

    let uptime = data.start_time.elapsed().as_secs();

    let notes_count = data.notes.read().await.len();

    // 同步状态
    let last_sync_at = data.last_sync_at.load(Ordering::Relaxed);
    let last_sync_duration_ms = data.last_sync_duration_ms.load(Ordering::Relaxed);
    let raw_status = data.sync_status.load(Ordering::Relaxed);
    let sync_status_str = match raw_status {
        x if x == sync_status::RUNNING => "running",
        x if x == sync_status::FAILED  => "failed",
        _ => "idle",
    };

    // 当前 Git commit（从本地仓库读取）
    let local_path_for_git = data.config.read().unwrap().local_path.clone();
    let git_commit = crate::git::GitClient::get_current_commit(&local_path_for_git)
        .await
        .unwrap_or_default();

    // v1.5.5：附上最近一次同步历史记录
    let last_sync_record = data.sync_history.read().await
        .back()
        .and_then(|r| serde_json::to_value(r).ok());

    let health_info = json!({
        "status": "healthy",
        "version": env!("CARGO_PKG_VERSION"),
        "notes_count": notes_count,
        "uptime_seconds": uptime,
        "sync_status": sync_status_str,
        "last_sync_at": last_sync_at,
        "last_sync_duration_ms": last_sync_duration_ms,
        "last_sync_record": last_sync_record,
        "git_commit": git_commit,
        "components": {
            "notes_index": "ok",
            "search_engine": "ok",
            "persistence": "ok"
        }
    });

    HttpResponse::Ok().json(health_info)
}


/// GET /api/stats - 笔记统计信息
/// 
/// 返回笔记库统计信息，用于前端统计面板显示：
/// - 笔记总数
/// - 标签总数
/// - 最近更新笔记（最近 7 天）
/// - 最早/最新笔记时间
#[get("/api/stats")]
pub async fn stats_handler(data: web::Data<Arc<AppState>>) -> impl Responder {
    use serde_json::json;
    use std::time::{SystemTime, UNIX_EPOCH, Duration};
    
    let notes = data.notes.read().await;
    let tag_index = data.tag_index.read().await;
    
    // 笔记总数
    let total_notes = notes.len();
    
    // 标签总数
    let total_tags = tag_index.len();
    
    // 计算最近 7 天更新的笔记数
    let seven_days_ago = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_secs()
        .saturating_sub(7 * 24 * 60 * 60);
    
    let recent_updated = notes.values()
        .filter(|note| {
            note.mtime
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs() > seven_days_ago)
                .unwrap_or(false)
        })
        .count();
    
    // 找出最新和最早的笔记时间
    let mut oldest_time = None;
    let mut newest_time = None;
    
    for note in notes.values() {
        let time_secs = note.mtime
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .ok();
        
        if let Some(secs) = time_secs {
            oldest_time = Some(oldest_time.map_or(secs, |old: u64| old.min(secs)));
            newest_time = Some(newest_time.map_or(secs, |new: u64| new.max(secs)));
        }
    }
    
    // 构建统计响应
    let stats = json!({
        "total_notes": total_notes,
        "total_tags": total_tags,
        "recent_updated": recent_updated,
        "oldest_note_time": oldest_time,
        "newest_note_time": newest_time,
    });
    
    HttpResponse::Ok().json(stats)
}

/// GET /api/preview - 获取笔记预览内容
/// 
/// 返回笔记的简化 HTML 内容，用于悬浮预览卡片显示
/// 
/// 查询参数：
/// - path: 笔记路径或标题
/// 
/// 返回 JSON 格式：
/// ```json
/// {
///   "title": "笔记标题",
///   "content": "<p>预览内容...</p>",
///   "path": "folder/note.md"
/// }
/// ```
#[derive(Debug, Deserialize)]
pub struct PreviewQuery {
    pub path: String,
    /// 可选提交 hash（v1.7.2）：指定后返回该提交时的历史版本预览
    pub commit: Option<String>,
}

#[get("/api/preview")]
pub async fn preview_handler(
    query: web::Query<PreviewQuery>,
    data: web::Data<Arc<AppState>>,
) -> impl Responder {
    use serde_json::json;

    let decoded_path = match percent_encoding::percent_decode_str(&query.path).decode_utf8() {
        Ok(s) => s.to_string(),
        Err(e) => {
            error!("❌ 预览路径解码失败: {} - 错误: {:?}", query.path, e);
            return HttpResponse::BadRequest().json(json!({"error": "Invalid UTF-8 in path"}));
        }
    };

    // v1.7.2：commit 参数存在时返回历史版本预览
    if let Some(commit) = &query.commit {
        if !is_valid_commit_hash(commit) {
            return HttpResponse::BadRequest().json(json!({"error": "无效的 commit hash"}));
        }
        let local_path = data.config.read().unwrap().local_path.clone();
        let raw_md = match GitClient::get_file_at_commit(&decoded_path, commit, &local_path).await {
            Ok(s) => s,
            Err(_) => return HttpResponse::NotFound().json(json!({"error": "历史版本不存在"})),
        };
        let (content_html, _, _, _, _) = crate::markdown::MarkdownProcessor::process(&raw_md);
        let preview_content = truncate_html(&content_html, 500);
        // 从当前索引取标题（历史版本标题可能不同，降级为路径名）
        let title = {
            let notes = data.notes.read().await;
            notes.get(&decoded_path).map(|n| n.title.clone())
                .unwrap_or_else(|| decoded_path.clone())
        };
        return HttpResponse::Ok().json(json!({
            "title": title,
            "content": preview_content,
            "path": decoded_path,
        }));
    }

    let notes = data.notes.read().await;
    let link_index = data.link_index.read().await;

    // 查找笔记（与 doc_handler 逻辑一致）
    let note_key = if notes.contains_key(&decoded_path) {
        Some(decoded_path.clone())
    } else if let Some(path) = link_index.get(&decoded_path) {
        Some(path.clone())
    } else {
        let with_md = format!("{}.md", decoded_path);
        if notes.contains_key(&with_md) { Some(with_md) } else { None }
    };

    if let Some(key) = note_key
        && let Some(note) = notes.get(&key) {
            let preview_content = truncate_html(&note.content_html, 500);
            return HttpResponse::Ok().json(json!({
                "title": note.title,
                "content": preview_content,
                "path": note.path,
            }));
        }

    HttpResponse::NotFound().json(json!({
        "error": "笔记未找到",
        "path": decoded_path
    }))
}

/// 从 HTML 中提取纯文本并截取到指定字符数
///
/// 先去除 HTML 标签，确保截断基于可见字符数而非原始 HTML 长度，
/// 避免大量标签占用字符配额导致预览内容过少。
fn truncate_html(html: &str, max_chars: usize) -> String {
    // 简单状态机去除 HTML 标签，提取可见文本
    let mut text = String::with_capacity(html.len());
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                text.push(' '); // 标签位置插入空格，避免文字粘连
            }
            _ if !in_tag => text.push(c),
            _ => {}
        }
    }
    // 合并多余空白
    let text: String = text.split_whitespace().collect::<Vec<_>>().join(" ");

    if text.chars().count() <= max_chars {
        return text;
    }

    let truncated: String = text.chars().take(max_chars).collect();
    format!("{}...", truncated)
}

/// GET /orphans — 孤立笔记页面（无任何出链且无入链的笔记列表）
///
/// 孤立笔记定义：`outgoing_links` 为空 且 在 `backlinks` 中无对应入链
#[get("/orphans")]
pub async fn orphans_handler(data: web::Data<Arc<AppState>>) -> impl Responder {
    use crate::templates::OrphansTemplate;

    let notes = data.notes.read().await;
    let backlinks = data.backlinks.read().await;
    let sidebar = data.sidebar.read().await;
    let flat_sidebar = crate::sidebar::flatten_sidebar(&sidebar);

    // 孤立笔记：无出链 且 无人链接到它
    let mut orphan_list: Vec<(String, String)> = notes
        .values()
        .filter(|note| {
            note.outgoing_links.is_empty() && !backlinks.contains_key(&note.title)
        })
        .map(|note| (note.title.clone(), note.path.clone()))
        .collect();
    orphan_list.sort_by(|a, b| a.0.cmp(&b.0));

    let empty_backlinks: Vec<String> = Vec::new();
    let tmpl = OrphansTemplate {
        title: "孤立笔记",
        sidebar: &flat_sidebar,
        backlinks: &empty_backlinks,
        orphans: &orphan_list,
    };

    match tmpl.render() {
        Ok(html) => HttpResponse::Ok().content_type("text/html").body(html),
        Err(e) => HttpResponse::InternalServerError().body(format!("模板渲染错误: {}", e)),
    }
}

/// GET /random — 随机跳转到一篇笔记
///
/// 从当前笔记列表中随机选择一篇并重定向到 `/doc/{path}`
#[get("/random")]
pub async fn random_handler(data: web::Data<Arc<AppState>>) -> impl Responder {
    let notes = data.notes.read().await;

    if notes.is_empty() {
        return HttpResponse::ServiceUnavailable().body("笔记库尚未加载，请稍后再试");
    }

    // 使用当前时间戳作为随机种子（无需额外依赖）
    let idx = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as usize)
        .unwrap_or(0)
        % notes.len();

    let path = notes.values().nth(idx).map(|n| n.path.clone());
    drop(notes);

    match path {
        Some(p) => {
            let encoded = percent_encoding::utf8_percent_encode(&p, percent_encoding::NON_ALPHANUMERIC).to_string();
            HttpResponse::Found()
                .append_header(("Location", format!("/doc/{}", encoded)))
                .finish()
        }
        None => HttpResponse::ServiceUnavailable().body("无法选择随机笔记"),
    }
}

/// GET /recent — 最近更新笔记页面（按修改时间降序）
///
/// 查询参数 `days`（可选，默认 30）：展示最近 N 天内修改的笔记
#[derive(Debug, serde::Deserialize)]
pub struct RecentQuery {
    #[serde(default = "default_recent_days")]
    pub days: u64,
}

fn default_recent_days() -> u64 {
    30
}

#[get("/recent")]
pub async fn recent_page_handler(
    query: web::Query<RecentQuery>,
    data: web::Data<Arc<AppState>>,
) -> impl Responder {
    use crate::templates::RecentNotesPageTemplate;
    use std::time::{SystemTime, UNIX_EPOCH, Duration};

    let days = query.days.clamp(1, 365);
    let cutoff = SystemTime::now()
        .checked_sub(Duration::from_secs(days * 24 * 3600))
        .unwrap_or(UNIX_EPOCH);

    let notes = data.notes.read().await;
    let sidebar = data.sidebar.read().await;
    let flat_sidebar = crate::sidebar::flatten_sidebar(&sidebar);

    // 收集在 cutoff 之后修改的笔记，按修改时间降序
    let mut recent: Vec<(String, String, i64)> = notes
        .values()
        .filter(|n| n.mtime > cutoff)
        .map(|n| {
            let mtime_ts = n.mtime
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            (n.title.clone(), n.path.clone(), mtime_ts)
        })
        .collect();
    recent.sort_by(|a, b| b.2.cmp(&a.2)); // 降序

    let empty_backlinks: Vec<String> = Vec::new();
    let tmpl = RecentNotesPageTemplate {
        title: &format!("最近 {} 天更新的笔记", days),
        sidebar: &flat_sidebar,
        backlinks: &empty_backlinks,
        notes: &recent,
        days,
    };

    match tmpl.render() {
        Ok(html) => HttpResponse::Ok().content_type("text/html").body(html),
        Err(e) => HttpResponse::InternalServerError().body(format!("模板渲染错误: {}", e)),
    }
}

/// GET /api/graph/global — 返回全库关系图谱数据
///
/// 包含所有笔记及其链接关系，用于全库图谱视图。
/// 节点数超过 500 时自动降采样（仅保留有连接的节点）。
///
/// 查询参数：
/// - `hide_isolated`（可选，默认 false）：是否隐藏孤立节点
#[derive(Debug, serde::Deserialize)]
pub struct GlobalGraphQuery {
    #[serde(default)]
    pub hide_isolated: bool,
}

#[get("/api/graph/global")]
pub async fn global_graph_handler(
    query: web::Query<GlobalGraphQuery>,
    data: web::Data<Arc<AppState>>,
) -> impl Responder {
    let notes = data.notes.read().await;
    let link_index = data.link_index.read().await;

    let graph_data = crate::graph::generate_global_graph(
        &notes,
        &link_index,
        query.hide_isolated,
    );

    HttpResponse::Ok().json(graph_data)
}

/// GET /api/titles — 返回所有笔记标题、路径和标签，供前端自动补全使用
///
/// 前端在首次搜索框聚焦时获取，缓存于 sessionStorage 中。
/// `titles`：向后兼容，仅标题字符串列表；
/// `note_items`：新字段（v1.5.2），包含 title 和 path，可展示路径上下文。
#[get("/api/titles")]
pub async fn titles_api_handler(data: web::Data<Arc<AppState>>) -> impl Responder {
    let notes = data.notes.read().await;
    let tag_index = data.tag_index.read().await;

    // 保留向后兼容的 titles 字段
    let titles: Vec<&str> = notes.values().map(|n| n.title.as_str()).collect();
    let tags: Vec<&str> = tag_index.keys().map(|t| t.as_str()).collect();
    // note_items：包含 title/path，v1.8.4 新增 mtime（供图谱热力图使用）
    let note_items: Vec<serde_json::Value> = notes.values()
        .map(|n| {
            let mtime = n.mtime
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            serde_json::json!({"title": n.title, "path": n.path, "mtime": mtime})
        })
        .collect();

    HttpResponse::Ok().json(serde_json::json!({
        "titles": titles,
        "tags": tags,
        "note_items": note_items,
    }))
}

/// GET /api/sync/events — Server-Sent Events 端点，实时推送同步进度（v1.5.5）
///
/// 客户端连接后订阅 broadcast channel，同步执行期间接收各阶段进度事件（JSON Lines）。
/// 连接在 broadcast sender 关闭（应用退出）时自动断开；"done" / "error" 事件后前端可主动关闭。
#[get("/api/sync/events")]
pub async fn sync_events_handler(data: web::Data<Arc<AppState>>) -> impl Responder {
    use actix_web::web::Bytes;
    use futures_util::stream;
    use tokio::sync::broadcast::error::RecvError;

    let rx = data.sync_progress_tx.subscribe();

    // A1 修复：使用 (rx, finished) 二元组作为 unfold 状态，
    // "done"/"error" 事件发送后将 finished=true，下次调用时返回 None 关闭流，
    // 避免客户端连接在同步完成后永久挂起占用服务端资源。
    let event_stream = stream::unfold((rx, false), |(mut rx, finished)| async move {
        if finished {
            return None; // 上一轮已发出终止事件，关闭流
        }
        match rx.recv().await {
            Ok(event) => {
                // "done" 或 "error" 阶段是流的终止信号
                let is_final = event.stage == "done" || event.stage == "error";
                let json = serde_json::to_string(&event).unwrap_or_default();
                let sse_line = format!("data: {}\n\n", json);
                Some((
                    Ok::<Bytes, actix_web::Error>(Bytes::from(sse_line)),
                    (rx, is_final), // 若 is_final=true，下次调用返回 None
                ))
            }
            Err(RecvError::Closed) => None,  // 发送端关闭（应用退出），结束流
            Err(RecvError::Lagged(n)) => {
                // 接收过慢，跳过了若干消息，发送一条 lag 通知
                let lag_msg = format!("data: {{\"stage\":\"lag\",\"progress\":0,\"message\":\"跳过 {} 条消息\"}}\n\n", n);
                Some((Ok(Bytes::from(lag_msg)), (rx, false)))
            }
        }
    });

    HttpResponse::Ok()
        .content_type("text/event-stream; charset=utf-8")
        .insert_header(("Cache-Control", "no-cache, no-store"))
        .insert_header(("X-Accel-Buffering", "no")) // 禁用 Nginx 缓冲，确保实时推送
        .insert_header(("Connection", "keep-alive"))
        .streaming(event_stream)
}

/// GET /api/sync/history — 返回最近 10 次同步的历史记录（v1.5.5）
#[get("/api/sync/history")]
pub async fn sync_history_handler(data: web::Data<Arc<AppState>>) -> impl Responder {
    let history = data.sync_history.read().await;
    let records: Vec<&crate::sync::SyncRecord> = history.iter().rev().collect(); // 最新在前
    HttpResponse::Ok().json(serde_json::json!({ "history": records }))
}

/// 搜索建议查询参数
#[derive(Debug, serde::Deserialize)]
pub struct SuggestQuery {
    /// 搜索关键词
    pub q: String,
    /// 返回条数上限（默认 10，最大 20）
    #[serde(default = "default_suggest_limit")]
    pub limit: usize,
}

fn default_suggest_limit() -> usize { 10 }

/// GET /api/suggest — 搜索建议端点，返回模糊匹配的笔记标题和路径
///
/// 先通过内存前缀匹配快速过滤（大小写不敏感），
/// 再通过 Tantivy FuzzyTermQuery 补充编辑距离 ≤1 的模糊建议，合并去重后返回。
/// 返回格式：`[{"title": "...", "path": "..."}]`，按相关度降序。
#[get("/api/suggest")]
pub async fn suggest_handler(
    query: web::Query<SuggestQuery>,
    data: web::Data<Arc<AppState>>,
) -> impl Responder {
    let q = query.q.trim().to_lowercase();
    if q.is_empty() {
        return HttpResponse::Ok().json(Vec::<serde_json::Value>::new());
    }

    let limit = query.limit.min(20);

    // 1. 内存前缀/包含匹配（快速路径）
    let notes = data.notes.read().await;
    let mut seen_paths = std::collections::HashSet::new();
    let mut results: Vec<serde_json::Value> = Vec::new();

    // 优先返回标题前缀匹配的结果
    for note in notes.values() {
        if note.title.to_lowercase().starts_with(&q) {
            results.push(serde_json::json!({"title": note.title, "path": note.path}));
            seen_paths.insert(note.path.clone());
        }
    }
    // 其次返回标题包含匹配的结果（非前缀）
    for note in notes.values() {
        if results.len() >= limit { break; }
        if !seen_paths.contains(&note.path) && note.title.to_lowercase().contains(&q) {
            results.push(serde_json::json!({"title": note.title, "path": note.path}));
            seen_paths.insert(note.path.clone());
        }
    }
    drop(notes);

    // 2. 若内存匹配结果不足，用 FuzzyTermQuery 补充（Tantivy 模糊匹配，容错编辑距离 1）
    if results.len() < limit {
        let remaining = limit - results.len();
        if let Ok(fuzzy_results) = data.search_engine.fuzzy_suggest(&q, remaining + seen_paths.len()) {
            for (title, path, _score) in fuzzy_results {
                if results.len() >= limit { break; }
                if !seen_paths.contains(&path) {
                    results.push(serde_json::json!({"title": title, "path": path}));
                    seen_paths.insert(path);
                }
            }
        }
    }

    HttpResponse::Ok().json(results)
}

/// POST /webhook/sync — Webhook 触发同步端点
///
/// 支持 GitHub Push Event（`X-Hub-Signature-256`）和 GitLab Push Hook（`X-Gitlab-Token`）。
/// 仅当 `config.webhook.enabled = true` 且签名/令牌验证通过时才触发同步。
pub async fn webhook_sync_handler(
    req: actix_web::HttpRequest,
    body: web::Bytes,
    data: web::Data<Arc<AppState>>,
) -> impl Responder {
    let (webhook_enabled, webhook_secret) = {
        let cfg = data.config.read().unwrap();
        (cfg.webhook.enabled, cfg.webhook.secret.clone())
    };
    if !webhook_enabled {
        return HttpResponse::Forbidden().body("Webhook 未启用");
    }

    let secret = &webhook_secret;
    if secret.is_empty() {
        return HttpResponse::InternalServerError().body("Webhook 密钥未配置");
    }

    // 验证 GitHub 签名（X-Hub-Signature-256）
    if let Some(sig_header) = req.headers().get("X-Hub-Signature-256") {
        let sig = sig_header.to_str().unwrap_or("");
        if !verify_github_signature(secret, &body, sig) {
            return HttpResponse::Unauthorized().body("GitHub 签名验证失败");
        }
    }
    // 验证 GitLab 令牌（X-Gitlab-Token）
    else if let Some(token_header) = req.headers().get("X-Gitlab-Token") {
        let token = token_header.to_str().unwrap_or("");
        if token != secret {
            return HttpResponse::Unauthorized().body("GitLab 令牌验证失败");
        }
    } else {
        return HttpResponse::Unauthorized().body("缺少认证头（X-Hub-Signature-256 或 X-Gitlab-Token）");
    }

    // 尝试触发同步（若已在同步中则跳过）
    let _guard = match data.sync_lock.try_lock() {
        Ok(g) => g,
        Err(_) => return HttpResponse::Conflict().body("同步正在进行中，跳过本次触发"),
    };

    tracing::info!("📡 Webhook 触发同步");
    // B1 修复：记录失败同步的历史
    let wh_start = std::time::Instant::now();
    let wh_start_ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    match crate::sync::perform_sync(&data).await {
        Ok(_) => HttpResponse::Ok().body("同步完成"),
        Err(e) => {
            tracing::error!("Webhook 触发同步失败: {:?}", e);
            let end_ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            let notes_count = data.notes.read().await.len();
            let record = crate::sync::SyncRecord {
                started_at: wh_start_ts,
                finished_at: end_ts,
                notes_count,
                status: "failed".to_string(),
                error_msg: Some(e.to_string()),
                duration_ms: wh_start.elapsed().as_millis() as u64,
            };
            let mut history = data.sync_history.write().await;
            if history.len() >= 10 { history.pop_front(); }
            history.push_back(record);
            HttpResponse::InternalServerError().body(format!("同步失败: {}", e))
        }
    }
}

/// 使用 HMAC-SHA256 验证 GitHub Webhook 签名
///
/// `signature` 格式为 `sha256=<hex>`，使用常数时间比较防止时序攻击。
fn verify_github_signature(secret: &str, body: &[u8], signature: &str) -> bool {
    use hmac::Mac;
    use sha2::Sha256;
    // hmac 0.13：使用 SimpleHmac + KeyInit::new_from_slice
    use hmac::digest::KeyInit;
    type HmacSha256 = hmac::SimpleHmac<Sha256>;

    let prefix = "sha256=";
    if !signature.starts_with(prefix) {
        return false;
    }
    let sig_hex = &signature[prefix.len()..];

    let sig_bytes = match hex_decode(sig_hex) {
        Some(b) => b,
        None => return false,
    };

    // 计算 HMAC-SHA256 并使用常数时间比较（防时序攻击）
    let mut mac = match HmacSha256::new_from_slice(secret.as_bytes()) {
        Ok(m) => m,
        Err(_) => return false,
    };
    mac.update(body);
    mac.verify_slice(&sig_bytes).is_ok()
}

/// 将十六进制字符串解码为字节序列
fn hex_decode(hex: &str) -> Option<Vec<u8>> {
    if !hex.len().is_multiple_of(2) {
        return None;
    }
    hex.as_bytes()
        .chunks(2)
        .map(|c| {
            let hi = (c[0] as char).to_digit(16)? as u8;
            let lo = (c[1] as char).to_digit(16)? as u8;
            Some((hi << 4) | lo)
        })
        .collect()
}

/// POST /api/config/reload — 配置热重载端点（需认证）
///
/// 从磁盘重新读取 `config.ron`，更新 `ignore_patterns` 等运行时配置，
/// 并触发一次完整同步以应用新的忽略规则。
/// 不支持热更新的字段：`listen_addr`、`repo_url`（需重启服务器）。
pub async fn config_reload_handler(
    req: actix_web::HttpRequest,
    data: web::Data<Arc<AppState>>,
) -> impl Responder {
    use actix_web::HttpMessage;
    // 仅允许已认证用户调用
    if req.extensions().get::<String>().is_none() {
        return HttpResponse::Unauthorized().json(serde_json::json!({
            "error": "未认证，请先登录"
        }));
    }
    // v1.5.3：角色检查——config_reload 需要 admin 权限
    use crate::auth_db::UserRole;
    if let Some(role) = req.extensions().get::<UserRole>()
        && !role.is_admin() {
        return HttpResponse::Forbidden().json(serde_json::json!({
            "error": "配置热重载需要管理员权限"
        }));
    }

    // 读取新配置
    let new_config = match crate::config::AppConfig::load("config.ron") {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("热重载配置失败: {:?}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("配置文件读取失败: {}", e)
            }));
        }
    };

    // 记录变更
    let (old_patterns, patterns_changed) = {
        let cfg = data.config.read().unwrap();
        let changed = cfg.ignore_patterns != new_config.ignore_patterns;
        (cfg.ignore_patterns.clone(), changed)
    };
    let _ = old_patterns; // 仅用于日志，可扩展
    tracing::info!("🔄 配置热重载：ignore_patterns 变更 = {}", patterns_changed);

    // B2 修复：将新配置写入 AppState.config（真正实现热重载）
    *data.config.write().unwrap() = new_config;
    tracing::info!("✅ 新配置已应用到运行时状态");

    // 触发完整同步以应用新配置（ignoring 持久化缓存）
    let _guard = match data.sync_lock.try_lock() {
        Ok(g) => g,
        Err(_) => return HttpResponse::Conflict().json(serde_json::json!({
            "error": "同步正在进行中，请稍后重试"
        })),
    };

    match crate::sync::perform_sync(&data).await {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({
            "message": "配置热重载完成，同步已触发",
            "note": "listen_addr 和 repo_url 的变更需要重启服务器才能生效"
        })),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("重载后同步失败: {}", e)
        })),
    }
}

/// 图谱最短路径 API 查询参数（v1.9.2）
#[derive(Debug, Deserialize)]
pub struct PathQuery {
    /// 起点笔记标识符（标题或路径）
    pub from: String,
    /// 终点笔记标识符（标题或路径）
    pub to: String,
}

/// GET /api/graph/path — BFS 最短路径查找（v1.9.2）
///
/// 在笔记链接图中寻找 `from` 到 `to` 的最短路径（最多 6 跳）。
///
/// 响应格式：
/// ```json
/// {"nodes":[...],"edges":[...],"hops":3,"message":null}
/// // 无路径时：
/// {"nodes":[],"edges":[],"hops":0,"message":"这两篇笔记之间暂无链接路径（最多 6 跳）"}
/// ```
#[get("/api/graph/path")]
pub async fn graph_path_handler(
    query: web::Query<PathQuery>,
    data:  web::Data<Arc<AppState>>,
) -> impl Responder {
    let from = query.from.trim();
    let to   = query.to.trim();

    if from.is_empty() || to.is_empty() {
        return HttpResponse::BadRequest().json(
            serde_json::json!({"error": "from 和 to 参数不能为空"})
        );
    }

    let notes      = data.notes.read().await;
    let link_index = data.link_index.read().await;

    match crate::graph::find_shortest_path(from, to, &notes, &link_index, 6) {
        Some((graph_data, hops)) => {
            HttpResponse::Ok().json(serde_json::json!({
                "nodes":   graph_data.nodes,
                "edges":   graph_data.edges,
                "hops":    hops,
                "message": serde_json::Value::Null,
            }))
        }
        None => {
            HttpResponse::Ok().json(serde_json::json!({
                "nodes":   [],
                "edges":   [],
                "hops":    0,
                "message": "这两篇笔记之间暂无链接路径（最多 6 跳）",
            }))
        }
    }
}

/// GET /graph — 全局知识图谱专页（v1.7.0）
///
/// 独立全屏图谱页面，支持全局图谱与单笔记子图切换。
/// 工具栏包含：搜索框（高亮节点）、深度选择器、孤立节点开关、聚类着色开关。
#[get("/graph")]
pub async fn graph_page_handler(
    data: web::Data<Arc<AppState>>,
) -> impl Responder {
    let sidebar_data = data.sidebar.read().await;
    let flat_sidebar = flatten_sidebar(&sidebar_data);
    let backlinks_empty: Vec<String> = vec![];

    let tmpl = GraphPageTemplate {
        title: "知识图谱",
        sidebar: &flat_sidebar,
        backlinks: &backlinks_empty,
    };
    match tmpl.render() {
        Ok(html) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html),
        Err(e) => {
            error!("图谱专页模板渲染失败: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "模板渲染失败"}))
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// v1.7.4：多仓库支持
// ──────────────────────────────────────────────────────────────────────────────

/// GET /api/vaults — 返回所有已配置仓库的名称列表（v1.7.4）
///
/// 供前端仓库切换器读取。返回格式：
/// ```json
/// {"vaults": [{"name": "personal", "is_primary": true}, ...]}
/// ```
#[get("/api/vaults")]
pub async fn vaults_list_handler(
    vault_registry: web::Data<Arc<crate::state::VaultRegistry>>,
) -> impl Responder {
    let vaults: Vec<serde_json::Value> = vault_registry.vaults.iter().enumerate()
        .map(|(i, (name, _))| serde_json::json!({
            "name": name,
            "is_primary": i == 0,
        }))
        .collect();
    HttpResponse::Ok().json(serde_json::json!({ "vaults": vaults }))
}

// ──────────────────────────────────────────────────────────────────────────────
// v1.7.3：笔记洞察 Dashboard
// ──────────────────────────────────────────────────────────────────────────────

/// GET /insights — 笔记洞察 Dashboard 主页（v1.7.3）
///
/// 展示写作趋势、知识库健康度（孤立/断链/超大笔记）和标签云。
/// 数据来自每次同步后缓存的 `InsightsCache`，页面本身为静态 HTML，
/// 图表由内嵌 JS 读取 `/api/insights/stats` 动态渲染。
#[get("/insights")]
pub async fn insights_page_handler(
    data: web::Data<Arc<AppState>>,
) -> impl Responder {
    let sidebar_data = data.sidebar.read().await;
    let flat_sidebar = flatten_sidebar(&sidebar_data);
    let backlinks_empty: Vec<String> = vec![];

    let tmpl = InsightsTemplate {
        title: "笔记洞察",
        sidebar: &flat_sidebar,
        backlinks: &backlinks_empty,
    };
    match tmpl.render() {
        Ok(html) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html),
        Err(e) => {
            error!("洞察页面模板渲染失败: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "模板渲染失败"}))
        }
    }
}

/// GET /api/insights/stats — 笔记洞察统计数据（v1.7.3）
///
/// 返回完整 `InsightsCache` 的 JSON 序列化，供前端图表读取。
/// 若尚未同步（缓存为空），返回默认零值结构。
#[get("/api/insights/stats")]
pub async fn insights_stats_handler(
    data: web::Data<Arc<AppState>>,
) -> impl Responder {
    let cache = data.insights_cache.read().await;
    HttpResponse::Ok().json(&*cache)
}

// ──────────────────────────────────────────────────────────────────────────────
// v1.8.4：可视化增强 — 时间线视图
// ──────────────────────────────────────────────────────────────────────────────

/// 从 serde_yaml::Value 中提取字符串，兼容 String 和 Tagged（YAML timestamp）两种变体
fn yaml_value_as_str(val: &serde_yaml::Value) -> Option<&str> {
    match val {
        serde_yaml::Value::String(s) => Some(s.as_str()),
        // serde_yaml 0.9 将无引号的 YAML timestamp（如 2024-01-15 12:00:00）解析为
        // Value::Tagged，内部仍是 String；直接用 as_str() 会返回 None 导致回退到 mtime
        serde_yaml::Value::Tagged(tagged) => {
            if let serde_yaml::Value::String(s) = &tagged.value {
                Some(s.as_str())
            } else {
                None
            }
        }
        _ => None,
    }
}

/// 从笔记的 frontmatter 提取日期字符串，若无则回退到 mtime
///
/// 按优先级依次尝试：`date` → `created` → `created_at` → mtime
fn extract_note_date_str(note: &crate::domain::Note) -> String {
    use chrono::{DateTime, Utc};

    for field in &["date", "created", "created_at"] {
        if let Some(val) = note.frontmatter.0.get(*field) {
            if let Some(s) = yaml_value_as_str(val) {
                let n = s.len().min(10);
                if n >= 7 {
                    return s[..n].replace('/', "-");
                }
            }
        }
    }

    // 回退到 mtime（git checkout 时间，精度较低）
    let dt: DateTime<Utc> = note.mtime.into();
    dt.format("%Y-%m-%d").to_string()
}

/// GET /timeline — 时间线视图页面（v1.8.4）
///
/// 展示按时间排列的笔记轴，前端 JS 从 `/api/timeline` 获取数据渲染。
/// 支持按月/年折叠、标签过滤、悬停预览。
#[get("/timeline")]
pub async fn timeline_page_handler(
    data: web::Data<Arc<AppState>>,
) -> impl Responder {
    let sidebar_data = data.sidebar.read().await;
    let flat_sidebar = flatten_sidebar(&sidebar_data);
    let backlinks_empty: Vec<String> = vec![];

    let tmpl = crate::templates::TimelineTemplate {
        title: "时间线",
        sidebar: &flat_sidebar,
        backlinks: &backlinks_empty,
    };
    match tmpl.render() {
        Ok(html) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html),
        Err(e) => {
            error!("时间线模板渲染失败: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "模板渲染失败"}))
        }
    }
}

/// GET /api/timeline — 时间线数据 API（v1.8.4）
///
/// 返回所有笔记的时间线数据（按日期降序），每条包含：
/// - `title`：笔记标题
/// - `path`：笔记路径
/// - `date`：`YYYY-MM-DD`（优先 frontmatter date，否则 mtime）
/// - `tags`：标签列表
/// - `mtime`：Unix 时间戳秒
#[get("/api/timeline")]
pub async fn timeline_api_handler(
    data: web::Data<Arc<AppState>>,
) -> impl Responder {
    let notes = data.notes.read().await;

    let mut items: Vec<serde_json::Value> = notes.values().map(|n| {
        let date  = extract_note_date_str(n);
        let mtime = n.mtime
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        serde_json::json!({
            "title": n.title,
            "path":  n.path,
            "date":  date,
            "tags":  n.tags,
            "mtime": mtime,
        })
    }).collect();

    // 按日期降序排列（最新在前）
    items.sort_by(|a, b| {
        let da = a["date"].as_str().unwrap_or("");
        let db = b["date"].as_str().unwrap_or("");
        db.cmp(da)
    });

    HttpResponse::Ok().json(items)
}

// ──────────────────────────────────────────────────────────────────────────────
// v1.8.2：导出与发布
// ──────────────────────────────────────────────────────────────────────────────

/// 将字符串中的 XML 特殊字符转义
fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
     .replace('<', "&lt;")
     .replace('>', "&gt;")
     .replace('"', "&quot;")
}

/// SystemTime → RFC 3339 字符串（Atom feed 使用）
fn to_rfc3339(t: std::time::SystemTime) -> String {
    use chrono::{DateTime, Utc};
    let dt: DateTime<Utc> = t.into();
    dt.format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

/// RSS/Atom 订阅查询参数
#[derive(Debug, Deserialize)]
pub struct FeedQuery {
    /// 按标签过滤（可选）
    pub tag:    Option<String>,
    /// 按文件夹前缀过滤（可选）
    pub folder: Option<String>,
}

/// GET /feed.xml — Atom 1.0 订阅（v1.8.2）
///
/// 返回全库最近 50 篇笔记（按 mtime 降序），支持按标签或文件夹过滤。
/// `<content>` 包含完整渲染 HTML，标准 RSS 阅读器可订阅。
#[get("/feed.xml")]
pub async fn feed_handler(
    query: web::Query<FeedQuery>,
    data:  web::Data<Arc<AppState>>,
) -> impl Responder {
    let notes  = data.notes.read().await;
    let config = data.config.read().unwrap().clone();
    let base   = config.public_base_url.as_deref().unwrap_or("http://localhost:8080").trim_end_matches('/');

    // 过滤 + 排序
    let mut filtered: Vec<&crate::domain::Note> = notes.values().collect();
    if let Some(tag) = &query.tag {
        filtered.retain(|n| n.tags.iter().any(|t| t == tag));
    }
    if let Some(folder) = &query.folder {
        let prefix = format!("{}/", folder.trim_end_matches('/'));
        filtered.retain(|n| n.path.starts_with(&prefix) || n.path == *folder);
    }
    filtered.sort_by(|a, b| b.mtime.cmp(&a.mtime));
    filtered.truncate(50);

    let now = to_rfc3339(std::time::SystemTime::now());

    let mut xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title>Obsidian Mirror</title>
  <link href="{base}/" rel="alternate"/>
  <link href="{base}/feed.xml" rel="self"/>
  <updated>{now}</updated>
  <id>{base}/</id>
"#
    );

    for note in &filtered {
        let encoded_path = percent_encoding::utf8_percent_encode(
            &note.path, percent_encoding::NON_ALPHANUMERIC,
        ).to_string();
        let link    = format!("{}/doc/{}", base, encoded_path);
        let updated = to_rfc3339(note.mtime);
        xml.push_str(&format!(
            r#"  <entry>
    <title>{}</title>
    <link href="{}"/>
    <id>{}</id>
    <updated>{}</updated>
    <content type="html"><![CDATA[{}]]></content>
  </entry>
"#,
            xml_escape(&note.title),
            link,
            link,
            updated,
            note.content_html,  // CDATA 块无需转义
        ));
    }
    xml.push_str("</feed>\n");

    HttpResponse::Ok()
        .content_type("application/atom+xml; charset=utf-8")
        .body(xml)
}

/// 将笔记路径转换为静态导出的 HTML 文件路径
/// 例如：`folder/note.md` → `folder/note.html`
fn note_path_to_html(note_path: &str) -> String {
    if let Some(stem) = note_path.strip_suffix(".md") {
        format!("{}.html", stem)
    } else {
        format!("{}.html", note_path)
    }
}

/// 生成静态站点导出用的最小化独立 HTML
fn build_static_note_html(note: &crate::domain::Note, all_notes: &[(&str, &str)]) -> String {
    // all_notes: Vec<(path, title)> 排序后的列表，用于生成侧边栏
    let nav_links: String = all_notes.iter().map(|(path, title)| {
        let html_path = note_path_to_html(path);
        format!("<li><a href=\"/{}\">{}</a></li>", html_path, xml_escape(title))
    }).collect::<Vec<_>>().join("\n");

    let css = r#"body{display:flex;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;
margin:0;background:#fff;color:#222;line-height:1.75;}
nav{width:230px;min-height:100vh;padding:12px 8px;border-right:1px solid #e5e7eb;
font-size:12.5px;overflow-y:auto;flex-shrink:0;position:sticky;top:0;max-height:100vh;}
nav h2{font-size:13px;color:#6b7280;margin:0 0 8px;padding:0 4px;}
nav ul{list-style:none;margin:0;padding:0;}
nav li a{display:block;padding:3px 8px;border-radius:4px;text-decoration:none;
color:#374151;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;}
nav li a:hover{background:#f3f4f6;}
main{flex:1;padding:32px 40px;max-width:780px;}
h1{font-size:1.7rem;margin-top:0;}
pre{background:#f8f9fa;padding:12px;border-radius:4px;overflow-x:auto;}
code{background:#f3f4f6;padding:1px 4px;border-radius:3px;font-size:0.9em;}
blockquote{border-left:3px solid #d1d5db;margin:0;padding:0 14px;color:#6b7280;}
img{max-width:100%;height:auto;}
table{border-collapse:collapse;width:100%;}
th,td{border:1px solid #e5e7eb;padding:6px 10px;}
@media(max-width:600px){body{flex-direction:column;}
nav{width:100%;min-height:auto;max-height:200px;border-right:none;border-bottom:1px solid #e5e7eb;}}"#;

    format!(
        r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>{title} - Obsidian Mirror Export</title>
<style>{css}</style>
</head>
<body>
<nav>
  <h2>所有笔记</h2>
  <ul>{nav}</ul>
</nav>
<main>
<h1>{title}</h1>
{content}
</main>
</body>
</html>"#,
        title   = xml_escape(&note.title),
        css     = css,
        nav     = nav_links,
        content = note.content_html,
    )
}

/// POST /api/export/html — 静态站点 zip 导出（v1.8.2）
///
/// 将整个 vault 渲染为自包含的静态 HTML 文件树，打包为 zip 下载。
/// 可直接部署到 GitHub Pages、Netlify 等静态托管平台。
/// 不包含搜索、认证等服务端功能；内部链接指向 `/doc/PATH`，需 web 服务器才能生效。
#[post("/api/export/html")]
pub async fn export_html_handler(
    data: web::Data<Arc<AppState>>,
) -> impl Responder {
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;
    use std::io::{Cursor, Write};

    let notes = data.notes.read().await;

    // 排序后的笔记列表（供侧边栏导航使用）
    let mut sorted_notes: Vec<(&str, &str)> = notes.iter()
        .map(|(path, note)| (path.as_str(), note.title.as_str()))
        .collect();
    sorted_notes.sort_by_key(|(path, _)| *path);

    // 构建 zip
    let cursor = Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(cursor);
    let opts = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    // 生成每篇笔记的 HTML
    for (path, note) in notes.iter() {
        let html_path = note_path_to_html(path);
        let html = build_static_note_html(note, &sorted_notes);
        if let Err(e) = zip.start_file(&html_path, opts) {
            error!("zip start_file 失败 {}: {:?}", html_path, e);
            continue;
        }
        // v1.8.7 B1：记录写入失败，避免静默产生损坏条目
        if let Err(e) = zip.write_all(html.as_bytes()) {
            error!("zip write_all 失败 {}: {:?}", html_path, e);
        }
    }

    // index.html：笔记列表首页
    let index_links: String = sorted_notes.iter().map(|(path, title)| {
        let html_path = note_path_to_html(path);
        format!("<li><a href=\"{}\">{}</a></li>", html_path, xml_escape(title))
    }).collect::<Vec<_>>().join("\n");

    let index_html = format!(
        r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Obsidian Mirror Export</title>
<style>body{{font-family:sans-serif;max-width:800px;margin:32px auto;padding:0 16px;}}
h1{{font-size:1.5rem;}}ul{{list-style:none;padding:0;}}
li a{{display:block;padding:6px 8px;border-radius:4px;text-decoration:none;color:#374151;}}
li a:hover{{background:#f3f4f6;}}</style>
</head>
<body>
<h1>Obsidian Mirror 笔记导出</h1>
<p style="color:#6b7280">共 {} 篇笔记</p>
<ul>{}</ul>
</body>
</html>"#,
        sorted_notes.len(),
        index_links,
    );

    let _ = zip.start_file("index.html", opts);
    let _ = zip.write_all(index_html.as_bytes());

    // README.md
    let readme = "# Obsidian Mirror 静态导出\n\n将此 zip 解压后部署到任意 web 服务器（GitHub Pages / Netlify 等）即可浏览。\n\n- 笔记内部链接需 web 服务器才能正常工作\n- 搜索/认证等功能不包含在静态导出中\n";
    let _ = zip.start_file("README.md", opts);
    let _ = zip.write_all(readme.as_bytes());

    let result = match zip.finish() {
        Ok(c)  => c,
        Err(e) => {
            error!("zip 打包失败: {:?}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({"error": "打包失败"}));
        }
    };

    HttpResponse::Ok()
        .content_type("application/zip")
        .insert_header(("Content-Disposition", "attachment; filename=\"obsidian-mirror-export.zip\""))
        .body(result.into_inner())
}

// ──────────────────────────────────────────────────────────────────────────────
// v1.7.2：Git 版本历史查看
// ──────────────────────────────────────────────────────────────────────────────

/// 验证 commit hash 是否合法（仅允许十六进制字符，长度 4–64）。
///
/// 防止将用户输入的 commit 参数直接传入 git 命令时产生路径注入风险。
fn is_valid_commit_hash(hash: &str) -> bool {
    let len = hash.len();
    len >= 4 && len <= 64 && hash.chars().all(|c| c.is_ascii_hexdigit())
}

/// 将 unified diff 文本渲染为带颜色标记的 HTML 表格。
///
/// 新增行（`+`）渲染为绿色，删除行（`-`）渲染为红色，
/// hunk 标头（`@@`）渲染为蓝色，上下文行为默认色。
/// 所有内容均经过 HTML 转义，防止 XSS。
fn render_diff_html(diff: &str) -> String {
    use crate::markdown::MarkdownProcessor;
    let mut html = String::from("<table class=\"diff-table\"><tbody>");
    for line in diff.lines() {
        // 跳过文件头行（--- / +++ / diff --git 等）
        if line.starts_with("--- ") || line.starts_with("+++ ") || line.starts_with("diff ") {
            continue;
        }
        if line.starts_with("@@ ") {
            html.push_str(&format!(
                "<tr class=\"diff-hunk\"><td colspan=\"2\"><code>{}</code></td></tr>",
                MarkdownProcessor::html_escape_text(line)
            ));
        } else if let Some(rest) = line.strip_prefix('+') {
            html.push_str(&format!(
                "<tr class=\"diff-add\"><td class=\"diff-marker\">+</td><td><code>{}</code></td></tr>",
                MarkdownProcessor::html_escape_text(rest)
            ));
        } else if let Some(rest) = line.strip_prefix('-') {
            html.push_str(&format!(
                "<tr class=\"diff-del\"><td class=\"diff-marker\">-</td><td><code>{}</code></td></tr>",
                MarkdownProcessor::html_escape_text(rest)
            ));
        } else {
            let text = line.strip_prefix(' ').unwrap_or(line);
            html.push_str(&format!(
                "<tr class=\"diff-ctx\"><td class=\"diff-marker\"> </td><td><code>{}</code></td></tr>",
                MarkdownProcessor::html_escape_text(text)
            ));
        }
    }
    html.push_str("</tbody></table>");
    html
}

/// GET /doc/{path}/history — 笔记提交历史列表（v1.7.2）
///
/// 展示指定笔记所有历史提交（时间降序）。
/// 未被 Git 追踪的文件显示空列表。
pub async fn note_history_handler(
    req: actix_web::HttpRequest,
    data: web::Data<Arc<AppState>>,
) -> impl Responder {
    // 从路径中提取笔记相对路径（去掉 /doc/ 前缀和 /history 后缀）
    let full_path = req.path();
    let note_path_raw = full_path
        .strip_prefix("/doc/")
        .and_then(|p| p.strip_suffix("/history"))
        .unwrap_or("");

    let note_path = percent_encoding::percent_decode_str(note_path_raw)
        .decode_utf8()
        .unwrap_or_default()
        .replace('\\', "/");

    if note_path.is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({"error": "路径不能为空"}));
    }

    let local_path = data.config.read().unwrap().local_path.clone();
    let commits = match GitClient::get_file_history(&note_path, &local_path).await {
        Ok(v) => v,
        Err(e) => {
            error!("获取文件历史失败: {:?}", e);
            vec![]
        }
    };

    // 从 notes 获取标题（用于页面显示）
    let note_title = {
        let notes = data.notes.read().await;
        notes.get(&note_path).map(|n| n.title.clone()).unwrap_or_else(|| note_path.clone())
    };

    let sidebar_data = data.sidebar.read().await;
    let flat_sidebar = flatten_sidebar(&sidebar_data);
    let backlinks_empty: Vec<String> = vec![];
    let page_title = format!("{} — 历史", note_title);

    let tmpl = NoteHistoryTemplate {
        title: &page_title,
        sidebar: &flat_sidebar,
        backlinks: &backlinks_empty,
        note_title: &note_title,
        note_path: &note_path,
        commits: &commits,
    };
    match tmpl.render() {
        Ok(html) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html),
        Err(e) => {
            error!("历史列表模板渲染失败: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "模板渲染失败"}))
        }
    }
}

/// GET /doc/{path}/at/{commit} — 历史版本快照（v1.7.2）
///
/// 展示指定提交时的笔记内容（Markdown → HTML 渲染）。
/// commit 参数必须为合法十六进制字符串，否则返回 400。
pub async fn note_history_at_handler(
    req: actix_web::HttpRequest,
    data: web::Data<Arc<AppState>>,
) -> impl Responder {
    let full_path = req.path();

    // 提取 note_path 和 commit（路径格式：/doc/{path}/at/{commit}）
    let after_doc = match full_path.strip_prefix("/doc/") {
        Some(s) => s,
        None => return HttpResponse::BadRequest().finish(),
    };
    // 从右侧找 /at/
    let at_pos = match after_doc.rfind("/at/") {
        Some(p) => p,
        None => return HttpResponse::BadRequest().finish(),
    };
    let note_path_raw = &after_doc[..at_pos];
    let commit_raw = &after_doc[at_pos + 4..]; // 跳过 "/at/"

    let note_path = percent_encoding::percent_decode_str(note_path_raw)
        .decode_utf8()
        .unwrap_or_default()
        .replace('\\', "/");
    let commit = percent_encoding::percent_decode_str(commit_raw)
        .decode_utf8()
        .unwrap_or_default()
        .to_string();

    if note_path.is_empty() || !is_valid_commit_hash(&commit) {
        return HttpResponse::BadRequest().json(serde_json::json!({"error": "参数无效"}));
    }

    let local_path = data.config.read().unwrap().local_path.clone();

    // 获取历史内容
    let raw_md = match GitClient::get_file_at_commit(&note_path, &commit, &local_path).await {
        Ok(s) => s,
        Err(e) => {
            error!("获取历史快照失败: {:?}", e);
            return HttpResponse::NotFound().json(serde_json::json!({"error": "无法获取该历史版本"}));
        }
    };

    // 渲染 Markdown
    let (content_html, _, _, _, toc) = crate::markdown::MarkdownProcessor::process(&raw_md);

    // 获取提交元信息（从历史列表中查找；若找不到则构造最小信息）
    let commits = GitClient::get_file_history(&note_path, &local_path).await.unwrap_or_default();
    let commit_info = commits.iter()
        .find(|c| c.hash.starts_with(&commit) || commit.starts_with(&c.hash))
        .cloned()
        .unwrap_or_else(|| CommitInfo {
            hash: commit.clone(),
            hash_short: commit.chars().take(8).collect(),
            author: String::new(),
            date: String::new(),
            subject: String::new(),
        });

    let note_title = {
        let notes = data.notes.read().await;
        notes.get(&note_path).map(|n| n.title.clone()).unwrap_or_else(|| note_path.clone())
    };

    let sidebar_data = data.sidebar.read().await;
    let flat_sidebar = flatten_sidebar(&sidebar_data);
    let backlinks_empty: Vec<String> = vec![];
    let page_title = format!("{} @ {}", note_title, &commit_info.hash_short);

    let tmpl = NoteHistoryAtTemplate {
        title: &page_title,
        sidebar: &flat_sidebar,
        backlinks: &backlinks_empty,
        note_title: &note_title,
        note_path: &note_path,
        commit: &commit_info,
        content_html: &content_html,
        toc: &toc,
    };
    match tmpl.render() {
        Ok(html) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html),
        Err(e) => {
            error!("历史快照模板渲染失败: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "模板渲染失败"}))
        }
    }
}

/// GET /doc/{path}/diff/{commit} — 提交 diff 查看（v1.7.2）
///
/// 展示指定提交与其上一提交的行级差异，渲染为 HTML 表格。
/// commit 参数必须为合法十六进制字符串，否则返回 400。
pub async fn note_history_diff_handler(
    req: actix_web::HttpRequest,
    data: web::Data<Arc<AppState>>,
) -> impl Responder {
    let full_path = req.path();

    let after_doc = match full_path.strip_prefix("/doc/") {
        Some(s) => s,
        None => return HttpResponse::BadRequest().finish(),
    };
    let diff_pos = match after_doc.rfind("/diff/") {
        Some(p) => p,
        None => return HttpResponse::BadRequest().finish(),
    };
    let note_path_raw = &after_doc[..diff_pos];
    let commit_raw = &after_doc[diff_pos + 6..]; // 跳过 "/diff/"

    let note_path = percent_encoding::percent_decode_str(note_path_raw)
        .decode_utf8()
        .unwrap_or_default()
        .replace('\\', "/");
    let commit = percent_encoding::percent_decode_str(commit_raw)
        .decode_utf8()
        .unwrap_or_default()
        .to_string();

    if note_path.is_empty() || !is_valid_commit_hash(&commit) {
        return HttpResponse::BadRequest().json(serde_json::json!({"error": "参数无效"}));
    }

    let local_path = data.config.read().unwrap().local_path.clone();

    let diff_text = match GitClient::get_file_diff(&note_path, &commit, &local_path).await {
        Ok(s) => s,
        Err(e) => {
            error!("获取 diff 失败: {:?}", e);
            return HttpResponse::NotFound().json(serde_json::json!({"error": "无法获取 diff"}));
        }
    };

    let diff_html = render_diff_html(&diff_text);

    // 获取提交元信息
    let commits = GitClient::get_file_history(&note_path, &local_path).await.unwrap_or_default();
    let commit_info = commits.iter()
        .find(|c| c.hash.starts_with(&commit) || commit.starts_with(&c.hash))
        .cloned()
        .unwrap_or_else(|| CommitInfo {
            hash: commit.clone(),
            hash_short: commit.chars().take(8).collect(),
            author: String::new(),
            date: String::new(),
            subject: String::new(),
        });

    let note_title = {
        let notes = data.notes.read().await;
        notes.get(&note_path).map(|n| n.title.clone()).unwrap_or_else(|| note_path.clone())
    };

    let sidebar_data = data.sidebar.read().await;
    let flat_sidebar = flatten_sidebar(&sidebar_data);
    let backlinks_empty: Vec<String> = vec![];
    let page_title = format!("{} — Diff {}", note_title, &commit_info.hash_short);

    let tmpl = NoteHistoryDiffTemplate {
        title: &page_title,
        sidebar: &flat_sidebar,
        backlinks: &backlinks_empty,
        note_title: &note_title,
        note_path: &note_path,
        commit: &commit_info,
        diff_html: &diff_html,
    };
    match tmpl.render() {
        Ok(html) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html),
        Err(e) => {
            error!("Diff 模板渲染失败: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "模板渲染失败"}))
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// v1.9.5：知识地图（方向 C）
// ──────────────────────────────────────────────────────────────────────────────

/// GET /knowledge-map — 知识地图专页（v1.9.5）
///
/// 全屏 Canvas 渲染，由前端调用 WASM `computeKnowledgeMap` 计算布局，
/// 按标签相似度聚类，支持平移/缩放/悬停/点击。
#[get("/knowledge-map")]
pub async fn knowledge_map_page_handler(
    data: web::Data<Arc<AppState>>,
) -> impl Responder {
    let sidebar_data = data.sidebar.read().await;
    let flat_sidebar = flatten_sidebar(&sidebar_data);
    let backlinks_empty: Vec<String> = vec![];

    let tmpl = KnowledgeMapTemplate {
        title:     "知识地图",
        sidebar:   &flat_sidebar,
        backlinks: &backlinks_empty,
    };
    match tmpl.render() {
        Ok(html) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html),
        Err(e) => {
            error!("知识地图模板渲染失败: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "模板渲染失败"}))
        }
    }
}

/// GET /api/knowledge-map — 返回所有笔记的标签与 PageRank，供 WASM 布局计算（v1.9.5）
///
/// 响应格式：`[{id, title, path, tags, pagerank}]`
/// - `id`/`path`：笔记相对路径
/// - `tags`：标签列表（空标签笔记也包含，WASM 侧处理无标签节点）
/// - `pagerank`：归一化影响力分数（0.0–1.0，来自 generate_global_graph）
#[get("/api/knowledge-map")]
pub async fn knowledge_map_api_handler(
    data: web::Data<Arc<AppState>>,
) -> impl Responder {
    let notes     = data.notes.read().await;
    let link_index = data.link_index.read().await;

    // 获取全局图谱数据以提取 PageRank 分数
    let graph_data = crate::graph::generate_global_graph(&notes, &link_index, false);
    drop(link_index);

    let pr_map: std::collections::HashMap<&str, f32> = graph_data.nodes.iter()
        .map(|n| (n.id.as_str(), n.pagerank))
        .collect();

    let km_notes: Vec<serde_json::Value> = notes.values()
        .map(|n| {
            let pr = pr_map.get(n.path.as_str()).copied().unwrap_or(0.0);
            serde_json::json!({
                "id":       n.path,
                "title":    n.title,
                "path":     n.path,
                "tags":     n.tags,
                "pagerank": pr,
            })
        })
        .collect();

    HttpResponse::Ok().json(km_notes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};
    use std::path::PathBuf;
    use std::sync::Arc;

    /// 创建最小化的测试用 AppState
    async fn make_test_state() -> Arc<AppState> {
        use crate::config::{AppConfig, DatabaseConfig, SecurityConfig, WebhookConfig};
        use crate::search_engine::SearchEngine;
        use crate::share_db::ShareDatabase;
        use crate::reading_progress_db::ReadingProgressDatabase;
        use tempfile::TempDir;

        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join("share.db");
        let rp_path = tmp.path().join("rp.db");
        let idx_path = tmp.path().join(".search_index");

        let config = AppConfig {
            repo_url: String::new(),
            local_path: PathBuf::from("./vault_data"),
            listen_addr: "127.0.0.1:8080".into(),
            workers: 1,
            ignore_patterns: vec![],
            database: DatabaseConfig {
                index_db_path: tmp.path().join("index.db"),
                auth_db_path: tmp.path().join("auth.db"),
                share_db_path: db_path.clone(),
                reading_progress_db_path: rp_path.clone(),
            },
            security: SecurityConfig::default(),
            sync_interval_minutes: 0,
            webhook: WebhookConfig::default(),
            public_base_url: None,
            repos: vec![],
        };

        let search_engine = Arc::new(SearchEngine::new(&idx_path).unwrap());
        let share_db = Arc::new(ShareDatabase::open(&db_path).unwrap());
        let rp_db = Arc::new(ReadingProgressDatabase::open(&rp_path).unwrap());

        // TempDir must stay alive; leak it for the test lifetime
        std::mem::forget(tmp);

        Arc::new(AppState::new(config, search_engine, share_db, rp_db))
    }

    #[actix_web::test]
    async fn test_health_response_structure() {
        // /health 返回 JSON，包含 status / version / notes_count 字段
        let state = make_test_state().await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(state))
                .service(health_handler),
        )
        .await;

        let req = test::TestRequest::get().uri("/health").to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success(), "/health 应返回 2xx");

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["status"], "healthy", "status 应为 healthy");
        assert!(body["version"].is_string(), "version 应存在");
        assert!(body["notes_count"].is_number(), "notes_count 应存在");
        assert!(body["sync_status"].is_string(), "sync_status 应存在");
    }

    #[actix_web::test]
    async fn test_titles_api_empty() {
        // /api/titles 空库时返回空数组
        let state = make_test_state().await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(state))
                .service(titles_api_handler),
        )
        .await;

        let req = test::TestRequest::get().uri("/api/titles").to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert!(body["titles"].is_array(), "应有 titles 数组");
        assert!(body["tags"].is_array(), "应有 tags 数组");
        assert_eq!(body["titles"].as_array().unwrap().len(), 0, "空库 titles 应为空");
    }

    #[actix_web::test]
    async fn test_orphans_empty_library() {
        // /orphans 空库时列表为空
        let state = make_test_state().await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(state))
                .service(orphans_handler),
        )
        .await;

        let req = test::TestRequest::get().uri("/orphans").to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success(), "/orphans 应返回 2xx");
        // 响应是 HTML，空库时应包含"共 0 篇孤立笔记"或"太棒了"的提示
        let body = String::from_utf8(test::read_body(resp).await.to_vec()).unwrap();
        assert!(
            body.contains("孤立笔记") || body.contains("孤"),
            "/orphans 页面应包含相关内容"
        );
    }

    #[actix_web::test]
    async fn test_config_reload_handler_requires_auth() {
        // config_reload_handler 在无认证扩展时应返回 401
        let state = make_test_state().await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(state))
                .route("/api/config/reload", web::post().to(config_reload_handler)),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/config/reload")
            .to_request();
        let resp = test::call_service(&app, req).await;
        // 未注入认证扩展，应返回 401
        assert_eq!(
            resp.status(),
            actix_web::http::StatusCode::UNAUTHORIZED,
            "config_reload 未认证时应返回 401"
        );
    }

    // ─── 路径遍历防护测试（S1 修复回归） ─────────────────────────────────

    #[actix_web::test]
    async fn test_is_path_within_blocks_traversal() {
        use tempfile::TempDir;

        let tmp = TempDir::new().unwrap();
        let base = tmp.path();

        // 在 base 内创建一个合法文件
        let legal_file = base.join("note.md");
        std::fs::write(&legal_file, "内容").unwrap();

        // 合法路径：base/note.md → 应通过
        assert!(is_path_within(base, &legal_file), "base 内的文件应通过检查");

        // 路径遍历：父目录在 base 之外 → 应被拒绝
        let parent = base.parent().unwrap();
        assert!(!is_path_within(base, parent), "base 父目录不应通过检查");

        // 不存在的路径（canonicalize 失败）→ 应返回 false
        let nonexistent = base.join("does_not_exist.md");
        assert!(!is_path_within(base, &nonexistent), "不存在的路径应返回 false");

        // base 自身（目录）→ 路径在 base 内
        assert!(is_path_within(base, base), "base 目录自身应通过前缀检查");
    }

    #[actix_web::test]
    async fn test_assets_handler_blocks_path_traversal() {
        // /assets/../../etc/passwd 类请求应返回 404（路径不存在于索引）或 403（canonicalize 越界）
        let state = make_test_state().await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(state))
                .service(assets_handler),
        )
        .await;

        // 尝试路径遍历（编码后的 ../..）
        let req = test::TestRequest::get()
            .uri("/assets/..%2F..%2Fetc%2Fpasswd")
            .to_request();
        let resp = test::call_service(&app, req).await;
        // 应返回 403 或 404，绝不能返回 200
        assert!(
            resp.status() == actix_web::http::StatusCode::FORBIDDEN
                || resp.status() == actix_web::http::StatusCode::NOT_FOUND,
            "路径遍历请求应被拒绝，实际状态码: {}",
            resp.status()
        );
    }

    // ─── Webhook 签名验证测试（T2） ────────────────────────────────────────

    #[actix_web::test]
    async fn test_verify_github_signature_correct() {
        // 使用已知 secret + body 计算正确签名，验证应通过
        use hmac::{Mac, SimpleHmac};
        use hmac::digest::KeyInit;
        use sha2::Sha256;
        type HmacSha256 = SimpleHmac<Sha256>;

        let secret = "webhook_secret";
        let body = b"payload body";

        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(body);
        let result = mac.finalize().into_bytes();
        let hex: String = result.iter().map(|b| format!("{:02x}", b)).collect();
        let sig = format!("sha256={}", hex);

        assert!(
            verify_github_signature(secret, body, &sig),
            "正确签名应通过验证"
        );
    }

    #[actix_web::test]
    async fn test_verify_github_signature_wrong_secret() {
        use hmac::{Mac, SimpleHmac};
        use hmac::digest::KeyInit;
        use sha2::Sha256;
        type HmacSha256 = SimpleHmac<Sha256>;

        let body = b"payload body";
        let mut mac = HmacSha256::new_from_slice(b"wrong_secret").unwrap();
        mac.update(body);
        let result = mac.finalize().into_bytes();
        let hex: String = result.iter().map(|b| format!("{:02x}", b)).collect();
        let sig = format!("sha256={}", hex);

        assert!(
            !verify_github_signature("correct_secret", body, &sig),
            "错误 secret 生成的签名应验证失败"
        );
    }

    #[actix_web::test]
    async fn test_verify_github_signature_missing_prefix() {
        // 签名缺少 sha256= 前缀应被拒绝
        assert!(
            !verify_github_signature("secret", b"body", "abcdef1234"),
            "缺少 sha256= 前缀应验证失败"
        );
    }

    #[actix_web::test]
    async fn test_verify_github_signature_invalid_hex() {
        // 非法十六进制字符串应被拒绝
        assert!(
            !verify_github_signature("secret", b"body", "sha256=ZZZZZZ"),
            "非法十六进制应验证失败"
        );
    }

    #[actix_web::test]
    async fn test_hex_decode_odd_length() {
        // 奇数长度十六进制字符串应返回 None
        assert!(hex_decode("abc").is_none(), "奇数长度十六进制应返回 None");
    }

    #[actix_web::test]
    async fn test_hex_decode_valid() {
        // 合法十六进制解码
        assert_eq!(hex_decode("deadbeef"), Some(vec![0xde, 0xad, 0xbe, 0xef]));
    }

    // ── v1.7.2：Git 历史相关单元测试 ──────────────────────────────────────────

    #[actix_web::test]
    async fn test_is_valid_commit_hash_valid() {
        // 合法 commit hash：40 位全小写十六进制
        assert!(is_valid_commit_hash("a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2"));
        // 缩短 hash（8 位）也合法
        assert!(is_valid_commit_hash("af605ea7"));
        // 最小长度 4 位
        assert!(is_valid_commit_hash("abcd"));
    }

    #[actix_web::test]
    async fn test_is_valid_commit_hash_invalid() {
        // 空字符串不合法
        assert!(!is_valid_commit_hash(""));
        // 包含非十六进制字符（g、空格）
        assert!(!is_valid_commit_hash("g1b2c3d4"));
        assert!(!is_valid_commit_hash("a1b2 c3d4"));
        // 长度不足 4 位
        assert!(!is_valid_commit_hash("abc"));
        // 长度超过 64 位
        assert!(!is_valid_commit_hash(&"a".repeat(65)));
        // 路径注入尝试
        assert!(!is_valid_commit_hash("HEAD~1"));
        assert!(!is_valid_commit_hash("HEAD^"));
        assert!(!is_valid_commit_hash("../secret"));
    }

    #[actix_web::test]
    async fn test_render_diff_html_basic() {
        // 验证 diff 渲染结果包含正确的 CSS class
        let diff = "--- a/note.md\n+++ b/note.md\n@@ -1,2 +1,3 @@\n context\n-deleted\n+added\n";
        let html = render_diff_html(diff);
        assert!(html.contains("diff-add"), "新增行应有 diff-add class");
        assert!(html.contains("diff-del"), "删除行应有 diff-del class");
        assert!(html.contains("diff-hunk"), "hunk 标头应有 diff-hunk class");
        assert!(html.contains("diff-ctx"), "上下文行应有 diff-ctx class");
        // 文件头行（--- +++）应被跳过
        assert!(!html.contains("--- a/"), "文件头行不应出现在渲染结果中");
    }

    #[actix_web::test]
    async fn test_render_diff_html_xss_escaped() {
        // diff 内容含 HTML 特殊字符时应被转义，防止 XSS
        let diff = "+<script>alert(1)</script>\n";
        let html = render_diff_html(diff);
        assert!(!html.contains("<script>"), "script 标签应被 HTML 转义");
        assert!(html.contains("&lt;script&gt;"), "尖括号应被转义为 &lt;/&gt;");
    }

    // ── v1.8.2：导出与发布 ────────────────────────────────────────────────────

    #[actix_web::test]
    async fn test_xml_escape() {
        // XML 特殊字符应被正确转义
        assert_eq!(xml_escape("a & b"), "a &amp; b");
        assert_eq!(xml_escape("<script>"), "&lt;script&gt;");
        assert_eq!(xml_escape("\"quoted\""), "&quot;quoted&quot;");
    }

    #[actix_web::test]
    async fn test_note_path_to_html() {
        // .md 后缀应替换为 .html
        assert_eq!(note_path_to_html("folder/note.md"), "folder/note.html");
        assert_eq!(note_path_to_html("root.md"), "root.html");
        // 无 .md 后缀时追加 .html
        assert_eq!(note_path_to_html("note"), "note.html");
    }

    #[actix_web::test]
    async fn test_build_static_note_html_contains_title() {
        use crate::domain::Frontmatter;
        use std::time::SystemTime;
        let note = crate::domain::Note {
            path:         "test.md".to_string(),
            title:        "测试笔记".to_string(),
            content_html: "<p>内容</p>".to_string(),
            backlinks:    vec![],
            tags:         vec![],
            toc:          vec![],
            mtime:        SystemTime::UNIX_EPOCH,
            frontmatter:  Frontmatter(serde_yaml::Value::Null),
            outgoing_links: vec![],
        };
        let html = build_static_note_html(&note, &[("test.md", "测试笔记")]);
        assert!(html.contains("测试笔记"), "HTML 应包含笔记标题");
        assert!(html.contains("<p>内容</p>"), "HTML 应包含笔记内容");
    }

    #[actix_web::test]
    async fn test_to_rfc3339_epoch() {
        // Unix epoch 应返回 UTC 时间字符串
        let result = to_rfc3339(std::time::SystemTime::UNIX_EPOCH);
        assert_eq!(result, "1970-01-01T00:00:00Z", "epoch 应为 1970-01-01T00:00:00Z");
    }
}
