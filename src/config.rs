use anyhow::Context;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

/// 单个仓库配置（v1.7.4 多仓库支持）
///
/// 在 `config.ron` 的 `repos` 数组中使用，每项描述一个 Git 仓库。
/// 单仓库兼容：若 `repos` 为空，系统自动从顶级 `repo_url`/`local_path`/`ignore_patterns` 构造一个名为 "default" 的仓库。
#[derive(Debug, Deserialize, Clone)]
pub struct RepoConfig {
    /// 仓库唯一名称（用于 URL 前缀 /r/{name}/...）
    pub name: String,
    /// Git 仓库远程地址
    #[serde(default)]
    pub repo_url: String,
    /// 本地克隆路径
    pub local_path: PathBuf,
    /// 忽略的文件/目录模式（与全局 ignore_patterns 相同格式）
    #[serde(default)]
    pub ignore_patterns: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    /// 单仓库兼容字段：Git 仓库地址（若 repos 非空则忽略）
    #[serde(default)]
    pub repo_url: String,
    /// 单仓库兼容字段：本地路径（若 repos 非空则忽略）
    #[serde(default = "default_local_path")]
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

    /// 定时自动同步间隔（分钟），0 = 禁用
    #[serde(default)]
    pub sync_interval_minutes: u32,

    /// Webhook 触发同步配置
    #[serde(default)]
    pub webhook: WebhookConfig,

    /// 公开基础 URL（用于生成分享链接，优先于从请求头推断的 scheme+host）
    ///
    /// 反向代理场景（Nginx/Caddy）下建议设置，例如 `"https://notes.example.com"`。
    /// 未设置时使用 X-Forwarded-Proto header 推断，最终 fallback 到 Host header。
    #[serde(default)]
    pub public_base_url: Option<String>,

    /// 多仓库配置列表（v1.7.4）
    ///
    /// 非空时取代顶级 `repo_url`/`local_path`/`ignore_patterns`；
    /// 空时向后兼容，自动将顶级字段包装成名为 "default" 的单仓库。
    /// 第一个仓库为主仓库，持有后向兼容的无前缀路由。
    #[serde(default)]
    pub repos: Vec<RepoConfig>,
}

/// Webhook 配置
#[derive(Debug, Deserialize, Clone, Default)]
pub struct WebhookConfig {
    /// 是否启用 Webhook 触发同步端点
    #[serde(default)]
    pub enabled: bool,

    /// Webhook 共享密钥（用于验证 GitHub/GitLab 推送签名）
    #[serde(default)]
    pub secret: String,
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

    /// 是否强制在 Cookie 上设置 Secure 标志（仅 HTTPS 环境下应启用）
    ///
    /// 默认 `false`：HTTP 环境下（内网/开发）Cookie 可正常工作。
    /// 生产环境通过 Nginx/Caddy 反向代理启用 HTTPS 时，将此项设为 `true`。
    #[serde(default)]
    pub force_https_cookie: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            auth_enabled: false,
            jwt_secret: default_jwt_secret(),
            token_lifetime_hours: default_token_lifetime(),
            default_admin_username: default_admin_username(),
            default_admin_password: default_admin_password(),
            force_https_cookie: false,
        }
    }
}

fn default_local_path() -> PathBuf {
    PathBuf::from("./vault_data")
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
    /// 返回有效的仓库列表（v1.7.4）。
    ///
    /// 若 `repos` 非空，直接返回；否则从顶级 `repo_url`/`local_path`/`ignore_patterns`
    /// 构造一个名为 "default" 的单仓库以实现向后兼容。
    pub fn effective_repos(&self) -> Vec<RepoConfig> {
        if !self.repos.is_empty() {
            return self.repos.clone();
        }
        vec![RepoConfig {
            name: "default".to_string(),
            repo_url: self.repo_url.clone(),
            local_path: self.local_path.clone(),
            ignore_patterns: self.ignore_patterns.clone(),
        }]
    }

    /// 是否启用多仓库模式（`repos` 字段非空）。
    pub fn is_multi_vault(&self) -> bool {
        !self.repos.is_empty()
    }

    pub fn load(path: &str) -> anyhow::Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path))?;
        let config: Self =
            ron::from_str(&content).with_context(|| "Failed to parse config file")?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_single_vault_config() -> AppConfig {
        AppConfig {
            repo_url: "https://git.example.com/notes.git".to_string(),
            local_path: PathBuf::from("./vault"),
            listen_addr: "127.0.0.1:8080".to_string(),
            workers: 4,
            ignore_patterns: vec![".obsidian".to_string()],
            database: DatabaseConfig::default(),
            security: SecurityConfig::default(),
            sync_interval_minutes: 0,
            webhook: WebhookConfig::default(),
            public_base_url: None,
            repos: vec![],
        }
    }

    #[test]
    fn test_effective_repos_single_vault() {
        // 单仓库模式：从顶级字段构造名为 "default" 的仓库
        let config = make_single_vault_config();
        let repos = config.effective_repos();
        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0].name, "default");
        assert_eq!(repos[0].repo_url, "https://git.example.com/notes.git");
        assert_eq!(repos[0].local_path, PathBuf::from("./vault"));
        assert_eq!(repos[0].ignore_patterns, vec![".obsidian"]);
    }

    #[test]
    fn test_is_multi_vault_false_when_repos_empty() {
        let config = make_single_vault_config();
        assert!(!config.is_multi_vault());
    }

    #[test]
    fn test_effective_repos_multi_vault() {
        // 多仓库模式：直接返回 repos 列表
        let mut config = make_single_vault_config();
        config.repos = vec![
            RepoConfig {
                name: "personal".to_string(),
                repo_url: "https://git.example.com/personal.git".to_string(),
                local_path: PathBuf::from("./personal"),
                ignore_patterns: vec![],
            },
            RepoConfig {
                name: "work".to_string(),
                repo_url: "https://git.example.com/work.git".to_string(),
                local_path: PathBuf::from("./work"),
                ignore_patterns: vec!["drafts/".to_string()],
            },
        ];
        let repos = config.effective_repos();
        assert_eq!(repos.len(), 2);
        assert_eq!(repos[0].name, "personal");
        assert_eq!(repos[1].name, "work");
        assert!(config.is_multi_vault());
    }

    #[test]
    fn test_effective_repos_multi_vault_ignores_top_level_fields() {
        // 多仓库模式下，顶级 repo_url/local_path 被忽略
        let mut config = make_single_vault_config();
        config.repos = vec![RepoConfig {
            name: "vault1".to_string(),
            repo_url: "https://git.example.com/v1.git".to_string(),
            local_path: PathBuf::from("./v1"),
            ignore_patterns: vec![],
        }];
        let repos = config.effective_repos();
        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0].name, "vault1");
        // 顶级 repo_url 不出现在结果中
        assert_ne!(repos[0].repo_url, "https://git.example.com/notes.git");
    }
}

