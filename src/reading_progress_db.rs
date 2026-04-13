//! 阅读进度数据库模块
//!
//! 管理用户的笔记阅读进度和历史记录

use anyhow::{Context, Result};
use redb::{Database, ReadableDatabase, TableDefinition};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::SystemTime;
use tracing::{debug, info};

/// 阅读进度表定义
/// Key: "{username}:{note_path}" (String)
/// Value: ReadingProgress (序列化为 JSON)
const READING_PROGRESS_TABLE: TableDefinition<&str, &str> =
    TableDefinition::new("reading_progress");

/// 阅读历史表定义
/// Key: "{username}:{timestamp_nanos}:{note_path}" (String)
/// Value: ReadingHistory (序列化为 JSON)
const READING_HISTORY_TABLE: TableDefinition<&str, &str> = TableDefinition::new("reading_history");

/// 阅读进度数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadingProgress {
    /// 用户名
    pub username: String,

    /// 笔记路径
    pub note_path: String,

    /// 笔记标题
    pub note_title: String,

    /// 滚动位置（像素）
    pub scroll_position: u32,

    /// 滚动百分比（0-100）
    pub scroll_percentage: f32,

    /// 最后阅读时间
    pub last_read_at: SystemTime,

    /// 阅读时长（秒）
    pub reading_duration: u64,

    /// 是否已读完
    pub is_completed: bool,
}

impl ReadingProgress {
    /// 创建新的阅读进度记录
    pub fn new(
        username: String,
        note_path: String,
        note_title: String,
        scroll_position: u32,
        scroll_percentage: f32,
    ) -> Self {
        Self {
            username,
            note_path,
            note_title,
            scroll_position,
            scroll_percentage,
            last_read_at: SystemTime::now(),
            reading_duration: 0,
            is_completed: scroll_percentage >= 95.0, // 滚动到 95% 认为已读完
        }
    }

    /// 更新阅读进度
    pub fn update(&mut self, scroll_position: u32, scroll_percentage: f32, duration_delta: u64) {
        self.scroll_position = scroll_position;
        self.scroll_percentage = scroll_percentage;
        self.last_read_at = SystemTime::now();
        self.reading_duration += duration_delta;
        self.is_completed = scroll_percentage >= 95.0;
    }

    /// 生成数据库键
    pub fn db_key(&self) -> String {
        format!("{}:{}", self.username, self.note_path)
    }
}

/// 阅读历史记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadingHistory {
    /// 用户名
    pub username: String,

    /// 笔记路径
    pub note_path: String,

    /// 笔记标题
    pub note_title: String,

    /// 访问时间
    pub visited_at: SystemTime,

    /// 阅读时长（秒）
    pub duration: u64,
}

impl ReadingHistory {
    /// 创建新的历史记录
    pub fn new(username: String, note_path: String, note_title: String, duration: u64) -> Self {
        Self {
            username,
            note_path,
            note_title,
            visited_at: SystemTime::now(),
            duration,
        }
    }

    /// 生成数据库键（带时间戳保证唯一性）
    pub fn db_key(&self) -> String {
        let timestamp = self
            .visited_at
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        format!("{}:{}:{}", self.username, timestamp, self.note_path)
    }
}

/// 阅读进度数据库
pub struct ReadingProgressDatabase {
    db: Database,
}

impl ReadingProgressDatabase {
    /// 打开或创建数据库（自动创建父目录）
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        // 确保父目录存在
        if let Some(parent) = path.as_ref().parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("无法创建阅读进度数据库目录: {}", parent.display()))?;
            }
        }
        let db = Database::create(path.as_ref())
            .with_context(|| format!("无法打开阅读进度数据库: {}", path.as_ref().display()))?;

        info!("✅ 阅读进度数据库已打开: {}", path.as_ref().display());

        // 初始化表结构（如果表不存在则创建）
        let write_txn = db.begin_write()?;
        {
            // 创建阅读进度表
            let _progress_table = write_txn.open_table(READING_PROGRESS_TABLE)?;
            // 创建阅读历史表
            let _history_table = write_txn.open_table(READING_HISTORY_TABLE)?;
        }
        write_txn.commit()?;

        info!("✅ 数据库表结构已初始化");

        Ok(Self { db })
    }

    /// 保存或更新阅读进度
    pub fn save_progress(&self, progress: &ReadingProgress) -> Result<()> {
        let key = progress.db_key();

        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(READING_PROGRESS_TABLE)?;
            let json = serde_json::to_string(progress)?;
            table.insert(key.as_str(), json.as_str())?;
        }
        write_txn.commit()?;

        debug!(
            "✅ 保存阅读进度: {} -> {} ({}%)",
            progress.username, progress.note_path, progress.scroll_percentage
        );

        Ok(())
    }

    /// 获取阅读进度
    pub fn get_progress(&self, username: &str, note_path: &str) -> Result<Option<ReadingProgress>> {
        let key = format!("{}:{}", username, note_path);

        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(READING_PROGRESS_TABLE)?;

        let result = table.get(key.as_str())?;

        if let Some(json_value) = result {
            let json = json_value.value();
            let progress: ReadingProgress = serde_json::from_str(json)?;
            Ok(Some(progress))
        } else {
            Ok(None)
        }
    }

    /// 获取用户的所有阅读进度（按最后阅读时间降序）
    ///
    /// 利用 redb 范围查询（前缀 `{username}:`），
    /// 避免全表扫描，只读取属于该用户的记录。
    pub fn get_user_progress(&self, username: &str, limit: usize) -> Result<Vec<ReadingProgress>> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(READING_PROGRESS_TABLE)?;

        // key 格式："{username}:{note_path}"
        // ':' 是 ASCII 58，';' 是 ASCII 59，前缀范围 "user:" .. "user;" 精确匹配该用户的所有记录
        let lower = format!("{}:", username);
        let upper = format!("{};", username);
        let mut progress_list = Vec::new();

        for item in table.range(lower.as_str()..upper.as_str())? {
            let (_, json_value) = item?;
            let progress: ReadingProgress = serde_json::from_str(json_value.value())?;
            progress_list.push(progress);
        }

        // 按最后阅读时间降序排序
        progress_list.sort_by(|a, b| b.last_read_at.cmp(&a.last_read_at));
        progress_list.truncate(limit);

        Ok(progress_list)
    }

    /// 删除阅读进度
    pub fn delete_progress(&self, username: &str, note_path: &str) -> Result<bool> {
        let key = format!("{}:{}", username, note_path);

        let write_txn = self.db.begin_write()?;
        let removed = {
            let mut table = write_txn.open_table(READING_PROGRESS_TABLE)?;
            table.remove(key.as_str())?.is_some()
        };
        write_txn.commit()?;

        if removed {
            debug!("✅ 删除阅读进度: {} -> {}", username, note_path);
        }

        Ok(removed)
    }

    /// 添加阅读历史记录，并自动清理超出上限的旧记录（保留最近 200 条）
    pub fn add_history(&self, history: &ReadingHistory) -> Result<()> {
        let key = history.db_key();

        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(READING_HISTORY_TABLE)?;
            let json = serde_json::to_string(history)?;
            table.insert(key.as_str(), json.as_str())?;
        }
        write_txn.commit()?;

        debug!(
            "✅ 添加阅读历史: {} -> {} ({}秒)",
            history.username, history.note_path, history.duration
        );

        // 自动清理旧历史记录，防止无限增长（保留最近 200 条）
        let _ = self.cleanup_old_history(&history.username, 200);

        Ok(())
    }

    /// 获取用户的阅读历史（按时间降序）
    ///
    /// 利用 redb 范围查询（前缀 `{username}:`），
    /// 避免全表扫描，只读取属于该用户的记录。
    pub fn get_user_history(&self, username: &str, limit: usize) -> Result<Vec<ReadingHistory>> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(READING_HISTORY_TABLE)?;

        // key 格式："{username}:{timestamp_nanos}:{note_path}"
        // 前缀范围 "user:" .. "user;" 精确匹配该用户的所有历史记录
        let lower = format!("{}:", username);
        let upper = format!("{};", username);
        let mut history_list = Vec::new();

        for item in table.range(lower.as_str()..upper.as_str())? {
            let (_, json_value) = item?;
            let history: ReadingHistory = serde_json::from_str(json_value.value())?;
            history_list.push(history);
        }

        // 按访问时间降序排序
        history_list.sort_by(|a, b| b.visited_at.cmp(&a.visited_at));
        history_list.truncate(limit);

        Ok(history_list)
    }

    /// 清理旧的历史记录（保留最近 N 条）
    pub fn cleanup_old_history(&self, username: &str, keep_count: usize) -> Result<usize> {
        let history_list = self.get_user_history(username, usize::MAX)?;

        if history_list.len() <= keep_count {
            return Ok(0);
        }

        let to_remove = &history_list[keep_count..];
        let mut removed_count = 0;

        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(READING_HISTORY_TABLE)?;

            for history in to_remove {
                let key = history.db_key();
                if table.remove(key.as_str())?.is_some() {
                    removed_count += 1;
                }
            }
        }
        write_txn.commit()?;

        if removed_count > 0 {
            info!(
                "🧹 清理了 {} 条旧的阅读历史 (用户: {})",
                removed_count, username
            );
        }

        Ok(removed_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reading_progress_creation() {
        let progress = ReadingProgress::new(
            "admin".to_string(),
            "test/note.md".to_string(),
            "测试笔记".to_string(),
            1000,
            50.0,
        );

        assert_eq!(progress.username, "admin");
        assert_eq!(progress.note_path, "test/note.md");
        assert_eq!(progress.scroll_position, 1000);
        assert_eq!(progress.scroll_percentage, 50.0);
        assert!(!progress.is_completed);
    }

    #[test]
    fn test_reading_progress_update() {
        let mut progress = ReadingProgress::new(
            "admin".to_string(),
            "test/note.md".to_string(),
            "测试笔记".to_string(),
            1000,
            50.0,
        );

        progress.update(2000, 96.0, 30);

        assert_eq!(progress.scroll_position, 2000);
        assert_eq!(progress.scroll_percentage, 96.0);
        assert_eq!(progress.reading_duration, 30);
        assert!(progress.is_completed);
    }

    #[test]
    fn test_db_key_generation() {
        let progress = ReadingProgress::new(
            "admin".to_string(),
            "test/note.md".to_string(),
            "测试笔记".to_string(),
            1000,
            50.0,
        );

        assert_eq!(progress.db_key(), "admin:test/note.md");
    }

    #[test]
    fn test_history_creation() {
        let history = ReadingHistory::new(
            "admin".to_string(),
            "test/note.md".to_string(),
            "测试笔记".to_string(),
            120,
        );

        assert_eq!(history.username, "admin");
        assert_eq!(history.note_path, "test/note.md");
        assert_eq!(history.duration, 120);

        // 验证 db_key 包含时间戳
        let key = history.db_key();
        assert!(key.starts_with("admin:"));
        assert!(key.ends_with(":test/note.md"));
    }
}
