//! Web 路由定义

#[cfg(feature = "web")]
use axum::{
    routing::{delete, get, post},
    Router,
};

use crate::web::{
    handlers::{pages::library_debug_page, *},
    library::{
        batch_delete_library_records_v2, clear_stats_cache, delete_library_record_v2,
        download_library_record_v2, export_library_records_v2, get_cache_stats,
        get_language_distribution, get_library_record_by_id_v2, get_library_records_v2,
        get_library_stats_v2, get_stats_dashboard, get_today_stats, handlers as lib_handlers,
        initialize_v2_indexes, invalidate_stats_cache,
    },
    types::AppState,
};
use std::sync::Arc;

/// 创建所有路由
#[cfg(feature = "web")]
pub fn create_routes() -> Router<Arc<AppState>> {
    Router::new()
        // 页面路由
        .route("/", get(index))
        .route("/library", get(library_page))
        .route("/library/:id", get(view_translated_html))
        .route("/library-debug", get(library_debug_page))
        .route("/bookmarklet", get(bookmarklet_page))
        .route("/website/*url", get(website_bookmarklet))
        // 智能路由
        .route("/smart-website/*url", get(smart_website_handler))
        // API 路由
        .route("/api/translate", post(translate_url))
        .route("/api/content", post(get_content))
        .route("/api/cache/stats", get(get_cache_stats))
        .route("/api/cache/clear", post(clear_cache))
        .route("/api/cache/cleanup", post(cleanup_cache))
        .route("/api/bookmarklet", get(generate_bookmarklet))
        // 链接状态API (V2)
        .route("/api/v2/link-status", get(check_link_status))
        .route("/api/v2/link-status/batch", post(check_batch_link_status))
        .route("/api/v2/domain-links", get(get_domain_links))
        // 智能拦截器API
        .route("/api/v2/interceptor/script", get(get_interceptor_script))
        .route("/api/v2/interceptor/injection", get(get_interceptor_injection_script))
        .route("/api/v2/interceptor/bookmarklet", get(get_enhanced_bookmarklet_script))
        // Library API 路由 (V1 - Legacy)
        .route("/api/library", get(lib_handlers::get_library_data))
        .route("/api/library/stats", get(lib_handlers::get_library_stats))
        .route("/api/library/domain", get(lib_handlers::get_domain_details))
        .route("/api/library/action", post(library_action))
        .route("/api/library/batch-delete", post(batch_delete_urls))
        .route("/api/library/size-stats", get(get_size_statistics))
        .route("/api/library/health", get(health_check))
        // Library API V2 路由 (New)
        .route("/api/v2/library", get(get_library_records_v2))
        .route("/api/v2/library/stats", get(get_library_stats_v2))
        .route("/api/v2/library/:id", get(get_library_record_by_id_v2))
        .route("/api/v2/library/:id", delete(delete_library_record_v2))
        .route(
            "/api/v2/library/:id/download",
            get(download_library_record_v2),
        )
        .route(
            "/api/v2/library/batch-delete",
            delete(batch_delete_library_records_v2),
        )
        .route("/api/v2/library/export", post(export_library_records_v2))
        .route("/api/v2/library/indexes/init", post(initialize_v2_indexes))
        // 增强统计API
        .route("/api/v2/library/stats/today", get(get_today_stats))
        .route(
            "/api/v2/library/stats/languages",
            get(get_language_distribution),
        )
        .route("/api/v2/library/stats/cache", get(get_cache_stats))
        .route("/api/v2/library/stats/dashboard", get(get_stats_dashboard))
        // 缓存管理API
        .route("/api/v2/library/cache/clear", post(clear_stats_cache))
        .route(
            "/api/v2/library/cache/invalidate",
            post(invalidate_stats_cache),
        )
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
