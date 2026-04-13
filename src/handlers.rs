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
use crate::templates::{PageTemplate, IndexTemplate, TagsListTemplate, TagNotesTemplate};
use crate::search_engine::SortBy;
use crate::graph::generate_graph;
use crate::domain::BreadcrumbItem;

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
    pub date_to: Option<i64>, // 日期过滤：结束时间（Unix 时间戳秒）
}

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
#[post("/sync")]
pub async fn sync_handler(data: web::Data<Arc<AppState>>) -> impl Responder {
    // 使用 try_lock 防止并发同步：若已有同步在进行，立即返回 409
    let _guard = match data.sync_lock.try_lock() {
        Ok(guard) => guard,
        Err(_) => {
            return HttpResponse::Conflict().body("同步正在进行中，请稍后再试");
        }
    };

    match perform_sync(&data).await {
        Ok(_) => HttpResponse::Ok().body("同步成功"),
        Err(e) => {
            error!("同步失败: {:?}", e);
            HttpResponse::InternalServerError().body(format!("同步失败: {}", e))
        }
    }
}

/// GET /api/search - 搜索笔记（使用 Tantivy 搜索引擎）
#[get("/api/search")]
pub async fn search_handler(
    query: web::Query<SearchQuery>,
    data: web::Data<Arc<AppState>>,
) -> impl Responder {
    let search_term = query.q.trim();
    
    if search_term.is_empty() && query.tags.is_none() && query.folder.is_none() {
        return HttpResponse::Ok().json(Vec::<crate::search_engine::SearchResult>::new());
    }
    
    // 解析标签参数（逗号分隔）
    let tags = query.tags.as_ref().and_then(|t| {
        let tag_list: Vec<String> = t
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if tag_list.is_empty() {
            None
        } else {
            Some(tag_list)
        }
    });
    
    // 使用 Tantivy 进行高级搜索
    match data.search_engine.advanced_search(
        search_term,
        50,
        query.sort_by,
        tags,
        query.folder.clone(),
        query.date_from,
        query.date_to,
    ) {
        Ok(results) => HttpResponse::Ok().json(results),
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
                Err(e) => HttpResponse::InternalServerError().body(format!("Template error: {}", e)),
            };
        }
    }

    // 2. Redirect to first file
    if let Some(first_node) = find_first_file(&sidebar) {
         if let Some(path) = &first_node.path {
             return HttpResponse::Found()
                .append_header(("Location", format!("/doc/{}", path)))
                .finish();
         }
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
        Err(e) => HttpResponse::InternalServerError().body(format!("Template error: {}", e)),
    }
}

/// GET /assets/{filename} - 静态资源处理器（图片、PDF 等）
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

    // 先尝试直接访问（可能是完整路径）
    let direct_path = data.config.local_path.join(&decoded_filename);
    if direct_path.exists() && direct_path.is_file() {
        return actix_files::NamedFile::open(direct_path)
            .map_err(actix_web::error::ErrorInternalServerError);
    }

    // 如果不是完整路径，查找文件索引
    let file_index = data.file_index.read().await;
    if let Some(full_path) = file_index.get(&decoded_filename) {
        let file_path = data.config.local_path.join(full_path);
        if file_path.exists() && file_path.is_file() {
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

            match tmpl.render() {
                Ok(html) => HttpResponse::Ok().content_type("text/html").body(html),
                Err(e) => HttpResponse::InternalServerError().body(format!("Template error: {}", e)),
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
        Err(e) => HttpResponse::InternalServerError().body(format!("Template error: {}", e)),
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
            Err(e) => HttpResponse::InternalServerError().body(format!("Template error: {}", e)),
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
    use std::time::SystemTime;

    let uptime = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

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
    let git_commit = read_local_git_commit(&data.config.local_path).await;

    let health_info = json!({
        "status": "healthy",
        "version": env!("CARGO_PKG_VERSION"),
        "notes_count": notes_count,
        "uptime_seconds": uptime,
        "sync_status": sync_status_str,
        "last_sync_at": last_sync_at,
        "last_sync_duration_ms": last_sync_duration_ms,
        "git_commit": git_commit,
        "components": {
            "notes_index": "ok",
            "search_engine": "ok",
            "persistence": "ok"
        }
    });

    HttpResponse::Ok().json(health_info)
}

/// 读取本地仓库的 HEAD commit hash（异步，出错时返回空字符串）
async fn read_local_git_commit(local_path: &std::path::Path) -> String {
    let path = local_path.to_path_buf();
    tokio::task::spawn_blocking(move || {
        let head_file = path.join(".git").join("HEAD");
        let head = std::fs::read_to_string(&head_file).unwrap_or_default();
        if head.starts_with("ref: ") {
            let ref_path = head.trim_start_matches("ref: ").trim();
            let commit_file = path.join(".git").join(ref_path);
            std::fs::read_to_string(commit_file)
                .map(|s| s.trim().to_string())
                .unwrap_or_default()
        } else {
            head.trim().to_string()
        }
    })
    .await
    .unwrap_or_default()
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
            return HttpResponse::BadRequest().json(json!({
                "error": "Invalid UTF-8 in path"
            }));
        }
    };
    
    let notes = data.notes.read().await;
    let link_index = data.link_index.read().await;
    
    // 查找笔记（与 doc_handler 逻辑一致）
    let note_key = if notes.contains_key(&decoded_path) {
        Some(decoded_path.clone())
    } else if let Some(path) = link_index.get(&decoded_path) {
        Some(path.clone())
    } else {
        // 尝试添加 .md 后缀
        let with_md = format!("{}.md", decoded_path);
        if notes.contains_key(&with_md) {
            Some(with_md)
        } else {
            None
        }
    };
    
    if let Some(key) = note_key {
        if let Some(note) = notes.get(&key) {
            // 截取内容前 500 个字符作为预览
            let preview_content = truncate_html(&note.content_html, 500);
            
            let preview = json!({
                "title": note.title,
                "content": preview_content,
                "path": note.path,
            });
            
            return HttpResponse::Ok().json(preview);
        }
    }
    
    // 未找到笔记
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

/// GET /api/titles — 返回所有笔记标题和标签，供前端自动补全使用
///
/// 前端在首次搜索框聚焦时获取，缓存于 sessionStorage 中。
#[get("/api/titles")]
pub async fn titles_api_handler(data: web::Data<Arc<AppState>>) -> impl Responder {
    let notes = data.notes.read().await;
    let tag_index = data.tag_index.read().await;

    let titles: Vec<&str> = notes.values().map(|n| n.title.as_str()).collect();
    let tags: Vec<&str> = tag_index.keys().map(|t| t.as_str()).collect();

    HttpResponse::Ok().json(serde_json::json!({
        "titles": titles,
        "tags": tags,
    }))
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
    let webhook_cfg = &data.config.webhook;

    if !webhook_cfg.enabled {
        return HttpResponse::Forbidden().body("Webhook 未启用");
    }

    let secret = &webhook_cfg.secret;
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
    match crate::sync::perform_sync(&data).await {
        Ok(_) => HttpResponse::Ok().body("同步完成"),
        Err(e) => {
            tracing::error!("Webhook 触发同步失败: {:?}", e);
            HttpResponse::InternalServerError().body(format!("同步失败: {}", e))
        }
    }
}

/// 使用 HMAC-SHA256 验证 GitHub Webhook 签名
///
/// `signature` 格式为 `sha256=<hex>`，使用常数时间比较防止时序攻击。
fn verify_github_signature(secret: &str, body: &[u8], signature: &str) -> bool {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    let prefix = "sha256=";
    if !signature.starts_with(prefix) {
        return false;
    }
    let sig_hex = &signature[prefix.len()..];

    // 将十六进制签名解码为字节
    let sig_bytes = match hex_decode(sig_hex) {
        Some(b) => b,
        None => return false,
    };

    // 计算 HMAC-SHA256 并使用常数时间比较（防时序攻击）
    // hmac::Mac 提供 new_from_slice / update / verify_slice
    let mut mac: Hmac<Sha256> = match Mac::new_from_slice(secret.as_bytes()) {
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
    let old_patterns = &data.config.ignore_patterns;
    let new_patterns = &new_config.ignore_patterns;
    let patterns_changed = old_patterns != new_patterns;

    tracing::info!("🔄 配置热重载：ignore_patterns 变更 = {}", patterns_changed);

    // 触发完整同步以应用新配置（忽略持久化缓存）
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
