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
) -> impl Responder {
    // 获取用户
    let user = match auth_db.get_user(&form.username) {
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

    // 验证密码
    match PasswordManager::verify_password(&form.password, &user.password_hash) {
        Ok(true) => {
            // 生成 JWT token
            match jwt_manager.generate_token(&user.username) {
                Ok(token) => {
                    // 更新最后登录时间
                    let _ = auth_db.update_last_login(&user.username);

                    // 设置 Cookie（有效期根据 remember_me 决定）
                    let max_age = if form.remember_me.unwrap_or(false) {
                        Duration::days(30)
                    } else {
                        Duration::hours(24)
                    };

                    // 安全属性：HttpOnly 防止 JS 读取；Secure 防止 HTTP 明文传输；
                    // SameSite::Lax 防止 CSRF 同时允许顶层导航跳转后携带 Cookie
                    let cookie = Cookie::build("auth_token", token.clone())
                        .path("/")
                        .max_age(max_age)
                        .http_only(true)
                        .secure(true)
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
pub async fn logout_handler() -> impl Responder {
    // 删除 Cookie（属性必须与设置时完全一致，浏览器才能正确清除）
    let cookie = Cookie::build("auth_token", "")
        .path("/")
        .max_age(Duration::seconds(0))
        .http_only(true)
        .secure(true)
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

    // 获取用户
    let user = match auth_db.get_user(&username) {
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

    // 验证旧密码
    match PasswordManager::verify_password(&form.old_password, &user.password_hash) {
        Ok(true) => {
            // 加密新密码
            match PasswordManager::hash_password(&form.new_password) {
                Ok(new_hash) => {
                    // 更新密码
                    match auth_db.change_password(&username, &new_hash) {
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
