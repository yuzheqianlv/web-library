//! # 统计专用处理器
//!
//! 提供优化的统计查询和缓存管理功能

#[cfg(feature = "web")]
use std::sync::Arc;

#[cfg(feature = "web")]
use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
};

#[cfg(feature = "web")]
use crate::web::types::AppState;

use super::v2_types::*;
use super::v2_service::LibraryServiceV2;

/// 获取今日统计信息处理器
#[cfg(feature = "web")]
pub async fn get_today_stats(
    State(state): State<Arc<AppState>>,
) -> Result<Json<TodayStats>, (StatusCode, Json<ApiError>)> {
    let request_id = uuid::Uuid::new_v4().to_string();
    
    if let Some(ref database) = state.mongo_database {
        let service = LibraryServiceV2::new(database.clone());
        
        match service.get_today_stats().await {
            Ok(stats) => Ok(Json(stats)),
            Err(e) => Err(create_api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "STATS_ERROR",
                &format!("获取今日统计失败: {}", e),
                request_id,
                None,
            ))
        }
    } else {
        Err(create_api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "DATABASE_UNAVAILABLE",
            "数据库服务不可用",
            request_id,
            None,
        ))
    }
}

/// 获取语言分布统计处理器
#[cfg(feature = "web")]
pub async fn get_language_distribution(
    State(state): State<Arc<AppState>>,
) -> Result<Json<LanguageStats>, (StatusCode, Json<ApiError>)> {
    let request_id = uuid::Uuid::new_v4().to_string();
    
    if let Some(ref database) = state.mongo_database {
        let service = LibraryServiceV2::new(database.clone());
        
        match service.get_language_distribution().await {
            Ok(stats) => Ok(Json(stats)),
            Err(e) => Err(create_api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "STATS_ERROR",
                &format!("获取语言分布失败: {}", e),
                request_id,
                None,
            ))
        }
    } else {
        Err(create_api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "DATABASE_UNAVAILABLE",
            "数据库服务不可用",
            request_id,
            None,
        ))
    }
}

/// 获取缓存统计信息处理器
#[cfg(feature = "web")]
pub async fn get_cache_stats(
    State(state): State<Arc<AppState>>,
) -> Result<Json<super::cache::CacheInfo>, (StatusCode, Json<ApiError>)> {
    let request_id = uuid::Uuid::new_v4().to_string();
    
    if let Some(ref database) = state.mongo_database {
        let service = LibraryServiceV2::new(database.clone());
        
        match service.get_cache_stats().await {
            Ok(stats) => Ok(Json(stats)),
            Err(e) => Err(create_api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "CACHE_ERROR",
                &format!("获取缓存统计失败: {}", e),
                request_id,
                None,
            ))
        }
    } else {
        Err(create_api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "DATABASE_UNAVAILABLE",
            "数据库服务不可用",
            request_id,
            None,
        ))
    }
}

/// 清理统计缓存处理器
#[cfg(feature = "web")]
pub async fn clear_stats_cache(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let request_id = uuid::Uuid::new_v4().to_string();
    
    if let Some(ref database) = state.mongo_database {
        let service = LibraryServiceV2::new(database.clone());
        
        match service.clear_stats_cache().await {
            Ok(()) => Ok(Json(serde_json::json!({
                "success": true,
                "message": "统计缓存已清理",
                "timestamp": chrono::Utc::now()
            }))),
            Err(e) => Err(create_api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "CACHE_ERROR",
                &format!("清理缓存失败: {}", e),
                request_id,
                None,
            ))
        }
    } else {
        Err(create_api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "DATABASE_UNAVAILABLE",
            "数据库服务不可用",
            request_id,
            None,
        ))
    }
}

/// 使统计缓存失效处理器
#[cfg(feature = "web")]
pub async fn invalidate_stats_cache(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let request_id = uuid::Uuid::new_v4().to_string();
    
    if let Some(ref database) = state.mongo_database {
        let service = LibraryServiceV2::new(database.clone());
        
        match service.invalidate_stats_cache().await {
            Ok(()) => Ok(Json(serde_json::json!({
                "success": true,
                "message": "统计缓存已失效",
                "timestamp": chrono::Utc::now()
            }))),
            Err(e) => Err(create_api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "CACHE_ERROR",
                &format!("使缓存失效失败: {}", e),
                request_id,
                None,
            ))
        }
    } else {
        Err(create_api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "DATABASE_UNAVAILABLE",
            "数据库服务不可用",
            request_id,
            None,
        ))
    }
}

/// 综合统计面板处理器
#[cfg(feature = "web")]
pub async fn get_stats_dashboard(
    State(state): State<Arc<AppState>>,
) -> Result<Json<StatsDashboard>, (StatusCode, Json<ApiError>)> {
    let request_id = uuid::Uuid::new_v4().to_string();
    
    if let Some(ref database) = state.mongo_database {
        let service = LibraryServiceV2::new(database.clone());
        
        // 并行获取各种统计信息
        let (today_stats_result, language_stats_result, cache_info_result) = tokio::join!(
            service.get_today_stats(),
            service.get_language_distribution(),
            service.get_cache_stats()
        );
        
        let today_stats = today_stats_result.map_err(|e| create_api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "STATS_ERROR",
            &format!("获取今日统计失败: {}", e),
            request_id.clone(),
            None,
        ))?;
        
        let language_stats = language_stats_result.map_err(|e| create_api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "STATS_ERROR",
            &format!("获取语言统计失败: {}", e),
            request_id.clone(),
            None,
        ))?;
        
        let cache_info = cache_info_result.map_err(|e| create_api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "CACHE_ERROR",
            &format!("获取缓存信息失败: {}", e),
            request_id.clone(),
            None,
        ))?;
        
        let dashboard = StatsDashboard {
            today: today_stats,
            languages: language_stats,
            cache: cache_info,
            last_updated: chrono::Utc::now(),
        };
        
        Ok(Json(dashboard))
    } else {
        Err(create_api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "DATABASE_UNAVAILABLE",
            "数据库服务不可用",
            request_id,
            None,
        ))
    }
}

/// 统计面板响应
#[cfg(feature = "web")]
#[derive(Debug, serde::Serialize)]
pub struct StatsDashboard {
    /// 今日统计
    pub today: TodayStats,
    /// 语言分布
    pub languages: LanguageStats,
    /// 缓存信息
    pub cache: super::cache::CacheInfo,
    /// 最后更新时间
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

/// 创建API错误响应的辅助函数
#[cfg(feature = "web")]
fn create_api_error(
    status: StatusCode,
    error_code: &str,
    message: &str,
    request_id: String,
    details: Option<serde_json::Value>,
) -> (StatusCode, Json<ApiError>) {
    (
        status,
        Json(ApiError {
            error_code: error_code.to_string(),
            message: message.to_string(),
            details,
            request_id,
            timestamp: chrono::Utc::now(),
        })
    )
}

/// 非 web feature 的占位函数
#[cfg(not(feature = "web"))]
pub async fn get_today_stats() -> Result<String, String> {
    Err("Web feature not enabled".to_string())
}

#[cfg(not(feature = "web"))]
pub async fn get_language_distribution() -> Result<String, String> {
    Err("Web feature not enabled".to_string())
}

#[cfg(not(feature = "web"))]
pub async fn get_cache_stats() -> Result<String, String> {
    Err("Web feature not enabled".to_string())
}

#[cfg(not(feature = "web"))]
pub async fn clear_stats_cache() -> Result<String, String> {
    Err("Web feature not enabled".to_string())
}

#[cfg(not(feature = "web"))]
pub async fn invalidate_stats_cache() -> Result<String, String> {
    Err("Web feature not enabled".to_string())
}

#[cfg(not(feature = "web"))]
pub async fn get_stats_dashboard() -> Result<String, String> {
    Err("Web feature not enabled".to_string())
}