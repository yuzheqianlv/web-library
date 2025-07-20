//! 页面处理器

use std::sync::Arc;

#[cfg(feature = "web")]
use axum::{
    extract::Path,
    response::Html,
};

use crate::web::{templates, types::AppState};

/// 主页处理器
#[cfg(feature = "web")]
pub async fn index() -> Html<String> {
    let config = crate::html_builder::HtmlBuilderConfig {
        template_dir: "templates".to_string(),
        inline_assets: true, // 内联所有资源以便单文件部署
        asset_base_path: "/".to_string(),
    };
    
    let builder = crate::html_builder::HtmlBuilder::new(config);
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

/// 书签脚本页面处理器
#[cfg(feature = "web")]
pub async fn bookmarklet_page() -> Html<String> {
    // 读取书签脚本页面模板
    let template_content = include_str!("../../../templates/bookmarklet.html");
    Html(template_content.to_string())
}

/// 书签脚本路由处理器 - 处理 /website/{url} 请求
#[cfg(feature = "web")]
pub async fn website_bookmarklet(Path(url_path): Path<String>) -> Html<String> {
    // URL 路径已经由 Axum 自动解码，去除前导斜杠
    let clean_url = url_path.trim_start_matches('/');
    
    // 验证 URL 格式
    let final_url = if clean_url.starts_with("http://") || clean_url.starts_with("https://") {
        clean_url.to_string()
    } else {
        format!("https://{}", clean_url)
    };
    
    // 构建带预加载 URL 的主页
    let config = crate::html_builder::HtmlBuilderConfig {
        template_dir: "templates".to_string(),
        inline_assets: true,
        asset_base_path: "/".to_string(),
    };
    
    let builder = crate::html_builder::HtmlBuilder::new(config);
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