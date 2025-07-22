//! Web 路由定义

#[cfg(feature = "web")]
use axum::{
    routing::{get, post},
    Router,
};

use crate::web::{handlers::*, library::handlers as lib_handlers, types::AppState};
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
        // Library API 路由
        .route("/api/library", get(lib_handlers::get_library_data))
        .route("/api/library/stats", get(lib_handlers::get_library_stats))
        .route("/api/library/domain", get(lib_handlers::get_domain_details))
        .route("/api/library/action", post(library_action))
        .route("/api/library/batch-delete", post(batch_delete_urls))
        .route("/api/library/size-stats", get(get_size_statistics))
        .route("/api/library/health", get(health_check))
        // Theme API 路由
        .route("/api/theme/list", get(get_themes))
        .route("/api/theme/current", get(get_current_theme_css))
        .route("/api/theme/set", post(set_theme))
        .route("/api/theme/css/:theme", get(get_theme_css))
        .route("/api/theme/preview/:theme", get(preview_theme))
        .route("/api/theme/selector", get(get_theme_selector))
        .route("/api/theme/script", get(get_theme_script))
        .route("/api/theme/register", post(register_theme))
        .route("/api/theme/delete/:theme", post(delete_theme))
        .route("/api/theme/preference", get(get_user_preference))
        .route("/api/theme/preference", post(set_user_preference))
        .route("/api/theme/auto-switch", post(auto_switch_theme))
}

/// 非 web feature 的占位函数
#[cfg(not(feature = "web"))]
pub fn create_routes() -> () {
    ()
}
