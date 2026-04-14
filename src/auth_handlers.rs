//! 认证相关的 HTTP 处理器

use actix_web::{
    cookie::{Cookie, SameSite},
    web, HttpMessage, HttpRequest, HttpResponse, Responder,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use time::Duration;

use crate::auth::{JwtManager, PasswordManager};
use crate::auth_db::{AuthDatabase, UserRole};

/// 登录请求体
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
    pub remember_me: Option<bool>,
}

/// 登录响应
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub success: bool,
    pub message: String,
    pub token: Option<String>,
}

/// 修改密码请求体
#[derive(Debug, Deserialize)]
pub struct ChangePasswordRequest {
    pub old_password: String,
    pub new_password: String,
}

/// 通用响应
#[derive(Debug, Serialize)]
pub struct ApiResponse {
    pub success: bool,
    pub message: String,
}

/// 登录处理器
pub async fn login_handler(
    form: web::Json<LoginRequest>,
    auth_db: web::Data<Arc<AuthDatabase>>,
    jwt_manager: web::Data<Arc<JwtManager>>,
    app_state: web::Data<Arc<crate::state::AppState>>,
) -> impl Responder {
    // A1 修复：redb IO 移入 spawn_blocking，避免阻塞 Tokio 线程池
    // 获取用户
    let db = Arc::clone(&*auth_db);
    let uname = form.username.clone();
    let user = match tokio::task::spawn_blocking(move || db.get_user(&uname))
        .await
        .unwrap_or_else(|e| Err(anyhow::anyhow!("spawn_blocking panic: {}", e)))
    {
        Ok(Some(user)) => user,
        Ok(None) => {
            return HttpResponse::Unauthorized().json(LoginResponse {
                success: false,
                message: "用户名或密码错误".to_string(),
                token: None,
            });
        }
        Err(e) => {
            tracing::error!("数据库错误: {}", e);
            return HttpResponse::InternalServerError().json(LoginResponse {
                success: false,
                message: "服务器内部错误".to_string(),
                token: None,
            });
        }
    };

    // 检查用户是否启用
    if !user.enabled {
        return HttpResponse::Forbidden().json(LoginResponse {
            success: false,
            message: "账户已被禁用".to_string(),
            token: None,
        });
    }

    // A1 修复：bcrypt verify 是 CPU 密集型操作，移入 spawn_blocking
    // B1 修复：spawn_blocking panic 不得静默为"密码错误"，须记录日志并返回 500
    let pwd = form.password.clone();
    let hash = user.password_hash.clone();
    let is_valid = tokio::task::spawn_blocking(move || {
        crate::auth::PasswordManager::verify_password(&pwd, &hash)
    })
    .await
    .unwrap_or_else(|e| {
        tracing::error!("bcrypt verify spawn_blocking panic: {}", e);
        Err(anyhow::anyhow!("spawn_blocking panic: {}", e))
    });

    match is_valid {
        Ok(true) => {
            // 生成 JWT token
            match jwt_manager.generate_token(&user.username, user.role.as_str()) {
                Ok(token) => {
                    // A1 修复：更新最后登录时间（fire-and-forget，不阻塞响应）
                    let db2 = Arc::clone(&*auth_db);
                    let uname2 = user.username.clone();
                    tokio::task::spawn_blocking(move || {
                        let _ = db2.update_last_login(&uname2);
                    });

                    // 设置 Cookie（有效期根据 remember_me 决定）
                    let max_age = if form.remember_me.unwrap_or(false) {
                        Duration::days(30)
                    } else {
                        Duration::hours(24)
                    };

                    // 安全属性：HttpOnly 防止 JS 读取；
                    // Secure 仅在 force_https_cookie=true（生产 HTTPS）时启用，
                    // HTTP（内网/开发）下设为 false 避免 Cookie 被浏览器静默丢弃
                    let secure = app_state.config.read().unwrap().security.force_https_cookie;
                    let cookie = Cookie::build("auth_token", token.clone())
                        .path("/")
                        .max_age(max_age)
                        .http_only(true)
                        .secure(secure)
                        .same_site(SameSite::Lax)
                        .finish();

                    HttpResponse::Ok()
                        .cookie(cookie)
                        .json(LoginResponse {
                            success: true,
                            message: "登录成功".to_string(),
                            token: Some(token),
                        })
                }
                Err(e) => {
                    tracing::error!("生成 token 失败: {}", e);
                    HttpResponse::InternalServerError().json(LoginResponse {
                        success: false,
                        message: "服务器内部错误".to_string(),
                        token: None,
                    })
                }
            }
        }
        Ok(false) => HttpResponse::Unauthorized().json(LoginResponse {
            success: false,
            message: "用户名或密码错误".to_string(),
            token: None,
        }),
        Err(e) => {
            tracing::error!("密码验证错误: {}", e);
            HttpResponse::InternalServerError().json(LoginResponse {
                success: false,
                message: "服务器内部错误".to_string(),
                token: None,
            })
        }
    }
}

/// 登出处理器
pub async fn logout_handler(
    app_state: web::Data<Arc<crate::state::AppState>>,
) -> impl Responder {
    // 删除 Cookie（Secure 标志必须与登录时一致，浏览器才能正确清除）
    let secure = app_state.config.read().unwrap().security.force_https_cookie;
    let cookie = Cookie::build("auth_token", "")
        .path("/")
        .max_age(Duration::seconds(0))
        .http_only(true)
        .secure(secure)
        .same_site(SameSite::Lax)
        .finish();

    HttpResponse::Ok()
        .cookie(cookie)
        .json(ApiResponse {
            success: true,
            message: "登出成功".to_string(),
        })
}

/// 修改密码处理器
pub async fn change_password_handler(
    req: HttpRequest,
    form: web::Json<ChangePasswordRequest>,
    auth_db: web::Data<Arc<AuthDatabase>>,
) -> impl Responder {
    // 从请求扩展中获取当前用户名（由中间件注入）
    let username = match req.extensions().get::<String>() {
        Some(username) => username.clone(),
        None => {
            return HttpResponse::Unauthorized().json(ApiResponse {
                success: false,
                message: "未认证".to_string(),
            });
        }
    };

    // A1 修复：redb IO 移入 spawn_blocking，避免阻塞 Tokio 线程池
    // 获取用户
    let db = Arc::clone(&*auth_db);
    let uname = username.clone();
    let user = match tokio::task::spawn_blocking(move || db.get_user(&uname))
        .await
        .unwrap_or_else(|e| Err(anyhow::anyhow!("spawn_blocking panic: {}", e)))
    {
        Ok(Some(user)) => user,
        Ok(None) => {
            return HttpResponse::NotFound().json(ApiResponse {
                success: false,
                message: "用户不存在".to_string(),
            });
        }
        Err(e) => {
            tracing::error!("数据库错误: {}", e);
            return HttpResponse::InternalServerError().json(ApiResponse {
                success: false,
                message: "服务器内部错误".to_string(),
            });
        }
    };

    // A1 修复：bcrypt verify 是 CPU 密集型操作，移入 spawn_blocking
    // B1 修复：spawn_blocking panic 不得静默为"旧密码错误"，须记录日志并返回 500
    let old_pwd = form.old_password.clone();
    let hash = user.password_hash.clone();
    let verify_result = tokio::task::spawn_blocking(move || {
        PasswordManager::verify_password(&old_pwd, &hash)
    })
    .await
    .unwrap_or_else(|e| {
        tracing::error!("bcrypt verify spawn_blocking panic: {}", e);
        Err(anyhow::anyhow!("spawn_blocking panic: {}", e))
    });

    match verify_result {
        Ok(true) => {
            // A1 修复：bcrypt hash 是 CPU 密集型操作，移入 spawn_blocking
            let new_pwd = form.new_password.clone();
            let hash_result = tokio::task::spawn_blocking(move || {
                PasswordManager::hash_password(&new_pwd)
            })
            .await
            .unwrap_or_else(|e| Err(anyhow::anyhow!("spawn_blocking panic: {}", e)));

            match hash_result {
                Ok(new_hash) => {
                    // A1 修复：redb IO 移入 spawn_blocking，避免阻塞 Tokio 线程池
                    let db2 = Arc::clone(&*auth_db);
                    let uname2 = username.clone();
                    let change_result =
                        tokio::task::spawn_blocking(move || db2.change_password(&uname2, &new_hash))
                            .await
                            .unwrap_or_else(|e| Err(anyhow::anyhow!("spawn_blocking panic: {}", e)));

                    match change_result {
                        Ok(_) => HttpResponse::Ok().json(ApiResponse {
                            success: true,
                            message: "密码修改成功".to_string(),
                        }),
                        Err(e) => {
                            tracing::error!("更新密码失败: {}", e);
                            HttpResponse::InternalServerError().json(ApiResponse {
                                success: false,
                                message: "服务器内部错误".to_string(),
                            })
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("密码加密失败: {}", e);
                    HttpResponse::InternalServerError().json(ApiResponse {
                        success: false,
                        message: "服务器内部错误".to_string(),
                    })
                }
            }
        }
        Ok(false) => HttpResponse::Unauthorized().json(ApiResponse {
            success: false,
            message: "旧密码错误".to_string(),
        }),
        Err(e) => {
            tracing::error!("密码验证错误: {}", e);
            HttpResponse::InternalServerError().json(ApiResponse {
                success: false,
                message: "服务器内部错误".to_string(),
            })
        }
    }
}

/// 获取当前用户信息
pub async fn current_user_handler(req: HttpRequest) -> impl Responder {
    // 从请求扩展中获取当前用户名
    let username = match req.extensions().get::<String>() {
        Some(username) => username.clone(),
        None => {
            return HttpResponse::Unauthorized().json(serde_json::json!({
                "success": false,
                "message": "未认证"
            }));
        }
    };

    HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "username": username
    }))
}

// ─── 管理员用户管理 API（v1.5.3）────────────────────────────────────────────

/// 从请求扩展中获取角色，若未注入（auth 未启用）则默认 admin（全放行）
fn get_user_role(req: &HttpRequest) -> UserRole {
    req.extensions().get::<UserRole>().cloned().unwrap_or(UserRole::Admin)
}

/// 校验请求者是否具有管理员权限；若无则返回 403 错误体
fn require_admin(req: &HttpRequest) -> Result<(), HttpResponse> {
    if get_user_role(req).is_admin() {
        Ok(())
    } else {
        Err(HttpResponse::Forbidden().json(serde_json::json!({
            "error": "需要管理员权限"
        })))
    }
}

/// 创建用户请求体
#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
    #[serde(default)]
    pub role: Option<String>, // "admin" | "editor" | "viewer"，默认 "viewer"
}

/// 重置密码请求体
#[derive(Debug, Deserialize)]
pub struct ResetPasswordRequest {
    pub new_password: String,
}

/// GET /admin/users — 管理员用户列表页面（Askama 模板）
pub async fn admin_users_page_handler(
    req: HttpRequest,
    auth_db: web::Data<Arc<AuthDatabase>>,
    app_state: web::Data<Arc<crate::state::AppState>>,
) -> impl Responder {
    if let Err(resp) = require_admin(&req) {
        return resp;
    }

    let db = Arc::clone(&*auth_db);
    let users = match tokio::task::spawn_blocking(move || db.list_users())
        .await
        .unwrap_or_else(|e| Err(anyhow::anyhow!("spawn_blocking panic: {}", e)))
    {
        Ok(u) => u,
        Err(e) => {
            tracing::error!("获取用户列表失败: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({"error": "获取失败"}));
        }
    };

    use crate::templates::AdminUsersTemplate;
    use crate::sidebar::flatten_sidebar;
    use askama::Template;

    let sidebar = app_state.sidebar.read().await;
    let flat_sidebar = flatten_sidebar(&sidebar);
    let empty_backlinks: Vec<String> = Vec::new();

    let user_data: Vec<(String, String, bool)> = users.iter()
        .map(|u| (u.username.clone(), u.role.as_str().to_string(), u.enabled))
        .collect();

    let tmpl = AdminUsersTemplate {
        title: "用户管理",
        sidebar: &flat_sidebar,
        backlinks: &empty_backlinks,
        users: &user_data,
    };

    match tmpl.render() {
        Ok(html) => HttpResponse::Ok().content_type("text/html").body(html),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": format!("模板渲染失败: {}", e)})),
    }
}

/// GET /api/admin/users — 获取所有用户列表（JSON）
pub async fn list_users_handler(
    req: HttpRequest,
    auth_db: web::Data<Arc<AuthDatabase>>,
) -> impl Responder {
    if let Err(resp) = require_admin(&req) {
        return resp;
    }

    let db = Arc::clone(&*auth_db);
    match tokio::task::spawn_blocking(move || db.list_users())
        .await
        .unwrap_or_else(|e| Err(anyhow::anyhow!("spawn_blocking panic: {}", e)))
    {
        Ok(users) => {
            let resp: Vec<serde_json::Value> = users.iter().map(|u| serde_json::json!({
                "username": u.username,
                "role": u.role.as_str(),
                "enabled": u.enabled,
                "created_at": u.created_at.to_rfc3339(),
                "last_login": u.last_login.map(|t| t.to_rfc3339()),
            })).collect();
            HttpResponse::Ok().json(serde_json::json!({"users": resp}))
        }
        Err(e) => {
            tracing::error!("获取用户列表失败: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "获取失败"}))
        }
    }
}

/// POST /api/admin/users — 创建新用户（admin only）
pub async fn create_user_handler(
    req: HttpRequest,
    body: web::Json<CreateUserRequest>,
    auth_db: web::Data<Arc<AuthDatabase>>,
) -> impl Responder {
    if let Err(resp) = require_admin(&req) {
        return resp;
    }

    let username = body.username.trim().to_string();
    if username.is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({"error": "用户名不能为空"}));
    }
    if body.password.is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({"error": "密码不能为空"}));
    }

    // 检查用户是否已存在
    let db_check = Arc::clone(&*auth_db);
    let uname_check = username.clone();
    let exists = tokio::task::spawn_blocking(move || db_check.get_user(&uname_check))
        .await
        .unwrap_or(Ok(None));

    match exists {
        Ok(Some(_)) => return HttpResponse::Conflict().json(serde_json::json!({"error": "用户名已存在"})),
        Err(e) => return HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})),
        Ok(None) => {}
    }

    // bcrypt 密码哈希（CPU 密集，移入 spawn_blocking）
    let pwd = body.password.clone();
    let hash = match tokio::task::spawn_blocking(move || PasswordManager::hash_password(&pwd))
        .await
        .unwrap_or_else(|e| Err(anyhow::anyhow!("spawn_blocking panic: {}", e)))
    {
        Ok(h) => h,
        Err(e) => return HttpResponse::InternalServerError().json(serde_json::json!({"error": format!("密码加密失败: {}", e)})),
    };

    let role = UserRole::parse(body.role.as_deref().unwrap_or("viewer"));
    let db = Arc::clone(&*auth_db);
    let uname = username.clone();
    match tokio::task::spawn_blocking(move || db.create_user_with_role(&uname, &hash, role))
        .await
        .unwrap_or_else(|e| Err(anyhow::anyhow!("spawn_blocking panic: {}", e)))
    {
        Ok(user) => {
            tracing::info!("✅ 管理员创建新用户: {}", user.username);
            HttpResponse::Ok().json(serde_json::json!({
                "message": "用户创建成功",
                "username": user.username,
                "role": user.role.as_str(),
            }))
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": format!("创建失败: {}", e)})),
    }
}

/// DELETE /api/admin/users/{username} — 删除用户（admin only，不能删除自己）
pub async fn delete_user_handler(
    req: HttpRequest,
    path: web::Path<String>,
    auth_db: web::Data<Arc<AuthDatabase>>,
) -> impl Responder {
    if let Err(resp) = require_admin(&req) {
        return resp;
    }

    let target = path.into_inner();

    // 不能删除当前登录的管理员自己
    let current = req.extensions().get::<String>().cloned().unwrap_or_default();
    if current == target {
        return HttpResponse::BadRequest().json(serde_json::json!({"error": "不能删除当前登录账户"}));
    }

    // 获取用户以确认存在
    let db_check = Arc::clone(&*auth_db);
    let t_clone = target.clone();
    let user_opt = tokio::task::spawn_blocking(move || db_check.get_user(&t_clone))
        .await
        .unwrap_or(Ok(None));

    match user_opt {
        Ok(None) => return HttpResponse::NotFound().json(serde_json::json!({"error": "用户不存在"})),
        Err(e) => return HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})),
        Ok(Some(_)) => {}
    }

    // 执行删除（通过 update_user 设置 enabled=false，或直接删除）
    // 使用 update_user 禁用更安全（保留数据）
    let db = Arc::clone(&*auth_db);
    let t_clone2 = target.clone();
    match tokio::task::spawn_blocking(move || {
        let mut user = db.get_user(&t_clone2)?.ok_or_else(|| anyhow::anyhow!("用户不存在"))?;
        user.enabled = false;
        db.update_user(&user)
    })
    .await
    .unwrap_or_else(|e| Err(anyhow::anyhow!("spawn_blocking panic: {}", e)))
    {
        Ok(_) => {
            tracing::info!("✅ 管理员禁用用户: {}", target);
            HttpResponse::Ok().json(serde_json::json!({"message": "用户已禁用"}))
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})),
    }
}

/// POST /api/admin/users/{username}/reset-password — 重置指定用户密码（admin only）
pub async fn reset_user_password_handler(
    req: HttpRequest,
    path: web::Path<String>,
    body: web::Json<ResetPasswordRequest>,
    auth_db: web::Data<Arc<AuthDatabase>>,
) -> impl Responder {
    if let Err(resp) = require_admin(&req) {
        return resp;
    }

    let target = path.into_inner();

    if body.new_password.is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({"error": "新密码不能为空"}));
    }

    let pwd = body.new_password.clone();
    let hash = match tokio::task::spawn_blocking(move || PasswordManager::hash_password(&pwd))
        .await
        .unwrap_or_else(|e| Err(anyhow::anyhow!("spawn_blocking panic: {}", e)))
    {
        Ok(h) => h,
        Err(e) => return HttpResponse::InternalServerError().json(serde_json::json!({"error": format!("密码加密失败: {}", e)})),
    };

    let db = Arc::clone(&*auth_db);
    let t_clone = target.clone();
    match tokio::task::spawn_blocking(move || db.change_password(&t_clone, &hash))
        .await
        .unwrap_or_else(|e| Err(anyhow::anyhow!("spawn_blocking panic: {}", e)))
    {
        Ok(_) => {
            tracing::info!("✅ 管理员重置用户密码: {}", target);
            HttpResponse::Ok().json(serde_json::json!({"message": "密码重置成功"}))
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})),
    }
}
