//! Web 服务器模块
//!
//! 为 monolith 工具提供基于 Web 的翻译和内容处理服务

use std::sync::Arc;

#[cfg(feature = "web")]
use axum::{
    extract::{Json as ExtractJson, Path, State},
    http::StatusCode,
    response::{Html, Json},
    routing::{get, post},
    Router,
};

#[cfg(feature = "web")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use tokio::task;

#[cfg(feature = "web")]
use tower_http::{cors::CorsLayer, services::ServeDir};

use crate::core::{create_monolithic_document, MonolithError, MonolithOptions};
use crate::session::Session;

#[cfg(feature = "translation")]
use crate::translation::{translate_dom_content, load_translation_config};
#[cfg(feature = "translation")]
use crate::html::{html_to_dom, serialize_document};

#[cfg(feature = "web")]
use crate::redis_cache::{RedisCache, RedisCacheConfig, create_cached_translation};

#[cfg(feature = "web")]
use crate::html_builder::{HtmlBuilder, HtmlBuilderConfig};

/// Web 服务器配置
#[derive(Debug, Clone)]
pub struct WebConfig {
    /// 绑定地址
    pub bind_addr: String,
    /// 端口
    pub port: u16,
    /// 静态文件目录
    pub static_dir: Option<String>,
    /// Redis 缓存配置
    pub redis_config: Option<RedisCacheConfig>,
}

impl Default for WebConfig {
    fn default() -> Self {
        Self {
            bind_addr: "127.0.0.1".to_string(),
            port: 7080,
            static_dir: Some("static".to_string()),
            redis_config: Some(RedisCacheConfig::default()),
        }
    }
}

/// 应用状态
#[cfg(feature = "web")]
#[derive(Clone)]
pub struct AppState {
    pub monolith_options: MonolithOptions,
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

/// Web 服务器
pub struct WebServer {
    config: WebConfig,
    monolith_options: MonolithOptions,
}

impl WebServer {
    /// 创建新的 Web 服务器
    pub fn new(config: WebConfig, monolith_options: MonolithOptions) -> Self {
        Self {
            config,
            monolith_options,
        }
    }

    /// 启动 Web 服务器
    #[cfg(feature = "web")]
    pub async fn start(&self) -> Result<(), MonolithError> {
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

        let mut app = Router::new()
            .route("/", get(index))
            .route("/bookmarklet", get(bookmarklet_page))
            .route("/website/*url", get(website_bookmarklet))
            .route("/api/translate", post(translate_url))
            .route("/api/content", post(get_content))
            .route("/api/cache/stats", get(get_cache_stats))
            .route("/api/cache/clear", post(clear_cache))
            .route("/api/cache/cleanup", post(cleanup_cache))
            .route("/api/bookmarklet", get(generate_bookmarklet))
            .layer(CorsLayer::permissive())
            .with_state(app_state);

        // 添加静态文件服务（如果配置了）
        if let Some(static_dir) = &self.config.static_dir {
            app = app.nest_service("/static", ServeDir::new(static_dir));
        }

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

/// 主页处理器
#[cfg(feature = "web")]
async fn index() -> Html<String> {
    let config = HtmlBuilderConfig {
        template_dir: "templates".to_string(),
        inline_assets: true, // 内联所有资源以便单文件部署
        asset_base_path: "/".to_string(),
    };
    
    let builder = HtmlBuilder::new(config);
    match builder.build_index_page() {
        Ok(html) => Html(html),
        Err(_) => {
            // 如果构建失败，返回基础 HTML
            let fallback_html = include_str!("../templates/index.html");
            Html(fallback_html.to_string())
        }
    }
}

/// 书签脚本页面处理器
#[cfg(feature = "web")]
async fn bookmarklet_page() -> Html<String> {
    // 读取书签脚本页面模板
    let template_content = include_str!("../templates/bookmarklet.html");
    Html(template_content.to_string())
}

/// 书签脚本路由处理器 - 处理 /website/{url} 请求
#[cfg(feature = "web")]
async fn website_bookmarklet(
    Path(url_path): Path<String>,
) -> Html<String> {
    // URL 路径已经由 Axum 自动解码，去除前导斜杠
    let clean_url = url_path.trim_start_matches('/');
    
    // 验证 URL 格式
    let final_url = if clean_url.starts_with("http://") || clean_url.starts_with("https://") {
        clean_url.to_string()
    } else {
        format!("https://{}", clean_url)
    };
    
    // 构建带预加载 URL 的主页
    let config = HtmlBuilderConfig {
        template_dir: "templates".to_string(),
        inline_assets: true,
        asset_base_path: "/".to_string(),
    };
    
    let builder = HtmlBuilder::new(config);
    match builder.build_index_page_with_url(&final_url) {
        Ok(html) => Html(html),
        Err(_) => {
            // 如果构建失败，返回带 JavaScript 预加载的基础 HTML
            let fallback_html = format!(
                r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Monolith 网页翻译器 - 正在加载</title>
    <style>
        body {{ font-family: Arial, sans-serif; text-align: center; padding: 50px; }}
        .loading {{ color: #667eea; }}
    </style>
</head>
<body>
    <div class="loading">
        <h2>正在重定向到翻译页面...</h2>
        <p>目标 URL: {}</p>
    </div>
    <script>
        // 重定向到主页并预加载 URL
        const url = "{}";
        const targetUrl = "/?preload_url=" + encodeURIComponent(url);
        window.location.href = targetUrl;
    </script>
</body>
</html>"#,
                final_url, final_url
            );
            Html(fallback_html)
        }
    }
}

/// 生成书签脚本
#[cfg(feature = "web")]
async fn generate_bookmarklet(
    State(state): State<Arc<AppState>>,
) -> Result<Json<BookmarkletResponse>, (StatusCode, Json<serde_json::Value>)> {
    // 获取当前服务器地址（这里简化处理，实际应用中可能需要从请求头获取）
    let server_url = "http://localhost:7080"; // 可以从配置中获取
    
    let bookmarklet_code = format!(
        r#"javascript:(function(){{
    var url = encodeURIComponent(window.location.href);
    var targetUrl = '{}/website/' + url;
    window.open(targetUrl, '_blank');
}})();"#,
        server_url
    );
    
    let installation_guide = r#"
## 书签脚本安装指南

### Chrome/Edge 浏览器:
1. 右键点击书签栏，选择"添加页面"
2. 名称填写: "翻译当前页面"
3. 网址填写下方的书签脚本代码
4. 点击保存

### Firefox 浏览器:
1. 右键点击书签栏，选择"新书签"
2. 名称填写: "翻译当前页面"
3. 位置填写下方的书签脚本代码
4. 点击添加

### Safari 浏览器:
1. 将下方代码复制到剪贴板
2. 添加任意页面为书签
3. 编辑书签，将 URL 替换为书签脚本代码
"#.to_string();
    
    let example_usage = r#"
## 使用方法

1. 在任意网页上点击"翻译当前页面"书签
2. 系统会在新标签页中打开翻译界面
3. 当前页面的 URL 会自动加载并开始翻译
4. 支持原文/译文/双语对照模式切换

## 技术说明

- 书签脚本会获取当前页面的 URL
- 自动跳转到 Monolith 翻译服务
- 无需手动复制粘贴 URL
- 支持所有主流浏览器
"#.to_string();
    
    let response = BookmarkletResponse {
        bookmarklet_code,
        installation_guide,
        example_usage,
    };
    
    Ok(Json(response))
}

/// 翻译 URL 处理器
#[cfg(feature = "web")]
async fn translate_url(
    State(state): State<Arc<AppState>>,
    ExtractJson(request): ExtractJson<TranslateRequest>,
) -> Result<Json<TranslateResponse>, (StatusCode, Json<serde_json::Value>)> {
    let url = request.url.clone();
    let target_lang = request.target_lang.unwrap_or_else(|| "zh".to_string());
    let source_lang = request.source_lang.unwrap_or_else(|| "auto".to_string());

    // 检查缓存
    if let Some(ref cache) = state.redis_cache {
        if let Ok(Some(cached)) = cache.get(&url, &source_lang, &target_lang) {
            let response = TranslateResponse {
                original_html: cached.original_html,
                translated_html: cached.translated_html,
                title: cached.title,
                url: cached.url,
            };
            return Ok(Json(response));
        }
    }

    // 克隆选项用于并行处理
    let options_original = state.monolith_options.clone();
    let options_translated = state.monolith_options.clone();

    // 并行处理原文和译文
    let original_future = {
        let url = url.clone();
        let options = options_original;
        task::spawn_blocking(move || {
            let session = Session::new(None, None, options);
            create_monolithic_document(session, url)
        })
    };

    let translated_future = {
        let url = url.clone();
        let options = options_translated;
        let target_lang = target_lang.clone();
        task::spawn_blocking(move || -> Result<(Vec<u8>, Option<String>), MonolithError> {
            // 先获取原始内容
            let session = Session::new(None, None, options.clone());
            let original_result = create_monolithic_document(session, url.clone())?;

            // 如果启用了翻译功能，进行翻译
            #[cfg(feature = "translation")]
            {
                let (original_data, title) = original_result;
                let dom = html_to_dom(&original_data, url);
                
                // 加载翻译配置，从配置文件加载 API URL
                let translation_config = load_translation_config(&target_lang, None);
                
                // 翻译需要在异步上下文中运行，这里简化处理
                // 在实际应用中，您可能需要使用异步翻译或者同步包装
                let translated_dom = tokio::runtime::Handle::current()
                    .block_on(translate_dom_content(dom, &target_lang, Some(&translation_config.deeplx_api_url)))
                    .map_err(|e| MonolithError::new(&format!("Translation error: {}", e)))?;
                
                let translated_data = serialize_document(translated_dom, "UTF-8".to_string(), &options);
                Ok((translated_data, title))
            }
            
            #[cfg(not(feature = "translation"))]
            Ok(original_result)
        })
    };

    // 等待两个任务完成
    let (original_result, translated_result) = match tokio::try_join!(original_future, translated_future) {
        Ok((original, translated)) => (original, translated),
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": true,
                    "message": format!("Task execution error: {}", e)
                }))
            ));
        }
    };

    let (original_data, original_title) = match original_result {
        Ok(result) => result,
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": true,
                    "message": format!("Failed to process original: {}", e)
                }))
            ));
        }
    };

    let (translated_data, translated_title) = match translated_result {
        Ok(result) => result,
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": true,
                    "message": format!("Failed to process translation: {}", e)
                }))
            ));
        }
    };

    let original_html = String::from_utf8_lossy(&original_data).to_string();
    let translated_html = String::from_utf8_lossy(&translated_data).to_string();

    // 使用标题
    let title = translated_title.or(original_title);

    // 缓存结果
    if let Some(ref cache) = state.redis_cache {
        let cached_translation = create_cached_translation(
            url,
            original_html.clone(),
            translated_html.clone(),
            title.clone(),
            source_lang,
            target_lang,
            None, // 使用默认 TTL
        );
        
        if let Err(e) = cache.set(&cached_translation) {
            eprintln!("警告: 缓存存储失败: {}", e);
        }
    }

    let response = TranslateResponse {
        original_html,
        translated_html,
        title,
        url: request.url,
    };

    Ok(Json(response))
}

/// 获取内容处理器
#[cfg(feature = "web")]
async fn get_content(
    State(state): State<Arc<AppState>>,
    ExtractJson(request): ExtractJson<ContentRequest>,
) -> Result<Json<ContentResponse>, (StatusCode, Json<serde_json::Value>)> {
    let url = request.url.clone();
    let options = state.monolith_options.clone();

    let (data, title) = task::spawn_blocking(move || {
        let session = Session::new(None, None, options);
        create_monolithic_document(session, url)
    })
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": true,
                "message": format!("Task execution error: {}", e)
            }))
        )
    })?
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": true,
                "message": format!("Failed to process content: {}", e)
            }))
        )
    })?;

    let html = String::from_utf8_lossy(&data).to_string();

    let response = ContentResponse {
        html,
        title,
        url: request.url,
    };

    Ok(Json(response))
}

/// 从 HTML 中提取标题
fn extract_title(html: &str) -> Option<String> {
    // 简单的标题提取，使用正则表达式
    use regex::Regex;
    
    let re = Regex::new(r"<title[^>]*>([^<]+)</title>").ok()?;
    let caps = re.captures(html)?;
    let title = caps.get(1)?.as_str().trim();
    
    if title.is_empty() {
        None
    } else {
        Some(title.to_string())
    }
}

/// 获取缓存统计信息
#[cfg(feature = "web")]
async fn get_cache_stats(
    State(state): State<Arc<AppState>>,
) -> Result<Json<CacheStatsResponse>, (StatusCode, Json<serde_json::Value>)> {
    if let Some(ref cache) = state.redis_cache {
        match cache.get_stats() {
            Ok(stats) => {
                let response = CacheStatsResponse {
                    enabled: true,
                    total_keys: stats.total_keys,
                    total_size_bytes: stats.total_size_bytes,
                    expired_keys: stats.expired_keys,
                };
                Ok(Json(response))
            }
            Err(e) => {
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "error": true,
                        "message": format!("获取缓存统计失败: {}", e)
                    }))
                ))
            }
        }
    } else {
        let response = CacheStatsResponse {
            enabled: false,
            total_keys: 0,
            total_size_bytes: 0,
            expired_keys: 0,
        };
        Ok(Json(response))
    }
}

/// 清理缓存
#[cfg(feature = "web")]
async fn clear_cache(
    State(state): State<Arc<AppState>>,
    ExtractJson(request): ExtractJson<CacheClearRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    if let Some(ref cache) = state.redis_cache {
        let result = if let (Some(url), Some(source_lang), Some(target_lang)) = 
            (request.url, request.source_lang, request.target_lang) {
            // 清理特定缓存条目
            cache.delete(&url, &source_lang, &target_lang)
                .map(|deleted| if deleted { 1 } else { 0 })
        } else {
            // 清理所有缓存
            cache.clear_all()
        };

        match result {
            Ok(deleted_count) => {
                Ok(Json(serde_json::json!({
                    "success": true,
                    "message": format!("成功清理 {} 个缓存条目", deleted_count),
                    "deleted_count": deleted_count
                })))
            }
            Err(e) => {
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "error": true,
                        "message": format!("清理缓存失败: {}", e)
                    }))
                ))
            }
        }
    } else {
        Ok(Json(serde_json::json!({
            "success": false,
            "message": "缓存未启用"
        })))
    }
}

/// 清理过期缓存
#[cfg(feature = "web")]
async fn cleanup_cache(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    if let Some(ref cache) = state.redis_cache {
        match cache.cleanup_expired() {
            Ok(deleted_count) => {
                Ok(Json(serde_json::json!({
                    "success": true,
                    "message": format!("成功清理 {} 个过期缓存条目", deleted_count),
                    "deleted_count": deleted_count
                })))
            }
            Err(e) => {
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "error": true,
                        "message": format!("清理过期缓存失败: {}", e)
                    }))
                ))
            }
        }
    } else {
        Ok(Json(serde_json::json!({
            "success": false,
            "message": "缓存未启用"
        })))
    }
}

/// 非 web feature 的占位函数
#[cfg(not(feature = "web"))]
async fn index() -> Result<String, String> {
    Err("Web feature not enabled".to_string())
}

#[cfg(not(feature = "web"))]
async fn translate_url() -> Result<String, String> {
    Err("Web feature not enabled".to_string())
}

#[cfg(not(feature = "web"))]
async fn get_content() -> Result<String, String> {
    Err("Web feature not enabled".to_string())
}

#[cfg(not(feature = "web"))]
async fn get_cache_stats() -> Result<String, String> {
    Err("Web feature not enabled".to_string())
}

#[cfg(not(feature = "web"))]
async fn clear_cache() -> Result<String, String> {
    Err("Web feature not enabled".to_string())
}

#[cfg(not(feature = "web"))]
async fn cleanup_cache() -> Result<String, String> {
    Err("Web feature not enabled".to_string())
}