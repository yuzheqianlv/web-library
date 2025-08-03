//! 简化的翻译相关API处理器

use std::sync::Arc;

#[cfg(feature = "web")]
use axum::{
    extract::{Json as ExtractJson, State},
    http::StatusCode,
    response::Json,
};

#[cfg(feature = "web")]
use tokio::task;

use crate::core::{create_monolithic_document, MonolithError};
use crate::session::Session;
use crate::web::types::{AppState, TranslateRequest, TranslateResponse};

/// 简化的翻译 URL 处理器（已弃用，建议使用 /api/process）
#[cfg(feature = "web")]
pub async fn translate_url_deprecated(
    State(state): State<Arc<AppState>>,
    ExtractJson(request): ExtractJson<TranslateRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    tracing::warn!("使用了已弃用的API端点 /api/translate，建议使用 /api/process");
    
    // 转换为新的ProcessRequest格式
    let process_request = crate::web::handlers::api::process::ProcessRequest {
        url: request.url,
        translate: true,
        target_lang: request.target_lang,
        options: crate::web::handlers::api::process::ProcessOptions::default(),
    };
    
    // 调用新的统一处理器
    match crate::web::handlers::api::process::process_url(State(state), ExtractJson(process_request)).await {
        Ok(Json(response)) => {
            // 转换为旧的响应格式
            let legacy_response = TranslateResponse {
                original_html: response.original_html,
                translated_html: response.translated_html,
                title: response.title,
                url: response.url,
            };
            Ok(Json(serde_json::to_value(legacy_response).unwrap()))
        }
        Err((status, error)) => Err((status, error))
    }
}

/// 原始翻译 URL 处理器
#[cfg(feature = "web")]
pub async fn translate_url(
    State(state): State<Arc<AppState>>,
    ExtractJson(request): ExtractJson<TranslateRequest>,
) -> Result<Json<TranslateResponse>, (StatusCode, Json<serde_json::Value>)> {
    let url = request.url.clone();
    let target_lang = request
        .target_lang
        .clone()
        .unwrap_or_else(|| "zh".to_string());

    tracing::info!("开始处理URL翻译请求: {}", url);

    // 克隆选项用于并行处理
    let options_original = state.monolith_options.clone();
    let options_translated = state.monolith_options.clone();

    // 并行处理原文和译文
    let original_future = {
        let url = url.clone();
        let options = options_original;
        task::spawn_blocking(move || {
            let session = Session::new(None, None, options);
            create_monolithic_document(session, &url)
        })
    };

    let translated_future = {
        let url = url.clone();
        let options = options_translated;
        let target_lang_clone = target_lang.clone();
        task::spawn_blocking(
            move || -> Result<(Vec<u8>, Option<String>), MonolithError> {
                // 先获取原始内容
                let session = Session::new(None, None, options.clone());
                let original_result = create_monolithic_document(session, &url)?;

                // 如果启用了翻译功能，进行翻译
                #[cfg(feature = "translation")]
                {
                    use crate::parsers::html::{html_to_dom, serialize_document};
                    use crate::translation::translate_dom_content_sync;

                    tracing::info!("开始翻译处理，目标语言: {}", target_lang_clone);

                    // 解析HTML为DOM
                    let html_str = String::from_utf8_lossy(&original_result.0);
                    let dom = html_to_dom(html_str.as_bytes(), "UTF-8".to_string());

                    // 翻译DOM内容（使用同步版本）
                    match translate_dom_content_sync(dom, &target_lang_clone, None) {
                        Ok(translated_dom) => {
                            tracing::info!("翻译处理完成");
                            // 序列化回HTML
                            let translated_html = serialize_document(translated_dom, "UTF-8".to_string(), &options);
                            return Ok((translated_html, original_result.1));
                        }
                        Err(e) => {
                            tracing::error!("翻译失败: {}", e);
                            return Ok(original_result);
                        }
                    }
                }

                // 如果没有启用翻译功能，返回原始内容
                #[cfg(not(feature = "translation"))]
                {
                    let _ = target_lang_clone; // 避免未使用变量警告
                    tracing::warn!("翻译功能未启用，返回原始内容");
                    Ok(original_result)
                }
            }
        )
    };

    // 等待两个任务完成
    let (original_result, translated_result) = match tokio::try_join!(original_future, translated_future) {
        Ok((original, translated)) => (original, translated),
        Err(e) => {
            tracing::error!("处理任务失败: {}", e);
            let error_msg = format!("处理失败: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": error_msg,
                    "url": url
                }))
            ));
        }
    };

    // 处理结果
    let (original_html, title) = match original_result {
        Ok(result) => (String::from_utf8_lossy(&result.0).to_string(), result.1),
        Err(e) => {
            tracing::error!("获取原始内容失败: {}", e);
            let error_msg = format!("获取原始内容失败: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": error_msg,
                    "url": url
                }))
            ));
        }
    };

    let translated_html = match translated_result {
        Ok(result) => String::from_utf8_lossy(&result.0).to_string(),
        Err(e) => {
            tracing::warn!("翻译处理失败，返回原始内容: {}", e);
            original_html.clone()
        }
    };

    let response = TranslateResponse {
        original_html,
        translated_html,
        title: Some(title.unwrap_or_else(|| "网页翻译".to_string())),
        url,
    };

    tracing::info!("翻译处理完成，准备返回结果");
    Ok(Json(response))
}