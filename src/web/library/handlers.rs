//! # Library HTTP处理器
//!
//! 提供库管理功能的HTTP路由处理器

#[cfg(feature = "web")]
use std::sync::Arc;

#[cfg(feature = "web")]
use axum::{
    extract::{Json as ExtractJson, Query, State},
    http::StatusCode,
    response::Json,
};

#[cfg(feature = "web")]
use crate::web::types::AppState;

use super::types::*;
use super::service::LibraryService;

/// 获取库数据处理器
///
/// 支持分页、排序和域名过滤
#[cfg(feature = "web")]
pub async fn get_library_data(
    State(state): State<Arc<AppState>>,
    Query(request): Query<LibraryRequest>,
) -> Result<Json<LibraryResponse>, (StatusCode, Json<serde_json::Value>)> {
    if let Some(ref database) = state.mongo_database {
        let service = LibraryService::new(database.clone());
        
        match service.get_library_data(&request).await {
            Ok(response) => Ok(Json(response)),
            Err(e) => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": true,
                    "message": format!("获取库数据失败: {}", e)
                }))
            ))
        }
    } else {
        // 如果没有MongoDB连接，返回空数据
        Ok(Json(LibraryResponse {
            domains: Vec::new(),
            total_domains: 0,
            page: request.page.unwrap_or(1),
            limit: request.limit.unwrap_or(20),
            total_pages: 0,
        }))
    }
}

/// 获取域名详情处理器
#[cfg(feature = "web")]
pub async fn get_domain_details(
    State(state): State<Arc<AppState>>,
    Query(request): Query<DomainDetailsRequest>,
) -> Result<Json<DomainDetailsResponse>, (StatusCode, Json<serde_json::Value>)> {
    if let Some(ref database) = state.mongo_database {
        let service = LibraryService::new(database.clone());
        
        match service.get_domain_details(&request).await {
            Ok(response) => Ok(Json(response)),
            Err(e) => Err((
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": true,
                    "message": format!("获取域名详情失败: {}", e)
                })),
            ))
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

/// 获取库统计信息处理器
#[cfg(feature = "web")]
pub async fn get_library_stats(
    State(state): State<Arc<AppState>>,
) -> Result<Json<LibraryStats>, (StatusCode, Json<serde_json::Value>)> {
    if let Some(ref database) = state.mongo_database {
        let service = LibraryService::new(database.clone());
        
        match service.get_library_stats().await {
            Ok(stats) => Ok(Json(stats)),
            Err(e) => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": true,
                    "message": format!("获取统计信息失败: {}", e)
                })),
            ))
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

/// 库操作处理器（删除域名、删除URL等）
#[cfg(feature = "web")]
pub async fn library_action(
    State(state): State<Arc<AppState>>,
    ExtractJson(request): ExtractJson<LibraryActionRequest>,
) -> Result<Json<LibraryActionResponse>, (StatusCode, Json<serde_json::Value>)> {
    if let Some(ref database) = state.mongo_database {
        let service = LibraryService::new(database.clone());
    
    match request.action {
        LibraryAction::DeleteDomain => {
            if let Some(domain) = request.domain {
                match service.delete_domain(&domain).await {
                    Ok(deleted_count) => Ok(Json(LibraryActionResponse {
                        success: true,
                        message: format!("成功删除域名 {} 的 {} 个文件", domain, deleted_count),
                        affected_count: Some(deleted_count),
                        data: None,
                    })),
                    Err(e) => Err((
                        StatusCode::NOT_FOUND,
                        Json(serde_json::json!({
                            "error": true,
                            "message": format!("删除域名失败: {}", e)
                        })),
                    ))
                }
            } else {
                Err((
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({
                        "error": true,
                        "message": "缺少域名参数"
                    })),
                ))
            }
        }
        LibraryAction::DeleteUrl => {
            if let (Some(url), Some(source_lang), Some(target_lang)) = 
                (request.url, request.source_lang, request.target_lang) {
                match service.delete_url(&url, &source_lang, &target_lang).await {
                    Ok(deleted) => {
                        if deleted {
                            Ok(Json(LibraryActionResponse {
                                success: true,
                                message: "成功删除URL".to_string(),
                                affected_count: Some(1),
                                data: None,
                            }))
                        } else {
                            Ok(Json(LibraryActionResponse {
                                success: false,
                                message: "URL不存在".to_string(),
                                affected_count: Some(0),
                                data: None,
                            }))
                        }
                    }
                    Err(e) => Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({
                            "error": true,
                            "message": format!("删除URL失败: {}", e)
                        })),
                    ))
                }
            } else {
                Err((
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({
                        "error": true,
                        "message": "缺少URL或语言参数"
                    })),
                ))
            }
        }
        LibraryAction::RefreshDomain => {
            // 刷新整个库
            match service.refresh().await {
                Ok(()) => Ok(Json(LibraryActionResponse {
                    success: true,
                    message: "刷新成功".to_string(),
                    affected_count: None,
                    data: None,
                })),
                Err(e) => Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "error": true,
                        "message": format!("刷新失败: {}", e)
                    })),
                ))
            }
        }
        _ => Err((
            StatusCode::NOT_IMPLEMENTED,
            Json(serde_json::json!({
                "error": true,
                "message": "不支持的操作"
            })),
        ))
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

/// 初始化数据库索引处理器
#[cfg(feature = "web")]
pub async fn initialize_indexes(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    if let Some(ref database) = state.mongo_database {
        let service = LibraryService::new(database.clone());
        
        match service.create_indexes().await {
            Ok(()) => Ok(Json(serde_json::json!({
                "success": true,
                "message": "数据库索引创建成功"
            }))),
            Err(e) => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": true,
                    "message": format!("创建索引失败: {}", e)
                })),
            ))
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

/// 清理过期数据处理器
#[cfg(feature = "web")]
pub async fn cleanup_expired(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    if let Some(ref database) = state.mongo_database {
        use bson::doc;
        
        // 删除标记为过期的文档（注意：CachedHtml没有is_expired字段，所以这里删除空的记录）
        let collection: mongodb::Collection<crate::web::types::CachedHtml> = database.collection("html_cache");
        let filter = doc! { 
            "$or": [
                { "original_html": { "$exists": false } },
                { "translated_html": { "$exists": false } },
                { "original_html": "" },
                { "translated_html": "" }
            ]
        };
        let result = collection.delete_many(filter).await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "error": true,
                        "message": format!("清理过期数据失败: {}", e)
                    }))
                )
            })?;
        
        Ok(Json(serde_json::json!({
            "success": true,
            "message": format!("成功清理 {} 条过期数据", result.deleted_count),
            "cleaned_count": result.deleted_count
        })))
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

/// 获取库状态处理器
#[cfg(feature = "web")]
pub async fn get_library_status(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let cache_status = if state.mongo_database.is_some() {
        "connected"
    } else {
        "disconnected"
    };

    Ok(Json(serde_json::json!({
        "cache_enabled": state.mongo_database.is_some(),
        "cache_type": "mongodb",
        "cache_status": cache_status,
        "features": {
            "domain_management": true,
            "stats": true,
            "cleanup": true,
            "batch_operations": true,
            "indexing": true
        }
    })))
}

/// 非 web feature 的占位函数
#[cfg(not(feature = "web"))]
pub async fn get_library_data() -> Result<String, String> {
    Err("Web feature not enabled".to_string())
}

#[cfg(not(feature = "web"))]
pub async fn get_domain_details() -> Result<String, String> {
    Err("Web feature not enabled".to_string())
}

#[cfg(not(feature = "web"))]
pub async fn get_library_stats() -> Result<String, String> {
    Err("Web feature not enabled".to_string())
}

#[cfg(not(feature = "web"))]
pub async fn delete_domain() -> Result<String, String> {
    Err("Web feature not enabled".to_string())
}

#[cfg(not(feature = "web"))]
pub async fn refresh_domain() -> Result<String, String> {
    Err("Web feature not enabled".to_string())
}

#[cfg(not(feature = "web"))]
pub async fn cleanup_expired() -> Result<String, String> {
    Err("Web feature not enabled".to_string())
}

#[cfg(not(feature = "web"))]
pub async fn get_library_status() -> Result<String, String> {
    Err("Web feature not enabled".to_string())
}