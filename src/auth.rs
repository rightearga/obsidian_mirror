//! JWT 认证和密码加密模块

use anyhow::{Context, Result};
use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

/// JWT 声明信息
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// 用户名
    pub sub: String,
    /// 过期时间（Unix 时间戳）
    pub exp: i64,
    /// 签发时间（Unix 时间戳）
    pub iat: i64,
    /// 用户角色（v1.5.3 新增；旧 token 中无此字段，默认反序列化为 "viewer"，重新登录后获得正确角色）
    #[serde(default = "default_role")]
    pub role: String,
}

/// JWT Claims 中 role 字段的默认值（旧 token 兼容）
fn default_role() -> String {
    "viewer".to_string()
}

/// JWT 管理器
#[derive(Clone)]
pub struct JwtManager {
    secret: String,
    token_lifetime_hours: i64,
}

impl JwtManager {
    /// 创建新的 JWT 管理器
    pub fn new(secret: String, token_lifetime_hours: i64) -> Self {
        Self {
            secret,
            token_lifetime_hours,
        }
    }

    /// 生成 JWT token（包含用户角色，供中间件做权限检查）
    pub fn generate_token(&self, username: &str, role: &str) -> Result<String> {
        let now = Utc::now();
        let expiration = now + Duration::hours(self.token_lifetime_hours);

        let claims = Claims {
            sub: username.to_string(),
            exp: expiration.timestamp(),
            iat: now.timestamp(),
            role: role.to_string(),
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.secret.as_bytes()),
        )
        .context("无法生成 JWT token")?;

        Ok(token)
    }

    /// 验证并解析 JWT token
    pub fn verify_token(&self, token: &str) -> Result<Claims> {
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.secret.as_bytes()),
            &Validation::default(),
        )
        .context("无法验证 JWT token")?;

        Ok(token_data.claims)
    }
}

/// 密码加密管理器
pub struct PasswordManager;

impl PasswordManager {
    /// 对密码进行哈希加密
    pub fn hash_password(password: &str) -> Result<String> {
        hash(password, DEFAULT_COST).context("无法加密密码")
    }

    /// 验证密码是否匹配
    pub fn verify_password(password: &str, hash: &str) -> Result<bool> {
        verify(password, hash).context("无法验证密码")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwt_generation_and_verification() {
        let jwt_manager = JwtManager::new("test_secret".to_string(), 24);

        // 生成 token
        let token = jwt_manager.generate_token("admin", "admin").unwrap();
        assert!(!token.is_empty());

        // 验证 token
        let claims = jwt_manager.verify_token(&token).unwrap();
        assert_eq!(claims.sub, "admin");
    }

    #[test]
    fn test_password_hashing_and_verification() {
        let password = "my_secure_password";

        // 加密密码
        let hash = PasswordManager::hash_password(password).unwrap();
        assert_ne!(hash, password);

        // 验证正确密码
        assert!(PasswordManager::verify_password(password, &hash).unwrap());

        // 验证错误密码
        assert!(!PasswordManager::verify_password("wrong_password", &hash).unwrap());
    }
}
