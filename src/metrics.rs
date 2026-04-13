use lazy_static::lazy_static;
use prometheus::{
    IntCounter, IntGauge, Histogram, HistogramOpts, Registry, TextEncoder, Encoder,
};
use actix_web::{get, web, HttpResponse, Responder};
use std::sync::Arc;
use crate::state::AppState;

lazy_static! {
    /// Prometheus 注册表
    pub static ref REGISTRY: Registry = Registry::new();
    
    /// HTTP 请求总数
    pub static ref HTTP_REQUESTS_TOTAL: IntCounter = IntCounter::new(
        "http_requests_total",
        "Total number of HTTP requests"
    ).expect("metric can be created");
    
    /// 同步操作总数
    pub static ref SYNC_TOTAL: IntCounter = IntCounter::new(
        "sync_operations_total",
        "Total number of sync operations"
    ).expect("metric can be created");
    
    /// 搜索请求总数
    pub static ref SEARCH_REQUESTS_TOTAL: IntCounter = IntCounter::new(
        "search_requests_total",
        "Total number of search requests"
    ).expect("metric can be created");
    
    /// 当前笔记数量
    pub static ref NOTES_COUNT: IntGauge = IntGauge::new(
        "notes_count",
        "Current number of notes"
    ).expect("metric can be created");
    
    /// HTTP 请求延迟直方图
    pub static ref HTTP_REQUEST_DURATION: Histogram = Histogram::with_opts(
        HistogramOpts::new(
            "http_request_duration_seconds",
            "HTTP request latencies in seconds"
        ).buckets(vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0, 10.0])
    ).expect("metric can be created");
}

/// 初始化 Prometheus 指标
///
/// 指标已注册时静默忽略（如测试环境多次调用），防止 panic。
pub fn init_metrics() {
    // 指标已注册（AlreadyReg）时忽略错误，防止测试环境多次初始化时 panic
    let _ = REGISTRY.register(Box::new(HTTP_REQUESTS_TOTAL.clone()));
    let _ = REGISTRY.register(Box::new(SYNC_TOTAL.clone()));
    let _ = REGISTRY.register(Box::new(SEARCH_REQUESTS_TOTAL.clone()));
    let _ = REGISTRY.register(Box::new(NOTES_COUNT.clone()));
    let _ = REGISTRY.register(Box::new(HTTP_REQUEST_DURATION.clone()));
}

/// GET /metrics - Prometheus 指标端点
/// 
/// 暴露 Prometheus 格式的指标数据，用于：
/// - Prometheus 服务器抓取
/// - Grafana 可视化
/// - 监控告警
/// 
/// 指标包括：
/// - http_requests_total: HTTP 请求总数
/// - sync_operations_total: 同步操作总数
/// - search_requests_total: 搜索请求总数
/// - notes_count: 当前笔记数量
/// - http_request_duration_seconds: HTTP 请求延迟直方图
#[get("/metrics")]
pub async fn metrics_handler(data: web::Data<Arc<AppState>>) -> impl Responder {
    // 更新笔记数量指标
    let notes_count = data.notes.read().await.len() as i64;
    NOTES_COUNT.set(notes_count);
    
    // 编码所有指标为 Prometheus 文本格式
    let encoder = TextEncoder::new();
    let metric_families = REGISTRY.gather();
    let mut buffer = vec![];
    
    encoder.encode(&metric_families, &mut buffer)
        .expect("Failed to encode metrics");
    
    let output = String::from_utf8(buffer)
        .expect("Failed to convert metrics to string");
    
    HttpResponse::Ok()
        .content_type("text/plain; version=0.0.4")
        .body(output)
}
