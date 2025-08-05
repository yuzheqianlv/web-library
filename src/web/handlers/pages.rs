//! 页面处理器

#[cfg(feature = "web")]
use axum::{extract::Path, extract::State, response::Html};

use crate::web::types::AppState;
use std::sync::Arc;

/// 主页处理器 - 使用完整的index.html模板
#[cfg(feature = "web")]
pub async fn index() -> Html<String> {
    // 使用完整功能的index.html模板
    let index_html = include_str!("../../../templates/core/index.html");
    Html(index_html.to_string())
}

// 库管理页面已被删除 - 轻量化版本不再支持

// 书签脚本和website处理器已被删除 - 简化为直接预览模式

/// 增强的预览处理器 - 直接重定向到主页面并自动处理URL
#[cfg(feature = "web")]
pub async fn preview_handler(
    Path(url_path): Path<String>,
    State(_state): State<Arc<AppState>>,
) -> Result<Html<String>, (axum::http::StatusCode, Html<String>)> {
    // URL 路径已经由 Axum 自动解码，去除前导斜杠
    let clean_url = url_path.trim_start_matches('/');

    // 验证 URL 格式
    let final_url = if clean_url.starts_with("http://") || clean_url.starts_with("https://") {
        clean_url.to_string()
    } else {
        format!("https://{}", clean_url)
    };

    tracing::info!("预览处理URL: {}", final_url);

    // 创建预览页面，使用index.html模板并自动加载URL
    let preview_html = create_enhanced_preview_page(&final_url);
    Ok(Html(preview_html))
}

/// 创建增强的预览页面 - 基于完整的index.html模板
#[cfg(feature = "web")]
fn create_enhanced_preview_page(url: &str) -> String {
    // 读取完整的index.html模板
    let mut template = include_str!("../../../templates/core/index.html").to_string();
    
    // 在模板中注入自动处理URL的JavaScript
    let auto_process_script = format!(
        r#"
        <script>
            // 自动处理预览URL
            document.addEventListener('DOMContentLoaded', () => {{
                console.log('🔄 预览模式：自动处理URL');
                const urlInput = document.getElementById('url-input');
                if (urlInput) {{
                    urlInput.value = '{}';
                    // 自动触发翻译处理
                    setTimeout(() => {{
                        if (window.monolithApp) {{
                            window.monolithApp.processUrl();
                        }}
                    }}, 1000); // 延迟1秒以确保所有脚本加载完成
                }}
            }});
        </script>
        </body>"#,
        url.replace('"', "&quot;").replace('\\', "\\\\")
    );
    
    // 替换</body>标签以注入自动处理脚本
    template = template.replace("</body>", &auto_process_script);
    
    template
}

/// 创建简单预览页面HTML（备用方案，现已弃用）
#[cfg(feature = "web")]
fn _create_legacy_preview_page(url: &str, title: &str, content: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{} - Monolith 预览</title>
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            margin: 0;
            padding: 0;
            background: #f5f5f5;
        }}
        .header {{
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
            padding: 1rem 2rem;
            box-shadow: 0 2px 10px rgba(0,0,0,0.1);
        }}
        .header h1 {{
            margin: 0;
            font-size: 1.5rem;
            font-weight: 600;
        }}
        .header .url {{
            margin: 0.5rem 0 0 0;
            font-size: 0.9rem;
            opacity: 0.9;
            font-family: monospace;
            word-break: break-all;
        }}
        .nav-bar {{
            background: white;
            border-bottom: 1px solid #e0e0e0;
            padding: 0.5rem 2rem;
            display: flex;
            align-items: center;
            gap: 1rem;
        }}
        .nav-btn {{
            background: #667eea;
            color: white;
            border: none;
            padding: 0.5rem 1rem;
            border-radius: 6px;
            text-decoration: none;
            font-size: 0.9rem;
            transition: background-color 0.2s;
        }}
        .nav-btn:hover {{
            background: #5a67d8;
        }}
        .content-container {{
            background: white;
            margin: 1rem 2rem;
            border-radius: 8px;
            box-shadow: 0 2px 10px rgba(0,0,0,0.1);
            overflow: hidden;
        }}
        .content-frame {{
            width: 100%;
            min-height: 80vh;
            border: none;
        }}
        @media (max-width: 768px) {{
            .header, .nav-bar, .content-container {{
                margin: 0;
                border-radius: 0;
            }}
        }}
    </style>
</head>
<body>
    <div class="header">
        <h1>🚀 Monolith 网页预览</h1>
        <div class="url">📄 {}</div>
    </div>
    
    <div class="nav-bar">
        <a href="/" class="nav-btn">🏠 返回首页</a>
        <a href="/preview/{}" class="nav-btn">🔄 重新处理</a>
    </div>
    
    <div class="content-container">
        <iframe class="content-frame" srcdoc="{}" sandbox="allow-same-origin allow-scripts allow-forms allow-popups allow-popups-to-escape-sandbox">
            您的浏览器不支持iframe显示。
        </iframe>
    </div>
    
    <script>
        // 自动调整iframe高度
        const iframe = document.querySelector('.content-frame');
        iframe.onload = function() {{
            try {{
                const doc = iframe.contentDocument || iframe.contentWindow.document;
                const height = Math.max(doc.body.scrollHeight, doc.body.offsetHeight, 
                                      doc.documentElement.clientHeight, doc.documentElement.scrollHeight, 
                                      doc.documentElement.offsetHeight);
                iframe.style.height = Math.max(height, 600) + 'px';
            }} catch(e) {{
                // 跨域限制，使用默认高度
                iframe.style.height = '80vh';
            }}
        }};
    </script>
</body>
</html>"#,
        title, url, url, content.replace('"', "&quot;").replace('\n', "")
    )
}

// 创建错误页面HTML（备用方案，已移除以避免死代码警告）

// 库管理和缓存查看功能已被删除 - 轻量化版本专注核心功能

/// 非 web feature 的占位函数
#[cfg(not(feature = "web"))]
pub async fn index() -> Result<String, String> {
    Err("Web feature not enabled".to_string())
}
