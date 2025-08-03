//! 内容相关API处理器

use std::sync::Arc;

#[cfg(feature = "web")]
use axum::{
    extract::{Json as ExtractJson, State},
    http::StatusCode,
    response::Json,
};

#[cfg(feature = "web")]
use tokio::task;

use crate::core::create_monolithic_document;
use crate::session::Session;
use crate::web::types::{AppState, ContentRequest, ContentResponse};

/// 获取内容处理器（已弃用，建议使用 /api/process）
#[cfg(feature = "web")]
pub async fn get_content_deprecated(
    State(state): State<Arc<AppState>>,
    ExtractJson(request): ExtractJson<ContentRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    tracing::warn!("使用了已弃用的API端点 /api/content，建议使用 /api/process");
    
    // 转换为新的ProcessRequest格式
    let process_request = crate::web::handlers::api::process::ProcessRequest {
        url: request.url,
        translate: false, // content端点默认不翻译
        target_lang: None,
        options: crate::web::handlers::api::process::ProcessOptions::default(),
    };
    
    // 调用新的统一处理器
    match crate::web::handlers::api::process::process_url(State(state), ExtractJson(process_request)).await {
        Ok(Json(response)) => {
            // 转换为旧的响应格式
            let legacy_response = ContentResponse {
                html: response.original_html,
                title: response.title,
                url: response.url,
            };
            Ok(Json(serde_json::to_value(legacy_response).unwrap()))
        }
        Err((status, error)) => Err((status, error))
    }
}

/// 原始获取内容处理器
#[cfg(feature = "web")]
pub async fn get_content(
    State(state): State<Arc<AppState>>,
    ExtractJson(request): ExtractJson<ContentRequest>,
) -> Result<Json<ContentResponse>, (StatusCode, Json<serde_json::Value>)> {
    let url = request.url.clone();
    let options = state.monolith_options.clone();

    let (data, title) = task::spawn_blocking(move || {
        let session = Session::new(None, None, options);
        create_monolithic_document(session, &url)
    })
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": true,
                "message": format!("Task execution error: {}", e)
            })),
        )
    })?
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": true,
                "message": format!("Failed to process content: {}", e)
            })),
        )
    })?;

    let html = String::from_utf8_lossy(&data).to_string();

    let response = ContentResponse {
        html,
        title,
        url: request.url,
    };

    Ok(Json(response))
}

/// 非 web feature 的占位函数
#[cfg(not(feature = "web"))]
pub async fn get_content() -> Result<String, String> {
    Err("Web feature not enabled".to_string())
}
