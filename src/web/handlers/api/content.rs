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

/// 获取内容处理器
#[cfg(feature = "web")]
pub async fn get_content(
    State(state): State<Arc<AppState>>,
    ExtractJson(request): ExtractJson<ContentRequest>,
) -> Result<Json<ContentResponse>, (StatusCode, Json<serde_json::Value>)> {
    let url = request.url.clone();
    let options = state.monolith_options.clone();

    let (data, title) = task::spawn_blocking(move || {
        let session = Session::new(None, None, options);
        create_monolithic_document(session, url)
    })
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": true,
                "message": format!("Task execution error: {}", e)
            }))
        )
    })?
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": true,
                "message": format!("Failed to process content: {}", e)
            }))
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