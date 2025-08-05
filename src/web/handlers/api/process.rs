//! 统一的URL处理API

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
use crate::web::types::{AppState, ProcessRequest, ProcessResponse};

/// 统一的URL处理端点 - 轻量化版本
/// 
/// 这是书签脚本的核心API，整合了资源提取和翻译功能
#[cfg(feature = "web")]
pub async fn process_url(
    State(state): State<Arc<AppState>>,
    ExtractJson(request): ExtractJson<ProcessRequest>,
) -> Result<Json<ProcessResponse>, (StatusCode, Json<serde_json::Value>)> {
    let url = request.url.clone();
    let translate = request.translate.unwrap_or(false);
    let target_lang = request.target_lang.clone().unwrap_or_else(|| "zh".to_string());

    tracing::info!("处理URL请求: {} (翻译: {})", url, translate);

    // 克隆选项用于处理
    let mut options_original = state.monolith_options.clone();
    let mut options_translated = state.monolith_options.clone();

    // 应用请求中的选项
    if let Some(ref opts) = request.options {
        options_original.no_css = opts.no_css.unwrap_or(false);
        options_original.no_js = opts.no_js.unwrap_or(true); // 默认移除JS提高安全性
        options_original.no_images = opts.no_images.unwrap_or(false);
        options_original.no_fonts = opts.no_fonts.unwrap_or(false);
        options_original.timeout = opts.timeout.unwrap_or(30);

        options_translated.no_css = opts.no_css.unwrap_or(false);
        options_translated.no_js = opts.no_js.unwrap_or(true);
        options_translated.no_images = opts.no_images.unwrap_or(false);
        options_translated.no_fonts = opts.no_fonts.unwrap_or(false);
        options_translated.timeout = opts.timeout.unwrap_or(30);
    }

    // 始终处理原文
    let original_future = {
        let url = url.clone();
        let options = options_original;
        task::spawn_blocking(move || {
            let session = Session::new(None, None, options);
            create_monolithic_document(session, &url)
        })
    };

    // 根据请求决定是否处理译文
    let translated_future = if translate {
        let url = url.clone();
        let options = options_translated;
        let _target_lang_clone = target_lang.clone();
        
        Some(task::spawn_blocking(
            move || -> Result<(Vec<u8>, Option<String>), MonolithError> {
                // 先获取原始内容
                let session = Session::new(None, None, options.clone());
                let original_result = create_monolithic_document(session, &url)?;

                // 如果启用了翻译功能，进行翻译
                #[cfg(feature = "translation")]
                {
                    use crate::parsers::html::{html_to_dom, serialize_document};
                    use crate::translation::{load_translation_config, translate_dom_content};

                    let (original_data, title) = original_result;
                    let dom = html_to_dom(&original_data, url.clone());

                    // 加载翻译配置
                    let translation_config = load_translation_config(&_target_lang_clone, None);

                    // 翻译DOM内容
                    let translated_dom = tokio::runtime::Handle::current()
                        .block_on(translate_dom_content(
                            dom,
                            &_target_lang_clone,
                            Some(&translation_config.api_url),
                        ))
                        .map_err(|e| MonolithError::new(&format!("翻译错误: {}", e)))?;

                    let translated_data = serialize_document(translated_dom, "UTF-8".to_string(), &options);
                    Ok((translated_data, title))
                }

                #[cfg(not(feature = "translation"))]
                {
                    tracing::warn!("翻译功能未启用，返回原文内容");
                    Ok(original_result)
                }
            },
        ))
    } else {
        None
    };

    // 等待原文处理完成
    let original_result = match original_future.await {
        Ok(result) => result,
        Err(e) => {
            tracing::error!("原文处理任务失败: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "status": "error",
                    "error": format!("任务执行失败: {}", e)
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
                    "status": "error",
                    "error": format!("原文处理失败: {}", e)
                })),
            ));
        }
    };

    let original_html = String::from_utf8_lossy(&original_data).to_string();
    
    // 处理译文（如果需要）
    let translated_html = if let Some(future) = translated_future {
        match future.await {
            Ok(Ok((translated_data, _translated_title))) => {
                let html = String::from_utf8_lossy(&translated_data).to_string();
                
                // 对翻译后的HTML进行链接重写（如果启用）
                #[cfg(feature = "translation")]
                if state.monolith_options.rewrite_links {
                    use crate::parsers::link_rewriter::rewrite_links_in_html;
                    
                    let translation_base_path = state
                        .monolith_options
                        .translation_base_path
                        .as_deref()
                        .unwrap_or("/preview/");

                    match rewrite_links_in_html(&html, &url, Some(translation_base_path)) {
                        Ok(rewritten_html) => Some(rewritten_html),
                        Err(e) => {
                            tracing::warn!("链接重写失败: {}, 使用原始翻译结果", e);
                            Some(html)
                        }
                    }
                } else {
                    Some(html)
                }
                
                #[cfg(not(feature = "translation"))]
                Some(html)
            }
            Ok(Err(e)) => {
                tracing::error!("翻译处理失败: {}", e);
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "status": "error",
                        "error": format!("翻译处理失败: {}", e)
                    })),
                ));
            }
            Err(e) => {
                tracing::error!("翻译任务执行失败: {}", e);
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "status": "error",
                        "error": format!("翻译任务失败: {}", e)
                    })),
                ));
            }
        }
    } else {
        None
    };

    // 生成预览URL
    let preview_url = format!("/preview/{}", urlencoding::encode(&url));

    let response = ProcessResponse {
        status: "success".to_string(),
        original_html: Some(original_html),
        translated_html,
        title: original_title,
        url: url.clone(),
        preview_url: Some(preview_url),
        translate_enabled: translate,
        target_lang: if translate { Some(target_lang) } else { None },
    };

    tracing::info!("URL处理完成: {}", url);
    Ok(Json(response))
}

/// 非 web feature 的占位函数
#[cfg(not(feature = "web"))]
pub async fn process_url() -> Result<String, String> {
    Err("Web feature not enabled".to_string())
}