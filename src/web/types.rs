//! Web 模块的数据类型定义

#[cfg(feature = "web")]
use serde::{Deserialize, Serialize};

// MongoDB 相关结构已移除 - 轻量化版本不再使用数据库

/// 简化的应用状态 - 轻量化版本
#[cfg(feature = "web")]
#[derive(Clone)]
pub struct AppState {
    pub monolith_options: crate::core::MonolithOptions,
    pub theme_manager: std::sync::Arc<std::sync::Mutex<crate::web::theme::ThemeManager>>,
}

/// 翻译请求
#[cfg(feature = "web")]
#[derive(Deserialize)]
pub struct TranslateRequest {
    pub url: String,
    pub target_lang: Option<String>,
    pub source_lang: Option<String>,
}

/// 翻译响应
#[cfg(feature = "web")]
#[derive(Serialize)]
pub struct TranslateResponse {
    pub original_html: String,
    pub translated_html: String,
    pub title: Option<String>,
    pub url: String,
}

/// 翻译信息结构（重新导出给其他模块使用）
#[cfg(feature = "web")]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TranslationInfo {
    /// 是否启用了翻译
    pub enabled: bool,
    /// 目标语言
    pub target_language: String,
    /// 翻译是否成功
    pub success: bool,
    /// 翻译错误信息（如果有）
    pub error: Option<String>,
}

/// 内容请求
#[cfg(feature = "web")]
#[derive(Deserialize)]
pub struct ContentRequest {
    pub url: String,
}

/// 内容响应
#[cfg(feature = "web")]
#[derive(Serialize)]
pub struct ContentResponse {
    pub status: String,
    pub message: Option<String>,
    pub redirect_to: Option<String>,
    pub url: String,
}

/// 缓存统计响应
#[cfg(feature = "web")]
#[derive(Serialize)]
pub struct CacheStatsResponse {
    pub enabled: bool,
    pub total_keys: usize,
    pub total_size_bytes: usize,
    pub expired_keys: usize,
}

/// 缓存清理请求
#[cfg(feature = "web")]
#[derive(Deserialize)]
pub struct CacheClearRequest {
    pub url: Option<String>,
    pub source_lang: Option<String>,
    pub target_lang: Option<String>,
}

/// 书签脚本响应
#[cfg(feature = "web")]
#[derive(Serialize)]
pub struct BookmarkletResponse {
    pub bookmarklet_code: String,
    pub installation_guide: String,
    pub example_usage: String,
}

/// 统一处理请求
#[cfg(feature = "web")]
#[derive(Deserialize)]
pub struct ProcessRequest {
    pub url: String,
    pub translate: Option<bool>,
    pub target_lang: Option<String>,
    pub source_lang: Option<String>,
    pub options: Option<ProcessOptions>,
}

/// 处理选项
#[cfg(feature = "web")]
#[derive(Deserialize)]
pub struct ProcessOptions {
    pub no_css: Option<bool>,
    pub no_js: Option<bool>,
    pub no_images: Option<bool>,
    pub no_fonts: Option<bool>,
    pub timeout: Option<u64>,
}

/// 统一处理响应
#[cfg(feature = "web")]
#[derive(Serialize)]
pub struct ProcessResponse {
    pub status: String,
    pub original_html: Option<String>,
    pub translated_html: Option<String>,
    pub title: Option<String>,
    pub url: String,
    pub preview_url: Option<String>,
    pub translate_enabled: bool,
    pub target_lang: Option<String>,
}

// 非 web feature 的占位类型
#[cfg(not(feature = "web"))]
#[derive(Clone)]
pub struct AppState;
