//! 认证相关的 HTTP 处理器

use actix_web::{
    cookie::{Cookie, SameSite},
    web, HttpMessage, HttpRequest, HttpResponse, Responder,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use time::Duration;

use crate::auth::{JwtManager, PasswordManager};
use crate::auth_db::AuthDatabase;

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
            match jwt_manager.generate_token(&user.username) {
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
