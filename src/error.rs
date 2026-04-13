use std::error::Error as StdError;
use std::fmt;

/// 应用自定义错误类型
#[derive(Debug)]
pub enum AppError {
    /// Git 操作错误
    GitError(String),

    /// 文件系统错误
    IoError(std::io::Error),

    /// Markdown 处理错误
    MarkdownError(String),

    /// 搜索引擎错误
    SearchError(String),

    /// 持久化错误
    PersistenceError(String),

    /// 认证错误
    AuthError(String),

    /// 配置错误
    ConfigError(String),

    /// HTTP 错误
    HttpError(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::GitError(msg) => write!(f, "Git 错误: {}", msg),
            AppError::IoError(err) => write!(f, "IO 错误: {}", err),
            AppError::MarkdownError(msg) => write!(f, "Markdown 处理错误: {}", msg),
            AppError::SearchError(msg) => write!(f, "搜索错误: {}", msg),
            AppError::PersistenceError(msg) => write!(f, "持久化错误: {}", msg),
            AppError::AuthError(msg) => write!(f, "认证错误: {}", msg),
            AppError::ConfigError(msg) => write!(f, "配置错误: {}", msg),
            AppError::HttpError(msg) => write!(f, "HTTP 错误: {}", msg),
        }
    }
}

impl StdError for AppError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            AppError::IoError(err) => Some(err),
            _ => None,
        }
    }
}

// 从标准库错误类型转换
impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::IoError(err)
    }
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        AppError::HttpError(err.to_string())
    }
}

/// 应用统一的 Result 类型
pub type Result<T> = std::result::Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = AppError::GitError("commit not found".to_string());
        assert_eq!(err.to_string(), "Git 错误: commit not found");

        let err = AppError::MarkdownError("invalid syntax".to_string());
        assert_eq!(err.to_string(), "Markdown 处理错误: invalid syntax");
    }

    #[test]
    fn test_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let app_err: AppError = io_err.into();

        assert!(matches!(app_err, AppError::IoError(_)));
    }
}
