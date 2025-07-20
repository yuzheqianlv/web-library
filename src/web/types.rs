//! Web 模块的数据类型定义

#[cfg(feature = "web")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::redis_cache::RedisCache;

/// 应用状态
#[cfg(feature = "web")]
#[derive(Clone)]
pub struct AppState {
    pub monolith_options: crate::core::MonolithOptions,
    pub redis_cache: Option<RedisCache>,
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

/// 库域名分类请求
#[cfg(feature = "web")]
#[derive(Deserialize)]
pub struct LibraryRequest {
    pub domain: Option<String>,
    pub page: Option<usize>,
    pub limit: Option<usize>,
}

/// 库域名分类响应
#[cfg(feature = "web")]
#[derive(Serialize)]
pub struct LibraryResponse {
    pub domains: Vec<LibraryDomain>,
    pub total_domains: usize,
    pub page: usize,
    pub limit: usize,
}

/// 域名库条目
#[cfg(feature = "web")]
#[derive(Serialize)]
pub struct LibraryDomain {
    pub domain: String,
    pub cache_count: usize,
    pub total_size_bytes: usize,
    pub last_updated: u64,
    pub urls: Vec<LibraryUrl>,
}

/// 域名下的URL条目
#[cfg(feature = "web")]
#[derive(Serialize)]
pub struct LibraryUrl {
    pub url: String,
    pub title: Option<String>,
    pub source_lang: String,
    pub target_lang: String,
    pub created_at: u64,
    pub expires_at: u64,
    pub size_bytes: usize,
}

/// 库操作请求
#[cfg(feature = "web")]
#[derive(Deserialize)]
pub struct LibraryActionRequest {
    pub action: String, // "delete_domain", "delete_url", "refresh_domain"
    pub domain: Option<String>,
    pub url: Option<String>,
    pub source_lang: Option<String>,
    pub target_lang: Option<String>,
}

// 非 web feature 的占位类型
#[cfg(not(feature = "web"))]
#[derive(Clone)]
pub struct AppState;