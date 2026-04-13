use anyhow::Context;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub repo_url: String,
    pub local_path: PathBuf,
    pub listen_addr: String,
    #[serde(default = "default_workers")]
    pub workers: usize,
    pub ignore_patterns: Vec<String>,

    // 数据库配置
    #[serde(default)]
    pub database: DatabaseConfig,

    // 安全认证配置
    #[serde(default)]
    pub security: SecurityConfig,
}

/// 数据库配置
#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    /// 搜索索引数据库路径
    #[serde(default = "default_index_db_path")]
    pub index_db_path: PathBuf,

    /// 用户认证数据库路径
    #[serde(default = "default_auth_db_path")]
    pub auth_db_path: PathBuf,

    /// 分享链接数据库路径
    #[serde(default = "default_share_db_path")]
    pub share_db_path: PathBuf,

    /// 阅读进度数据库路径
    #[serde(default = "default_reading_progress_db_path")]
    pub reading_progress_db_path: PathBuf,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            index_db_path: default_index_db_path(),
            auth_db_path: default_auth_db_path(),
            share_db_path: default_share_db_path(),
            reading_progress_db_path: default_reading_progress_db_path(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct SecurityConfig {
    /// 是否启用认证
    #[serde(default)]
    pub auth_enabled: bool,

    /// JWT 密钥（如果启用认证，必须设置）
    #[serde(default = "default_jwt_secret")]
    pub jwt_secret: String,

    /// Token 有效期（小时）
    #[serde(default = "default_token_lifetime")]
    pub token_lifetime_hours: i64,

    /// 默认管理员用户名
    #[serde(default = "default_admin_username")]
    pub default_admin_username: String,

    /// 默认管理员密码（仅用于首次初始化）
    #[serde(default = "default_admin_password")]
    pub default_admin_password: String,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            auth_enabled: false,
            jwt_secret: default_jwt_secret(),
            token_lifetime_hours: default_token_lifetime(),
            default_admin_username: default_admin_username(),
            default_admin_password: default_admin_password(),
        }
    }
}

fn default_index_db_path() -> PathBuf {
    PathBuf::from("./index.db")
}

fn default_auth_db_path() -> PathBuf {
    PathBuf::from("./auth.db")
}

fn default_share_db_path() -> PathBuf {
    PathBuf::from("./share.db")
}

fn default_reading_progress_db_path() -> PathBuf {
    PathBuf::from("./reading_progress.db")
}

fn default_jwt_secret() -> String {
    "CHANGE_THIS_TO_A_RANDOM_SECRET_KEY".to_string()
}

fn default_token_lifetime() -> i64 {
    24 // 24 小时
}

fn default_admin_username() -> String {
    "admin".to_string()
}

fn default_admin_password() -> String {
    "admin".to_string()
}

fn default_workers() -> usize {
    num_cpus::get()
}

impl AppConfig {
    pub fn load(path: &str) -> anyhow::Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path))?;
        let config: Self =
            ron::from_str(&content).with_context(|| "Failed to parse config file")?;
        Ok(config)
    }
}
