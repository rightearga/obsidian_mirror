//! 认证中间件
//! 
//! 拦截需要认证的请求，验证 JWT token

use actix_web::{
    body::EitherBody,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage, HttpResponse,
};
use futures_util::future::LocalBoxFuture;
use std::future::{ready, Ready};
use std::rc::Rc;

use crate::auth::JwtManager;

/// 认证中间件工厂
pub struct AuthMiddleware {
    jwt_manager: Rc<JwtManager>,
    enabled: bool,
}

impl AuthMiddleware {
    pub fn new(jwt_manager: JwtManager, enabled: bool) -> Self {
        Self {
            jwt_manager: Rc::new(jwt_manager),
            enabled,
        }
    }
}

impl<S, B> Transform<S, ServiceRequest> for AuthMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = AuthMiddlewareService<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AuthMiddlewareService {
            service: Rc::new(service),
            jwt_manager: self.jwt_manager.clone(),
            enabled: self.enabled,
        }))
    }
}

pub struct AuthMiddlewareService<S> {
    service: Rc<S>,
    jwt_manager: Rc<JwtManager>,
    enabled: bool,
}

impl<S, B> Service<ServiceRequest> for AuthMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        // 如果认证未启用，直接放行
        if !self.enabled {
            let service = self.service.clone();
            return Box::pin(async move {
                let res = service.call(req).await?;
                Ok(res.map_into_left_body())
            });
        }

        let path = req.path().to_string();

        // 公开路径判断：
        // - 精确路径（不以 / 结尾）使用 == 精确匹配，防止 /login-admin 等路径绕过认证
        // - 有子路径的前缀（以 / 结尾）使用 starts_with 前缀匹配
        let is_public = path == "/login"
            || path == "/api/auth/login"
            || path.starts_with("/static/")
            || path.starts_with("/share/");

        // 如果是公开路径，直接放行
        if is_public {
            let service = self.service.clone();
            return Box::pin(async move {
                let res = service.call(req).await?;
                Ok(res.map_into_left_body())
            });
        }
        
        // 从 Cookie 或 Authorization header 中获取 token
        let token = req
            .cookie("auth_token")
            .map(|c| c.value().to_string())
            .or_else(|| {
                req.headers()
                    .get("Authorization")
                    .and_then(|h| h.to_str().ok())
                    .and_then(|h| h.strip_prefix("Bearer "))
                    .map(|s| s.to_string())
            });
        
        let jwt_manager = self.jwt_manager.clone();
        
        match token {
            Some(token) => {
                // 验证 token
                match jwt_manager.verify_token(&token) {
                    Ok(claims) => {
                        // Token 有效，将用户信息添加到请求扩展中
                        req.extensions_mut().insert(claims.sub.clone());
                        
                        let service = self.service.clone();
                        Box::pin(async move {
                            let res = service.call(req).await?;
                            Ok(res.map_into_left_body())
                        })
                    }
                    Err(_) => {
                        // Token 无效：API 路径返回 401 JSON，页面路径重定向到 /login
                        let is_api = path.starts_with("/api/");
                        Box::pin(async move {
                            let (req, _) = req.into_parts();
                            let response = if is_api {
                                HttpResponse::Unauthorized()
                                    .content_type("application/json")
                                    .body(r#"{"error":"未认证，请先登录"}"#)
                                    .map_into_right_body()
                            } else {
                                HttpResponse::Found()
                                    .insert_header(("Location", "/login"))
                                    .finish()
                                    .map_into_right_body()
                            };
                            Ok(ServiceResponse::new(req, response))
                        })
                    }
                }
            }
            None => {
                // 无 token：API 路径返回 401 JSON，页面路径重定向到 /login
                let is_api = path.starts_with("/api/");
                Box::pin(async move {
                    let (req, _) = req.into_parts();
                    let response = if is_api {
                        HttpResponse::Unauthorized()
                            .content_type("application/json")
                            .body(r#"{"error":"未认证，请先登录"}"#)
                            .map_into_right_body()
                    } else {
                        HttpResponse::Found()
                            .insert_header(("Location", "/login"))
                            .finish()
                            .map_into_right_body()
                    };
                    Ok(ServiceResponse::new(req, response))
                })
            }
        }
    }
}
