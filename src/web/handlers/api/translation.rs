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

#[cfg(feature = "web")]
use crate::redis_cache::create_cached_translation;

/// 翻译 URL 处理器
#[cfg(feature = "web")]
pub async fn translate_url(
    State(state): State<Arc<AppState>>,
    ExtractJson(request): ExtractJson<TranslateRequest>,
) -> Result<Json<TranslateResponse>, (StatusCode, Json<serde_json::Value>)> {
    let url = request.url.clone();
    let target_lang = request.target_lang.unwrap_or_else(|| "zh".to_string());
    let source_lang = request.source_lang.unwrap_or_else(|| "auto".to_string());

    // 检查缓存
    if let Some(ref cache) = state.redis_cache {
        if let Ok(Some(cached)) = cache.get(&url, &source_lang, &target_lang) {
            let response = TranslateResponse {
                original_html: cached.original_html,
                translated_html: cached.translated_html,
                title: cached.title,
                url: cached.url,
            };
            return Ok(Json(response));
        }
    }

    // 克隆选项用于并行处理
    let options_original = state.monolith_options.clone();
    let options_translated = state.monolith_options.clone();

    // 并行处理原文和译文
    let original_future = {
        let url = url.clone();
        let options = options_original;
        task::spawn_blocking(move || {
            let session = Session::new(None, None, options);
            create_monolithic_document(session, url)
        })
    };

    let translated_future = {
        let url = url.clone();
        let options = options_translated;
        let target_lang = target_lang.clone();
        task::spawn_blocking(move || -> Result<(Vec<u8>, Option<String>), MonolithError> {
            // 先获取原始内容
            let session = Session::new(None, None, options.clone());
            let original_result = create_monolithic_document(session, url.clone())?;

            // 如果启用了翻译功能，进行翻译
            #[cfg(feature = "translation")]
            {
                use crate::html::{html_to_dom, serialize_document};
                use crate::translation::{translate_dom_content, load_translation_config};

                let (original_data, title) = original_result;
                let dom = html_to_dom(&original_data, url);
                
                // 加载翻译配置，从配置文件加载 API URL
                let translation_config = load_translation_config(&target_lang, None);
                
                // 翻译需要在异步上下文中运行，这里简化处理
                let translated_dom = tokio::runtime::Handle::current()
                    .block_on(translate_dom_content(dom, &target_lang, Some(&translation_config.deeplx_api_url)))
                    .map_err(|e| MonolithError::new(&format!("Translation error: {}", e)))?;
                
                let translated_data = serialize_document(translated_dom, "UTF-8".to_string(), &options);
                Ok((translated_data, title))
            }
            
            #[cfg(not(feature = "translation"))]
            Ok(original_result)
        })
    };

    // 等待两个任务完成
    let (original_result, translated_result) = match tokio::try_join!(original_future, translated_future) {
        Ok((original, translated)) => (original, translated),
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": true,
                    "message": format!("Task execution error: {}", e)
                }))
            ));
        }
    };

    let (original_data, original_title) = match original_result {
        Ok(result) => result,
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": true,
                    "message": format!("Failed to process original: {}", e)
                }))
            ));
        }
    };

    let (translated_data, translated_title) = match translated_result {
        Ok(result) => result,
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": true,
                    "message": format!("Failed to process translation: {}", e)
                }))
            ));
        }
    };

    let original_html = String::from_utf8_lossy(&original_data).to_string();
    let translated_html = String::from_utf8_lossy(&translated_data).to_string();

    // 使用标题
    let title = translated_title.or(original_title);

    // 缓存结果
    if let Some(ref cache) = state.redis_cache {
        let cached_translation = create_cached_translation(
            url,
            original_html.clone(),
            translated_html.clone(),
            title.clone(),
            source_lang,
            target_lang,
            None, // 使用默认 TTL
        );
        
        if let Err(e) = cache.set(&cached_translation) {
            eprintln!("警告: 缓存存储失败: {}", e);
        }
    }

    let response = TranslateResponse {
        original_html,
        translated_html,
        title,
        url: request.url,
    };

    Ok(Json(response))
}

/// 非 web feature 的占位函数
#[cfg(not(feature = "web"))]
pub async fn translate_url() -> Result<String, String> {
    Err("Web feature not enabled".to_string())
}