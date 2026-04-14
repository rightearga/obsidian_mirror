//! 阅读进度 API 处理器

use actix_web::{web, HttpResponse, HttpRequest, HttpMessage};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::error;

use crate::reading_progress_db::{ReadingProgress, ReadingHistory};
use crate::state::AppState;

/// 保存阅读进度请求
#[derive(Debug, Deserialize)]
pub struct SaveProgressRequest {
    /// 笔记路径
    pub note_path: String,

    /// 笔记标题
    pub note_title: String,

    /// 滚动位置（像素）
    pub scroll_position: u32,

    /// 滚动百分比（0-100）
    pub scroll_percentage: f32,

    /// 自上次保存以来的阅读时长（秒）
    #[serde(default)]
    pub duration_delta: u64,
}

/// 阅读进度响应
#[derive(Debug, Serialize)]
pub struct ProgressResponse {
    pub note_path: String,
    pub note_title: String,
    pub scroll_position: u32,
    pub scroll_percentage: f32,
    pub last_read_at: String,
    pub reading_duration: u64,
    pub is_completed: bool,
}

impl From<&ReadingProgress> for ProgressResponse {
    fn from(progress: &ReadingProgress) -> Self {
        use chrono::{DateTime, Utc};

        let last_read_at = DateTime::<Utc>::from(progress.last_read_at)
            .to_rfc3339();

        Self {
            note_path: progress.note_path.clone(),
            note_title: progress.note_title.clone(),
            scroll_position: progress.scroll_position,
            scroll_percentage: progress.scroll_percentage,
            last_read_at,
            reading_duration: progress.reading_duration,
            is_completed: progress.is_completed,
        }
    }
}

/// 阅读历史响应
#[derive(Debug, Serialize)]
pub struct HistoryResponse {
    pub note_path: String,
    pub note_title: String,
    pub visited_at: String,
    pub duration: u64,
}

impl From<&ReadingHistory> for HistoryResponse {
    fn from(history: &ReadingHistory) -> Self {
        use chrono::{DateTime, Utc};

        let visited_at = DateTime::<Utc>::from(history.visited_at)
            .to_rfc3339();

        Self {
            note_path: history.note_path.clone(),
            note_title: history.note_title.clone(),
            visited_at,
            duration: history.duration,
        }
    }
}

/// 保存阅读进度
///
/// POST /api/reading/progress
pub async fn save_progress_handler(
    req: HttpRequest,
    body: web::Json<SaveProgressRequest>,
    app_state: web::Data<Arc<AppState>>,
) -> HttpResponse {
    // 从请求扩展中获取用户名
    let username = match req.extensions().get::<String>() {
        Some(user) => user.clone(),
        None => {
            return HttpResponse::Unauthorized().json(serde_json::json!({
                "error": "未认证"
            }));
        }
    };

    // A1 修复：redb IO 移入 spawn_blocking，避免阻塞 Tokio 线程池
    // 获取现有进度或创建新进度
    let db = Arc::clone(&app_state.reading_progress_db);
    let uname = username.clone();
    let note_path = body.note_path.clone();
    let mut progress = match tokio::task::spawn_blocking(move || db.get_progress(&uname, &note_path))
        .await
        .unwrap_or_else(|e| Err(anyhow::anyhow!("spawn_blocking panic: {}", e)))
    {
        Ok(Some(p)) => p,
        Ok(None) => {
            ReadingProgress::new(
                username.clone(),
                body.note_path.clone(),
                body.note_title.clone(),
                body.scroll_position,
                body.scroll_percentage,
            )
        }
        Err(e) => {
            error!("查询阅读进度失败: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "查询阅读进度失败"
            }));
        }
    };

    // 更新进度
    progress.update(
        body.scroll_position,
        body.scroll_percentage,
        body.duration_delta,
    );

    // A1 修复：redb IO 移入 spawn_blocking，避免阻塞 Tokio 线程池
    // 保存到数据库
    let db2 = Arc::clone(&app_state.reading_progress_db);
    let progress_clone = progress.clone();
    if let Err(e) = tokio::task::spawn_blocking(move || db2.save_progress(&progress_clone))
        .await
        .unwrap_or_else(|e| Err(anyhow::anyhow!("spawn_blocking panic: {}", e)))
    {
        error!("保存阅读进度失败: {}", e);
        return HttpResponse::InternalServerError().json(serde_json::json!({
            "error": "保存阅读进度失败"
        }));
    }

    HttpResponse::Ok().json(serde_json::json!({
        "message": "保存成功",
        "progress": ProgressResponse::from(&progress)
    }))
}

/// 获取阅读进度
///
/// GET /api/reading/progress/{note_path}
pub async fn get_progress_handler(
    req: HttpRequest,
    path: web::Path<String>,
    app_state: web::Data<Arc<AppState>>,
) -> HttpResponse {
    // 从请求扩展中获取用户名
    let username = match req.extensions().get::<String>() {
        Some(user) => user.clone(),
        None => {
            return HttpResponse::Unauthorized().json(serde_json::json!({
                "error": "未认证"
            }));
        }
    };

    let note_path = path.into_inner();

    // A1 修复：redb IO 移入 spawn_blocking，避免阻塞 Tokio 线程池
    // 获取进度
    let db = Arc::clone(&app_state.reading_progress_db);
    let uname = username.clone();
    let np = note_path.clone();
    match tokio::task::spawn_blocking(move || db.get_progress(&uname, &np))
        .await
        .unwrap_or_else(|e| Err(anyhow::anyhow!("spawn_blocking panic: {}", e)))
    {
        Ok(Some(progress)) => {
            HttpResponse::Ok().json(ProgressResponse::from(&progress))
        }
        Ok(None) => {
            HttpResponse::NotFound().json(serde_json::json!({
                "error": "未找到阅读进度"
            }))
        }
        Err(e) => {
            error!("查询阅读进度失败: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "查询阅读进度失败"
            }))
        }
    }
}

/// 获取用户的所有阅读进度
///
/// GET /api/reading/progress
pub async fn list_progress_handler(
    req: HttpRequest,
    query: web::Query<ListProgressQuery>,
    app_state: web::Data<Arc<AppState>>,
) -> HttpResponse {
    // 从请求扩展中获取用户名
    let username = match req.extensions().get::<String>() {
        Some(user) => user.clone(),
        None => {
            return HttpResponse::Unauthorized().json(serde_json::json!({
                "error": "未认证"
            }));
        }
    };

    let limit = query.limit.unwrap_or(20).min(100); // 最多返回 100 条

    // A1 修复：redb IO 移入 spawn_blocking，避免阻塞 Tokio 线程池
    // 获取所有进度
    let db = Arc::clone(&app_state.reading_progress_db);
    let uname = username.clone();
    match tokio::task::spawn_blocking(move || db.get_user_progress(&uname, limit))
        .await
        .unwrap_or_else(|e| Err(anyhow::anyhow!("spawn_blocking panic: {}", e)))
    {
        Ok(progress_list) => {
            let response: Vec<ProgressResponse> = progress_list
                .iter()
                .map(|p| p.into())
                .collect();

            HttpResponse::Ok().json(serde_json::json!({
                "progress": response
            }))
        }
        Err(e) => {
            error!("查询阅读进度列表失败: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "查询阅读进度列表失败"
            }))
        }
    }
}

/// 删除阅读进度
///
/// DELETE /api/reading/progress/{note_path}
pub async fn delete_progress_handler(
    req: HttpRequest,
    path: web::Path<String>,
    app_state: web::Data<Arc<AppState>>,
) -> HttpResponse {
    // 从请求扩展中获取用户名
    let username = match req.extensions().get::<String>() {
        Some(user) => user.clone(),
        None => {
            return HttpResponse::Unauthorized().json(serde_json::json!({
                "error": "未认证"
            }));
        }
    };

    let note_path = path.into_inner();

    // A1 修复：redb IO 移入 spawn_blocking，避免阻塞 Tokio 线程池
    // 删除进度
    let db = Arc::clone(&app_state.reading_progress_db);
    let uname = username.clone();
    let np = note_path.clone();
    match tokio::task::spawn_blocking(move || db.delete_progress(&uname, &np))
        .await
        .unwrap_or_else(|e| Err(anyhow::anyhow!("spawn_blocking panic: {}", e)))
    {
        Ok(true) => {
            HttpResponse::Ok().json(serde_json::json!({
                "message": "删除成功"
            }))
        }
        Ok(false) => {
            HttpResponse::NotFound().json(serde_json::json!({
                "error": "未找到阅读进度"
            }))
        }
        Err(e) => {
            error!("删除阅读进度失败: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "删除阅读进度失败"
            }))
        }
    }
}

/// 添加阅读历史
///
/// POST /api/reading/history
pub async fn add_history_handler(
    req: HttpRequest,
    body: web::Json<AddHistoryRequest>,
    app_state: web::Data<Arc<AppState>>,
) -> HttpResponse {
    // 从请求扩展中获取用户名
    let username = match req.extensions().get::<String>() {
        Some(user) => user.clone(),
        None => {
            return HttpResponse::Unauthorized().json(serde_json::json!({
                "error": "未认证"
            }));
        }
    };

    let history = ReadingHistory::new(
        username,
        body.note_path.clone(),
        body.note_title.clone(),
        body.duration,
    );

    // A1 修复：redb IO 移入 spawn_blocking，避免阻塞 Tokio 线程池
    // 添加到数据库
    let db = Arc::clone(&app_state.reading_progress_db);
    let history_clone = history.clone();
    if let Err(e) = tokio::task::spawn_blocking(move || db.add_history(&history_clone))
        .await
        .unwrap_or_else(|e| Err(anyhow::anyhow!("spawn_blocking panic: {}", e)))
    {
        error!("添加阅读历史失败: {}", e);
        return HttpResponse::InternalServerError().json(serde_json::json!({
            "error": "添加阅读历史失败"
        }));
    }

    HttpResponse::Ok().json(serde_json::json!({
        "message": "添加成功"
    }))
}

/// 获取阅读历史
///
/// GET /api/reading/history
pub async fn list_history_handler(
    req: HttpRequest,
    query: web::Query<ListHistoryQuery>,
    app_state: web::Data<Arc<AppState>>,
) -> HttpResponse {
    // 从请求扩展中获取用户名
    let username = match req.extensions().get::<String>() {
        Some(user) => user.clone(),
        None => {
            return HttpResponse::Unauthorized().json(serde_json::json!({
                "error": "未认证"
            }));
        }
    };

    let limit = query.limit.unwrap_or(50).min(200); // 最多返回 200 条

    // A1 修复：redb IO 移入 spawn_blocking，避免阻塞 Tokio 线程池
    // 获取历史记录
    let db = Arc::clone(&app_state.reading_progress_db);
    let uname = username.clone();
    match tokio::task::spawn_blocking(move || db.get_user_history(&uname, limit))
        .await
        .unwrap_or_else(|e| Err(anyhow::anyhow!("spawn_blocking panic: {}", e)))
    {
        Ok(history_list) => {
            let response: Vec<HistoryResponse> = history_list
                .iter()
                .map(|h| h.into())
                .collect();

            HttpResponse::Ok().json(serde_json::json!({
                "history": response
            }))
        }
        Err(e) => {
            error!("查询阅读历史失败: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "查询阅读历史失败"
            }))
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ListProgressQuery {
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct AddHistoryRequest {
    pub note_path: String,
    pub note_title: String,
    pub duration: u64,
}

#[derive(Debug, Deserialize)]
pub struct ListHistoryQuery {
    pub limit: Option<usize>,
}
