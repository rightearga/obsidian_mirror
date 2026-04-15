use actix_web::{web, App, HttpServer};
use actix_files as fs;
use std::sync::Arc;
use std::path::PathBuf;
use tracing::{info, warn};

use obsidian_mirror::{
    config::{AppConfig, DatabaseConfig},
    state::{AppState, VaultRegistry},
    sync::perform_sync,
    search_engine::SearchEngine,
    handlers::{sync_handler, search_handler, graph_handler, assets_handler, doc_handler, index_handler, tags_list_handler, tag_notes_handler, health_handler, stats_handler, preview_handler, orphans_handler, random_handler, recent_page_handler, titles_api_handler, suggest_handler, global_graph_handler, webhook_sync_handler, config_reload_handler, sync_events_handler, sync_history_handler, graph_page_handler, note_history_handler, note_history_at_handler, note_history_diff_handler, insights_page_handler, insights_stats_handler, vaults_list_handler, feed_handler, export_html_handler, timeline_page_handler, timeline_api_handler, graph_path_handler},
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

    // 初始化认证系统（全局，所有仓库共用）
    let auth_system = init_auth_system(&config)?;

    // 初始化分享链接数据库（全局）
    let share_db = init_share_database(&config)?;

    // 初始化阅读进度数据库（全局）
    let reading_progress_db = init_reading_progress_database(&config)?;

    // v1.7.4：为每个仓库创建独立的 AppState（包含独立搜索索引和同步状态）
    let repos = config.effective_repos();
    let is_multi = config.is_multi_vault();
    let mut vault_list: Vec<(String, Arc<AppState>)> = Vec::new();

    for (idx, repo) in repos.iter().enumerate() {
        info!("========================================");
        info!("📚 初始化仓库 [{}/{}]: {}", idx + 1, repos.len(), repo.name);

        // 为每个仓库派生独立的数据库路径（主仓库沿用全局配置，其他仓库加名称前缀）
        let vault_db = derive_vault_db_config(&config.database, &repo.name, is_multi);

        // 为该仓库创建专属搜索引擎
        let vault_search = init_vault_search_engine(&vault_db, &repo.name)?;

        // 构造该仓库的 AppConfig（全局设置 + 仓库特定的 repo_url/local_path/ignore_patterns）
        let vault_config = AppConfig {
            repo_url:           repo.repo_url.clone(),
            local_path:         repo.local_path.clone(),
            ignore_patterns:    repo.ignore_patterns.clone(),
            database:           vault_db,
            repos:              vec![],  // 仓库级别 config 中不再嵌套 repos
            ..config.clone()
        };

        let vault_state = Arc::new(AppState::new(
            vault_config,
            vault_search,
            share_db.clone(),
            reading_progress_db.clone(),
        ));
        info!("✅ 仓库 {} 状态创建完成", repo.name);

        vault_list.push((repo.name.clone(), vault_state));
    }

    // 执行各仓库初始同步
    for (name, vault_state) in &vault_list {
        info!("🔄 仓库 {} 开始初始同步...", name);
        perform_initial_sync(vault_state).await;
    }

    // 启动每个仓库的定时自动同步（若 sync_interval_minutes > 0）
    let sync_interval = config.sync_interval_minutes;
    if sync_interval > 0 {
        for (name, vault_state) in &vault_list {
            let vault_name   = name.clone();
            let state_timer  = vault_state.clone();
            tokio::spawn(async move {
                use tokio::time::{interval as tick_interval, Duration};
                let mut ticker = tick_interval(Duration::from_secs(sync_interval as u64 * 60));
                ticker.tick().await; // 跳过第一次立即触发
                loop {
                    ticker.tick().await;
                    match state_timer.sync_lock.try_lock() {
                        Ok(_guard) => {
                            info!("⏰ 仓库 {} 定时同步开始", vault_name);
                            if let Err(e) = perform_sync(&state_timer).await {
                                tracing::error!("仓库 {} 定时同步失败: {:?}", vault_name, e);
                            }
                        }
                        Err(_) => info!("⏰ 仓库 {} 定时同步跳过：另一个同步任务正在进行", vault_name),
                    }
                }
            });
        }
        info!("✅ 所有仓库定时同步任务已启动（间隔 {} 分钟）", sync_interval);
    }

    // 构建 VaultRegistry
    let vault_registry = Arc::new(VaultRegistry { vaults: vault_list });

    // 启动 HTTP 服务器
    let result = start_http_server(vault_registry, config, auth_system).await;

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
                repos: vec![],
            }
        }
    }
}

/// 派生每个仓库的专属数据库配置（v1.7.4）
///
/// 主仓库（name = "default" 或单仓库模式）沿用全局配置；
/// 其余仓库在 index_db_path 同级目录下生成 `{name}_index.db`，
/// auth/share/reading_progress 数据库保持全局共享。
fn derive_vault_db_config(base: &DatabaseConfig, vault_name: &str, is_multi: bool) -> DatabaseConfig {
    if !is_multi || vault_name == "default" {
        return base.clone();
    }
    // 文件名安全化：替换路径分隔符和空格
    let safe_name = vault_name.replace(['/', '\\', ' ', ':'], "_");
    let parent = base.index_db_path.parent().unwrap_or(std::path::Path::new("."));
    DatabaseConfig {
        index_db_path: parent.join(format!("{}_index.db", safe_name)),
        // 认证、分享、阅读进度数据库跨仓库共享
        auth_db_path:               base.auth_db_path.clone(),
        share_db_path:              base.share_db_path.clone(),
        reading_progress_db_path:   base.reading_progress_db_path.clone(),
    }
}

/// 为指定仓库初始化搜索引擎（v1.7.4）
///
/// 搜索索引目录从 `index_db_path` 的同级位置派生：
/// - 单仓库 / 主仓库：`.search_index/`
/// - 非主仓库：`.search_index_{name}/`
fn init_vault_search_engine(vault_db: &DatabaseConfig, vault_name: &str) -> anyhow::Result<Arc<SearchEngine>> {
    info!("🔍 初始化搜索引擎（仓库: {}）...", vault_name);
    let index_dir = vault_db.index_db_path
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join(if vault_name == "default" {
            ".search_index".to_string()
        } else {
            let safe = vault_name.replace(['/', '\\', ' ', ':'], "_");
            format!(".search_index_{}", safe)
        });
    info!("  └─ 索引目录: {}", index_dir.display());
    let engine = Arc::new(SearchEngine::new(&index_dir)?);
    info!("✅ 搜索引擎初始化完成（仓库: {}）", vault_name);
    Ok(engine)
}

/// 初始化搜索引擎（单仓库向后兼容入口，供外部代码调用）
#[allow(dead_code)]
fn init_search_engine(config: &AppConfig) -> anyhow::Result<Arc<SearchEngine>> {
    init_vault_search_engine(&config.database, "default")
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

/// 为单个仓库构建路由 scope（v1.7.4）
///
/// 使用 actix-web 的 `.app_data()` 覆盖机制：scope 内的所有处理器
/// 接收到的 `web::Data<Arc<AppState>>` 将是该仓库的 AppState，
/// 无需修改任何现有 handler。
fn build_vault_scope(scope_path: &str, vault_state: Arc<AppState>) -> actix_web::Scope {
    web::scope(scope_path)
        .app_data(web::Data::new(vault_state))
        .service(stats_handler)
        .service(preview_handler)
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
        .service(suggest_handler)
        .service(sync_events_handler)
        .service(sync_history_handler)
        .service(global_graph_handler)
        .service(graph_page_handler)
        .service(insights_page_handler)
        .service(insights_stats_handler)
        .service(feed_handler)           // GET /feed.xml — Atom 订阅（v1.8.2）
        .service(export_html_handler)    // POST /api/export/html（v1.8.2）
        .service(timeline_page_handler)  // GET /timeline — 时间线视图（v1.8.4）
        .service(timeline_api_handler)   // GET /api/timeline（v1.8.4）
        .route("/webhook/sync",                          web::post().to(webhook_sync_handler))
        .route("/api/config/reload",                     web::post().to(config_reload_handler))
        // Git 历史路由需在 doc_handler 之前注册
        .route("/doc/{path:.*}/history",                 web::get().to(note_history_handler))
        .route("/doc/{path:.*}/at/{commit}",             web::get().to(note_history_at_handler))
        .route("/doc/{path:.*}/diff/{commit}",           web::get().to(note_history_diff_handler))
        .service(doc_handler)
        .service(index_handler)
}

/// 启动 HTTP 服务器（v1.7.4：接受 VaultRegistry 支持多仓库）
async fn start_http_server(
    vault_registry: Arc<VaultRegistry>,
    config: AppConfig,
    auth_system: Option<(Arc<AuthDatabase>, Arc<JwtManager>)>,
) -> anyhow::Result<()> {
    let bind_addr = config.listen_addr.clone();
    let workers = config.workers;
    let auth_enabled = config.security.auth_enabled;

    // 主仓库（第一个）的 AppState，供优雅关闭时保存索引
    let primary_for_shutdown = vault_registry.primary();
    let vault_registry_for_shutdown = vault_registry.clone();
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
    if vault_registry.vaults.len() > 1 {
        info!("📚 多仓库模式：{} 个仓库（{}）",
            vault_registry.vaults.len(),
            vault_registry.names().join(", "));
    }
    info!("========================================");
    info!("✨ 服务器已就绪，按 Ctrl+C 停止");
    info!("========================================");

    // JWT manager（未启用认证时使用 dummy）
    let jwt_manager = auth_system.as_ref()
        .map(|(_, mgr)| mgr.clone())
        .unwrap_or_else(|| Arc::new(JwtManager::new("dummy".to_string(), 24)));

    let server = HttpServer::new(move || {
        let primary_state = vault_registry.primary();

        let mut app = App::new()
            .wrap(actix_web::middleware::Logger::default())
            .wrap(AuthMiddleware::new((*jwt_manager).clone(), auth_enabled))
            // 主仓库 AppState（向后兼容的无前缀路由使用此数据）
            .app_data(web::Data::new(primary_state))
            // VaultRegistry（供 /api/vaults 端点读取）
            .app_data(web::Data::new(vault_registry.clone()));

        // 认证相关路由（全局）
        if let Some((ref auth_db, ref jwt_manager)) = auth_system {
            app = app
                .app_data(web::Data::new(auth_db.clone()))
                .app_data(web::Data::new(jwt_manager.clone()))
                .route("/login", web::get().to(login_page_handler))
                .route("/change-password", web::get().to(change_password_page_handler))
                .route("/api/auth/login",            web::post().to(login_handler))
                .route("/api/auth/logout",           web::post().to(logout_handler))
                .route("/api/auth/change-password",  web::post().to(change_password_handler))
                .route("/api/auth/current-user",     web::get().to(current_user_handler))
                .route("/admin/users",                        web::get().to(admin_users_page_handler))
                .route("/api/admin/users",                    web::get().to(list_users_handler))
                .route("/api/admin/users",                    web::post().to(create_user_handler))
                .route("/api/admin/users/{username}",         web::delete().to(delete_user_handler))
                .route("/api/admin/users/{username}/reset-password", web::post().to(reset_user_password_handler));
        }

        // ── 全局路由（与仓库无关）──────────────────────────────────────────────
        app = app
            .service(fs::Files::new("/static", "static").show_files_listing())
            .route("/sw.js",        web::get().to(serve_service_worker))
            .route("/manifest.json", web::get().to(serve_manifest))
            .service(health_handler)
            .service(metrics_handler)
            .service(vaults_list_handler)  // GET /api/vaults（v1.7.4）
            .service(feed_handler)         // GET /feed.xml — Atom 订阅（v1.8.2）
            .service(export_html_handler)  // POST /api/export/html — 静态站点导出（v1.8.2）
            // 分享链接路由（跨仓库共享）
            .route("/api/share/create",          web::post().to(create_share_handler))
            .route("/api/share/list",             web::get().to(list_shares_handler))
            .route("/api/share/{token}",          web::delete().to(revoke_share_handler))
            .route("/share/{token}",              web::get().to(access_share_handler))
            .route("/share/{token}",              web::post().to(access_share_handler))
            // 阅读进度路由（跨仓库共享）
            .route("/api/reading/progress",                          web::post().to(save_progress_handler))
            .route("/api/reading/progress",                          web::get().to(list_progress_handler))
            .route("/api/reading/progress/{note_path:.*}",           web::get().to(get_progress_handler))
            .route("/api/reading/progress/{note_path:.*}",           web::delete().to(delete_progress_handler))
            .route("/api/reading/history",        web::post().to(add_history_handler))
            .route("/api/reading/history",        web::get().to(list_history_handler))
            // 搜索历史路由
            .route("/api/search/history",         web::post().to(add_search_history_handler))
            .route("/api/search/history",         web::get().to(get_search_history_handler))
            .route("/api/search/history",         web::delete().to(clear_search_history_handler));

        // ── 主仓库向后兼容路由（无前缀，使用外层 app_data 的主仓库 AppState）──
        app = app
            .service(stats_handler)
            .service(preview_handler)
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
            .service(suggest_handler)
            .service(sync_events_handler)
            .service(sync_history_handler)
            .service(global_graph_handler)
            .service(graph_path_handler)         // GET /api/graph/path（v1.9.2）
            .service(graph_page_handler)
            .service(insights_page_handler)
            .service(insights_stats_handler)
            .service(timeline_page_handler)
            .service(timeline_api_handler)
            .route("/webhook/sync",              web::post().to(webhook_sync_handler))
            .route("/api/config/reload",         web::post().to(config_reload_handler))
            .route("/doc/{path:.*}/history",     web::get().to(note_history_handler))
            .route("/doc/{path:.*}/at/{commit}", web::get().to(note_history_at_handler))
            .route("/doc/{path:.*}/diff/{commit}", web::get().to(note_history_diff_handler))
            .service(doc_handler)
            .service(index_handler);

        // ── 多仓库：为每个仓库注册 /r/{name}/... 前缀路由 ─────────────────────
        for (name, vault_state) in &vault_registry.vaults {
            let scope_path = format!("/r/{}", name);
            app = app.service(build_vault_scope(&scope_path, vault_state.clone()));
        }

        app
    })
    .workers(workers)
    .bind(&bind_addr)?;

    let server = server.run();
    let server_handle = server.handle();
    let server_task = tokio::spawn(server);

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
            { std::future::pending::<()>().await }
        } => {
            info!("========================================");
            info!("📢 收到 SIGTERM 信号，开始优雅关闭...");
            info!("========================================");
        }
    }

    server_handle.stop(true).await;
    server_task.await??;
    info!("✅ HTTP 服务器已关闭");

    // v1.5.5：等待所有仓库的后台任务完成（上限 30 秒）
    info!("⏳ 等待后台任务完成...");
    let mut all_bg: Vec<tokio::task::JoinHandle<()>> = Vec::new();
    for (_, vault_state) in &vault_registry_for_shutdown.vaults {
        let mut lock = vault_state.background_tasks.lock().unwrap();
        all_bg.extend(lock.drain(..));
    }
    if !all_bg.is_empty() {
        let timeout = tokio::time::Duration::from_secs(30);
        match tokio::time::timeout(timeout, futures_util::future::join_all(all_bg)).await {
            Ok(_) => info!("✅ 所有后台任务已完成"),
            Err(_) => warn!("⚠️ 后台任务等待超时（30 秒），强制退出"),
        }
    }

    // 保存所有仓库的持久化索引
    info!("💾 保存持久化索引...");
    for (name, vault_state) in &vault_registry_for_shutdown.vaults {
        let vault_cfg = vault_state.config.read().unwrap().clone();
        if let Ok(persistence) = obsidian_mirror::persistence::IndexPersistence::open(
            &vault_cfg.database.index_db_path
        ) {
            if let Ok(git_commit) = obsidian_mirror::git::GitClient::get_current_commit(
                &vault_cfg.local_path
            ).await {
                let notes      = vault_state.notes.read().await.clone();
                let link_index = vault_state.link_index.read().await.clone();
                let backlinks  = vault_state.backlinks.read().await.clone();
                let tag_index  = vault_state.tag_index.read().await.clone();
                let sidebar    = vault_state.sidebar.read().await.clone();

                if let Err(e) = persistence.save_indexes(
                    &git_commit, &vault_cfg.ignore_patterns,
                    &notes, &link_index, &backlinks, &tag_index, &sidebar
                ) {
                    warn!("⚠️ 仓库 {} 持久化索引保存失败: {:?}", name, e);
                } else {
                    info!("✅ 仓库 {} 持久化索引已保存", name);
                }
            }
        }
    }

    // 消除未使用警告（primary_for_shutdown 仅作兜底引用）
    let _ = primary_for_shutdown;
    let _ = config_for_shutdown;

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
