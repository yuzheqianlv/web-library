//! 网页处理API - 核心处理端点

use std::sync::Arc;

#[cfg(feature = "web")]
use axum::{
    extract::{Json as ExtractJson, State},
    http::StatusCode,
    response::Json,
};

#[cfg(feature = "web")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use tokio::task;

use crate::core::create_monolithic_document;
use crate::session::Session;
use crate::web::types::{AppState, TranslationInfo};

/// 处理请求结构
#[cfg(feature = "web")]
#[derive(Debug, Deserialize)]
pub struct ProcessRequest {
    /// 要处理的URL
    pub url: String,
    /// 是否启用翻译
    #[serde(default)]
    pub translate: bool,
    /// 目标语言（翻译时使用）
    pub target_lang: Option<String>,
    /// 处理选项
    #[serde(default)]
    pub options: ProcessOptions,
}

/// 处理选项
#[cfg(feature = "web")]
#[derive(Debug, Deserialize, Default)]
pub struct ProcessOptions {
    /// 禁用CSS
    #[serde(default)]
    pub no_css: bool,
    /// 禁用JavaScript
    #[serde(default)]
    pub no_js: bool,
    /// 禁用图片
    #[serde(default)]
    pub no_images: bool,
    /// 禁用字体
    #[serde(default)]
    pub no_fonts: bool,
    /// 超时时间（秒）
    pub timeout: Option<u64>,
}

/// 处理响应结构 - 统一返回原文和译文
#[cfg(feature = "web")]
#[derive(Debug, Serialize)]
pub struct ProcessResponse {
    /// 处理状态
    pub status: String,
    /// 原始HTML内容
    pub original_html: String,
    /// 翻译后的HTML内容（总是尝试生成，如果翻译失败则与原文相同）
    pub translated_html: String,
    /// 页面标题
    pub title: Option<String>,
    /// 原始URL
    pub url: String,
    /// 预览URL
    pub preview_url: String,
    /// 翻译状态信息
    pub translation_info: TranslationInfo,
}


/// 网页处理API端点
#[cfg(feature = "web")]
pub async fn process_url(
    State(state): State<Arc<AppState>>,
    ExtractJson(request): ExtractJson<ProcessRequest>,
) -> Result<Json<ProcessResponse>, (StatusCode, Json<serde_json::Value>)> {
    let url = request.url.clone();
    
    // 验证URL格式
    let final_url = if url.starts_with("http://") || url.starts_with("https://") {
        url
    } else {
        format!("https://{}", url)
    };

    tracing::info!("开始处理URL: {}", final_url);

    // 配置Monolith选项
    let mut options = state.monolith_options.clone();
    options.no_css = request.options.no_css;
    options.no_js = request.options.no_js;
    options.no_images = request.options.no_images;
    options.no_fonts = request.options.no_fonts;
    if let Some(timeout) = request.options.timeout {
        options.timeout = timeout;
    }

    // 处理原始内容
    let url_clone = final_url.clone();
    let (original_html_bytes, title) = match task::spawn_blocking(move || {
        let session = Session::new(None, None, options);
        create_monolithic_document(session, &url_clone)
    }).await {
        Ok(Ok((html_bytes, title))) => (html_bytes, title),
        Ok(Err(e)) => {
            tracing::error!("处理原始内容失败: {}", e);
            let error_msg = format!("处理失败: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": error_msg,
                    "url": final_url
                }))
            ));
        }
        Err(e) => {
            tracing::error!("任务执行失败: {}", e);
            let error_msg = format!("服务器错误: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": error_msg,
                    "url": final_url
                }))
            ));
        }
    };

    let original_html = String::from_utf8_lossy(&original_html_bytes).to_string();

    // 统一处理翻译逻辑 - 使用异步版本避免运行时冲突
    let target_lang = request.target_lang.unwrap_or_else(|| "zh".to_string());
    let (translated_html, translation_info) = if request.translate {
        #[cfg(feature = "translation")]
        {
            use crate::parsers::html::{html_to_dom, serialize_document};
            use crate::translation::translate_dom_content;

            tracing::info!("开始翻译处理，目标语言: {}", target_lang);

            // 解析HTML为DOM
            let dom = html_to_dom(&original_html_bytes, "UTF-8".to_string());

            // 使用异步翻译函数
            match translate_dom_content(dom, &target_lang, None).await {
                Ok(translated_dom) => {
                    tracing::info!("翻译处理完成");
                    // 序列化回HTML
                    let translated_html = serialize_document(
                        translated_dom, 
                        "UTF-8".to_string(), 
                        &state.monolith_options
                    );
                    let translated_content = String::from_utf8_lossy(&translated_html).to_string();
                    
                    let info = TranslationInfo {
                        enabled: true,
                        target_language: target_lang.clone(),
                        success: true,
                        error: None,
                    };
                    
                    (translated_content, info)
                }
                Err(e) => {
                    tracing::error!("翻译失败: {}", e);
                    let info = TranslationInfo {
                        enabled: true,
                        target_language: target_lang.clone(),
                        success: false,
                        error: Some(e.to_string()),
                    };
                    // 翻译失败时返回原文
                    (original_html.clone(), info)
                }
            }
        }

        #[cfg(not(feature = "translation"))]
        {
            tracing::warn!("翻译功能未启用");
            let info = TranslationInfo {
                enabled: false,
                target_language: target_lang.clone(),
                success: false,
                error: Some("Translation feature not enabled".to_string()),
            };
            // 翻译功能未启用时返回原文
            (original_html.clone(), info)
        }
    } else {
        // 未启用翻译时返回原文
        let info = TranslationInfo {
            enabled: false,
            target_language: target_lang.clone(),
            success: false,
            error: None,
        };
        (original_html.clone(), info)
    };

    // 生成预览URL
    let preview_url = format!("/preview/{}", urlencoding::encode(&final_url));

    let response = ProcessResponse {
        status: "success".to_string(),
        original_html,
        translated_html,
        title,
        url: final_url,
        preview_url,
        translation_info,
    };

    tracing::info!("处理完成，返回结果");
    Ok(Json(response))
}

/// 非web功能的占位函数
#[cfg(not(feature = "web"))]
pub async fn process_url() -> Result<String, String> {
    Err("Web feature not enabled".to_string())
}