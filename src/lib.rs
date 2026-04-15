// Obsidian Mirror 核心库
//
// 此文件将各个模块导出为库，便于单元测试和代码复用

pub mod config;
pub mod git;
pub mod scanner;
pub mod markdown;
pub mod domain;
pub mod search_engine;
pub mod state;
pub mod templates;
pub mod sidebar;
pub mod sync;
pub mod handlers;
pub mod indexer;
pub mod tags;
pub mod graph;
pub mod persistence;
pub mod metrics;
pub mod error;

// 认证模块
pub mod auth;
pub mod auth_db;
pub mod auth_middleware;
pub mod auth_handlers;

// 分享链接模块
pub mod share_db;
pub mod share_handlers;

// 阅读进度模块
pub mod reading_progress_db;
pub mod reading_progress_handlers;
pub mod insights;

// 应用版本信息
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const APP_NAME: &str = "Obsidian Mirror";
