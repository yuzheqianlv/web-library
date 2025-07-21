//! Web 服务器模块
//!
//! 为 monolith 工具提供基于 Web 的翻译和内容处理服务

pub mod config;
pub mod handlers;
pub mod routes;
pub mod templates;
pub mod types;

pub use config::*;
pub use handlers::*;
pub use routes::*;
pub use types::*;

use std::sync::Arc;

#[cfg(feature = "web")]
use axum::Router;
#[cfg(feature = "web")]
use tower_http::{cors::CorsLayer, services::ServeDir};

use crate::core::MonolithError;

/// Web 服务器
pub struct WebServer {
    config: WebConfig,
    monolith_options: crate::core::MonolithOptions,
}

impl WebServer {
    /// 创建新的 Web 服务器
    pub fn new(config: WebConfig, monolith_options: crate::core::MonolithOptions) -> Self {
        Self {
            config,
            monolith_options,
        }
    }

    /// 启动 Web 服务器
    #[cfg(feature = "web")]
    pub async fn start(&self) -> Result<(), MonolithError> {
        use crate::redis_cache::RedisCache;

        // 初始化 Redis 缓存
        let redis_cache = if let Some(ref redis_config) = self.config.redis_config {
            match RedisCache::new(redis_config.clone()) {
                Ok(cache) => {
                    match cache.test_connection() {
                        Ok(_) => {
                            println!("Redis 缓存连接成功: {}", redis_config.url);
                            Some(cache)
                        }
                        Err(e) => {
                            eprintln!("警告: Redis 连接失败: {}", e);
                            eprintln!("继续运行，但缓存功能将不可用");
                            None
                        }
                    }
                }
                Err(e) => {
                    eprintln!("警告: 无法初始化 Redis 缓存: {}", e);
                    eprintln!("继续运行，但缓存功能将不可用");
                    None
                }
            }
        } else {
            println!("未配置 Redis 缓存");
            None
        };

        let app_state = Arc::new(AppState {
            monolith_options: self.monolith_options.clone(),
            redis_cache,
        });

        let app = create_router(app_state, &self.config);

        let listener = tokio::net::TcpListener::bind(format!(
            "{}:{}",
            self.config.bind_addr, self.config.port
        ))
        .await
        .map_err(|e| MonolithError::new(&format!("Failed to bind server: {}", e)))?;

        println!(
            "Web server starting at http://{}:{}",
            self.config.bind_addr, self.config.port
        );

        axum::serve(listener, app)
            .await
            .map_err(|e| MonolithError::new(&format!("Server error: {}", e)))?;

        Ok(())
    }

    /// 启动 Web 服务器（非 web feature 版本）
    #[cfg(not(feature = "web"))]
    pub async fn start(&self) -> Result<(), MonolithError> {
        Err(MonolithError::new("Web feature not enabled"))
    }
}

/// 创建路由器
#[cfg(feature = "web")]
fn create_router(app_state: Arc<AppState>, config: &WebConfig) -> Router {
    let mut app = create_routes().with_state(app_state);

    // 添加CORS支持
    app = app.layer(CorsLayer::permissive());

    // 添加静态文件服务（如果配置了）
    if let Some(static_dir) = &config.static_dir {
        app = app.nest_service("/static", ServeDir::new(static_dir));
    }

    app
}