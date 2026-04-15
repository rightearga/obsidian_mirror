// 应用状态定义
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicI64, AtomicU64, AtomicU8};
use std::sync::{Arc, RwLock};
use tokio::sync::{broadcast, Mutex, RwLock as TokioRwLock};

use crate::config::AppConfig;
use crate::domain::{Note, SidebarNode};
use crate::insights::InsightsCache;
use crate::reading_progress_db::ReadingProgressDatabase;
use crate::search_engine::SearchEngine;
use crate::share_db::ShareDatabase;
use crate::sync::{SyncProgressEvent, SyncRecord};

/// 应用程序的全局状态
pub struct AppState {
    /// 运行时配置，使用 std::sync::RwLock 保护，支持热重载
    /// 读取：`.config.read().unwrap()`（读取前请勿持有锁跨越 .await）
    /// 热重载写入：`*data.config.write().unwrap() = new_config`
    pub config: RwLock<AppConfig>,
    /// 所有笔记的映射：相对路径 -> Note
    pub notes: TokioRwLock<HashMap<String, Note>>,
    /// 侧边栏树形结构
    pub sidebar: TokioRwLock<Vec<SidebarNode>>,
    /// 反向链接映射：笔记标题 -> 链接到它的笔记标题列表
    pub backlinks: TokioRwLock<HashMap<String, Vec<String>>>,
    /// 链接索引：笔记标题（或文件名）-> 相对路径
    pub link_index: TokioRwLock<HashMap<String, String>>,
    /// 文件索引：文件名 -> 完整相对路径（用于图片等资源）
    pub file_index: TokioRwLock<HashMap<String, String>>,
    /// 标签索引：标签名 -> 包含该标签的笔记标题列表
    pub tag_index: TokioRwLock<HashMap<String, Vec<String>>>,
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
    /// 应用启动时间，供 /health 端点计算真实运行时长
    pub start_time: std::time::Instant,

    // ── v1.5.5 新增字段 ─────────────────────────────────────────────────────

    /// SSE 同步进度广播发送端（v1.5.5）
    ///
    /// 订阅：`data.sync_progress_tx.subscribe()`；capacity=128 足以缓冲一次同步的全部事件。
    /// 发送端在同步各阶段广播 `SyncProgressEvent`，`GET /api/sync/events` 订阅并返回 SSE 流。
    pub sync_progress_tx: broadcast::Sender<SyncProgressEvent>,

    /// 同步历史记录（最近 10 条，v1.5.5）
    ///
    /// 每次同步（无论成功或失败）在完成时追加一条 `SyncRecord`，
    /// 超出 10 条时自动从头部删除最旧记录。
    pub sync_history: TokioRwLock<VecDeque<SyncRecord>>,

    /// 后台任务句柄集合（v1.5.5）
    ///
    /// 同步期间启动的 spawn_blocking 任务（Tantivy 重建、redb 持久化）
    /// 的 `JoinHandle` 存储于此，优雅关闭时 await 全部完成（上限 30 秒）。
    /// 使用 `std::sync::Mutex` 以便在 async 上下文中快速获取而不持有锁跨越 .await。
    pub background_tasks: std::sync::Mutex<Vec<tokio::task::JoinHandle<()>>>,

    /// 笔记洞察缓存（v1.7.3）
    ///
    /// 在每次 `perform_sync` 完成后由 `compute_insights` 重新计算。
    /// 包含写作趋势、健康度报告（孤立/断链/超大笔记）和标签云。
    pub insights_cache: TokioRwLock<InsightsCache>,
}

/// 多仓库注册表（v1.7.4）
///
/// 持有所有已初始化的仓库状态，按配置顺序排列。
/// 第一个仓库为主仓库，持有向后兼容的无前缀路由（`/doc/{path}` 等）。
/// 非主仓库通过 `/r/{name}/` 前缀访问。
pub struct VaultRegistry {
    /// 仓库列表：`(name, Arc<AppState>)`，顺序与 `config.effective_repos()` 一致
    pub vaults: Vec<(String, Arc<AppState>)>,
}

impl VaultRegistry {
    /// 返回主仓库（列表中第一个）的 `AppState`
    pub fn primary(&self) -> Arc<AppState> {
        self.vaults[0].1.clone()
    }

    /// 按名称查找仓库的 `AppState`，不存在时返回 `None`
    pub fn get(&self, name: &str) -> Option<Arc<AppState>> {
        self.vaults.iter()
            .find(|(n, _)| n == name)
            .map(|(_, s)| s.clone())
    }

    /// 返回所有仓库名称列表
    pub fn names(&self) -> Vec<&str> {
        self.vaults.iter().map(|(n, _)| n.as_str()).collect()
    }
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
        // broadcast channel：capacity=128，足以缓冲一次完整同步过程的所有进度事件
        let (sync_progress_tx, _) = broadcast::channel(128);

        Self {
            config: RwLock::new(config),
            notes: TokioRwLock::new(HashMap::new()),
            sidebar: TokioRwLock::new(Vec::new()),
            backlinks: TokioRwLock::new(HashMap::new()),
            link_index: TokioRwLock::new(HashMap::new()),
            file_index: TokioRwLock::new(HashMap::new()),
            tag_index: TokioRwLock::new(HashMap::new()),
            search_engine,
            share_db,
            reading_progress_db,
            sync_lock: Mutex::new(()),
            last_sync_at: AtomicI64::new(0),
            last_sync_duration_ms: AtomicU64::new(0),
            sync_status: AtomicU8::new(sync_status::IDLE),
            start_time: std::time::Instant::now(),
            sync_progress_tx,
            sync_history: TokioRwLock::new(VecDeque::new()),
            background_tasks: std::sync::Mutex::new(Vec::new()),
            insights_cache: TokioRwLock::new(InsightsCache::default()),
        }
    }
}
