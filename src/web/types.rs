//! Web 模块的数据类型定义

#[cfg(feature = "web")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use mongodb::{Client as MongoClient, Collection, Database};

/// 存储的HTML数据结构
#[cfg(feature = "web")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedHtml {
    pub url: String,
    pub original_html: String,
    pub translated_html: String,
    pub title: Option<String>,
    pub source_lang: String,
    pub target_lang: String,
    pub created_at: bson::DateTime,
}

/// 应用状态
#[cfg(feature = "web")]
#[derive(Clone)]
pub struct AppState {
    pub monolith_options: crate::core::MonolithOptions,
    pub mongo_client: Option<MongoClient>,
    pub mongo_collection: Option<Collection<CachedHtml>>,
    pub mongo_database: Option<Database>,
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
    pub html: String,
    pub title: Option<String>,
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

// 非 web feature 的占位类型
#[cfg(not(feature = "web"))]
#[derive(Clone)]
pub struct AppState;
