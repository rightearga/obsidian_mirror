// 应用状态定义
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, AtomicU64, AtomicU8};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

use crate::config::AppConfig;
use crate::domain::{Note, SidebarNode};
use crate::reading_progress_db::ReadingProgressDatabase;
use crate::search_engine::SearchEngine;
use crate::share_db::ShareDatabase;

/// 应用程序的全局状态
pub struct AppState {
    pub config: AppConfig,
    /// 所有笔记的映射：相对路径 -> Note
    pub notes: RwLock<HashMap<String, Note>>,
    /// 侧边栏树形结构
    pub sidebar: RwLock<Vec<SidebarNode>>,
    /// 反向链接映射：笔记标题 -> 链接到它的笔记标题列表
    pub backlinks: RwLock<HashMap<String, Vec<String>>>,
    /// 链接索引：笔记标题（或文件名）-> 相对路径
    pub link_index: RwLock<HashMap<String, String>>,
    /// 文件索引：文件名 -> 完整相对路径（用于图片等资源）
    pub file_index: RwLock<HashMap<String, String>>,
    /// 标签索引：标签名 -> 包含该标签的笔记标题列表
    pub tag_index: RwLock<HashMap<String, Vec<String>>>,
    /// Tantivy 搜索引擎（线程安全）
    pub search_engine: Arc<SearchEngine>,
    /// 分享链接数据库
    pub share_db: Arc<ShareDatabase>,
    /// 阅读进度数据库
    pub reading_progress_db: Arc<ReadingProgressDatabase>,
    /// 同步互斥锁：防止并发 /sync 请求导致 Tantivy IndexWriter 冲突和数据竞争
    pub sync_lock: Mutex<()>,
    /// 上次同步完成时间（Unix 时间戳秒），0 表示从未同步
    pub last_sync_at: AtomicI64,
    /// 上次同步耗时（毫秒），0 表示从未同步
    pub last_sync_duration_ms: AtomicU64,
    /// 同步状态：0 = idle，1 = running，2 = failed
    pub sync_status: AtomicU8,
}

/// 同步状态常量
pub mod sync_status {
    pub const IDLE: u8    = 0;
    pub const RUNNING: u8 = 1;
    pub const FAILED: u8  = 2;
}

impl AppState {
    /// 创建新的应用状态实例
    pub fn new(
        config: AppConfig,
        search_engine: Arc<SearchEngine>,
        share_db: Arc<ShareDatabase>,
        reading_progress_db: Arc<ReadingProgressDatabase>,
    ) -> Self {
        Self {
            config,
            notes: RwLock::new(HashMap::new()),
            sidebar: RwLock::new(Vec::new()),
            backlinks: RwLock::new(HashMap::new()),
            link_index: RwLock::new(HashMap::new()),
            file_index: RwLock::new(HashMap::new()),
            tag_index: RwLock::new(HashMap::new()),
            search_engine,
            share_db,
            reading_progress_db,
            sync_lock: Mutex::new(()),
            last_sync_at: AtomicI64::new(0),
            last_sync_duration_ms: AtomicU64::new(0),
            sync_status: AtomicU8::new(sync_status::IDLE),
        }
    }
}
