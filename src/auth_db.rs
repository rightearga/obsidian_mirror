//! 用户认证数据库模块
//!
//! 使用 redb 作为嵌入式数据库存储用户信息

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use redb::{Database, ReadableDatabase, ReadableTable, ReadableTableMetadata, TableDefinition};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;

/// 用户表定义：用户名 -> 用户数据（JSON）
const USERS_TABLE: TableDefinition<&str, &str> = TableDefinition::new("users");

/// 用户信息结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// 用户名（唯一标识）
    pub username: String,
    /// 密码哈希（bcrypt）
    pub password_hash: String,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 最后登录时间
    pub last_login: Option<DateTime<Utc>>,
    /// 是否启用
    pub enabled: bool,
}

/// 用户数据库管理器
#[derive(Clone)]
pub struct AuthDatabase {
    db: Arc<Database>,
}

impl AuthDatabase {
    /// 打开或创建数据库（自动创建父目录）
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        // 确保父目录存在，防止 OS error 3（路径不存在）
        if let Some(parent) = path.as_ref().parent()
            && !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("无法创建认证数据库目录: {}", parent.display()))?;
            }
        let db = Database::create(path.as_ref()).context("无法创建/打开用户数据库")?;

        // 初始化表结构
        let write_txn = db.begin_write()?;
        {
            let _ = write_txn.open_table(USERS_TABLE)?;
        }
        write_txn.commit()?;

        Ok(Self { db: Arc::new(db) })
    }

    /// 创建新用户
    pub fn create_user(&self, username: &str, password_hash: &str) -> Result<User> {
        let user = User {
            username: username.to_string(),
            password_hash: password_hash.to_string(),
            created_at: Utc::now(),
            last_login: None,
            enabled: true,
        };

        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(USERS_TABLE)?;
            let user_json = serde_json::to_string(&user)?;
            table.insert(username, user_json.as_str())?;
        }
        write_txn.commit()?;

        Ok(user)
    }

    /// 根据用户名获取用户
    pub fn get_user(&self, username: &str) -> Result<Option<User>> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(USERS_TABLE)?;

        match table.get(username)? {
            Some(user_json) => {
                let user: User = serde_json::from_str(user_json.value())?;
                Ok(Some(user))
            }
            None => Ok(None),
        }
    }

    /// 更新用户信息
    pub fn update_user(&self, user: &User) -> Result<()> {
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(USERS_TABLE)?;
            let user_json = serde_json::to_string(user)?;
            table.insert(user.username.as_str(), user_json.as_str())?;
        }
        write_txn.commit()?;
        Ok(())
    }

    /// 更新最后登录时间
    pub fn update_last_login(&self, username: &str) -> Result<()> {
        if let Some(mut user) = self.get_user(username)? {
            user.last_login = Some(Utc::now());
            self.update_user(&user)?;
        }
        Ok(())
    }

    /// 修改用户密码
    pub fn change_password(&self, username: &str, new_password_hash: &str) -> Result<()> {
        if let Some(mut user) = self.get_user(username)? {
            user.password_hash = new_password_hash.to_string();
            self.update_user(&user)?;
            Ok(())
        } else {
            anyhow::bail!("用户不存在: {}", username)
        }
    }

    /// 检查数据库是否为空（无用户）
    pub fn is_empty(&self) -> Result<bool> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(USERS_TABLE)?;
        Ok(table.is_empty()?)
    }

    /// 获取所有用户（用于管理）
    pub fn list_users(&self) -> Result<Vec<User>> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(USERS_TABLE)?;

        let mut users = Vec::new();
        for item in table.iter()? {
            let (_, user_json) = item?;
            let user: User = serde_json::from_str(user_json.value())?;
            users.push(user);
        }

        Ok(users)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_create_and_get_user() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = AuthDatabase::open(&db_path).unwrap();

        // 创建用户
        let user = db.create_user("admin", "hashed_password").unwrap();
        assert_eq!(user.username, "admin");
        assert_eq!(user.password_hash, "hashed_password");

        // 获取用户
        let retrieved = db.get_user("admin").unwrap().unwrap();
        assert_eq!(retrieved.username, "admin");
        assert_eq!(retrieved.password_hash, "hashed_password");
    }

    #[test]
    fn test_update_last_login() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = AuthDatabase::open(&db_path).unwrap();

        db.create_user("admin", "password").unwrap();

        // 更新登录时间
        db.update_last_login("admin").unwrap();

        let user = db.get_user("admin").unwrap().unwrap();
        assert!(user.last_login.is_some());
    }
}
