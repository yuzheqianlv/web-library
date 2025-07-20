//! Web 路由定义

#[cfg(feature = "web")]
use axum::{
    routing::{get, post},
    Router,
};

use crate::web::{handlers::*, types::AppState};
use std::sync::Arc;

/// 创建所有路由
#[cfg(feature = "web")]
pub fn create_routes() -> Router<Arc<AppState>> {
    Router::new()
        // 页面路由
        .route("/", get(index))
        .route("/library", get(library_page))
        .route("/bookmarklet", get(bookmarklet_page))
        .route("/website/*url", get(website_bookmarklet))
        
        // API 路由
        .route("/api/translate", post(translate_url))
        .route("/api/content", post(get_content))
        .route("/api/cache/stats", get(get_cache_stats))
        .route("/api/cache/clear", post(clear_cache))
        .route("/api/cache/cleanup", post(cleanup_cache))
        .route("/api/bookmarklet", get(generate_bookmarklet))
        .route("/api/library", get(get_library_data))
        .route("/api/library/action", post(library_action))
}

/// 非 web feature 的占位函数
#[cfg(not(feature = "web"))]
pub fn create_routes() -> () {
    ()
}