//! 简化的缓存管理API处理器

use std::sync::Arc;

#[cfg(feature = "web")]
use axum::{extract::State, http::StatusCode, response::Json};

use crate::web::types::{AppState, CacheStatsResponse, CacheClearRequest};

/// 获取缓存统计信息 - 简化版本
#[cfg(feature = "web")]
pub async fn get_cache_stats(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<CacheStatsResponse>, (StatusCode, Json<serde_json::Value>)> {
    // 轻量化版本：返回简化的缓存统计
    let stats = CacheStatsResponse {
        enabled: false, // 不再使用MongoDB缓存
        total_keys: 0,
        total_size_bytes: 0,
        expired_keys: 0,
    };

    Ok(Json(stats))
}

/// 清理缓存 - 简化版本
#[cfg(feature = "web")]
pub async fn clear_cache(
    State(_state): State<Arc<AppState>>,
    Json(_request): Json<CacheClearRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    // 轻量化版本：由于不使用数据库缓存，直接返回成功
    tracing::info!("缓存清理请求 - 轻量化版本无需处理");

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "轻量化版本不使用持久化缓存",
        "cleared_count": 0
    })))
}

/// 清理缓存 - 简化版本
#[cfg(feature = "web")]
pub async fn cleanup_cache(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    // 轻量化版本：直接返回成功
    tracing::info!("缓存清理请求 - 轻量化版本无需处理");

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "轻量化版本不使用持久化缓存"
    })))
}