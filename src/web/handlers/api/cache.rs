//! 缓存相关API处理器

use std::sync::Arc;

#[cfg(feature = "web")]
use axum::{
    extract::{Json as ExtractJson, State},
    http::StatusCode,
    response::Json,
};

use crate::web::types::{AppState, CacheStatsResponse, CacheClearRequest};

/// 获取缓存统计信息
#[cfg(feature = "web")]
pub async fn get_cache_stats(
    State(state): State<Arc<AppState>>,
) -> Result<Json<CacheStatsResponse>, (StatusCode, Json<serde_json::Value>)> {
    if let Some(ref cache) = state.redis_cache {
        match cache.get_stats() {
            Ok(stats) => {
                let response = CacheStatsResponse {
                    enabled: true,
                    total_keys: stats.total_keys,
                    total_size_bytes: stats.total_size_bytes,
                    expired_keys: stats.expired_keys,
                };
                Ok(Json(response))
            }
            Err(e) => {
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "error": true,
                        "message": format!("获取缓存统计失败: {}", e)
                    }))
                ))
            }
        }
    } else {
        let response = CacheStatsResponse {
            enabled: false,
            total_keys: 0,
            total_size_bytes: 0,
            expired_keys: 0,
        };
        Ok(Json(response))
    }
}

/// 清理缓存
#[cfg(feature = "web")]
pub async fn clear_cache(
    State(state): State<Arc<AppState>>,
    ExtractJson(request): ExtractJson<CacheClearRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    if let Some(ref cache) = state.redis_cache {
        let result = if let (Some(url), Some(source_lang), Some(target_lang)) = 
            (request.url, request.source_lang, request.target_lang) {
            // 清理特定缓存条目
            cache.delete(&url, &source_lang, &target_lang)
                .map(|deleted| if deleted { 1 } else { 0 })
        } else {
            // 清理所有缓存
            cache.clear_all()
        };

        match result {
            Ok(deleted_count) => {
                Ok(Json(serde_json::json!({
                    "success": true,
                    "message": format!("成功清理 {} 个缓存条目", deleted_count),
                    "deleted_count": deleted_count
                })))
            }
            Err(e) => {
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "error": true,
                        "message": format!("清理缓存失败: {}", e)
                    }))
                ))
            }
        }
    } else {
        Ok(Json(serde_json::json!({
            "success": false,
            "message": "缓存未启用"
        })))
    }
}

/// 清理过期缓存
#[cfg(feature = "web")]
pub async fn cleanup_cache(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    if let Some(ref cache) = state.redis_cache {
        match cache.cleanup_expired() {
            Ok(deleted_count) => {
                Ok(Json(serde_json::json!({
                    "success": true,
                    "message": format!("成功清理 {} 个过期缓存条目", deleted_count),
                    "deleted_count": deleted_count
                })))
            }
            Err(e) => {
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "error": true,
                        "message": format!("清理过期缓存失败: {}", e)
                    }))
                ))
            }
        }
    } else {
        Ok(Json(serde_json::json!({
            "success": false,
            "message": "缓存未启用"
        })))
    }
}

/// 非 web feature 的占位函数
#[cfg(not(feature = "web"))]
pub async fn get_cache_stats() -> Result<String, String> {
    Err("Web feature not enabled".to_string())
}

#[cfg(not(feature = "web"))]
pub async fn clear_cache() -> Result<String, String> {
    Err("Web feature not enabled".to_string())
}

#[cfg(not(feature = "web"))]
pub async fn cleanup_cache() -> Result<String, String> {
    Err("Web feature not enabled".to_string())
}