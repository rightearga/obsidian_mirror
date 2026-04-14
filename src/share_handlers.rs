//! 分享链接 API 处理器

use actix_web::{web, HttpResponse, HttpRequest, HttpMessage};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info};
use askama::Template;

use crate::share_db::ShareLink;
use crate::state::AppState;
use crate::templates::ShareTemplate;

/// 创建分享链接请求
#[derive(Debug, Deserialize)]
pub struct CreateShareRequest {
    /// 笔记路径
    pub note_path: String,

    /// 过期时间（秒，None 表示永久）
    pub expires_in_seconds: Option<u64>,

    /// 访问密码（可选）
    pub password: Option<String>,

    /// 最大访问次数（可选）
    pub max_visits: Option<u32>,
}

/// 创建分享链接响应
#[derive(Debug, Serialize)]
pub struct CreateShareResponse {
    /// 分享 token
    pub token: String,

    /// 完整的分享 URL
    pub share_url: String,

    /// 创建时间（ISO 8601 格式）
    pub created_at: String,

    /// 过期时间（ISO 8601 格式，None 表示永久）
    pub expires_at: Option<String>,
}

/// 访问分享链接请求
#[derive(Debug, Deserialize)]
pub struct AccessShareRequest {
    /// 访问密码（如果需要）
    pub password: Option<String>,
}

/// 分享链接信息响应
#[derive(Debug, Serialize)]
pub struct ShareInfoResponse {
    pub token: String,
    pub note_path: String,
    pub creator: String,
    pub created_at: String,
    pub expires_at: Option<String>,
    pub has_password: bool,
    pub max_visits: Option<u32>,
    pub visit_count: u32,
    pub is_valid: bool,
}

impl From<&ShareLink> for ShareInfoResponse {
    fn from(share: &ShareLink) -> Self {
        use chrono::{DateTime, Utc};

        let created_at = DateTime::<Utc>::from(share.created_at)
            .to_rfc3339();

        let expires_at = share.expires_at.map(|t| {
            DateTime::<Utc>::from(t).to_rfc3339()
        });

        Self {
            token: share.token.clone(),
            note_path: share.note_path.clone(),
            creator: share.creator.clone(),
            created_at,
            expires_at,
            has_password: share.password_hash.is_some(),
            max_visits: share.max_visits,
            visit_count: share.visit_count,
            is_valid: share.is_valid(),
        }
    }
}

/// 创建分享链接
///
/// POST /api/share/create
pub async fn create_share_handler(
    req: HttpRequest,
    body: web::Json<CreateShareRequest>,
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

    // 验证笔记是否存在
    let notes = app_state.notes.read().await;
    if !notes.contains_key(&body.note_path) {
        return HttpResponse::NotFound().json(serde_json::json!({
            "error": "笔记不存在"
        }));
    }
    drop(notes);

    // 创建分享链接
    let expires_in = body.expires_in_seconds.map(Duration::from_secs);
    let share_link = ShareLink::new(
        body.note_path.clone(),
        username.clone(),
        expires_in,
        body.password.clone(),
        body.max_visits,
    );

    // A1 修复：redb IO 移入 spawn_blocking，避免阻塞 Tokio 线程池
    // 保存到数据库
    let db = Arc::clone(&app_state.share_db);
    let link = share_link.clone();
    if let Err(e) = tokio::task::spawn_blocking(move || db.create_share(&link))
        .await
        .unwrap_or_else(|e| Err(anyhow::anyhow!("spawn_blocking panic: {}", e)))
    {
        error!("创建分享链接失败: {}", e);
        return HttpResponse::InternalServerError().json(serde_json::json!({
            "error": "创建分享链接失败"
        }));
    }

    // Q2 修复：优先使用 public_base_url 配置，其次读取 X-Forwarded-Proto header
    let share_url = {
        let public_base_url = app_state.config.read().unwrap().public_base_url.clone();
        if let Some(base_url) = public_base_url {
            format!("{}/share/{}", base_url.trim_end_matches('/'), share_link.token)
        } else {
            let host = req.headers()
                .get("host")
                .and_then(|h| h.to_str().ok())
                .unwrap_or("localhost");
            let scheme = req.headers()
                .get("X-Forwarded-Proto")
                .and_then(|v| v.to_str().ok())
                .unwrap_or_else(|| {
                    if host.contains("localhost") || host.starts_with("127.") { "http" } else { "https" }
                });
            format!("{}://{}/share/{}", scheme, host, share_link.token)
        }
    };

    use chrono::{DateTime, Utc};
    let created_at = DateTime::<Utc>::from(share_link.created_at).to_rfc3339();
    let expires_at = share_link.expires_at.map(|t| DateTime::<Utc>::from(t).to_rfc3339());

    info!("✅ 用户 {} 创建分享链接: {} -> {}", username, share_link.token, body.note_path);

    HttpResponse::Ok().json(CreateShareResponse {
        token: share_link.token,
        share_url,
        created_at,
        expires_at,
    })
}

/// 访问分享链接
///
/// GET /share/{token}
/// POST /share/{token} (带密码)
pub async fn access_share_handler(
    path: web::Path<String>,
    body: Option<web::Json<AccessShareRequest>>,
    app_state: web::Data<Arc<AppState>>,
) -> HttpResponse {
    let token = path.into_inner();

    // A1 修复：redb IO 移入 spawn_blocking，避免阻塞 Tokio 线程池
    // 获取分享链接
    let db = Arc::clone(&app_state.share_db);
    let token_clone = token.clone();
    let mut share_link = match tokio::task::spawn_blocking(move || db.get_share(&token_clone))
        .await
        .unwrap_or_else(|e| Err(anyhow::anyhow!("spawn_blocking panic: {}", e)))
    {
        Ok(Some(share)) => share,
        Ok(None) => {
            return HttpResponse::NotFound().body(
                "<html><body><h1>分享链接不存在</h1><p>该分享链接不存在或已被删除。</p></body></html>"
            );
        }
        Err(e) => {
            error!("查询分享链接失败: {}", e);
            return HttpResponse::InternalServerError().body(
                "<html><body><h1>服务器错误</h1><p>查询分享链接失败，请稍后重试。</p></body></html>"
            );
        }
    };

    // 检查是否有效
    if !share_link.is_valid() {
        return HttpResponse::Gone().body(
            "<html><body><h1>分享链接已过期</h1><p>该分享链接已过期或已达到访问次数上限。</p></body></html>"
        );
    }

    // 验证密码
    let password = body.as_ref().and_then(|b| b.password.as_deref());
    if !share_link.verify_password(password) {
        // 如果需要密码但未提供，返回密码输入页面
        let password_form = format!(
            r#"
            <!DOCTYPE html>
            <html lang="zh-CN">
            <head>
                <meta charset="UTF-8">
                <meta name="viewport" content="width=device-width, initial-scale=1.0">
                <title>输入密码</title>
                <style>
                    body {{
                        margin: 0;
                        padding: 0;
                        font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
                        background: #f8f9fa;
                        display: flex;
                        align-items: center;
                        justify-content: center;
                        min-height: 100vh;
                    }}
                    .password-container {{
                        background: white;
                        border-radius: 12px;
                        padding: 40px;
                        box-shadow: 0 4px 16px rgba(0,0,0,0.1);
                        max-width: 400px;
                        width: 90%;
                    }}
                    h1 {{
                        margin: 0 0 20px 0;
                        font-size: 24px;
                        color: #2c3e50;
                    }}
                    p {{
                        color: #7f8c8d;
                        margin-bottom: 24px;
                    }}
                    input[type="password"] {{
                        width: 100%;
                        padding: 12px;
                        border: 1px solid #e0e0e0;
                        border-radius: 6px;
                        font-size: 16px;
                        margin-bottom: 16px;
                        box-sizing: border-box;
                    }}
                    button {{
                        width: 100%;
                        padding: 12px;
                        background: #4a90e2;
                        color: white;
                        border: none;
                        border-radius: 6px;
                        font-size: 16px;
                        font-weight: 500;
                        cursor: pointer;
                    }}
                    button:hover {{
                        background: #357abd;
                    }}
                    .error {{
                        color: #e74c3c;
                        margin-bottom: 16px;
                        display: none;
                    }}
                    .error.show {{
                        display: block;
                    }}
                </style>
            </head>
            <body>
                <div class="password-container">
                    <h1>🔒 需要密码</h1>
                    <p>此分享内容受密码保护，请输入密码以继续访问。</p>
                    <div id="error" class="error">密码错误，请重试</div>
                    <form id="password-form">
                        <input type="password" id="password" placeholder="请输入密码" required autofocus>
                        <button type="submit">解锁</button>
                    </form>
                </div>
                <script>
                    document.getElementById('password-form').addEventListener('submit', async (e) => {{
                        e.preventDefault();
                        const password = document.getElementById('password').value;
                        const error = document.getElementById('error');

                        try {{
                            const response = await fetch('/share/{}', {{
                                method: 'POST',
                                headers: {{
                                    'Content-Type': 'application/json',
                                }},
                                body: JSON.stringify({{ password: password }})
                            }});

                            if (response.ok) {{
                                // 密码正确，重新加载页面
                                window.location.reload();
                            }} else {{
                                error.classList.add('show');
                            }}
                        }} catch (err) {{
                            error.textContent = '请求失败，请重试';
                            error.classList.add('show');
                        }}
                    }});
                </script>
            </body>
            </html>
            "#,
            token
        );

        return HttpResponse::Forbidden()
            .content_type("text/html; charset=utf-8")
            .body(password_form);
    }

    // 增加访问次数
    share_link.increment_visit();
    // A1 修复：redb IO 移入 spawn_blocking（fire-and-forget，不阻塞响应）
    let db2 = Arc::clone(&app_state.share_db);
    let updated_link = share_link.clone();
    tokio::task::spawn_blocking(move || {
        if let Err(e) = db2.update_share(&updated_link) {
            tracing::error!("更新分享链接访问次数失败: {}", e);
        }
    });

    // 获取笔记内容
    let notes = app_state.notes.read().await;
    let note = match notes.get(&share_link.note_path) {
        Some(n) => n.clone(),
        None => {
            drop(notes);
            return HttpResponse::NotFound().body(
                "<html><body><h1>笔记不存在</h1><p>该笔记可能已被删除。</p></body></html>"
            );
        }
    };
    drop(notes);

    info!("✅ 分享链接 {} 被访问: {} (访问次数: {})",
        token, share_link.note_path, share_link.visit_count);

    // 渲染分享页面
    use chrono::{DateTime, Local};
    let created_at = DateTime::<Local>::from(share_link.created_at)
        .format("%Y年%m月%d日 %H:%M")
        .to_string();

    let template = ShareTemplate {
        note_title: &note.title,
        content_html: &note.content_html,
        creator: &share_link.creator,
        created_at: &created_at,
        visit_count: share_link.visit_count,
    };

    // 渲染模板
    match template.render() {
        Ok(html) => HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(html),
        Err(e) => {
            error!("渲染分享页面失败: {}", e);
            HttpResponse::InternalServerError().body(
                "<html><body><h1>渲染错误</h1><p>页面渲染失败，请联系管理员。</p></body></html>"
            )
        }
    }
}

/// 获取用户的所有分享链接
///
/// GET /api/share/list
pub async fn list_shares_handler(
    req: HttpRequest,
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
    // 获取用户的所有分享链接
    let db = Arc::clone(&app_state.share_db);
    let uname = username.clone();
    let shares = match tokio::task::spawn_blocking(move || db.get_user_shares(&uname))
        .await
        .unwrap_or_else(|e| Err(anyhow::anyhow!("spawn_blocking panic: {}", e)))
    {
        Ok(shares) => shares,
        Err(e) => {
            error!("查询分享链接失败: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "查询分享链接失败"
            }));
        }
    };

    let share_infos: Vec<ShareInfoResponse> = shares.iter().map(|s| s.into()).collect();

    HttpResponse::Ok().json(serde_json::json!({
        "shares": share_infos
    }))
}

/// 撤销分享链接
///
/// DELETE /api/share/{token}
pub async fn revoke_share_handler(
    req: HttpRequest,
    path: web::Path<String>,
    app_state: web::Data<Arc<AppState>>,
) -> HttpResponse {
    let token = path.into_inner();

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
    // 获取分享链接，验证所有权
    let db = Arc::clone(&app_state.share_db);
    let token_clone = token.clone();
    let share_link = match tokio::task::spawn_blocking(move || db.get_share(&token_clone))
        .await
        .unwrap_or_else(|e| Err(anyhow::anyhow!("spawn_blocking panic: {}", e)))
    {
        Ok(Some(share)) => share,
        Ok(None) => {
            return HttpResponse::NotFound().json(serde_json::json!({
                "error": "分享链接不存在"
            }));
        }
        Err(e) => {
            error!("查询分享链接失败: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "查询分享链接失败"
            }));
        }
    };

    // 验证所有权
    if share_link.creator != username {
        return HttpResponse::Forbidden().json(serde_json::json!({
            "error": "无权撤销此分享链接"
        }));
    }

    // A1 修复：redb IO 移入 spawn_blocking，避免阻塞 Tokio 线程池
    // 删除分享链接
    let db2 = Arc::clone(&app_state.share_db);
    let token_clone2 = token.clone();
    let delete_result = tokio::task::spawn_blocking(move || db2.delete_share(&token_clone2))
        .await
        .unwrap_or_else(|e| Err(anyhow::anyhow!("spawn_blocking panic: {}", e)));

    match delete_result {
        Ok(true) => {
            info!("✅ 用户 {} 撤销分享链接: {}", username, token);
            HttpResponse::Ok().json(serde_json::json!({
                "message": "分享链接已撤销"
            }))
        }
        Ok(false) => {
            HttpResponse::NotFound().json(serde_json::json!({
                "error": "分享链接不存在"
            }))
        }
        Err(e) => {
            error!("删除分享链接失败: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "删除分享链接失败"
            }))
        }
    }
}
