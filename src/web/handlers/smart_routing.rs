//! 智能链接路由处理器
//!
//! 提供基于缓存状态的智能路由决策和页面生成功能

#[cfg(feature = "web")]
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
};

#[cfg(feature = "web")]
use serde::Deserialize;

#[cfg(feature = "web")]
use crate::web::{
    services::cache_query_engine::{CacheQueryEngine, RoutingStrategy},
    types::AppState,
};

/// 智能网站路由查询参数
#[cfg(feature = "web")]
#[derive(Debug, Deserialize)]
pub struct SmartRouteQuery {
    /// 源语言
    pub source_lang: Option<String>,
    /// 目标语言
    pub target_lang: Option<String>,
    /// 是否强制重新处理
    pub force_reprocess: Option<bool>,
    /// 是否显示调试信息
    pub debug: Option<bool>,
}

/// 智能网站路由处理器
/// 
/// 这是核心的智能路由处理器，根据缓存状态智能决策如何处理链接请求
#[cfg(feature = "web")]
pub async fn smart_website_handler(
    State(state): State<std::sync::Arc<AppState>>,
    Path(url): Path<String>,
    Query(params): Query<SmartRouteQuery>,
) -> Result<Response, StatusCode> {
    // 构建完整的URL（处理路径参数中的URL）
    let target_url = if url.starts_with("http://") || url.starts_with("https://") {
        url
    } else {
        format!("https://{}", url)
    };

    // 获取语言参数
    let source_lang = params.source_lang.as_deref();
    let target_lang = params.target_lang.as_deref();
    let force_reprocess = params.force_reprocess.unwrap_or(false);
    let debug_mode = params.debug.unwrap_or(false);

    // 如果强制重新处理，直接跳转到翻译API
    if force_reprocess {
        return Ok(redirect_to_translation_api(&target_url, source_lang, target_lang).into_response());
    }

    // 检查是否有MongoDB支持
    let Some(ref db) = state.mongo_database else {
        tracing::warn!("MongoDB未配置，回退到直接翻译");
        return Ok(redirect_to_translation_api(&target_url, source_lang, target_lang).into_response());
    };

    // 创建缓存查询引擎
    let cache_engine = CacheQueryEngine::new(db.clone());

    // 查询缓存状态
    match cache_engine.query_cache_status(&target_url, source_lang, target_lang).await {
        Ok(cache_result) => {
            tracing::info!(
                "缓存查询结果: URL={}, 状态={:?}, 策略={:?}",
                target_url,
                cache_result.cache_status,
                cache_result.routing_strategy
            );

            // 如果是调试模式，显示调试信息
            if debug_mode {
                return Ok(generate_debug_page(&target_url, &cache_result, &params).into_response());
            }

            // 根据路由策略处理请求
            handle_routing_strategy(
                &state,
                &target_url,
                cache_result,
                source_lang,
                target_lang,
            ).await
        }
        Err(e) => {
            tracing::error!("缓存查询失败: {}", e);
            // 查询失败时回退到直接翻译
            Ok(redirect_to_translation_api(&target_url, source_lang, target_lang).into_response())
        }
    }
}

/// 根据路由策略处理请求
#[cfg(feature = "web")]
async fn handle_routing_strategy(
    _state: &AppState,
    url: &str,
    cache_result: crate::web::services::cache_query_engine::CacheQueryResult,
    source_lang: Option<&str>,
    target_lang: Option<&str>,
) -> Result<Response, StatusCode> {
    match cache_result.routing_strategy {
        RoutingStrategy::UseCache(record_id) => {
            // 直接使用缓存记录进行预览
            tracing::info!("使用缓存记录: {}", record_id);
            Ok(redirect_to_library_preview(&record_id).into_response())
        }
        
        RoutingStrategy::WaitForProcessing => {
            // 显示等待处理页面
            tracing::info!("显示等待处理页面: {}", url);
            Ok(generate_processing_wait_page(url, &cache_result).into_response())
        }
        
        RoutingStrategy::ReprocessWithCheck => {
            // 重新处理但优先检查状态
            tracing::info!("重新处理并检查状态: {}", url);
            Ok(generate_reprocess_choice_page(url, &cache_result, source_lang, target_lang).into_response())
        }
        
        RoutingStrategy::FullReprocess => {
            // 完全重新处理
            tracing::info!("完全重新处理: {}", url);
            Ok(redirect_to_translation_api(url, source_lang, target_lang).into_response())
        }
    }
}

/// 重定向到翻译API
#[cfg(feature = "web")]
fn redirect_to_translation_api(
    url: &str,
    source_lang: Option<&str>,
    target_lang: Option<&str>,
) -> Redirect {
    let mut redirect_url = format!("/website/{}", url);
    
    let mut params = Vec::new();
    if let Some(source) = source_lang {
        params.push(format!("source_lang={}", source));
    }
    if let Some(target) = target_lang {
        params.push(format!("target_lang={}", target));
    }
    
    if !params.is_empty() {
        redirect_url.push('?');
        redirect_url.push_str(&params.join("&"));
    }
    
    Redirect::to(&redirect_url)
}

/// 重定向到库预览页面
#[cfg(feature = "web")]
fn redirect_to_library_preview(record_id: &str) -> Redirect {
    Redirect::to(&format!("/library/{}", record_id))
}

/// 生成等待处理页面
#[cfg(feature = "web")]
fn generate_processing_wait_page(
    url: &str,
    _cache_result: &crate::web::services::cache_query_engine::CacheQueryResult,
) -> Html<String> {
    let html = format!(
        r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>处理中 - Monolith智能翻译</title>
    <style>
        body {{
            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
            max-width: 800px;
            margin: 0 auto;
            padding: 20px;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
            min-height: 100vh;
        }}
        .container {{
            background: rgba(255, 255, 255, 0.1);
            border-radius: 20px;
            padding: 40px;
            backdrop-filter: blur(10px);
            box-shadow: 0 8px 32px rgba(0, 0, 0, 0.3);
        }}
        .spinner {{
            width: 50px;
            height: 50px;
            border: 4px solid rgba(255, 255, 255, 0.3);
            border-top: 4px solid white;
            border-radius: 50%;
            animation: spin 1s linear infinite;
            margin: 20px auto;
        }}
        @keyframes spin {{
            0% {{ transform: rotate(0deg); }}
            100% {{ transform: rotate(360deg); }}
        }}
        .url-info {{
            background: rgba(255, 255, 255, 0.2);
            padding: 15px;
            border-radius: 10px;
            margin: 20px 0;
            word-break: break-all;
        }}
        .actions {{
            margin-top: 30px;
            text-align: center;
        }}
        .btn {{
            display: inline-block;
            padding: 12px 24px;
            margin: 0 10px;
            background: rgba(255, 255, 255, 0.2);
            color: white;
            text-decoration: none;
            border-radius: 25px;
            border: 1px solid rgba(255, 255, 255, 0.3);
            transition: all 0.3s ease;
        }}
        .btn:hover {{
            background: rgba(255, 255, 255, 0.3);
            transform: translateY(-2px);
        }}
        .status-info {{
            font-size: 0.9em;
            margin-top: 20px;
            opacity: 0.8;
        }}
    </style>
    <script>
        // 自动刷新页面检查处理状态
        setTimeout(function() {{
            window.location.reload();
        }}, 5000); // 5秒后刷新
        
        // 实时状态检查（可选实现）
        function checkStatus() {{
            // 这里可以添加AJAX调用来检查处理状态
            fetch('/api/v2/link-status?url=' + encodeURIComponent('{url}'))
                .then(response => response.json())
                .then(data => {{
                    if (data.status === 'success') {{
                        window.location.href = '/library/' + data.record_id;
                    }}
                }})
                .catch(console.error);
        }}
        
        // 每3秒检查一次状态
        setInterval(checkStatus, 3000);
    </script>
</head>
<body>
    <div class="container">
        <h1>🔄 正在处理您的请求</h1>
        <div class="spinner"></div>
        
        <p>我们正在翻译和处理以下页面，请稍候...</p>
        
        <div class="url-info">
            <strong>目标URL:</strong> {url}
        </div>
        
        <div class="status-info">
            <p><strong>处理状态:</strong> 正在进行中</p>
            <p><strong>预计时间:</strong> 通常需要30-60秒</p>
            <p>页面将自动刷新并在处理完成后跳转到结果页面</p>
        </div>
        
        <div class="actions">
            <a href="/smart-website/{url}?force_reprocess=true" class="btn">
                🔄 强制重新处理
            </a>
            <a href="/" class="btn">
                🏠 返回首页
            </a>
            <a href="/smart-website/{url}?debug=true" class="btn">
                🐛 调试信息
            </a>
        </div>
    </div>
</body>
</html>"#,
        url = url
    );

    Html(html)
}

/// 生成重新处理选择页面
#[cfg(feature = "web")]
fn generate_reprocess_choice_page(
    url: &str,
    cache_result: &crate::web::services::cache_query_engine::CacheQueryResult,
    source_lang: Option<&str>,
    target_lang: Option<&str>,
) -> Html<String> {
    let record_info = if let Some(ref record) = cache_result.cached_record {
        format!(
            r#"
            <div class="cache-info">
                <h3>📄 发现已缓存的版本</h3>
                <p><strong>标题:</strong> {}</p>
                <p><strong>源语言:</strong> {}</p>
                <p><strong>目标语言:</strong> {}</p>
                <p><strong>状态:</strong> {}</p>
                <p><strong>创建时间:</strong> {}</p>
                <p><strong>更新时间:</strong> {}</p>
            </div>"#,
            record.title.as_deref().unwrap_or("未知"),
            record.source_lang,
            record.target_lang,
            record.status,
            record.created_at.to_chrono().format("%Y-%m-%d %H:%M:%S"),
            record.updated_at.to_chrono().format("%Y-%m-%d %H:%M:%S")
        )
    } else {
        String::new()
    };

    let cache_action = if let Some(record_id) = &cache_result.record_id {
        format!(
            r#"<a href="/library/{}" class="btn btn-primary">
                📖 查看缓存版本
            </a>"#,
            record_id
        )
    } else {
        String::new()
    };

    let reprocess_url = {
        let mut url_with_params = format!("/smart-website/{}?force_reprocess=true", url);
        if let Some(source) = source_lang {
            url_with_params.push_str(&format!("&source_lang={}", source));
        }
        if let Some(target) = target_lang {
            url_with_params.push_str(&format!("&target_lang={}", target));
        }
        url_with_params
    };

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>选择处理方式 - Monolith智能翻译</title>
    <style>
        body {{
            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
            max-width: 900px;
            margin: 0 auto;
            padding: 20px;
            background: linear-gradient(135deg, #ff7e5f 0%, #feb47b 100%);
            color: #333;
            min-height: 100vh;
        }}
        .container {{
            background: white;
            border-radius: 20px;
            padding: 40px;
            box-shadow: 0 10px 40px rgba(0, 0, 0, 0.2);
        }}
        .cache-info {{
            background: #f8f9fa;
            border-left: 4px solid #007bff;
            padding: 20px;
            margin: 20px 0;
            border-radius: 5px;
        }}
        .url-info {{
            background: #e3f2fd;
            padding: 15px;
            border-radius: 10px;
            margin: 20px 0;
            word-break: break-all;
        }}
        .choice-section {{
            display: grid;
            grid-template-columns: 1fr 1fr;
            gap: 20px;
            margin: 30px 0;
        }}
        .choice-card {{
            border: 2px solid #e0e0e0;
            border-radius: 15px;
            padding: 25px;
            text-align: center;
            transition: all 0.3s ease;
        }}
        .choice-card:hover {{
            border-color: #007bff;
            transform: translateY(-5px);
            box-shadow: 0 10px 25px rgba(0, 123, 255, 0.15);
        }}
        .btn {{
            display: inline-block;
            padding: 15px 30px;
            margin: 10px;
            text-decoration: none;
            border-radius: 25px;
            font-weight: bold;
            transition: all 0.3s ease;
            border: none;
            cursor: pointer;
        }}
        .btn-primary {{
            background: linear-gradient(45deg, #007bff, #0056b3);
            color: white;
        }}
        .btn-warning {{
            background: linear-gradient(45deg, #ffc107, #e0a800);
            color: #333;
        }}
        .btn-secondary {{
            background: #6c757d;
            color: white;
        }}
        .btn:hover {{
            transform: translateY(-2px);
            box-shadow: 0 5px 15px rgba(0, 0, 0, 0.2);
        }}
        .status-badge {{
            display: inline-block;
            padding: 5px 12px;
            border-radius: 20px;
            font-size: 0.8em;
            font-weight: bold;
        }}
        .status-expired {{
            background: #fff3cd;
            color: #856404;
        }}
        .status-failed {{
            background: #f8d7da;
            color: #721c24;
        }}
    </style>
</head>
<body>
    <div class="container">
        <h1>🤔 选择处理方式</h1>
        
        <div class="url-info">
            <strong>目标URL:</strong> {url}
        </div>
        
        {record_info}
        
        <div class="choice-section">
            <div class="choice-card">
                <h3>📖 使用缓存版本</h3>
                <p>快速查看已有的翻译版本</p>
                <p><small>⚡ 立即可用，无需等待</small></p>
                {cache_action}
            </div>
            
            <div class="choice-card">
                <h3>🔄 重新处理</h3>
                <p>获取最新的网页内容并重新翻译</p>
                <p><small>⏱️ 需要30-60秒处理时间</small></p>
                <a href="{reprocess_url}" class="btn btn-warning">
                    🔄 重新处理
                </a>
            </div>
        </div>
        
        <div style="text-align: center; margin-top: 30px;">
            <a href="/" class="btn btn-secondary">
                🏠 返回首页
            </a>
            <a href="/smart-website/{url}?debug=true" class="btn btn-secondary">
                🐛 调试信息
            </a>
        </div>
    </div>
</body>
</html>"#,
        url = url,
        record_info = record_info,
        cache_action = cache_action,
        reprocess_url = reprocess_url
    );

    Html(html)
}

/// 生成调试页面
#[cfg(feature = "web")]
fn generate_debug_page(
    url: &str,
    cache_result: &crate::web::services::cache_query_engine::CacheQueryResult,
    params: &SmartRouteQuery,
) -> Html<String> {
    let cache_record_json = if let Some(ref record) = cache_result.cached_record {
        serde_json::to_string_pretty(record).unwrap_or_else(|_| "序列化失败".to_string())
    } else {
        "无缓存记录".to_string()
    };

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>调试信息 - Monolith智能翻译</title>
    <style>
        body {{
            font-family: 'Courier New', monospace;
            max-width: 1200px;
            margin: 0 auto;
            padding: 20px;
            background: #1e1e1e;
            color: #ffffff;
        }}
        .debug-section {{
            background: #2d2d2d;
            border-radius: 10px;
            padding: 20px;
            margin: 20px 0;
            border: 1px solid #444;
        }}
        .debug-title {{
            color: #61dafb;
            border-bottom: 1px solid #444;
            padding-bottom: 10px;
            margin-bottom: 15px;
        }}
        .json-content {{
            background: #1e1e1e;
            padding: 15px;
            border-radius: 5px;
            white-space: pre-wrap;
            overflow-x: auto;
            font-size: 0.9em;
        }}
        .key {{
            color: #9cdcfe;
        }}
        .string {{
            color: #ce9178;
        }}
        .number {{
            color: #b5cea8;
        }}
        .boolean {{
            color: #569cd6;
        }}
        .btn {{
            display: inline-block;
            padding: 10px 20px;
            margin: 5px;
            background: #007acc;
            color: white;
            text-decoration: none;
            border-radius: 5px;
            border: none;
        }}
        .btn:hover {{
            background: #005a9e;
        }}
        .status-info {{
            display: grid;
            grid-template-columns: 1fr 1fr;
            gap: 20px;
        }}
    </style>
</head>
<body>
    <h1>🐛 智能路由调试信息</h1>
    
    <div class="debug-section">
        <h2 class="debug-title">请求信息</h2>
        <div class="json-content">URL: {}
源语言: {:?}
目标语言: {:?}
强制重新处理: {:?}
调试模式: {:?}</div>
    </div>
    
    <div class="debug-section">
        <h2 class="debug-title">缓存查询结果</h2>
        <div class="status-info">
            <div>
                <strong>记录ID:</strong> {:?}<br>
                <strong>缓存状态:</strong> {:?}<br>
                <strong>路由策略:</strong> {:?}
            </div>
        </div>
    </div>
    
    <div class="debug-section">
        <h2 class="debug-title">缓存记录详情</h2>
        <div class="json-content">{}</div>
    </div>
    
    <div class="debug-section">
        <h2 class="debug-title">操作选项</h2>
        <a href="/smart-website/{}" class="btn">🔄 正常模式</a>
        <a href="/smart-website/{}?force_reprocess=true" class="btn">⚡ 强制重新处理</a>
        <a href="/library/{}" class="btn">📖 查看缓存</a>
        <a href="/" class="btn">🏠 返回首页</a>
    </div>
</body>
</html>"#,
        url,
        params.source_lang,
        params.target_lang,
        params.force_reprocess,
        params.debug,
        cache_result.record_id,
        cache_result.cache_status,
        cache_result.routing_strategy,
        cache_record_json,
        url,
        url,
        cache_result.record_id.as_deref().unwrap_or("unknown")
    );

    Html(html)
}

// 非web feature的占位实现
#[cfg(not(feature = "web"))]
pub struct SmartRouteQuery;

#[cfg(not(feature = "web"))]
pub async fn smart_website_handler() {}