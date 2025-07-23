//! 页面处理器

#[cfg(feature = "web")]
use axum::{extract::Path, response::Html, http::StatusCode, extract::State};

use crate::web::{templates, types::AppState};
use std::sync::Arc;

/// 主页处理器
#[cfg(feature = "web")]
pub async fn index() -> Html<String> {
    let config = crate::builders::html_builder::HtmlBuilderConfig {
        template_dir: "templates".to_string(),
        inline_assets: true, // 内联所有资源以便单文件部署
        asset_base_path: "/".to_string(),
    };

    let builder = crate::builders::html_builder::HtmlBuilder::new(config);
    match builder.build_index_page() {
        Ok(html) => Html(html),
        Err(_) => {
            // 如果构建失败，返回基础 HTML
            let fallback_html = include_str!("../../../templates/index.html");
            Html(fallback_html.to_string())
        }
    }
}

/// 库管理页面处理器
#[cfg(feature = "web")]
pub async fn library_page() -> Html<String> {
    Html(templates::library_page())
}

/// 库调试页面处理器
#[cfg(feature = "web")]
pub async fn library_debug_page() -> Html<String> {
    let debug_html = include_str!("../../../templates/library-debug.html");
    Html(debug_html.to_string())
}

/// 书签脚本页面处理器
#[cfg(feature = "web")]
pub async fn bookmarklet_page() -> Html<String> {
    // 读取书签脚本页面模板
    let template_content = include_str!("../../../templates/bookmarklet.html");
    Html(template_content.to_string())
}

/// 书签脚本路由处理器 - 处理 /website/{url} 请求
#[cfg(feature = "web")]
pub async fn website_bookmarklet(
    Path(url_path): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Html<String> {
    // URL 路径已经由 Axum 自动解码，去除前导斜杠
    let clean_url = url_path.trim_start_matches('/');

    // 验证 URL 格式
    let final_url = if clean_url.starts_with("http://") || clean_url.starts_with("https://") {
        clean_url.to_string()
    } else {
        format!("https://{}", clean_url)
    };

    // 首先检查缓存中是否有翻译后的内容
    if let Some(cached_html) = check_translated_cache(&state, &final_url).await {
        println!("找到缓存的翻译内容，直接返回");
        return Html(cached_html);
    }

    // 如果没有缓存，构建带预加载 URL 的主页
    let config = crate::builders::html_builder::HtmlBuilderConfig {
        template_dir: "templates".to_string(),
        inline_assets: true,
        asset_base_path: "/".to_string(),
    };

    let builder = crate::builders::html_builder::HtmlBuilder::new(config);
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

/// 查看翻译后HTML内容的处理器
#[cfg(feature = "web")]
pub async fn view_translated_html(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<Html<String>, StatusCode> {
    // 从数据库获取记录
    match get_html_from_cache(state.mongo_client.as_ref(), &id).await {
        Ok(Some(html_content)) => {
            // 直接返回翻译后的HTML内容
            Ok(Html(html_content))
        }
        Ok(None) => {
            // 记录不存在
            Err(StatusCode::NOT_FOUND)
        }
        Err(_) => {
            // 数据库错误
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 从缓存中获取HTML内容
#[cfg(feature = "web")]
async fn get_html_from_cache(
    mongodb_client: Option<&mongodb::Client>,
    id: &str,
) -> Result<Option<String>, mongodb::error::Error> {
    use mongodb::bson::{doc, oid::ObjectId};
    
    // 检查MongoDB客户端是否可用
    let mongodb_client = match mongodb_client {
        Some(client) => client,
        None => return Ok(None),
    };
    
    // 解析ObjectId
    let object_id = ObjectId::parse_str(id).map_err(|_| {
        mongodb::error::Error::custom("Invalid ObjectId format".to_string())
    })?;
    
    let db = mongodb_client.database("monolith");
    let collection = db.collection::<mongodb::bson::Document>("html_cache");
    
    // 查询记录
    let filter = doc! { "_id": object_id };
    if let Some(document) = collection.find_one(filter).await? {
        // 优先返回翻译后的HTML，如果没有则返回原始HTML
        if let Some(translated_html) = document.get_str("translated_html").ok() {
            return Ok(Some(translated_html.to_string()));
        } else if let Some(original_html) = document.get_str("original_html").ok() {
            return Ok(Some(original_html.to_string()));
        }
    }
    
    Ok(None)
}

/// 根据URL检查缓存中的翻译内容
#[cfg(feature = "web")]
async fn check_translated_cache(state: &AppState, url: &str) -> Option<String> {
    use mongodb::bson::doc;
    
    // 检查MongoDB客户端是否可用
    let mongodb_client = state.mongo_client.as_ref()?;
    
    let db = mongodb_client.database("monolith");
    let collection = db.collection::<mongodb::bson::Document>("html_cache");
    
    // 查询该URL的翻译记录
    let filter = doc! { "url": url };
    
    if let Ok(Some(document)) = collection.find_one(filter).await {
        // 优先返回翻译后的HTML
        if let Ok(translated_html) = document.get_str("translated_html") {
            if !translated_html.is_empty() {
                return Some(translated_html.to_string());
            }
        }
    }
    
    None
}

/// 查看缓存HTML内容的处理器 - 用于预览路由
#[cfg(feature = "web")]
pub async fn view_cached_html(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<Html<String>, StatusCode> {
    // 从数据库获取翻译后的HTML内容
    match get_html_from_cache(state.mongo_client.as_ref(), &id).await {
        Ok(Some(html_content)) => {
            // 直接返回翻译后的HTML内容，让浏览器渲染
            Ok(Html(html_content))
        }
        Ok(None) => {
            // 记录不存在，返回404页面
            let error_html = format!(
                r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>页面未找到 - Monolith</title>
    <style>
        body {{ 
            font-family: Arial, sans-serif; 
            text-align: center; 
            padding: 50px; 
            background: #f8f9fa;
        }}
        .error-container {{
            max-width: 500px;
            margin: 0 auto;
            background: white;
            padding: 2rem;
            border-radius: 8px;
            box-shadow: 0 2px 8px rgba(0,0,0,0.1);
        }}
        .error-code {{ 
            font-size: 3rem; 
            color: #dc3545; 
            margin-bottom: 1rem;
        }}
        .error-message {{ 
            color: #6c757d; 
            margin-bottom: 2rem;
        }}
        .back-btn {{
            background: #007bff;
            color: white;
            border: none;
            padding: 0.75rem 1.5rem;
            border-radius: 4px;
            text-decoration: none;
            display: inline-block;
        }}
    </style>
</head>
<body>
    <div class="error-container">
        <div class="error-code">404</div>
        <h2>页面未找到</h2>
        <p class="error-message">缓存ID: {}</p>
        <p class="error-message">该翻译记录不存在或已被删除</p>
        <a href="/library" class="back-btn">返回翻译库</a>
    </div>
</body>
</html>"#,
                id
            );
            Err(StatusCode::NOT_FOUND)
        }
        Err(_) => {
            // 数据库错误
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 非 web feature 的占位函数
#[cfg(not(feature = "web"))]
pub async fn index() -> Result<String, String> {
    Err("Web feature not enabled".to_string())
}

#[cfg(not(feature = "web"))]
pub async fn library_page() -> Result<String, String> {
    Err("Web feature not enabled".to_string())
}

#[cfg(not(feature = "web"))]
pub async fn bookmarklet_page() -> Result<String, String> {
    Err("Web feature not enabled".to_string())
}

#[cfg(not(feature = "web"))]
pub async fn website_bookmarklet() -> Result<String, String> {
    Err("Web feature not enabled".to_string())
}
