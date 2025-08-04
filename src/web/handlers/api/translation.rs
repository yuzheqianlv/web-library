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
    let source_lang = request
        .source_lang
        .clone()
        .unwrap_or_else(|| "auto".to_string());

    // 检查URL是否已存在或正在处理中
    if let Some(ref collection) = state.mongo_collection {
        use mongodb::bson::doc;
        use crate::web::library::v2_service::LibraryServiceV2;
        
        let library_service = LibraryServiceV2::new(state.mongo_database.as_ref().unwrap().clone());
        
        // 首先检查是否已有完成的缓存（优先级最高）
        let success_filter = doc! {
            "url": &url,
            "source_lang": &source_lang,
            "target_lang": &target_lang,
            "status": "success"
        };

        if let Ok(Some(cached)) = collection.find_one(success_filter).await {
            tracing::info!("从缓存返回已完成的翻译结果: {}", url);
            let response = TranslateResponse {
                original_html: cached.original_html,
                translated_html: cached.translated_html,
                title: cached.title,
                url: cached.url,
            };
            return Ok(Json(response));
        }

        // 检查是否正在处理
        match library_service.is_url_processing(&url, &source_lang, &target_lang).await {
            Ok(true) => {
                tracing::warn!("URL正在处理中，请稍后再试: {}", url);
                return Err((
                    axum::http::StatusCode::TOO_MANY_REQUESTS,
                    Json(serde_json::json!({
                        "error": "URL正在处理中，请稍后再试",
                        "url": url,
                        "status": "processing",
                        "message": "页面正在翻译处理中，请稍等片刻后刷新"
                    }))
                ));
            }
            Ok(false) => {
                // URL不在处理中，可以开始新的处理
                tracing::info!("URL未在处理中，准备开始新的翻译处理: {}", url);
            }
            Err(e) => {
                tracing::error!("检查URL状态失败: {}", e);
                // 继续处理，不因为数据库错误阻塞请求
            }
        }
        
        // 标记URL为处理中
        match library_service.mark_url_processing(&url, &source_lang, &target_lang).await {
            Ok(_) => {
                tracing::info!("标记URL为处理中: {}", url);
            }
            Err(e) if e.contains("正在处理中") => {
                tracing::warn!("URL已在处理中，拒绝重复请求: {}", url);
                return Err((
                    axum::http::StatusCode::TOO_MANY_REQUESTS,
                    Json(serde_json::json!({
                        "error": "URL正在处理中，请稍后再试",
                        "url": url,
                        "status": "processing"
                    }))
                ));
            }
            Err(e) => {
                tracing::error!("标记URL处理状态失败: {}", e);
                // 继续处理，不因为数据库错误阻塞请求
            }
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
                    use crate::html::{html_to_dom, serialize_document};
                    use crate::translation::{load_translation_config, translate_dom_content};

                    let (original_data, title) = original_result;
                    let dom = html_to_dom(&original_data, url);

                    // 加载翻译配置，从配置文件加载 API URL
                    let translation_config = load_translation_config(&target_lang_clone, None);

                    // 翻译需要在异步上下文中运行，这里简化处理
                    let translated_dom = tokio::runtime::Handle::current()
                        .block_on(translate_dom_content(
                            dom,
                            &target_lang_clone,
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
            // 处理失败，清理处理状态
            if let Some(ref collection) = state.mongo_collection {
                use mongodb::bson::doc;
                let filter = doc! {
                    "url": &url,
                    "source_lang": &source_lang,
                    "target_lang": &target_lang,
                    "status": "pending"
                };
                let _ = collection.delete_one(filter).await;
                tracing::error!("原文处理失败，已清理处理状态: {}", url);
            }
            
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
            // 翻译处理失败，清理处理状态
            if let Some(ref collection) = state.mongo_collection {
                use mongodb::bson::doc;
                let filter = doc! {
                    "url": &url,
                    "source_lang": &source_lang,
                    "target_lang": &target_lang,
                    "status": "pending"
                };
                let _ = collection.delete_one(filter).await;
                tracing::error!("翻译处理失败，已清理处理状态: {}", url);
            }
            
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
                eprintln!("警告: 链接重写失败，使用原始翻译结果");
            }
        }
    }

    // 使用标题
    let title = translated_title.or(original_title);

    // 缓存结果到MongoDB并更新处理状态
    if let Some(ref collection) = state.mongo_collection {
        use crate::web::types::CachedHtml;
        use mongodb::bson::doc;

        let final_source_lang = request.source_lang.unwrap_or_else(|| "auto".to_string());
        let final_target_lang = request.target_lang.unwrap_or_else(|| "zh".to_string());

        // 更新或插入完成的翻译结果
        let filter = doc! {
            "url": &request.url,
            "source_lang": &final_source_lang,
            "target_lang": &final_target_lang
        };

        let cached_html = CachedHtml {
            url: request.url.clone(),
            original_html: original_html.clone(),
            translated_html: translated_html.clone(),
            title: title.clone(),
            source_lang: final_source_lang.clone(),
            target_lang: final_target_lang.clone(),
            status: "success".to_string(),
            created_at: bson::DateTime::now(),
            updated_at: bson::DateTime::now(),
            expires_at: None, // 成功的翻译不设置过期时间
            file_size: (original_html.len() + translated_html.len()) as i64,
            domain: Some({
                use url::Url;
                if let Ok(parsed_url) = Url::parse(&request.url) {
                    parsed_url.host_str().unwrap_or("unknown").to_string()
                } else {
                    "unknown".to_string()
                }
            }),
        };

        // 使用 upsert 操作更新或插入
        match collection.replace_one(filter, &cached_html).upsert(true).await {
            Ok(_) => {
                tracing::info!("成功缓存翻译结果: {}", request.url);
            }
            Err(e) => {
                tracing::error!("MongoDB缓存存储失败: {}", e);
            }
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