//! 链接状态查询API
//!
//! 提供链接状态查询和批量处理功能

#[cfg(feature = "web")]
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
};

#[cfg(feature = "web")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::web::{
    services::cache_query_engine::{CacheQueryEngine, CacheStatus, RoutingStrategy},
    types::AppState,
};

/// 单个链接状态查询请求
#[cfg(feature = "web")]
#[derive(Debug, Deserialize)]
pub struct LinkStatusQuery {
    /// 目标URL
    pub url: String,
    /// 源语言（可选）
    pub source_lang: Option<String>,
    /// 目标语言（可选）
    pub target_lang: Option<String>,
}

/// 批量链接状态查询请求
#[cfg(feature = "web")]
#[derive(Debug, Deserialize)]
pub struct BatchLinkStatusRequest {
    /// URL列表
    pub urls: Vec<String>,
    /// 源语言（可选）
    pub source_lang: Option<String>,
    /// 目标语言（可选）
    pub target_lang: Option<String>,
    /// 最大并发数（可选，默认10）
    pub max_concurrent: Option<usize>,
}

/// 链接状态响应
#[cfg(feature = "web")]
#[derive(Debug, Clone, Serialize)]
pub struct LinkStatusResponse {
    /// 目标URL
    pub url: String,
    /// 状态码
    pub status: String,
    /// 缓存记录ID（如果存在）
    pub record_id: Option<String>,
    /// 建议的操作
    pub suggested_action: String,
    /// 可用操作列表
    pub available_actions: Vec<ActionOption>,
    /// 缓存详情（可选）
    pub cache_details: Option<CacheDetails>,
    /// 处理时间戳
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// 操作选项
#[cfg(feature = "web")]
#[derive(Debug, Clone, Serialize)]
pub struct ActionOption {
    /// 操作类型
    pub action: String,
    /// 操作显示名称
    pub label: String,
    /// 操作URL
    pub url: String,
    /// 是否为推荐操作
    pub recommended: bool,
}

/// 缓存详情
#[cfg(feature = "web")]
#[derive(Debug, Clone, Serialize)]
pub struct CacheDetails {
    /// 缓存状态
    pub status: String,
    /// 标题
    pub title: Option<String>,
    /// 源语言
    pub source_lang: String,
    /// 目标语言
    pub target_lang: String,
    /// 创建时间
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// 更新时间
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// 文件大小
    pub file_size: i64,
}

/// 批量查询响应
#[cfg(feature = "web")]
#[derive(Debug, Serialize)]
pub struct BatchLinkStatusResponse {
    /// 查询结果列表
    pub results: Vec<LinkStatusResponse>,
    /// 查询摘要
    pub summary: QuerySummary,
    /// 处理时间
    pub processing_time_ms: u64,
}

/// 查询摘要
#[cfg(feature = "web")]
#[derive(Debug, Serialize)]
pub struct QuerySummary {
    /// 总数
    pub total_urls: usize,
    /// 成功查询数
    pub successful_queries: usize,
    /// 失败查询数
    pub failed_queries: usize,
    /// 有缓存的URL数
    pub cached_urls: usize,
    /// 需要处理的URL数
    pub needs_processing: usize,
}

/// 查询单个链接状态
#[cfg(feature = "web")]
pub async fn check_link_status(
    State(state): State<std::sync::Arc<AppState>>,
    Query(params): Query<LinkStatusQuery>,
) -> Result<Json<LinkStatusResponse>, StatusCode> {
    let start_time = std::time::Instant::now();

    // 检查MongoDB支持
    let Some(ref db) = state.mongo_database else {
        return Ok(Json(create_no_cache_response(&params.url)));
    };

    // 创建缓存查询引擎
    let cache_engine = CacheQueryEngine::new(db.clone());

    // 查询缓存状态
    match cache_engine
        .query_cache_status(
            &params.url,
            params.source_lang.as_deref(),
            params.target_lang.as_deref(),
        )
        .await
    {
        Ok(cache_result) => {
            let response = create_link_status_response(
                &params.url,
                cache_result,
                params.source_lang.as_deref(),
                params.target_lang.as_deref(),
            );
            
            tracing::debug!(
                "链接状态查询完成: URL={}, 耗时={}ms, 状态={:?}",
                params.url,
                start_time.elapsed().as_millis(),
                response.status
            );

            Ok(Json(response))
        }
        Err(e) => {
            tracing::error!("链接状态查询失败: URL={}, 错误={}", params.url, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 批量查询链接状态
#[cfg(feature = "web")]
pub async fn check_batch_link_status(
    State(state): State<std::sync::Arc<AppState>>,
    Json(request): Json<BatchLinkStatusRequest>,
) -> Result<Json<BatchLinkStatusResponse>, StatusCode> {
    let start_time = std::time::Instant::now();
    let max_concurrent = request.max_concurrent.unwrap_or(10).min(50); // 限制最大并发数

    tracing::info!(
        "开始批量链接状态查询: 总数={}, 并发数={}",
        request.urls.len(),
        max_concurrent
    );

    // 检查MongoDB支持
    let Some(ref db) = state.mongo_database else {
        let results: Vec<LinkStatusResponse> = request
            .urls
            .iter()
            .map(|url| create_no_cache_response(url))
            .collect();

        return Ok(Json(BatchLinkStatusResponse {
            results,
            summary: create_summary(&request.urls, &[]),
            processing_time_ms: start_time.elapsed().as_millis() as u64,
        }));
    };

    // 使用信号量控制并发
    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(max_concurrent));
    let mut handles = Vec::new();

    for url in request.urls.iter() {
        let url = url.clone();
        let source_lang = request.source_lang.clone();
        let target_lang = request.target_lang.clone();
        let db = db.clone();
        let semaphore = semaphore.clone();

        let handle = tokio::spawn(async move {
            let _permit = semaphore.acquire().await.unwrap();
            
            // 为每个任务创建独立的缓存查询引擎
            let cache_engine = CacheQueryEngine::new(db);
            
            match cache_engine
                .query_cache_status(&url, source_lang.as_deref(), target_lang.as_deref())
                .await
            {
                Ok(cache_result) => create_link_status_response(
                    &url,
                    cache_result,
                    source_lang.as_deref(),
                    target_lang.as_deref(),
                ),
                Err(e) => {
                    tracing::warn!("单个URL查询失败: URL={}, 错误={}", url, e);
                    create_error_response(&url, &format!("查询失败: {}", e))
                }
            }
        });

        handles.push(handle);
    }

    // 等待所有查询完成
    let mut results = Vec::new();
    let mut successful_results = Vec::new();

    for handle in handles {
        match handle.await {
            Ok(response) => {
                if response.status != "error" {
                    successful_results.push(response.clone());
                }
                results.push(response);
            }
            Err(e) => {
                tracing::error!("任务执行失败: {}", e);
                results.push(create_error_response("unknown", "任务执行失败"));
            }
        }
    }

    let processing_time = start_time.elapsed().as_millis() as u64;
    let summary = create_summary(&request.urls, &successful_results);

    tracing::info!(
        "批量链接状态查询完成: 总数={}, 成功={}, 失败={}, 耗时={}ms",
        summary.total_urls,
        summary.successful_queries,
        summary.failed_queries,
        processing_time
    );

    Ok(Json(BatchLinkStatusResponse {
        results,
        summary,
        processing_time_ms: processing_time,
    }))
}

/// 获取域名下的所有缓存链接
#[cfg(feature = "web")]
pub async fn get_domain_links(
    State(state): State<std::sync::Arc<AppState>>,
    Query(params): Query<DomainLinksQuery>,
) -> Result<Json<DomainLinksResponse>, StatusCode> {
    // 检查MongoDB支持
    let Some(ref db) = state.mongo_database else {
        return Ok(Json(DomainLinksResponse {
            domain: params.domain.clone(),
            links: vec![],
            total_count: 0,
            target_lang: params.target_lang.unwrap_or_else(|| "zh".to_string()),
        }));
    };

    // 创建缓存查询引擎
    let cache_engine = CacheQueryEngine::new(db.clone());

    match cache_engine
        .get_domain_cache_links(&params.domain, params.target_lang.as_deref())
        .await
    {
        Ok(cached_records) => {
            let links: Vec<DomainLinkInfo> = cached_records
                .into_iter()
                .map(|record| DomainLinkInfo {
                    url: record.url,
                    title: record.title,
                    record_id: record.id.map(|id| id.to_hex()).unwrap_or_default(),
                    source_lang: record.source_lang,
                    target_lang: record.target_lang,
                    created_at: record.created_at.to_chrono(),
                    file_size: record.file_size,
                })
                .collect();

            let total_count = links.len();

            Ok(Json(DomainLinksResponse {
                domain: params.domain,
                links,
                total_count,
                target_lang: params.target_lang.unwrap_or_else(|| "zh".to_string()),
            }))
        }
        Err(e) => {
            tracing::error!("获取域名链接失败: domain={}, 错误={}", params.domain, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 域名链接查询请求
#[cfg(feature = "web")]
#[derive(Debug, Deserialize)]
pub struct DomainLinksQuery {
    /// 域名
    pub domain: String,
    /// 目标语言（可选）
    pub target_lang: Option<String>,
}

/// 域名链接响应
#[cfg(feature = "web")]
#[derive(Debug, Serialize)]
pub struct DomainLinksResponse {
    /// 域名
    pub domain: String,
    /// 链接列表
    pub links: Vec<DomainLinkInfo>,
    /// 总数
    pub total_count: usize,
    /// 目标语言
    pub target_lang: String,
}

/// 域名链接信息
#[cfg(feature = "web")]
#[derive(Debug, Serialize)]
pub struct DomainLinkInfo {
    /// URL
    pub url: String,
    /// 标题
    pub title: Option<String>,
    /// 记录ID
    pub record_id: String,
    /// 源语言
    pub source_lang: String,
    /// 目标语言
    pub target_lang: String,
    /// 创建时间
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// 文件大小
    pub file_size: i64,
}

// 辅助函数

/// 创建链接状态响应
#[cfg(feature = "web")]
fn create_link_status_response(
    url: &str,
    cache_result: crate::web::services::cache_query_engine::CacheQueryResult,
    source_lang: Option<&str>,
    target_lang: Option<&str>,
) -> LinkStatusResponse {
    let (status, suggested_action) = match cache_result.cache_status {
        CacheStatus::Complete => ("cached", "use_cache"),
        CacheStatus::Processing => ("processing", "wait"),
        CacheStatus::Expired => ("expired", "reprocess"),
        CacheStatus::Failed => ("failed", "reprocess"),
        CacheStatus::NotFound => ("not_found", "process"),
    };

    let available_actions = create_action_options(url, &cache_result, source_lang, target_lang);
    
    let cache_details = if let Some(ref record) = cache_result.cached_record {
        Some(CacheDetails {
            status: record.status.clone(),
            title: record.title.clone(),
            source_lang: record.source_lang.clone(),
            target_lang: record.target_lang.clone(),
            created_at: record.created_at.to_chrono(),
            updated_at: record.updated_at.to_chrono(),
            file_size: record.file_size,
        })
    } else {
        None
    };

    LinkStatusResponse {
        url: url.to_string(),
        status: status.to_string(),
        record_id: cache_result.record_id,
        suggested_action: suggested_action.to_string(),
        available_actions,
        cache_details,
        timestamp: chrono::Utc::now(),
    }
}

/// 创建操作选项
#[cfg(feature = "web")]
fn create_action_options(
    url: &str,
    cache_result: &crate::web::services::cache_query_engine::CacheQueryResult,
    source_lang: Option<&str>,
    target_lang: Option<&str>,
) -> Vec<ActionOption> {
    let mut actions = Vec::new();

    match cache_result.routing_strategy {
        RoutingStrategy::UseCache(ref record_id) => {
            actions.push(ActionOption {
                action: "view_cache".to_string(),
                label: "查看缓存版本".to_string(),
                url: format!("/library/{}", record_id),
                recommended: true,
            });
            actions.push(ActionOption {
                action: "reprocess".to_string(),
                label: "重新处理".to_string(),
                url: build_reprocess_url(url, source_lang, target_lang),
                recommended: false,
            });
        }
        RoutingStrategy::WaitForProcessing => {
            actions.push(ActionOption {
                action: "wait".to_string(),
                label: "等待处理完成".to_string(),
                url: format!("/smart-website/{}", url),
                recommended: true,
            });
            actions.push(ActionOption {
                action: "force_reprocess".to_string(),
                label: "强制重新处理".to_string(),
                url: build_reprocess_url(url, source_lang, target_lang),
                recommended: false,
            });
        }
        RoutingStrategy::ReprocessWithCheck => {
            if let Some(ref record_id) = cache_result.record_id {
                actions.push(ActionOption {
                    action: "view_cache".to_string(),
                    label: "查看现有版本".to_string(),
                    url: format!("/library/{}", record_id),
                    recommended: false,
                });
            }
            actions.push(ActionOption {
                action: "reprocess".to_string(),
                label: "重新处理".to_string(),
                url: build_reprocess_url(url, source_lang, target_lang),
                recommended: true,
            });
        }
        RoutingStrategy::FullReprocess => {
            actions.push(ActionOption {
                action: "process".to_string(),
                label: "开始处理".to_string(),
                url: build_reprocess_url(url, source_lang, target_lang),
                recommended: true,
            });
        }
    }

    // 添加通用操作
    actions.push(ActionOption {
        action: "debug".to_string(),
        label: "调试信息".to_string(),
        url: format!("/smart-website/{}?debug=true", url),
        recommended: false,
    });

    actions
}

/// 构建重新处理URL
#[cfg(feature = "web")]
fn build_reprocess_url(url: &str, source_lang: Option<&str>, target_lang: Option<&str>) -> String {
    let mut reprocess_url = format!("/smart-website/{}?force_reprocess=true", url);
    
    if let Some(source) = source_lang {
        reprocess_url.push_str(&format!("&source_lang={}", source));
    }
    if let Some(target) = target_lang {
        reprocess_url.push_str(&format!("&target_lang={}", target));
    }
    
    reprocess_url
}

/// 创建无缓存响应
#[cfg(feature = "web")]
fn create_no_cache_response(url: &str) -> LinkStatusResponse {
    LinkStatusResponse {
        url: url.to_string(),
        status: "no_cache_support".to_string(),
        record_id: None,
        suggested_action: "process".to_string(),
        available_actions: vec![ActionOption {
            action: "process".to_string(),
            label: "处理链接".to_string(),
            url: format!("/website/{}", url),
            recommended: true,
        }],
        cache_details: None,
        timestamp: chrono::Utc::now(),
    }
}

/// 创建错误响应
#[cfg(feature = "web")]
fn create_error_response(url: &str, _error_message: &str) -> LinkStatusResponse {
    LinkStatusResponse {
        url: url.to_string(),
        status: "error".to_string(),
        record_id: None,
        suggested_action: "retry".to_string(),
        available_actions: vec![ActionOption {
            action: "retry".to_string(),
            label: "重试".to_string(),
            url: format!("/smart-website/{}", url),
            recommended: true,
        }],
        cache_details: None,
        timestamp: chrono::Utc::now(),
    }
}

/// 创建查询摘要
#[cfg(feature = "web")]
fn create_summary(
    requested_urls: &[String],
    successful_results: &[LinkStatusResponse],
) -> QuerySummary {
    let cached_urls = successful_results
        .iter()
        .filter(|r| r.status == "cached")
        .count();

    let needs_processing = successful_results
        .iter()
        .filter(|r| matches!(r.status.as_str(), "not_found" | "failed" | "expired"))
        .count();

    QuerySummary {
        total_urls: requested_urls.len(),
        successful_queries: successful_results.len(),
        failed_queries: requested_urls.len() - successful_results.len(),
        cached_urls,
        needs_processing,
    }
}

// 非web feature的占位实现
#[cfg(not(feature = "web"))]
pub struct LinkStatusQuery;

#[cfg(not(feature = "web"))]
pub async fn check_link_status() {}

#[cfg(not(feature = "web"))]
pub async fn check_batch_link_status() {}

#[cfg(not(feature = "web"))]
pub async fn get_domain_links() {}