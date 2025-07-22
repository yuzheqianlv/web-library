//! 缓存相关API处理器

use std::sync::Arc;

#[cfg(feature = "web")]
use axum::{
    extract::{Json as ExtractJson, State},
    http::StatusCode,
    response::Json,
};

use crate::web::types::{AppState, CacheClearRequest, CacheStatsResponse};

/// 获取缓存统计信息
#[cfg(feature = "web")]
pub async fn get_cache_stats(
    State(state): State<Arc<AppState>>,
) -> Result<Json<CacheStatsResponse>, (StatusCode, Json<serde_json::Value>)> {
    if let Some(ref collection) = state.mongo_collection {
        use mongodb::bson::doc;
        match collection.count_documents(doc! {}).await {
            Ok(count) => {
                let response = CacheStatsResponse {
                    enabled: true,
                    total_keys: count as usize,
                    total_size_bytes: 0, // MongoDB doesn't easily provide size info
                    expired_keys: 0, // No expiration in current implementation
                };
                Ok(Json(response))
            }
            Err(e) => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": true,
                    "message": format!("获取缓存统计失败: {}", e)
                })),
            )),
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
    if let Some(ref collection) = state.mongo_collection {
        use mongodb::bson::doc;
        
        let delete_result = if let (Some(url), Some(source_lang), Some(target_lang)) =
            (request.url, request.source_lang, request.target_lang)
        {
            // 清理特定缓存条目
            let filter = doc! {
                "url": url,
                "source_lang": source_lang,
                "target_lang": target_lang
            };
            collection.delete_many(filter).await
        } else {
            // 清理所有缓存
            collection.delete_many(doc! {}).await
        };

        match delete_result {
            Ok(result) => Ok(Json(serde_json::json!({
                "success": true,
                "message": format!("成功清理 {} 个缓存条目", result.deleted_count),
                "deleted_count": result.deleted_count
            }))),
            Err(e) => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": true,
                    "message": format!("清理缓存失败: {}", e)
                })),
            )),
        }
    } else {
        Ok(Json(serde_json::json!({
            "success": false,
            "message": "缓存未启用"
        })))
    }
}

/// 清理过期缓存（MongoDB版本中暂不支持过期功能）
#[cfg(feature = "web")]
pub async fn cleanup_cache(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    if state.mongo_collection.is_some() {
        // MongoDB版本中，我们可以根据创建时间清理旧的缓存条目
        // 这里返回成功但没有实际清理过期条目，因为当前实现中没有过期概念
        Ok(Json(serde_json::json!({
            "success": true,
            "message": "MongoDB版本暂不支持基于时间的缓存过期清理",
            "deleted_count": 0
        })))
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
