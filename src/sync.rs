// 数据同步逻辑
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::SystemTime;
use tracing::{info, warn, error};
use serde::Serialize;

/// SSE 同步进度事件（v1.5.5）
///
/// 通过 `AppState.sync_progress_tx` broadcast channel 发送，
/// `GET /api/sync/events` 以 SSE 格式推送给前端。
#[derive(Debug, Clone, Serialize)]
pub struct SyncProgressEvent {
    /// 阶段标识：git / scan / markdown / index / search / persist / done / error
    pub stage: String,
    /// 进度百分比（0-100）
    pub progress: u8,
    /// 人类可读的进度说明
    pub message: String,
    /// 已处理笔记数（markdown 阶段）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes_processed: Option<usize>,
    /// 笔记总数（done 阶段）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_notes: Option<usize>,
}

impl SyncProgressEvent {
    /// 快速构造进度事件
    pub fn new(stage: &str, progress: u8, message: &str) -> Self {
        Self {
            stage: stage.to_string(),
            progress,
            message: message.to_string(),
            notes_processed: None,
            total_notes: None,
        }
    }
}

/// 同步历史记录（v1.5.5）
#[derive(Debug, Clone, Serialize)]
pub struct SyncRecord {
    /// 开始时间（Unix 时间戳秒）
    pub started_at: i64,
    /// 结束时间（Unix 时间戳秒）
    pub finished_at: i64,
    /// 完成后笔记总数
    pub notes_count: usize,
    /// completed / failed
    pub status: String,
    /// 失败原因（仅 failed 时非空）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_msg: Option<String>,
    /// 耗时（毫秒）
    pub duration_ms: u64,
}

use crate::state::AppState;
use crate::git::{GitClient, SyncResult};
use crate::scanner::VaultScanner;
use crate::markdown::MarkdownProcessor;
use crate::domain::Note;
use crate::sidebar::build_sidebar;
use crate::indexer::{
    FileIndexBuilder, BacklinkBuilder, TagIndexBuilder,
    IndexUpdater, extract_search_data,
};
use crate::persistence::IndexPersistence;

/// 执行完整的数据同步流程
/// 
/// 流程包括：
/// 1. Git 同步（pull/clone），检测变更文件
/// 2. 根据 Git diff 结果决定全量或增量同步
/// 3. 扫描 Markdown 文件（增量模式仅扫描变更文件）
/// 4. 构建文件索引（图片等资源）
/// 5. 并行处理 Markdown 文件（增量更新）
/// 6. 更新笔记索引和链接索引
/// 7. 增量更新反向链接
/// 8. 重建侧边栏树
/// 9. 增量更新搜索索引
/// 计算并更新笔记洞察缓存（供所有 perform_sync 退出路径共用）
///
/// 在 NoChange/持久化命中等提前返回路径中同样需要调用，
/// 确保 `/api/insights/stats` 不会因未经历完整 sync 而返回零值。
async fn update_insights_cache(data: &Arc<AppState>) {
    let notes     = data.notes.read().await;
    let link_idx  = data.link_index.read().await;
    let tag_idx   = data.tag_index.read().await;
    let backlinks = data.backlinks.read().await;

    if notes.is_empty() {
        return; // 无笔记数据，跳过（避免全零写入覆盖旧缓存）
    }

    let mut new_cache = crate::insights::compute_insights(&notes, &link_idx, &tag_idx, &backlinks);

    // 阅读频率热力图（需 redb IO，使用 spawn_blocking）
    let rp_db = data.reading_progress_db.clone();
    if let Ok(Ok(visit_counts)) = tokio::task::spawn_blocking(move || rp_db.get_all_visit_counts(10)).await {
        new_cache.reading_hotmap = visit_counts.into_iter()
            .map(|(path, title, count)| crate::insights::ReadingHotEntry { path, title, visit_count: count })
            .collect();
    }

    *data.insights_cache.write().await = new_cache;
    tracing::info!("✅ 笔记洞察缓存已更新（notes={}）", notes.len());
}

pub async fn perform_sync(data: &Arc<AppState>) -> anyhow::Result<()> {
    use crate::state::sync_status;
    use crate::metrics::{SYNC_TOTAL, SYNC_DURATION_SECONDS, SYNC_LAST_TIMESTAMP_SECONDS};

    info!("========================================");
    info!("🔄 开始数据同步");
    info!("========================================");
    let sync_start = std::time::Instant::now();

    // 在同步开始前克隆配置快照，避免持有读锁跨越 .await 点
    // （config_reload_handler 可能在同步期间更新配置，本次同步使用启动时的快照）
    let config = data.config.read().unwrap().clone();

    // v1.5.5：记录同步开始时间戳（用于 SyncRecord）
    let sync_started_at = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    // v1.5.5：发送 SSE 进度事件的辅助闭包（无订阅者时 send 返回 Err，可安全忽略）
    let tx = data.sync_progress_tx.clone();
    let emit = |stage: &str, progress: u8, message: &str| {
        let _ = tx.send(SyncProgressEvent::new(stage, progress, message));
    };

    emit("starting", 5, "同步开始...");

    // 标记同步状态为 running，并注册 RAII 守卫：
    // 正常退出时在函数末尾手动设为 IDLE；异常（? 传播）退出时 guard Drop 将状态设为 FAILED。
    data.sync_status.store(sync_status::RUNNING, Ordering::Relaxed);

    // RAII 守卫：在 Drop 时若状态仍为 RUNNING（意味着未正常完成），设置为 FAILED
    struct SyncStatusGuard<'a> {
        status: &'a std::sync::atomic::AtomicU8,
        completed: bool,
    }
    impl Drop for SyncStatusGuard<'_> {
        fn drop(&mut self) {
            if !self.completed {
                // 未正常完成（中途 return Err 或 panic），标记为 FAILED
                self.status.store(sync_status::FAILED, Ordering::Relaxed);
            }
        }
    }
    let mut _status_guard = SyncStatusGuard {
        status: &data.sync_status,
        completed: false,
    };

    // ✅ 步骤 0: 在 Git 同步前，先尝试加载持久化数据（如果内存为空）
    let notes_count_before = data.notes.read().await.len();
    if notes_count_before == 0 {
        info!("📂 内存为空，尝试加载持久化索引...");
        
        // 先获取当前 Git 提交（用于验证持久化数据）
        if let Ok(current_commit) = GitClient::get_current_commit(&config.local_path).await {
            match IndexPersistence::open(&config.database.index_db_path) {
                Ok(persistence) => {
                    match persistence.load_indexes(&current_commit, &config.ignore_patterns) {
                        Ok(Some(loaded)) => {
                            info!("✅ 从持久化数据库恢复索引");
                            
                            // 更新应用状态
                            *data.notes.write().await = loaded.notes;
                            *data.link_index.write().await = loaded.link_index;
                            *data.backlinks.write().await = loaded.backlinks;
                            *data.tag_index.write().await = loaded.tag_index;
                            *data.sidebar.write().await = loaded.sidebar;
                            
                            let loaded_count = data.notes.read().await.len();
                            info!("  ├─ 笔记数: {}", loaded_count);
                            info!("  ├─ 链接索引: {}", data.link_index.read().await.len());
                            info!("  ├─ 标签索引: {}", data.tag_index.read().await.len());
                        }
                        Ok(None) => {
                            info!("⚠️  持久化索引不可用（Git 提交不匹配或版本不兼容）");
                        }
                        Err(e) => {
                            warn!("⚠️  加载持久化索引失败: {:?}", e);
                        }
                    }
                }
                Err(e) => {
                    warn!("⚠️  无法打开持久化数据库: {:?}", e);
                }
            }
        }
    } else {
        info!("✅ 内存中已有 {} 个笔记，跳过持久化加载", notes_count_before);
    }

    // 1. Git Pull/Clone 并检测变更
    info!("📥 步骤 1/9: Git 同步");
    emit("git", 10, "执行 Git 同步...");
    let sync_result = GitClient::sync(&config.repo_url, &config.local_path).await?;
    
    // 获取当前 Git 提交
    let current_git_commit = GitClient::get_current_commit(&config.local_path).await?;
    
    match &sync_result {
        SyncResult::InitialClone => {
            info!("✅ 首次克隆，执行全量同步");
        }
        SyncResult::IncrementalUpdate { changed, deleted } => {
            info!("✅ 增量更新：{} 个修改，{} 个删除", changed.len(), deleted.len());
        }
        SyncResult::NoChange => {
            // 检查内存中是否已有笔记数据
            let notes_count = data.notes.read().await.len();
            if notes_count > 0 {
                info!("✅ 无变更且已有 {} 个笔记，跳过笔记同步", notes_count);
                
                // 但仍然需要确保文件索引存在（用于图片等资源文件）
                let file_index_count = data.file_index.read().await.len();
                if file_index_count == 0 {
                    info!("⚠️  文件索引为空，重建文件索引");
                    let mut file_index_write = data.file_index.write().await;
                    *file_index_write = FileIndexBuilder::build(&config.local_path);
                    let count = file_index_write.len();
                    drop(file_index_write);
                    info!("✅ 文件索引重建完成，共 {} 个文件", count);
                }
                
                info!("========================================");
                // 即使无变更，也更新洞察缓存（修复 insights 全零 bug）
                update_insights_cache(data).await;
                return Ok(());
            } else {
                info!("⚠️  无 Git 变更，但内存为空，尝试加载持久化索引");
                
                // 尝试从持久化数据库加载索引
                match IndexPersistence::open(&config.database.index_db_path) {
                    Ok(persistence) => {
                        match persistence.load_indexes(&current_git_commit, &config.ignore_patterns) {
                            Ok(Some(loaded)) => {
                                // 成功加载持久化索引
                                info!("✅ 从持久化数据库恢复索引");
                                
                                // 更新应用状态
                                *data.notes.write().await = loaded.notes;
                                *data.link_index.write().await = loaded.link_index;
                                *data.backlinks.write().await = loaded.backlinks;
                                *data.tag_index.write().await = loaded.tag_index;
                                *data.sidebar.write().await = loaded.sidebar;
                                
                                // 重建文件索引（资源文件不持久化，每次启动都需要扫描）
                                info!("📦 重建资源文件索引");
                                let mut file_index_write = data.file_index.write().await;
                                *file_index_write = FileIndexBuilder::build(&config.local_path);
                                let file_count = file_index_write.len();
                                drop(file_index_write);
                                info!("✅ 资源文件索引重建完成，共 {} 个文件", file_count);
                                
                                // 搜索索引：Tantivy 磁盘索引已有内容则直接复用，避免重启每次重建
                                let existing_docs = data.search_engine.num_docs();
                                if existing_docs > 0 {
                                    info!("🔎 Tantivy 磁盘索引已存在（{} 条文档），跳过重建，直接复用", existing_docs);
                                } else {
                                    // v1.4.9：content_text 已从 Note 移除，无法从持久化数据重建搜索索引。
                                    // 触发 /sync 端点可强制重新处理所有文件并重建索引。
                                    warn!("⚠️  Tantivy 磁盘索引为空且 Note 不含原始内容，搜索功能暂时不可用。\
                                           请手动触发 POST /sync 以重建搜索索引。");
                                }
                                
                                let total_time = sync_start.elapsed();
                                let note_count = data.notes.read().await.len();
                                info!("========================================");
                                info!("✨ 从持久化数据恢复完成！");
                                info!("  ├─ 笔记总数: {}", note_count);
                                info!("  ├─ 资源文件总数: {}", file_count);
                                info!("  └─ 总耗时: {:.2}s", total_time.as_secs_f64());
                                info!("========================================");

                                // 持久化命中后也计算洞察缓存（修复 insights 全零 bug）
                                update_insights_cache(data).await;
                                return Ok(());
                            }
                            Ok(None) => {
                                info!("⚠️  持久化索引不可用（Git 提交不匹配或版本不兼容）");
                            }
                            Err(e) => {
                                warn!("⚠️  加载持久化索引失败: {:?}", e);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("⚠️  无法打开持久化数据库: {:?}", e);
                    }
                }
                
                info!("⚠️  执行全量同步");
            }
        }
    }

    // P3: 记录本次是否为增量同步（sync_result 将在下方被 move 消费）
    let is_incremental = matches!(&sync_result, SyncResult::IncrementalUpdate { .. });

    // 2. 根据同步结果决定处理策略
    let (files_to_process, files_to_delete) = match sync_result {
        SyncResult::InitialClone | SyncResult::NoChange => {
            // 首次克隆或无变更但内存为空，扫描所有文件
            info!("🔍 步骤 2/8: 扫描所有 Markdown 文件");
            let scanner = VaultScanner::new(config.clone());
            let files = scanner.scan()?;
            info!("✅ 扫描完成，发现 {} 个 Markdown 文件", files.len());
            (files, Vec::new())
        }
        SyncResult::IncrementalUpdate { changed, deleted } => {
            // 增量更新，仅处理变更的文件
            info!("🔍 步骤 2/8: 处理变更文件");
            
            // 过滤出 .md 文件，并排除忽略模式
            let local_path = &config.local_path;
            let ignore_patterns = &config.ignore_patterns;
            
            let changed_md: Vec<_> = changed.iter()
                .filter(|p| {
                    // 检查扩展名
                    if p.extension().and_then(|e| e.to_str()) != Some("md") {
                        return false;
                    }
                    
                    // 检查是否匹配忽略模式
                    for component in p.components() {
                        let s = component.as_os_str().to_string_lossy();
                        
                        // 忽略隐藏文件/目录
                        if s.starts_with('.') {
                            return false;
                        }
                        
                        // 检查自定义忽略模式
                        for pattern in ignore_patterns {
                            if s.eq_ignore_ascii_case(pattern) {
                                return false;
                            }
                        }
                    }
                    
                    true
                })
                .map(|p| local_path.join(p))
                .collect();
            
            let deleted_md: Vec<_> = deleted.iter()
                .filter(|p| {
                    // 检查扩展名
                    if p.extension().and_then(|e| e.to_str()) != Some("md") {
                        return false;
                    }
                    
                    // 检查是否匹配忽略模式
                    for component in p.components() {
                        let s = component.as_os_str().to_string_lossy();
                        
                        // 忽略隐藏文件/目录
                        if s.starts_with('.') {
                            return false;
                        }
                        
                        // 检查自定义忽略模式
                        for pattern in ignore_patterns {
                            if s.eq_ignore_ascii_case(pattern) {
                                return false;
                            }
                        }
                    }
                    
                    true
                })
                .map(|p| {
                    // ✅ 统一使用 '/' 作为路径分隔符（与 notes HashMap 的 key 格式一致）
                    p.to_string_lossy().replace("\\", "/")
                })
                .collect();
            
            info!("✅ 变更文件：{} 个 Markdown 待处理，{} 个待删除", 
                  changed_md.len(), deleted_md.len());
            
            (changed_md, deleted_md)
        }
    };

    // 3. Build file index (for all non-.md files like images, PDFs)
    info!("📦 步骤 3/8: 构建资源文件索引");
    emit("scan", 25, "构建资源文件索引...");
    let mut file_index_write = data.file_index.write().await;
    *file_index_write = FileIndexBuilder::build(&config.local_path);
    drop(file_index_write);

    // ✅ 在删除文件之前，先保存现有笔记用于增量更新
    let existing_notes = data.notes.read().await.clone();
    
    // 4. 处理删除的文件
    if !files_to_delete.is_empty() {
        info!("🗑️  删除 {} 个笔记", files_to_delete.len());
        let mut notes_write = data.notes.write().await;
        for path in &files_to_delete {
            notes_write.remove(path);
        }
        drop(notes_write);
    }

    // 5. 并行处理 Markdown 文件
    info!("⚙️  步骤 4/8: 并行处理笔记");
    emit("markdown", 40, "并行处理 Markdown 文件...");
    let process_start = std::time::Instant::now();
    
    let local_path = config.local_path.clone();
    
    // 使用 tokio::task::spawn_blocking 在线程池中并行处理
    let processed_notes = tokio::task::spawn_blocking(move || {
        process_markdown_files(files_to_process, local_path, existing_notes)
    }).await?;
    
    info!("✅ 笔记处理完成，耗时 {:.2}s，共处理 {} 个笔记", 
          process_start.elapsed().as_secs_f64(), 
          processed_notes.len());

    // 6. 更新索引
    info!("📇 步骤 5/8: 更新索引和反向链接");
    emit("index", 65, "更新反向链接和标签索引...");
    let mut notes_write = data.notes.write().await;
    let mut link_index_write = data.link_index.write().await;

    // P3: 在消费 processed_notes 之前，提取搜索索引所需数据
    // 只有 content = Some 的笔记（本次新处理）才需要更新 Tantivy 索引；
    // 缓存命中（content = None）的笔记内容已在 Tantivy 磁盘上，不需要重传。
    let search_data = extract_search_data(&processed_notes);

    // 更新笔记和链接索引
    IndexUpdater::update_notes_and_links(
        &mut notes_write,
        &mut link_index_write,
        processed_notes,
    );

    // 基于全量 notes 重建反向链接，确保增量同步时不会遗漏未变更笔记的出链
    let mut backlinks_write = data.backlinks.write().await;
    *backlinks_write = BacklinkBuilder::build(&notes_write);
    
    // 构建标签索引
    let mut tag_index_write = data.tag_index.write().await;
    *tag_index_write = TagIndexBuilder::build(&notes_write);
    
    drop(notes_write);
    drop(link_index_write);
    drop(backlinks_write);
    drop(tag_index_write);
    info!("✅ 索引更新完成");

    // 7. Rebuild Sidebar Tree
    info!("🗂️  步骤 6/8: 重建侧边栏树");
    let mut sidebar_write = data.sidebar.write().await;
    let notes_read = data.notes.read().await;
    *sidebar_write = build_sidebar(&notes_read);
    drop(notes_read);
    drop(sidebar_write);
    info!("✅ 侧边栏树重建完成");

    // 8. 更新搜索索引（使用 Tantivy）
    info!("🔎 步骤 7/8: 更新搜索索引");
    emit("search", 80, "更新搜索索引（后台进行）...");

    let search_engine = data.search_engine.clone();
    // v1.5.5：保存 JoinHandle 以支持优雅关闭时等待任务完成
    let search_handle = if is_incremental {
        // 增量同步：仅更新变更文件（有 content 的），删除已删除文件
        let deleted_for_search = files_to_delete.clone();
        let h = tokio::task::spawn_blocking(move || {
            if let Err(e) = search_engine.update_documents(
                search_data.into_iter(),
                &deleted_for_search,
            ) {
                error!("  └─ 增量更新搜索索引失败: {:?}", e);
            }
        });
        info!("✅ 搜索索引增量更新已启动（后台进行）");
        Some(h)
    } else {
        // 全量同步：用本次处理的全量数据重建 Tantivy 索引
        let h = tokio::task::spawn_blocking(move || {
            if let Err(e) = search_engine.rebuild_index(search_data.into_iter()) {
                error!("  └─ 重建搜索索引失败: {:?}", e);
            } else {
                info!("  └─ 搜索索引重建完成");
            }
        });
        info!("✅ 搜索索引全量重建已启动（后台进行）");
        Some(h)
    };

    // 9. 持久化索引到磁盘
    info!("💾 步骤 8/9: 持久化索引");
    emit("persist", 90, "持久化索引到磁盘（后台进行）...");
    let persist_handle = if let Ok(persistence) = IndexPersistence::open(&config.database.index_db_path) {
        let notes = data.notes.read().await.clone();
        let link_index = data.link_index.read().await.clone();
        let backlinks = data.backlinks.read().await.clone();
        let tag_index = data.tag_index.read().await.clone();
        let sidebar = data.sidebar.read().await.clone();

        let ignore_patterns = config.ignore_patterns.clone();
        let h = tokio::task::spawn_blocking(move || {
            if let Err(e) = persistence.save_indexes(
                &current_git_commit,
                &ignore_patterns,
                &notes,
                &link_index,
                &backlinks,
                &tag_index,
                &sidebar,
            ) {
                error!("  └─ 持久化索引失败: {:?}", e);
            }
        });
        Some(h)
    } else {
        warn!("  └─ 无法打开持久化数据库，跳过保存");
        None
    };

    // v1.5.5：将后台任务句柄存入 AppState，优雅关闭时等待完成
    if let Ok(mut tasks) = data.background_tasks.lock() {
        tasks.retain(|h| !h.is_finished()); // 清理已完成的旧句柄
        if let Some(h) = search_handle {
            tasks.push(h);
        }
        if let Some(h) = persist_handle {
            tasks.push(h);
        }
    }

    let total_time = sync_start.elapsed();
    let note_count = data.notes.read().await.len();
    info!("========================================");
    info!("✨ 同步完成！");
    info!("  ├─ 笔记总数: {}", note_count);
    info!("  ├─ 总耗时: {:.2}s", total_time.as_secs_f64());
    info!("  └─ 平均速度: {:.0} 笔记/秒",
          note_count as f64 / total_time.as_secs_f64().max(0.001));
    info!("========================================");

    // 记录同步完成时间和耗时（供 /health 端点和 Prometheus 使用）
    let now_ts = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let duration_ms = total_time.as_millis() as u64;
    data.last_sync_at.store(now_ts, Ordering::Relaxed);
    data.last_sync_duration_ms.store(duration_ms, Ordering::Relaxed);
    data.sync_status.store(sync_status::IDLE, Ordering::Relaxed);
    _status_guard.completed = true; // 正常完成，阻止 Drop 设置 FAILED

    // v1.5.5：广播 done 事件并追加同步历史记录
    {
        let mut done_event = SyncProgressEvent::new("done", 100, "同步完成");
        done_event.total_notes = Some(note_count);
        let _ = data.sync_progress_tx.send(done_event);
    }
    {
        let record = SyncRecord {
            started_at: sync_started_at,
            finished_at: now_ts,
            notes_count: note_count,
            status: "completed".to_string(),
            error_msg: None,
            duration_ms,
        };
        let mut history = data.sync_history.write().await;
        if history.len() >= 10 { history.pop_front(); }
        history.push_back(record);
    }

    // 更新 Prometheus 指标
    SYNC_TOTAL.inc();
    SYNC_DURATION_SECONDS.observe(total_time.as_secs_f64());
    SYNC_LAST_TIMESTAMP_SECONDS.set(now_ts);

    // v1.6.2：后台生成离线搜索索引（JSON 格式），供 WASM NoteIndex 加载
    // v1.6.6 B1：auth_enabled=true 时 index.json 含敏感内容且路径公开（/static/ 白名单），跳过生成
    // v1.6.6 B2：将句柄加入 background_tasks，优雅关闭时等待写入完成
    if !config.security.auth_enabled {
        let notes_for_index = data.notes.read().await.clone();
        let index_handle = tokio::task::spawn(async move {
            if let Ok(json) = generate_search_index_json(&notes_for_index) {
                // 写入 static/wasm/index.json（Service Worker 会缓存此文件）
                if let Err(e) = tokio::fs::write("static/wasm/index.json", json).await {
                    warn!("⚠️  生成离线搜索索引失败: {:?}", e);
                } else {
                    info!("✅ 离线搜索索引已更新（static/wasm/index.json）");
                }
            }
        });
        if let Ok(mut tasks) = data.background_tasks.lock() {
            tasks.retain(|h| !h.is_finished());
            tasks.push(index_handle);
        }
    }

    // v1.7.3：同步完成后重新计算笔记洞察缓存（使用公共 helper，与早期退出路径共用）
    update_insights_cache(data).await;
    {
        // v1.8.6：mtime 缓存仍需内联计算（需持有 notes 读锁）
        let notes = data.notes.read().await;

        // v1.8.6：填充 mtime 秒缓存，避免图谱构建热路径重复调用 SystemTime::duration_since()
        let new_mtime: std::collections::HashMap<String, i64> = notes.iter()
            .map(|(path, note)| {
                let secs = note.mtime
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0);
                (path.clone(), secs)
            })
            .collect();
        *data.mtime_cache.write().await = new_mtime;
    }

    Ok(())
}

/// 生成离线搜索索引的 JSON 字节序列（供 WASM NoteIndex 加载，v1.6.2）
///
/// 每条笔记提取 title、path、tags 和内容摘要（前 300 字符），
/// 序列化为 JSON 数组供浏览器端 `NoteIndex.loadJson()` 使用。
fn generate_search_index_json(notes: &HashMap<String, Note>) -> anyhow::Result<Vec<u8>> {
    use std::time::UNIX_EPOCH;

    #[derive(serde::Serialize)]
    struct IndexEntry<'a> {
        title:   &'a str,
        path:    &'a str,
        tags:    &'a [String],
        content: String,
        mtime:   i64,
    }

    let entries: Vec<IndexEntry<'_>> = notes.values()
        .map(|note| {
            // 从 content_html 提取纯文本（去除 HTML 标签）
            let plain_text = strip_html_tags(&note.content_html);
            let content: String = plain_text.chars().take(300).collect();

            let mtime = note.mtime
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);

            IndexEntry {
                title:   &note.title,
                path:    &note.path,
                tags:    &note.tags,
                content,
                mtime,
            }
        })
        .collect();

    let json = serde_json::to_vec(&entries)?;
    Ok(json)
}

/// 从 HTML 中剥离所有标签，提取纯文本（用于索引内容摘要）
fn strip_html_tags(html: &str) -> String {
    let mut text = String::with_capacity(html.len());
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => { in_tag = false; text.push(' '); }
            _ if !in_tag => text.push(c),
            _ => {}
        }
    }
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// 并行处理 Markdown 文件（使用 Rayon）
fn process_markdown_files(
    files: Vec<std::path::PathBuf>,
    local_path: std::path::PathBuf,
    existing_notes: HashMap<String, Note>,
) -> Vec<(String, Note, Vec<String>, Option<String>)> {
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use rayon::prelude::*;
    
    let results = Mutex::new(Vec::new());
    let processed = AtomicUsize::new(0);
    let failed = AtomicUsize::new(0);
    let skipped = AtomicUsize::new(0);
    
    files.par_iter().for_each(|path| {
        // 计算相对路径 - 确保 UTF-8 正确处理
        let relative_path = match pathdiff::diff_paths(path, &local_path) {
            Some(diff) => diff,
            None => path.strip_prefix(&local_path).unwrap_or(path).to_path_buf(),
        };
        
        // 将路径转换为 UTF-8 字符串，如果包含非 UTF-8 字符则跳过此文件
        let relative_path_str = match relative_path.to_str() {
            Some(s) => s.replace("\\", "/"),
            None => {
                error!("⚠️  路径包含非 UTF-8 字符，跳过: {:?}", relative_path);
                return;
            }
        };
        
        // 检查是否需要更新（增量更新）
        let needs_update = should_update_note(&relative_path_str, path, &existing_notes);
        
        if !needs_update {
            // 文件未变更，复用现有笔记（content = None：内容已在 Tantivy 磁盘上）
            if let Some(note) = existing_notes.get(&relative_path_str) {
                results.lock().unwrap().push((relative_path_str, note.clone(), Vec::new(), None));
                skipped.fetch_add(1, Ordering::Relaxed);
            }
            return;
        }
        
        // 读取并处理文件（content 单独返回，不存入 Note）
        match process_single_note(path, &relative_path_str) {
            Some((note, links, content)) => {
                results.lock().unwrap().push((relative_path_str, note, links, Some(content)));
                processed.fetch_add(1, Ordering::Relaxed);
            }
            None => {
                failed.fetch_add(1, Ordering::Relaxed);
            }
        }
    });
    
    let processed_count = processed.load(Ordering::Relaxed);
    let failed_count = failed.load(Ordering::Relaxed);
    let skipped_count = skipped.load(Ordering::Relaxed);
    
    info!("📊 处理统计:");
    info!("  ├─ 成功处理: {} 个", processed_count);
    info!("  ├─ 复用缓存: {} 个", skipped_count);
    if failed_count > 0 {
        warn!("  └─ ⚠️  失败: {} 个", failed_count);
    } else {
        info!("  └─ 失败: {} 个", failed_count);
    }
    
    // E1 修复：Mutex 中毒时（Rayon worker panic 持有锁）从 PoisonError 中恢复数据，
    // 避免主线程 panic 并泄漏 sync_status = RUNNING
    results.into_inner().unwrap_or_else(|e| e.into_inner())
}

/// 检查笔记是否需要更新
fn should_update_note(
    relative_path: &str,
    path: &std::path::Path,
    existing_notes: &HashMap<String, Note>,
) -> bool {
    if let Some(existing_note) = existing_notes.get(relative_path) {
        // 检查文件修改时间
        if let Ok(metadata) = std::fs::metadata(path)
            && let Ok(mtime) = metadata.modified() {
                return mtime > existing_note.mtime;
            }
    }
    true // 新文件或无法判断时，默认需要更新
}

/// 处理单个笔记文件
///
/// 返回 `(Note, 出链列表, 原始内容文本)`。
/// 内容文本单独返回（不存入 Note），由调用方决定是否传递给 Tantivy 索引引擎。
fn process_single_note(
    path: &std::path::Path,
    relative_path: &str,
) -> Option<(Note, Vec<String>, String)> {
    // 读取文件内容
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            error!("❌ 读取文件失败: {:?} - 错误: {:?}", path, e);
            return None;
        }
    };

    let metadata = match std::fs::metadata(path) {
        Ok(m) => m,
        Err(e) => {
            error!("❌ 获取文件元数据失败: {:?} - 错误: {:?}", path, e);
            return None;
        }
    };

    let mtime = metadata.modified().unwrap_or(SystemTime::now());

    let (html, links, tags, frontmatter, toc) = MarkdownProcessor::process(&content);

    // 标题：文件名去掉扩展名
    let title = match path.file_stem() {
        Some(s) => s.to_string_lossy().to_string(),
        None => {
            error!("❌ 无法获取文件名: {:?}", path);
            return None;
        }
    };

    let note = Note {
        path: relative_path.to_string(),
        title,
        content_html: html,
        backlinks: Vec::new(),
        tags,
        toc,
        mtime,
        frontmatter: crate::domain::Frontmatter(frontmatter),
        // 保存出链列表，用于增量同步时正确重建全量反向链接索引
        outgoing_links: links.clone(),
    };

    // content 单独返回，供调用方传递给 Tantivy 搜索索引（不存入 Note 避免内存翻倍）
    Some((note, links, content))
}

// get_current_commit 已迁移至 src/git.rs::GitClient::get_current_commit（Q3 合并重复逻辑）

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Frontmatter, TocItem};
    use std::collections::HashMap;
    use std::time::{Duration, SystemTime};
    use tempfile::NamedTempFile;

    /// 构造测试用 Note（指定 mtime）
    fn make_note_with_mtime(path: &str, mtime: SystemTime) -> Note {
        Note {
            path: path.to_string(),
            title: path.to_string(),
            content_html: String::new(),
            backlinks: Vec::new(),
            tags: Vec::new(),
            toc: Vec::<TocItem>::new(),
            mtime,
            frontmatter: Frontmatter(serde_yaml::Value::Null),
            outgoing_links: Vec::new(),
        }
    }

    #[test]
    fn test_should_update_note_new_file() {
        // 文件路径不在 existing_notes 中，视为新文件，应更新
        let existing: HashMap<String, Note> = HashMap::new();
        let f = NamedTempFile::new().unwrap();
        assert!(
            should_update_note("new/note.md", f.path(), &existing),
            "新文件应需要更新"
        );
    }

    #[test]
    fn test_should_update_note_same_mtime() {
        // 文件 mtime 与 existing_notes 中记录完全一致，不需要更新
        let f = NamedTempFile::new().unwrap();
        let actual_mtime = std::fs::metadata(f.path()).unwrap().modified().unwrap();

        let mut existing = HashMap::new();
        existing.insert("test.md".to_string(), make_note_with_mtime("test.md", actual_mtime));

        assert!(
            !should_update_note("test.md", f.path(), &existing),
            "mtime 相同时不应更新"
        );
    }

    #[test]
    fn test_should_update_note_older_mtime() {
        // existing_notes 中存储的 mtime 比实际文件更旧，应更新
        let f = NamedTempFile::new().unwrap();
        let actual_mtime = std::fs::metadata(f.path()).unwrap().modified().unwrap();

        // 记录中的 mtime 比文件早 1 秒
        let old_mtime = actual_mtime
            .checked_sub(Duration::from_secs(1))
            .unwrap_or(SystemTime::UNIX_EPOCH);

        let mut existing = HashMap::new();
        existing.insert("test.md".to_string(), make_note_with_mtime("test.md", old_mtime));

        assert!(
            should_update_note("test.md", f.path(), &existing),
            "文件 mtime 比记录新时应更新"
        );
    }

    #[test]
    fn test_should_update_note_nonexistent_path() {
        // 磁盘上不存在的路径，should_update_note 无法获取 metadata，默认返回 true
        let mut existing = HashMap::new();
        existing.insert(
            "ghost.md".to_string(),
            make_note_with_mtime("ghost.md", SystemTime::now()),
        );

        // 传入一个不存在的路径
        let nonexistent = std::path::Path::new("/nonexistent/path/ghost.md");
        assert!(
            should_update_note("ghost.md", nonexistent, &existing),
            "无法获取 metadata 时应默认需要更新"
        );
    }
}
