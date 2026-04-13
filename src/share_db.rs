//! 分享链接数据库模块
//!
//! 管理分享链接的创建、查询、验证和删除

use anyhow::{Context, Result};
use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::{Duration, SystemTime};
use tracing::{debug, info};
use uuid::Uuid;

/// 分享链接密码的 bcrypt 哈希成本，用于保护密码不被数据库泄露所暴露
const PASSWORD_BCRYPT_COST: u32 = bcrypt::DEFAULT_COST;

/// 分享链接表定义
/// Key: share_token (String)
/// Value: ShareLink (序列化为 JSON)
const SHARE_LINKS_TABLE: TableDefinition<&str, &str> = TableDefinition::new("share_links");

/// 分享链接数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareLink {
    /// 分享 token（唯一标识符）
    pub token: String,

    /// 被分享的笔记路径
    pub note_path: String,

    /// 创建者用户名
    pub creator: String,

    /// 创建时间
    pub created_at: SystemTime,

    /// 过期时间（None 表示永久有效）
    pub expires_at: Option<SystemTime>,

    /// 访问密码的 bcrypt 哈希（None 表示无需密码）
    /// 存储哈希而非明文，防止数据库泄露导致密码暴露
    pub password_hash: Option<String>,

    /// 访问次数限制（None 表示无限制）
    pub max_visits: Option<u32>,

    /// 当前访问次数
    pub visit_count: u32,
}

impl ShareLink {
    /// 创建新的分享链接
    ///
    /// 若提供了密码，将使用 bcrypt 对其进行单向哈希后存储，
    /// 防止数据库文件被盗取时密码明文暴露。
    pub fn new(
        note_path: String,
        creator: String,
        expires_in: Option<Duration>,
        password: Option<String>,
        max_visits: Option<u32>,
    ) -> Self {
        let token = Uuid::new_v4().to_string();
        let created_at = SystemTime::now();
        let expires_at = expires_in.map(|d| created_at + d);

        // 对密码进行 bcrypt 哈希，不存储明文
        let password_hash = password.map(|p| {
            bcrypt::hash(p, PASSWORD_BCRYPT_COST)
                .expect("bcrypt 哈希失败（不应发生）")
        });

        Self {
            token,
            note_path,
            creator,
            created_at,
            expires_at,
            password_hash,
            max_visits,
            visit_count: 0,
        }
    }

    /// 检查分享链接是否有效
    pub fn is_valid(&self) -> bool {
        // 检查是否过期
        if let Some(expires_at) = self.expires_at {
            if SystemTime::now() > expires_at {
                return false;
            }
        }

        // 检查访问次数限制
        if let Some(max_visits) = self.max_visits {
            if self.visit_count >= max_visits {
                return false;
            }
        }

        true
    }

    /// 验证访问密码
    ///
    /// 使用 bcrypt::verify 比较提供的明文密码与存储的哈希值。
    pub fn verify_password(&self, password: Option<&str>) -> bool {
        match (&self.password_hash, password) {
            (None, _) => true, // 无密码保护
            (Some(hash), Some(provided)) => {
                // bcrypt::verify 失败时（哈希格式错误等）视为验证失败
                bcrypt::verify(provided, hash).unwrap_or(false)
            }
            (Some(_), None) => false, // 需要密码但未提供
        }
    }

    /// 增加访问次数
    pub fn increment_visit(&mut self) {
        self.visit_count += 1;
    }
}

/// 分享链接数据库
pub struct ShareDatabase {
    db: Database,
}

impl ShareDatabase {
    /// 打开或创建数据库
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let db = Database::create(path.as_ref())
            .with_context(|| format!("无法打开分享链接数据库: {}", path.as_ref().display()))?;

        info!("✅ 分享链接数据库已打开: {}", path.as_ref().display());

        Ok(Self { db })
    }

    /// 创建新的分享链接
    pub fn create_share(&self, share_link: &ShareLink) -> Result<()> {
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(SHARE_LINKS_TABLE)?;
            let json = serde_json::to_string(share_link)?;
            table.insert(share_link.token.as_str(), json.as_str())?;
        }
        write_txn.commit()?;

        debug!(
            "✅ 创建分享链接: {} -> {}",
            share_link.token, share_link.note_path
        );

        Ok(())
    }

    /// 通过 token 获取分享链接
    pub fn get_share(&self, token: &str) -> Result<Option<ShareLink>> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(SHARE_LINKS_TABLE)?;

        let result = table.get(token)?;

        if let Some(json_value) = result {
            let json = json_value.value();
            let share_link: ShareLink = serde_json::from_str(json)?;
            Ok(Some(share_link))
        } else {
            Ok(None)
        }
    }

    /// 获取用户创建的所有分享链接
    pub fn get_user_shares(&self, username: &str) -> Result<Vec<ShareLink>> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(SHARE_LINKS_TABLE)?;

        let mut shares = Vec::new();

        for item in table.iter()? {
            let (_, json_value) = item?;
            let json = json_value.value();
            let share_link: ShareLink = serde_json::from_str(json)?;

            if share_link.creator == username {
                shares.push(share_link);
            }
        }

        Ok(shares)
    }

    /// 获取指定笔记的所有分享链接
    pub fn get_note_shares(&self, note_path: &str) -> Result<Vec<ShareLink>> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(SHARE_LINKS_TABLE)?;

        let mut shares = Vec::new();

        for item in table.iter()? {
            let (_, json_value) = item?;
            let json = json_value.value();
            let share_link: ShareLink = serde_json::from_str(json)?;

            if share_link.note_path == note_path {
                shares.push(share_link);
            }
        }

        Ok(shares)
    }

    /// 更新分享链接（用于增加访问次数）
    pub fn update_share(&self, share_link: &ShareLink) -> Result<()> {
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(SHARE_LINKS_TABLE)?;
            let json = serde_json::to_string(share_link)?;
            table.insert(share_link.token.as_str(), json.as_str())?;
        }
        write_txn.commit()?;

        debug!("✅ 更新分享链接: {}", share_link.token);

        Ok(())
    }

    /// 删除分享链接
    pub fn delete_share(&self, token: &str) -> Result<bool> {
        let write_txn = self.db.begin_write()?;
        let removed = {
            let mut table = write_txn.open_table(SHARE_LINKS_TABLE)?;
            table.remove(token)?.is_some()
        };
        write_txn.commit()?;

        if removed {
            debug!("✅ 删除分享链接: {}", token);
        }

        Ok(removed)
    }

    /// 清理过期的分享链接
    pub fn cleanup_expired(&self) -> Result<usize> {
        let mut expired_tokens = Vec::new();

        // 收集过期的 token
        {
            let read_txn = self.db.begin_read()?;
            let table = read_txn.open_table(SHARE_LINKS_TABLE)?;

            for item in table.iter()? {
                let (token_value, json_value) = item?;
                let json = json_value.value();
                let share_link: ShareLink = serde_json::from_str(json)?;

                if !share_link.is_valid() {
                    expired_tokens.push(token_value.value().to_string());
                }
            }
        }

        // 删除过期的分享链接
        let count = expired_tokens.len();
        for token in expired_tokens {
            self.delete_share(&token)?;
        }

        if count > 0 {
            info!("🧹 清理了 {} 个过期的分享链接", count);
        }

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_share_link_creation() {
        let share = ShareLink::new(
            "test/note.md".to_string(),
            "admin".to_string(),
            Some(Duration::from_secs(3600)), // 1 小时
            Some("password123".to_string()),
            Some(10),
        );

        assert!(!share.token.is_empty());
        assert_eq!(share.note_path, "test/note.md");
        assert_eq!(share.creator, "admin");
        assert_eq!(share.visit_count, 0);
        assert!(share.is_valid());
    }

    #[test]
    fn test_share_link_validity() {
        // 测试已过期的链接
        let mut expired_share = ShareLink::new(
            "test/note.md".to_string(),
            "admin".to_string(),
            Some(Duration::from_secs(0)), // 立即过期
            None,
            None,
        );

        // 等待一小段时间确保过期
        std::thread::sleep(Duration::from_millis(10));
        assert!(!expired_share.is_valid());

        // 测试访问次数限制
        expired_share.expires_at = None; // 移除过期时间
        expired_share.max_visits = Some(2);
        assert!(expired_share.is_valid());

        expired_share.increment_visit();
        assert!(expired_share.is_valid());

        expired_share.increment_visit();
        assert!(!expired_share.is_valid()); // 达到上限
    }

    #[test]
    fn test_password_verification() {
        let share_with_pwd = ShareLink::new(
            "test/note.md".to_string(),
            "admin".to_string(),
            None,
            Some("secret".to_string()),
            None,
        );

        // 验证 bcrypt 哈希后的密码不是明文
        assert!(share_with_pwd.password_hash.is_some(), "密码应被存储为哈希");
        let hash = share_with_pwd.password_hash.as_ref().unwrap();
        assert_ne!(hash.as_str(), "secret", "存储的不应是明文密码");
        assert!(hash.starts_with("$2"), "密码哈希应符合 bcrypt 格式（$2x$...）");

        // 验证正确密码通过 bcrypt::verify 验证
        assert!(share_with_pwd.verify_password(Some("secret")), "正确密码应验证成功");
        assert!(!share_with_pwd.verify_password(Some("wrong")), "错误密码应验证失败");
        assert!(!share_with_pwd.verify_password(None), "无密码时应验证失败");

        let share_without_pwd = ShareLink::new(
            "test/note.md".to_string(),
            "admin".to_string(),
            None,
            None,
            None,
        );

        assert!(share_without_pwd.verify_password(None), "无密码保护时任何访问均应通过");
        assert!(share_without_pwd.verify_password(Some("anything")), "无密码保护时提供密码也应通过");
    }
}
