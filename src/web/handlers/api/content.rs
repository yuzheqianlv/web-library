//! 内容处理API

use std::sync::Arc;

#[cfg(feature = "web")]
use axum::{
    extract::{Json as ExtractJson, State},
    http::StatusCode,
    response::Json,
};

use crate::web::types::{AppState, ContentRequest, ContentResponse};

/// 获取内容处理器 - 轻量化版本
/// 
/// 这个处理器提供快速的内容获取功能，不包含翻译
#[cfg(feature = "web")]
pub async fn get_content(
    State(_state): State<Arc<AppState>>,
    ExtractJson(request): ExtractJson<ContentRequest>,
) -> Result<Json<ContentResponse>, (StatusCode, Json<serde_json::Value>)> {
    let url = request.url.clone();
    
    tracing::info!("获取内容请求: {}", url);

    // 轻量化版本：直接重定向到统一处理端点
    // 这样可以保持API兼容性，同时简化代码维护
    let response = ContentResponse {
        status: "redirect".to_string(),
        message: Some("请使用 /api/process 端点进行统一处理".to_string()),
        redirect_to: Some("/api/process".to_string()),
        url: url.clone(),
    };

    tracing::info!("内容请求重定向: {} -> /api/process", url);
    Ok(Json(response))
}

/// 向后兼容的内容获取处理器（已弃用）
#[cfg(feature = "web")]
pub async fn get_content_deprecated(
    state: State<Arc<AppState>>,
    request: ExtractJson<ContentRequest>,
) -> Result<Json<ContentResponse>, (StatusCode, Json<serde_json::Value>)> {
    tracing::warn!("使用了已弃用的 /api/content 端点，建议使用 /api/process");
    get_content(state, request).await
}

/// 非 web feature 的占位函数
#[cfg(not(feature = "web"))]
pub async fn get_content() -> Result<String, String> {
    Err("Web feature not enabled".to_string())
}

/// 非 web feature 的占位函数
#[cfg(not(feature = "web"))]
pub async fn get_content_deprecated() -> Result<String, String> {
    Err("Web feature not enabled".to_string())
}