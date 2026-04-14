use actix_web::{web, App, HttpServer};
use actix_files as fs;
use std::sync::Arc;
use std::path::PathBuf;
use tracing::{info, warn};

use obsidian_mirror::{
    config::AppConfig,
    state::AppState,
    sync::perform_sync,
    search_engine::SearchEngine,
    handlers::{sync_handler, search_handler, graph_handler, assets_handler, doc_handler, index_handler, tags_list_handler, tag_notes_handler, health_handler, stats_handler, preview_handler, orphans_handler, random_handler, recent_page_handler, titles_api_handler, suggest_handler, global_graph_handler, webhook_sync_handler, config_reload_handler, sync_events_handler, sync_history_handler},
    metrics::{init_metrics, metrics_handler},
    auth::{JwtManager, PasswordManager},
    auth_db::AuthDatabase,
    auth_middleware::AuthMiddleware,
    auth_handlers::{login_handler, logout_handler, change_password_handler, current_user_handler,
        admin_users_page_handler, list_users_handler, create_user_handler,
        delete_user_handler, reset_user_password_handler},
    share_db::ShareDatabase,
    share_handlers::{create_share_handler, access_share_handler, list_shares_handler, revoke_share_handler},
    reading_progress_db::ReadingProgressDatabase,
    reading_progress_handlers::{save_progress_handler, get_progress_handler, list_progress_handler, delete_progress_handler, add_history_handler, list_history_handler, add_search_history_handler, get_search_history_handler, clear_search_history_handler},
    VERSION, APP_NAME,
};

/// GET /sw.js — 从根路径提供 Service Worker 文件
/// 浏览器安全限制：SW 只能控制与其注册路径同级或下级的资源，
/// 因此必须从 /sw.js 而非 /static/sw.js 提供。
async fn serve_service_worker(_req: actix_web::HttpRequest) -> actix_web::Result<actix_files::NamedFile> {
    actix_files::NamedFile::open("static/sw.js")
        .map_err(|_| actix_web::error::ErrorNotFound("sw.js not found"))
        .map(|f| f.use_last_modified(true).use_etag(true))
}

/// GET /manifest.json — 从根路径提供 Web App Manifest
async fn serve_manifest(_req: actix_web::HttpRequest) -> actix_web::Result<actix_files::NamedFile> {
    actix_files::NamedFile::open("static/manifest.json")
        .map_err(|_| actix_web::error::ErrorNotFound("manifest.json not found"))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    init_logging();
    
    // 初始化 Prometheus 指标
    init_metrics();
    
    // 打印启动信息
    print_startup_banner();
    
    // 加载配置
    let config = load_config("config.ron");
    print_config_info(&config);
    
    // 初始化搜索引擎
    let search_engine = init_search_engine(&config)?;
    
    // 初始化认证系统
    let auth_system = init_auth_system(&config)?;
    
    // 初始化分享链接数据库
    let share_db = init_share_database(&config)?;
    
    // 初始化阅读进度数据库
    let reading_progress_db = init_reading_progress_database(&config)?;
    
    // 创建应用状态
    info!("✨ 创建应用状态...");
    let app_state = Arc::new(AppState::new(config.clone(), search_engine, share_db, reading_progress_db));
    info!("✅ 应用状态创建完成");
    
    // 执行初始同步
    perform_initial_sync(&app_state).await;

    // 启动定时自动同步任务（若 sync_interval_minutes > 0）
    if config.sync_interval_minutes > 0 {
        let interval_minutes = config.sync_interval_minutes;
        let state_for_timer = app_state.clone();
        tokio::spawn(async move {
            use tokio::time::{interval, Duration};
            let mut ticker = interval(Duration::from_secs(interval_minutes as u64 * 60));
            ticker.tick().await; // 跳过第一次立即触发（启动时已执行初始同步）
            loop {
                ticker.tick().await;
                // 尝试获取同步锁，若已在同步中则跳过
                match state_for_timer.sync_lock.try_lock() {
                    Ok(_guard) => {
                        info!("⏰ 定时同步开始（间隔 {} 分钟）", interval_minutes);
                        if let Err(e) = perform_sync(&state_for_timer).await {
                            tracing::error!("定时同步失败: {:?}", e);
                        }
                    }
                    Err(_) => {
                        info!("⏰ 定时同步跳过：另一个同步任务正在进行");
                    }
                }
            }
        });
        info!("✅ 定时同步任务已启动（间隔 {} 分钟）", config.sync_interval_minutes);
    }

    // 启动 HTTP 服务器
    let result = start_http_server(app_state, config, auth_system).await;
    
    // 优雅退出
    info!("========================================");
    info!("👋 {} 服务器已停止", APP_NAME);
    info!("========================================");
    
    result
}

/// 打印启动横幅
fn print_startup_banner() {
    info!("========================================");
    info!("🌐 {} v{}", APP_NAME, VERSION);
    info!("========================================");
}

/// 打印配置信息
fn print_config_info(config: &AppConfig) {
    info!("📋 配置信息:");
    info!("  ├─ 监听地址: {}", config.listen_addr);
    info!("  ├─ Worker 线程数: {}", config.workers);
    info!("  ├─ 本地路径: {}", config.local_path.display());
    
    if !config.repo_url.is_empty() {
        info!("  ├─ Git 仓库: {}", config.repo_url);
    } else {
        warn!("  ├─ Git 仓库: 未配置（仅使用本地文件）");
    }
    
    if !config.ignore_patterns.is_empty() {
        info!("  ├─ 忽略模式: {:?}", config.ignore_patterns);
    } else {
        info!("  ├─ 忽略模式: 无");
    }
    
    // 打印认证配置
    if config.security.auth_enabled {
        info!("  └─ 🔒 认证: 已启用");
    } else {
        info!("  └─ 🔓 认证: 未启用");
    }
}

/// 初始化日志系统
/// 
/// 日志输出到：
/// - 控制台：所有级别（格式化，带颜色）
/// - 文件：./logs/app.log（INFO 及以上）
/// - 文件：./logs/error.log（ERROR 及以上）
fn init_logging() {
    use tracing_subscriber::{fmt, EnvFilter, layer::SubscriberExt, util::SubscriberInitExt, Layer};
    use tracing_appender::rolling::{RollingFileAppender, Rotation};
    
    // 创建日志目录
    std::fs::create_dir_all("./logs").ok();
    
    // 创建文件 appender
    let info_file = RollingFileAppender::new(
        Rotation::DAILY,
        "./logs",
        "app.log"
    );
    
    let error_file = RollingFileAppender::new(
        Rotation::DAILY,
        "./logs",
        "error.log"
    );
    
    // 创建 EnvFilter
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));
    
    // 控制台输出层（带颜色格式化）
    let console_layer = fmt::layer()
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false);
    
    // INFO 文件输出层（所有 INFO+ 日志）
    let info_file_layer = fmt::layer()
        .with_writer(info_file)
        .with_ansi(false)
        .with_target(true)
        .with_filter(EnvFilter::new("info"));
    
    // ERROR 文件输出层（仅 ERROR+ 日志）
    let error_file_layer = fmt::layer()
        .with_writer(error_file)
        .with_ansi(false)
        .with_target(true)
        .with_filter(EnvFilter::new("error"));
    
    // 组合所有层
    tracing_subscriber::registry()
        .with(env_filter)
        .with(console_layer)
        .with(info_file_layer)
        .with(error_file_layer)
        .init();
}


/// 加载配置文件
fn load_config(config_path: &str) -> AppConfig {
    info!("📂 加载配置文件: {}", config_path);
    
    match AppConfig::load(config_path) {
        Ok(config) => {
            info!("✅ 配置加载成功");
            config
        }
        Err(e) => {
            warn!("⚠️  配置加载失败: {}", e);
            warn!("⚠️  使用默认配置");
            
            // 开发环境回退配置
            AppConfig {
                repo_url: "".to_string(),
                local_path: PathBuf::from("vault_data"),
                listen_addr: "127.0.0.1:8080".to_string(),
                workers: 4,
                ignore_patterns: vec![],
                database: Default::default(),
                security: Default::default(),
                sync_interval_minutes: 0,
                webhook: Default::default(),
                public_base_url: None,
            }
        }
    }
}

/// 初始化搜索引擎
fn init_search_engine(config: &AppConfig) -> anyhow::Result<Arc<SearchEngine>> {
    info!("🔍 初始化搜索引擎...");
    // 搜索索引放在 index.db 的同级目录下，而不是 local_path 内部，
    // 避免 git clone/重建 local_path 时 Tantivy 的 IndexReader 持续报警
    let index_dir = config.database.index_db_path
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join(".search_index");
    info!("  └─ 索引目录: {}", index_dir.display());
    
    let search_engine = Arc::new(SearchEngine::new(&index_dir)?);
    info!("✅ 搜索引擎初始化完成");
    
    Ok(search_engine)
}

/// 初始化认证系统
fn init_auth_system(config: &AppConfig) -> anyhow::Result<Option<(Arc<AuthDatabase>, Arc<JwtManager>)>> {
    if !config.security.auth_enabled {
        return Ok(None);
    }
    
    info!("🔐 初始化认证系统...");
    
    // 验证 JWT 密钥
    if config.security.jwt_secret == "CHANGE_THIS_TO_A_RANDOM_SECRET_KEY" {
        warn!("⚠️  警告: 正在使用默认的 JWT 密钥！");
        warn!("⚠️  请在配置文件中设置 security.jwt_secret 为随机字符串");
    }
    
    // 打开用户数据库
    let auth_db = Arc::new(AuthDatabase::open(&config.database.auth_db_path)?);
    info!("  ├─ 数据库路径: {}", config.database.auth_db_path.display());
    
    // 检查是否需要创建默认管理员账户
    if auth_db.is_empty()? {
        info!("  ├─ 数据库为空，创建默认管理员账户...");
        let password_hash = PasswordManager::hash_password(&config.security.default_admin_password)?;
        auth_db.create_user(&config.security.default_admin_username, &password_hash)?;
        info!("  ├─ ✅ 默认账户创建成功");
        info!("  ├─ 用户名: {}", config.security.default_admin_username);
        warn!("  ├─ ⚠️  密码: {} (请立即修改！)", config.security.default_admin_password);
    } else {
        info!("  ├─ 用户数据库已存在");
    }
    
    // 创建 JWT 管理器
    let jwt_manager = Arc::new(JwtManager::new(
        config.security.jwt_secret.clone(),
        config.security.token_lifetime_hours,
    ));
    info!("  └─ Token 有效期: {} 小时", config.security.token_lifetime_hours);
    
    info!("✅ 认证系统初始化完成");
    
    Ok(Some((auth_db, jwt_manager)))
}

/// 初始化分享链接数据库
fn init_share_database(config: &AppConfig) -> anyhow::Result<Arc<ShareDatabase>> {
    info!("🔗 初始化分享链接数据库...");
    
    let share_db = Arc::new(ShareDatabase::open(&config.database.share_db_path)?);
    
    info!("  └─ 数据库路径: {}", config.database.share_db_path.display());
    info!("✅ 分享链接数据库初始化完成");
    
    Ok(share_db)
}

/// 初始化阅读进度数据库
fn init_reading_progress_database(config: &AppConfig) -> anyhow::Result<Arc<ReadingProgressDatabase>> {
    info!("📖 初始化阅读进度数据库...");
    
    let reading_db = Arc::new(ReadingProgressDatabase::open(&config.database.reading_progress_db_path)?);
    
    info!("  └─ 数据库路径: {}", config.database.reading_progress_db_path.display());
    info!("✅ 阅读进度数据库初始化完成");
    
    Ok(reading_db)
}

/// 执行初始数据同步
async fn perform_initial_sync(app_state: &Arc<AppState>) {
    info!("🔄 检查本地数据...");
    // 读取配置快照（std::sync::RwLock，非异步）
    let local_path = app_state.config.read().unwrap().local_path.clone();

    if local_path.exists() {
        info!("✅ 本地路径存在，开始初始同步...");

        match perform_sync(app_state).await {
            Ok(_) => {
                info!("✅ 初始同步完成");
            }
            Err(e) => {
                warn!("⚠️  初始同步失败: {}", e);
                warn!("⚠️  服务器将继续运行，但数据可能不完整");
            }
        }
    } else {
        warn!("⚠️  本地路径不存在: {}", local_path.display());
        warn!("⚠️  请先配置正确的本地路径或执行 Git 同步");
    }
}

/// 启动 HTTP 服务器
async fn start_http_server(
    app_state: Arc<AppState>,
    config: AppConfig,
    auth_system: Option<(Arc<AuthDatabase>, Arc<JwtManager>)>,
) -> anyhow::Result<()> {
    let bind_addr = config.listen_addr.clone();
    let workers = config.workers;
    let auth_enabled = config.security.auth_enabled;
    
    // 克隆 app_state 用于闭包
    let app_state_for_closure = app_state.clone();
    let app_state_for_shutdown = app_state.clone();
    let config_for_shutdown = config.clone();
    
    info!("========================================");
    info!("🚀 启动 HTTP 服务器");
    info!("========================================");
    info!("🌐 监听地址: http://{}", bind_addr);
    info!("⚙️  Worker 线程数: {}", workers);
    info!("📝 访问日志: 已启用");
    if auth_enabled {
        info!("🔒 认证: 已启用");
    }
    info!("========================================");
    info!("✨ 服务器已就绪，按 Ctrl+C 停止");
    info!("========================================");

    // 创建一个 dummy JWT manager 用于未启用认证的情况
    let jwt_manager = auth_system.as_ref()
        .map(|(_, mgr)| mgr.clone())
        .unwrap_or_else(|| Arc::new(JwtManager::new("dummy".to_string(), 24)));

    let server = HttpServer::new(move || {
        let mut app = App::new()
            .wrap(actix_web::middleware::Logger::default())
            // 始终应用认证中间件（但内部会检查 auth_enabled）
            .wrap(AuthMiddleware::new((*jwt_manager).clone(), auth_enabled))
            .app_data(web::Data::new(app_state_for_closure.clone()));
        
        // 如果有认证系统，添加相关的 Data 和路由
        if let Some((ref auth_db, ref jwt_manager)) = auth_system {
            app = app
                .app_data(web::Data::new(auth_db.clone()))
                .app_data(web::Data::new(jwt_manager.clone()))
                // 登录页面和 API
                .route("/login", web::get().to(login_page_handler))
                .route("/change-password", web::get().to(change_password_page_handler))
                .route("/api/auth/login", web::post().to(login_handler))
                .route("/api/auth/logout", web::post().to(logout_handler))
                .route("/api/auth/change-password", web::post().to(change_password_handler))
                .route("/api/auth/current-user", web::get().to(current_user_handler))
                // 管理员用户管理路由（v1.5.3）
                .route("/admin/users", web::get().to(admin_users_page_handler))
                .route("/api/admin/users", web::get().to(list_users_handler))
                .route("/api/admin/users", web::post().to(create_user_handler))
                .route("/api/admin/users/{username}", web::delete().to(delete_user_handler))
                .route("/api/admin/users/{username}/reset-password", web::post().to(reset_user_password_handler));
        }
        
        // 添加通用路由
        app
            .service(fs::Files::new("/static", "static").show_files_listing())
            // Service Worker 必须从根路径提供（浏览器安全限制：scope 由注册路径决定）
            .route("/sw.js", web::get().to(serve_service_worker))
            // Web App Manifest 也从根路径提供（标准做法）
            .route("/manifest.json", web::get().to(serve_manifest))
            .service(health_handler)  // 健康检查端点
            .service(metrics_handler)  // Prometheus 指标端点
            .service(stats_handler)   // 统计信息端点
            .service(preview_handler) // 笔记预览端点
            .service(sync_handler)
            .service(search_handler)
            .service(graph_handler)
            .service(assets_handler)
            .service(tags_list_handler)
            .service(tag_notes_handler)
            .service(orphans_handler)
            .service(random_handler)
            .service(recent_page_handler)
            .service(titles_api_handler)
            .service(suggest_handler)          // GET /api/suggest — 搜索建议（v1.5.2）
            .service(sync_events_handler)      // GET /api/sync/events — SSE 同步进度（v1.5.5）
            .service(sync_history_handler)     // GET /api/sync/history — 同步历史（v1.5.5）
            .service(global_graph_handler)
            // Webhook 触发同步（需在 config.webhook.enabled=true 时才有效）
            .route("/webhook/sync", web::post().to(webhook_sync_handler))
            // 配置热重载（需认证）
            .route("/api/config/reload", web::post().to(config_reload_handler))
            // 分享链接相关路由
            .route("/api/share/create", web::post().to(create_share_handler))
            .route("/api/share/list", web::get().to(list_shares_handler))
            .route("/api/share/{token}", web::delete().to(revoke_share_handler))
            .route("/share/{token}", web::get().to(access_share_handler))
            .route("/share/{token}", web::post().to(access_share_handler))
            // 阅读进度相关路由
            .route("/api/reading/progress", web::post().to(save_progress_handler))
            .route("/api/reading/progress", web::get().to(list_progress_handler))
            .route("/api/reading/progress/{note_path:.*}", web::get().to(get_progress_handler))
            .route("/api/reading/progress/{note_path:.*}", web::delete().to(delete_progress_handler))
            .route("/api/reading/history", web::post().to(add_history_handler))
            .route("/api/reading/history", web::get().to(list_history_handler))
            // 搜索历史相关路由（v1.5.2）
            .route("/api/search/history", web::post().to(add_search_history_handler))
            .route("/api/search/history", web::get().to(get_search_history_handler))
            .route("/api/search/history", web::delete().to(clear_search_history_handler))
            .service(doc_handler)
            .service(index_handler)
    })
    .workers(workers)
    .bind(&bind_addr)?;
    
    let server = server.run();
    
    // 设置优雅关闭处理
    let server_handle = server.handle();
    
    // 在后台运行服务器
    let server_task = tokio::spawn(server);
    
    // 监听关闭信号
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("========================================");
            info!("📢 收到 Ctrl+C 信号，开始优雅关闭...");
            info!("========================================");
        }
        _ = async {
            #[cfg(unix)]
            {
                use tokio::signal::unix::{signal, SignalKind};
                let mut sigterm = signal(SignalKind::terminate()).unwrap();
                sigterm.recv().await
            }
            #[cfg(not(unix))]
            {
                std::future::pending::<()>().await
            }
        } => {
            info!("========================================");
            info!("📢 收到 SIGTERM 信号，开始优雅关闭...");
            info!("========================================");
        }
    }
    
    // 停止接受新连接
    server_handle.stop(true).await;

    // 等待服务器完全关闭
    server_task.await??;

    info!("✅ HTTP 服务器已关闭");

    // v1.5.5：等待后台任务（Tantivy 重建、redb 持久化）完成，上限 30 秒
    info!("⏳ 等待后台任务完成...");
    let bg_tasks: Vec<_> = {
        let mut lock = app_state_for_shutdown.background_tasks.lock().unwrap();
        lock.drain(..).collect()
    };
    if !bg_tasks.is_empty() {
        let timeout = tokio::time::Duration::from_secs(30);
        match tokio::time::timeout(timeout, futures_util::future::join_all(bg_tasks)).await {
            Ok(_) => info!("✅ 所有后台任务已完成"),
            Err(_) => warn!("⚠️ 后台任务等待超时（30 秒），强制退出"),
        }
    }
    
    // 保存持久化索引（如果需要）
    info!("💾 保存持久化索引...");
    if let Ok(persistence) = obsidian_mirror::persistence::IndexPersistence::open(&config_for_shutdown.database.index_db_path) {
        // 获取当前 Git 提交（使用 GitClient 公共接口，不重复实现）
        if let Ok(git_commit) = obsidian_mirror::git::GitClient::get_current_commit(&config_for_shutdown.local_path).await {
            let notes = app_state_for_shutdown.notes.read().await.clone();
            let link_index = app_state_for_shutdown.link_index.read().await.clone();
            let backlinks = app_state_for_shutdown.backlinks.read().await.clone();
            let tag_index = app_state_for_shutdown.tag_index.read().await.clone();
            let sidebar = app_state_for_shutdown.sidebar.read().await.clone();
            
            let ignore_patterns = config_for_shutdown.ignore_patterns.clone();
            if let Err(e) = persistence.save_indexes(&git_commit, &ignore_patterns, &notes, &link_index, &backlinks, &tag_index, &sidebar) {
                warn!("⚠️  保存持久化索引失败: {:?}", e);
            } else {
                info!("✅ 持久化索引已保存");
            }
        }
    }
    
    info!("========================================");
    info!("👋 应用已优雅关闭");
    info!("========================================");

    Ok(())
}

/// 登录页面处理器
async fn login_page_handler() -> actix_web::Result<actix_web::HttpResponse> {
    let html = include_str!("../templates/login.html");
    Ok(actix_web::HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html))
}

/// 修改密码页面处理器
async fn change_password_page_handler() -> actix_web::Result<actix_web::HttpResponse> {
    let html = include_str!("../templates/change_password.html");
    Ok(actix_web::HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html))
}

// get_git_commit 已移除，统一使用 obsidian_mirror::git::GitClient::get_current_commit（Q3）
