//! 翻译相关API处理器

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

/// 翻译 URL 处理器
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
    let _source_lang = request
        .source_lang
        .clone()
        .unwrap_or_else(|| "auto".to_string());

    // 轻量化版本：移除MongoDB依赖，使用内存缓存
    // TODO: 实现基于内存的缓存和重复请求检测机制
    tracing::info!("轻量化模式：直接处理翻译请求，无持久化缓存");

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
        let _target_lang_clone = target_lang.clone();
        task::spawn_blocking(
            move || -> Result<(Vec<u8>, Option<String>), MonolithError> {
                // 先获取原始内容
                let session = Session::new(None, None, options.clone());
                let original_result = create_monolithic_document(session, &url)?;

                // 如果启用了翻译功能，进行翻译
                #[cfg(feature = "translation")]
                {
                    use crate::html::{html_to_dom, serialize_document};
                    use crate::translation::{load_translation_config, translate_dom_content};

                    let (original_data, title) = original_result;
                    let dom = html_to_dom(&original_data, url.clone());

                    // 加载翻译配置，从配置文件加载 API URL
                    let translation_config = load_translation_config(&_target_lang_clone, None);

                    // 翻译需要在异步上下文中运行，这里简化处理
                    let translated_dom = tokio::runtime::Handle::current()
                        .block_on(translate_dom_content(
                            dom,
                            &_target_lang_clone,
                            Some(&translation_config.api_url),
                        ))
                        .map_err(|e| MonolithError::new(&format!("Translation error: {}", e)))?;

                    let translated_data =
                        serialize_document(translated_dom, "UTF-8".to_string(), &options);
                    Ok((translated_data, title))
                }

                #[cfg(not(feature = "translation"))]
                Ok(original_result)
            },
        )
    };

    // 等待两个任务完成
    let (original_result, translated_result) =
        match tokio::try_join!(original_future, translated_future) {
            Ok((original, translated)) => (original, translated),
            Err(e) => {
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "error": true,
                        "message": format!("Task execution error: {}", e)
                    })),
                ));
            }
        };

    let (original_data, original_title) = match original_result {
        Ok(result) => result,
        Err(e) => {
            tracing::error!("原文处理失败: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": true,
                    "message": format!("Failed to process original: {}", e)
                })),
            ));
        }
    };

    let (translated_data, translated_title) = match translated_result {
        Ok(result) => result,
        Err(e) => {
            tracing::error!("翻译处理失败: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": true,
                    "message": format!("Failed to process translation: {}", e)
                })),
            ));
        }
    };

    let original_html = String::from_utf8_lossy(&original_data).to_string();
    let mut translated_html = String::from_utf8_lossy(&translated_data).to_string();

    // 对翻译后的HTML进行链接重写，使链接指向翻译服务
    #[cfg(feature = "translation")]
    {
        use crate::parsers::link_rewriter::rewrite_links_in_html;

        // 检查是否启用了链接重写功能
        if state.monolith_options.rewrite_links {
            let translation_base_path = state
                .monolith_options
                .translation_base_path
                .as_deref()
                .unwrap_or("/website/");

            if let Ok(rewritten_html) =
                rewrite_links_in_html(&translated_html, &url, Some(translation_base_path))
            {
                translated_html = rewritten_html;
            } else {
                // 如果链接重写失败，记录错误但继续使用原始HTML
                tracing::warn!("链接重写失败，使用原始翻译结果");
            }
        }
    }
    
    #[cfg(not(feature = "translation"))]
    {
        // 轻量化版本：不进行链接重写
    }

    // 使用标题
    let title = translated_title.or(original_title);

    // 轻量化版本：无持久化缓存，仅记录处理完成
    tracing::info!("翻译处理完成: {}", request.url);

    let response = TranslateResponse {
        original_html,
        translated_html,
        title,
        url: request.url,
    };

    Ok(Json(response))
}

/// 向后兼容的翻译处理器（已弃用）
#[cfg(feature = "web")]
pub async fn translate_url_deprecated(
    State(state): State<Arc<AppState>>,
    ExtractJson(request): ExtractJson<TranslateRequest>,
) -> Result<Json<TranslateResponse>, (StatusCode, Json<serde_json::Value>)> {
    tracing::warn!("使用了已弃用的 /api/translate 端点，建议使用 /api/process");
    
    // 将旧的翻译请求转换为新的处理请求
    use crate::web::types::{ProcessRequest, ProcessOptions};
    
    let process_request = ProcessRequest {
        url: request.url.clone(),
        translate: Some(true),
        target_lang: request.target_lang.clone(),
        source_lang: request.source_lang.clone(),
        options: Some(ProcessOptions {
            no_css: Some(false),
            no_js: Some(true),
            no_images: Some(false),
            no_fonts: Some(false),
            timeout: Some(30),
        }),
    };
    
    // 调用新的统一处理器
    match super::process::process_url(State(state), ExtractJson(process_request)).await {
        Ok(Json(process_response)) => {
            let translate_response = TranslateResponse {
                original_html: process_response.original_html.unwrap_or_default(),
                translated_html: process_response.translated_html.unwrap_or_default(),
                title: process_response.title,
                url: process_response.url,
            };
            Ok(Json(translate_response))
        }
        Err(error_response) => Err(error_response),
    }
}

/// 非 web feature 的占位函数
#[cfg(not(feature = "web"))]
pub async fn translate_url() -> Result<String, String> {
    Err("Web feature not enabled".to_string())
}

/// 非 web feature 的占位函数
#[cfg(not(feature = "web"))]
pub async fn translate_url_deprecated() -> Result<String, String> {
    Err("Web feature not enabled".to_string())
}