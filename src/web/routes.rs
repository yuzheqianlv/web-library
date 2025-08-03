//! Web 路由定义

#[cfg(feature = "web")]
use axum::{
    routing::{get, post},
    Router,
};

use crate::web::{
    handlers::*,
    types::AppState,
};
use std::sync::Arc;

/// 创建简化的路由结构
#[cfg(feature = "web")]
pub fn create_routes() -> Router<Arc<AppState>> {
    Router::new()
        // 核心页面路由
        .route("/", get(index))
        .route("/preview/*url", get(preview_handler))
        // 统一的API端点 - 使用process作为主要处理入口
        .route("/api/process", post(process_url))
        .route("/api/bookmarklet", get(generate_bookmarklet))
        // 基础管理功能
        .route("/api/cache/clear", post(clear_cache))
        // 向后兼容的API别名（已弃用，建议使用/api/process）
        .route("/api/translate", post(translate_url_deprecated))
        .route("/api/content", post(get_content_deprecated))
}

/// 非 web feature 的占位函数
#[cfg(not(feature = "web"))]
pub fn create_routes() -> () {
    ()
}
