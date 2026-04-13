use crate::domain::{Note, SidebarNode};
use anyhow::{Context, Result};
use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tracing::{info, warn};

// redb 表定义
const NOTES_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("notes");
const LINK_INDEX_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("link_index");
const BACKLINKS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("backlinks");
const TAG_INDEX_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("tag_index");
const SIDEBAR_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("sidebar");
const METADATA_TABLE: TableDefinition<&str, &str> = TableDefinition::new("metadata");

/// 持久化元数据
#[derive(Debug, Serialize, Deserialize)]
struct PersistenceMetadata {
    /// Git 提交 hash
    git_commit: String,
    /// 保存时间戳
    saved_at: i64,
    /// 版本号（用于兼容性检查）
    version: u32,
    /// 忽略模式（用于检测配置变更）
    #[serde(default)]
    ignore_patterns: Vec<String>,
}

const CURRENT_VERSION: u32 = 1;

/// 索引持久化管理器
pub struct IndexPersistence {
    db: Database,
}

impl IndexPersistence {
    /// 打开或创建持久化数据库
    pub fn open(db_path: &Path) -> Result<Self> {
        // 确保父目录存在
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).context("Failed to create persistence directory")?;
        }

        let db = Database::create(db_path).context("Failed to open persistence database")?;

        Ok(Self { db })
    }

    /// 保存所有索引到磁盘
    pub fn save_indexes(
        &self,
        git_commit: &str,
        ignore_patterns: &[String],
        notes: &HashMap<String, Note>,
        link_index: &HashMap<String, String>,
        backlinks: &HashMap<String, Vec<String>>,
        tag_index: &HashMap<String, Vec<String>>,
        sidebar: &[SidebarNode],
    ) -> Result<()> {
        info!("💾 开始持久化索引...");
        let start = std::time::Instant::now();

        let write_txn = self.db.begin_write()?;

        // 1. 保存元数据
        {
            let mut table = write_txn.open_table(METADATA_TABLE)?;
            let metadata = PersistenceMetadata {
                git_commit: git_commit.to_string(),
                saved_at: chrono::Utc::now().timestamp(),
                version: CURRENT_VERSION,
                ignore_patterns: ignore_patterns.to_vec(),
            };
            let metadata_json = serde_json::to_string(&metadata)?;
            table.insert("metadata", metadata_json.as_str())?;
        }

        // 2. 保存笔记索引
        {
            let mut table = write_txn.open_table(NOTES_TABLE)?;
            for (path, note) in notes {
                let serialized = postcard::to_allocvec(note).context("Failed to serialize note")?;
                table.insert(path.as_str(), serialized.as_slice())?;
            }
        }

        // 3. 保存链接索引
        {
            let mut table = write_txn.open_table(LINK_INDEX_TABLE)?;
            let serialized =
                postcard::to_allocvec(link_index).context("Failed to serialize link_index")?;
            table.insert("data", serialized.as_slice())?;
        }

        // 4. 保存反向链接
        {
            let mut table = write_txn.open_table(BACKLINKS_TABLE)?;
            let serialized =
                postcard::to_allocvec(backlinks).context("Failed to serialize backlinks")?;
            table.insert("data", serialized.as_slice())?;
        }

        // 5. 保存标签索引
        {
            let mut table = write_txn.open_table(TAG_INDEX_TABLE)?;
            let serialized =
                postcard::to_allocvec(tag_index).context("Failed to serialize tag_index")?;
            table.insert("data", serialized.as_slice())?;
        }

        // 6. 保存侧边栏
        {
            let mut table = write_txn.open_table(SIDEBAR_TABLE)?;
            let serialized =
                postcard::to_allocvec(sidebar).context("Failed to serialize sidebar")?;
            table.insert("data", serialized.as_slice())?;
        }

        write_txn.commit()?;

        info!(
            "✅ 索引持久化完成，耗时 {:.2}s",
            start.elapsed().as_secs_f64()
        );
        info!("  ├─ 笔记数: {}", notes.len());
        info!("  ├─ 链接索引: {}", link_index.len());
        info!("  ├─ 标签索引: {}", tag_index.len());
        info!("  └─ Git 提交: {}", &git_commit[..8]);

        Ok(())
    }

    /// 从磁盘加载索引
    pub fn load_indexes(
        &self,
        current_git_commit: &str,
        current_ignore_patterns: &[String],
    ) -> Result<Option<LoadedIndexes>> {
        info!("📂 尝试加载持久化索引...");

        let read_txn = self.db.begin_read()?;

        // 1. 检查元数据
        let metadata = {
            let table = read_txn.open_table(METADATA_TABLE)?;
            let metadata_str = match table.get("metadata")? {
                Some(guard) => guard.value().to_string(),
                None => {
                    info!("  └─ 未找到持久化数据");
                    return Ok(None);
                }
            };

            let metadata: PersistenceMetadata = serde_json::from_str(&metadata_str)?;

            // 检查版本兼容性
            if metadata.version != CURRENT_VERSION {
                warn!(
                    "  └─ 持久化版本不兼容 (v{} vs v{})",
                    metadata.version, CURRENT_VERSION
                );
                return Ok(None);
            }

            // 检查 Git 提交是否匹配
            if metadata.git_commit != current_git_commit {
                info!(
                    "  └─ Git 提交不匹配 ({} vs {})",
                    &metadata.git_commit[..8],
                    &current_git_commit[..8]
                );
                return Ok(None);
            }

            // 检查 ignore_patterns 是否变更
            if metadata.ignore_patterns != current_ignore_patterns {
                info!("  └─ 配置变更检测：ignore_patterns 已修改");
                info!("     旧配置: {:?}", metadata.ignore_patterns);
                info!("     新配置: {:?}", current_ignore_patterns);
                return Ok(None);
            }

            metadata
        };

        info!("  ├─ 找到匹配的持久化数据");
        info!("  ├─ Git 提交: {}", &metadata.git_commit[..8]);
        info!(
            "  ├─ 保存时间: {}",
            chrono::DateTime::<chrono::Utc>::from_timestamp(metadata.saved_at, 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_else(|| "未知".to_string())
        );

        // 2. 加载笔记索引
        let notes = {
            let table = read_txn.open_table(NOTES_TABLE)?;
            let mut notes = HashMap::new();

            for item in table.iter()? {
                let (key, value) = item?;
                let path = key.value().to_string();
                let note: Note =
                    postcard::from_bytes(value.value()).context("Failed to deserialize note")?;
                notes.insert(path, note);
            }

            notes
        };

        // 3. 加载链接索引
        let link_index = {
            let table = read_txn.open_table(LINK_INDEX_TABLE)?;
            match table.get("data")? {
                Some(guard) => postcard::from_bytes(guard.value())
                    .context("Failed to deserialize link_index")?,
                None => HashMap::new(),
            }
        };

        // 4. 加载反向链接
        let backlinks = {
            let table = read_txn.open_table(BACKLINKS_TABLE)?;
            match table.get("data")? {
                Some(guard) => postcard::from_bytes(guard.value())
                    .context("Failed to deserialize backlinks")?,
                None => HashMap::new(),
            }
        };

        // 5. 加载标签索引
        let tag_index = {
            let table = read_txn.open_table(TAG_INDEX_TABLE)?;
            match table.get("data")? {
                Some(guard) => postcard::from_bytes(guard.value())
                    .context("Failed to deserialize tag_index")?,
                None => HashMap::new(),
            }
        };

        // 6. 加载侧边栏
        let sidebar = {
            let table = read_txn.open_table(SIDEBAR_TABLE)?;
            match table.get("data")? {
                Some(guard) => {
                    postcard::from_bytes(guard.value()).context("Failed to deserialize sidebar")?
                }
                None => Vec::new(),
            }
        };

        info!("✅ 索引加载完成");
        info!("  ├─ 笔记数: {}", notes.len());
        info!("  ├─ 链接索引: {}", link_index.len());
        info!("  ├─ 标签索引: {}", tag_index.len());
        info!("  └─ 反向链接: {}", backlinks.len());

        Ok(Some(LoadedIndexes {
            notes,
            link_index,
            backlinks,
            tag_index,
            sidebar,
        }))
    }

    /// 清除所有持久化数据
    pub fn clear(&self) -> Result<()> {
        info!("🗑️  清除持久化索引...");

        let write_txn = self.db.begin_write()?;

        // 清空所有表
        {
            let mut table = write_txn.open_table(NOTES_TABLE)?;
            let keys: Vec<String> = table
                .iter()?
                .map(|item| item.map(|(k, _)| k.value().to_string()))
                .collect::<Result<Vec<_>, _>>()?;
            for key in keys {
                table.remove(key.as_str())?;
            }
        }

        {
            let mut table = write_txn.open_table(LINK_INDEX_TABLE)?;
            table.remove("data")?;
        }

        {
            let mut table = write_txn.open_table(BACKLINKS_TABLE)?;
            table.remove("data")?;
        }

        {
            let mut table = write_txn.open_table(SIDEBAR_TABLE)?;
            table.remove("data")?;
        }

        {
            let mut table = write_txn.open_table(METADATA_TABLE)?;
            table.remove("metadata")?;
        }

        write_txn.commit()?;

        info!("✅ 持久化索引已清除");

        Ok(())
    }
}

/// 加载的索引数据
pub struct LoadedIndexes {
    pub notes: HashMap<String, Note>,
    pub link_index: HashMap<String, String>,
    pub backlinks: HashMap<String, Vec<String>>,
    pub tag_index: HashMap<String, Vec<String>>,
    pub sidebar: Vec<SidebarNode>,
}
