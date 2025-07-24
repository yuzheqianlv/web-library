//! 额外的库管理API处理器

use std::sync::Arc;

#[cfg(feature = "web")]
use axum::{
    extract::{Json as ExtractJson, State},
    http::StatusCode,
    response::Json,
};

use crate::web::types::AppState;

/// 库操作处理器（批量操作等）
#[cfg(feature = "web")]
pub async fn library_action(
    State(_state): State<Arc<AppState>>,
    ExtractJson(request): ExtractJson<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    // 转发到library模块的处理器
    use crate::web::library::handlers::library_action as lib_action;
    use crate::web::library::types::LibraryActionRequest;

    // 尝试解析请求
    let action_request: LibraryActionRequest = serde_json::from_value(request).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": true,
                "message": format!("解析请求失败: {}", e)
            })),
        )
    })?;

    // 调用library模块的处理器
    match lib_action(
        axum::extract::State(_state),
        axum::extract::Json(action_request),
    )
    .await
    {
        Ok(response) => Ok(Json(serde_json::to_value(response.0).unwrap())),
        Err((status, json_err)) => Err((status, json_err)),
    }
}

/// 批量删除URLs处理器
#[cfg(feature = "web")]
pub async fn batch_delete_urls(
    State(state): State<Arc<AppState>>,
    ExtractJson(request): ExtractJson<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    if let Some(ref database) = state.mongo_database {
        use crate::web::library::service::LibraryService;

        let service = LibraryService::new(database.clone());

        // 解析URL列表
        let urls: Vec<String> = request
            .get("urls")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        if urls.is_empty() {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": true,
                    "message": "缺少URL列表参数"
                })),
            ));
        }

        match service.batch_delete_urls(&urls).await {
            Ok(deleted_count) => Ok(Json(serde_json::json!({
                "success": true,
                "message": format!("成功删除 {} 个 URL", deleted_count),
                "deleted_count": deleted_count
            }))),
            Err(e) => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": true,
                    "message": format!("批量删除失败: {}", e)
                })),
            )),
        }
    } else {
        Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": true,
                "message": "数据库服务不可用"
            })),
        ))
    }
}

/// 获取大小统计信息处理器
#[cfg(feature = "web")]
pub async fn get_size_statistics(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    if let Some(ref database) = state.mongo_database {
        use crate::web::library::service::LibraryService;

        let service = LibraryService::new(database.clone());

        match service.get_library_stats().await {
            Ok(stats) => {
                let average_size = if stats.total_urls > 0 {
                    stats.total_size_bytes / stats.total_urls
                } else {
                    0
                };

                Ok(Json(serde_json::json!({
                    "total_size": stats.total_size_bytes,
                    "average_size": average_size,
                    "largest_entries": stats.largest_entries,
                    "size_distribution": {
                        "total_entries": stats.total_urls,
                        "domains": stats.total_domains
                    }
                })))
            }
            Err(e) => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": true,
                    "message": format!("获取统计信息失败: {}", e)
                })),
            )),
        }
    } else {
        Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": true,
                "message": "数据库服务不可用"
            })),
        ))
    }
}

/// 健康检查处理器
#[cfg(feature = "web")]
pub async fn health_check(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let mongodb_status = if state.mongo_collection.is_some() {
        "connected"
    } else {
        "disconnected"
    };

    Ok(Json(serde_json::json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().timestamp(),
        "services": {
            "mongodb": mongodb_status,
            "monolith": "active"
        },
        "version": "2.11.0"
    })))
}

/// 非 web feature 的占位函数
#[cfg(not(feature = "web"))]
pub async fn library_action() -> Result<String, String> {
    Err("Web feature not enabled".to_string())
}

#[cfg(not(feature = "web"))]
pub async fn batch_delete_urls() -> Result<String, String> {
    Err("Web feature not enabled".to_string())
}

#[cfg(not(feature = "web"))]
pub async fn get_size_statistics() -> Result<String, String> {
    Err("Web feature not enabled".to_string())
}

#[cfg(not(feature = "web"))]
pub async fn health_check() -> Result<String, String> {
    Err("Web feature not enabled".to_string())
}
