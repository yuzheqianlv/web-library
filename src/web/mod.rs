//! Web 服务器模块
//!
//! 为 monolith 工具提供基于 Web 的翻译和内容处理服务

pub mod config;
pub mod handlers;
pub mod library;
pub mod routes;
pub mod services;
pub mod templates;
pub mod theme;
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
        use crate::web::types::CachedHtml;
        use mongodb::Client as MongoClient;

        // 初始化 MongoDB 连接
        let (mongo_client, mongo_collection, mongo_database) =
            if let Some(ref mongo_config) = self.config.mongo_config {
                match MongoClient::with_uri_str(&mongo_config.connection_string).await {
                    Ok(client) => {
                        let database = client.database(&mongo_config.database_name);
                        let collection =
                            database.collection::<CachedHtml>(&mongo_config.collection_name);

                        // 测试连接
                        match client.list_database_names().await {
                            Ok(_) => {
                                println!("MongoDB 连接成功: {}", mongo_config.connection_string);
                                (Some(client), Some(collection), Some(database))
                            }
                            Err(e) => {
                                eprintln!("警告: MongoDB 连接失败: {}", e);
                                eprintln!("继续运行，但缓存功能将不可用");
                                (None, None, None)
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("警告: 无法初始化 MongoDB 客户端: {}", e);
                        eprintln!("继续运行，但缓存功能将不可用");
                        (None, None, None)
                    }
                }
            } else {
                println!("未配置 MongoDB 缓存");
                (None, None, None)
            };

        // 初始化主题管理器
        let theme_manager = std::sync::Mutex::new(theme::ThemeManager::new());

        let app_state = Arc::new(AppState {
            monolith_options: self.monolith_options.clone(),
            mongo_client,
            mongo_collection,
            mongo_database,
            theme_manager: Arc::new(theme_manager),
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

    // 添加templates目录的静态文件服务
    app = app.nest_service("/templates", ServeDir::new("templates"));

    // 添加assets目录的静态文件服务（用于CSS/JS等）
    app = app.nest_service("/assets", ServeDir::new("templates/assets"));

    app
}
