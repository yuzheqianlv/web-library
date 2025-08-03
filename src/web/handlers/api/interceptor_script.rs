//! 智能链接拦截器脚本API
//!
//! 提供JavaScript拦截器脚本的服务

#[cfg(feature = "web")]
use axum::{
    extract::Query,
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Response},
};

#[cfg(feature = "web")]
use serde::Deserialize;

/// 拦截器脚本查询参数
#[cfg(feature = "web")]
#[derive(Debug, Deserialize)]
pub struct InterceptorScriptQuery {
    /// 是否使用轻量级版本
    pub lite: Option<bool>,
    /// 是否启用调试模式
    pub debug: Option<bool>,
    /// 自定义配置
    pub config: Option<String>,
}

/// 获取智能链接拦截器脚本
#[cfg(feature = "web")]
pub async fn get_interceptor_script(
    Query(params): Query<InterceptorScriptQuery>,
) -> Result<Response, StatusCode> {
    let is_lite = params.lite.unwrap_or(false);
    let debug = params.debug.unwrap_or(false);
    
    let script_content = if is_lite {
        get_lite_interceptor_script(debug, params.config.as_deref())
    } else {
        get_full_interceptor_script(debug, params.config.as_deref())
    };
    
    let mut headers = HeaderMap::new();
    headers.insert("Content-Type", "application/javascript; charset=utf-8".parse().unwrap());
    headers.insert("Cache-Control", "public, max-age=3600".parse().unwrap()); // 1小时缓存
    
    Ok((headers, script_content).into_response())
}

/// 获取拦截器注入脚本（用于嵌入HTML页面）
#[cfg(feature = "web")]
pub async fn get_interceptor_injection_script(
    Query(params): Query<InterceptorScriptQuery>,
) -> Result<Html<String>, StatusCode> {
    let is_lite = params.lite.unwrap_or(false);
    let debug = params.debug.unwrap_or(false);
    
    let script_url = if is_lite {
        "/assets/js/smart-link-interceptor-lite.js"
    } else {
        "/assets/js/smart-link-interceptor.js"
    };
    
    let config_override = if let Some(config) = params.config.as_deref() {
        format!("window.MONOLITH_CONFIG_OVERRIDE = {};", config)
    } else {
        String::new()
    };
    
    let debug_config = if debug {
        "if (window.MONOLITH_CONFIG) window.MONOLITH_CONFIG.DEBUG = true;"
    } else {
        ""
    };
    
    let injection_script = format!(
        r#"<script type="text/javascript">
// Monolith智能链接拦截器注入脚本
(function() {{
    'use strict';
    
    // 检查是否已经加载
    if (window.monolithInterceptorLoaded) {{
        return;
    }}
    
    // 标记页面以便拦截器识别
    const meta = document.createElement('meta');
    meta.name = 'monolith-page';
    meta.content = 'true';
    document.head.appendChild(meta);
    
    // 配置覆盖
    {config_override}
    
    // 调试模式配置
    {debug_config}
    
    // 动态加载脚本
    const script = document.createElement('script');
    script.type = 'text/javascript';
    script.async = true;
    script.src = '{script_url}';
    script.onload = function() {{
        window.monolithInterceptorLoaded = true;
        console.log('[Monolith] 智能链接拦截器加载完成');
    }};
    script.onerror = function() {{
        console.warn('[Monolith] 智能链接拦截器加载失败');
    }};
    
    // 添加到页面
    const firstScript = document.getElementsByTagName('script')[0];
    if (firstScript) {{
        firstScript.parentNode.insertBefore(script, firstScript);
    }} else {{
        document.head.appendChild(script);
    }}
}})();
</script>"#,
        script_url = script_url,
        config_override = config_override,
        debug_config = debug_config
    );
    
    Ok(Html(injection_script))
}

/// 获取完整版拦截器脚本
#[cfg(feature = "web")]
fn get_full_interceptor_script(debug: bool, config_override: Option<&str>) -> String {
    let mut script = include_str!("../../../../templates/assets/js/smart-link-interceptor.js").to_string();
    
    // 配置覆盖
    if let Some(config) = config_override {
        script = script.replace(
            "const CONFIG = {",
            &format!("const CONFIG_OVERRIDE = {};\nconst CONFIG = {{", config)
        );
        script = script.replace(
            "    };",
            "    ...CONFIG_OVERRIDE\n    };"
        );
    }
    
    // 调试模式
    if debug {
        script = script.replace("DEBUG: false", "DEBUG: true");
    }
    
    script
}

/// 获取轻量级拦截器脚本
#[cfg(feature = "web")]
fn get_lite_interceptor_script(debug: bool, config_override: Option<&str>) -> String {
    let mut script = include_str!("../../../../templates/assets/js/smart-link-interceptor-lite.js").to_string();
    
    // 配置覆盖
    if let Some(config) = config_override {
        script = script.replace(
            "const CONFIG = {",
            &format!("const CONFIG_OVERRIDE = {};\nconst CONFIG = {{", config)
        );
        script = script.replace(
            "    };",
            "    ...CONFIG_OVERRIDE\n    };"
        );
    }
    
    // 调试模式
    if debug {
        script = script.replace("DEBUG: false", "DEBUG: true");
    }
    
    script
}

/// 生成书签脚本（增强版）
#[cfg(feature = "web")]
pub async fn get_enhanced_bookmarklet_script() -> Result<Html<String>, StatusCode> {
    let bookmarklet_code = r#"javascript:(function(){
    // Monolith增强书签脚本
    if(window.monolithInterceptor){
        console.log('拦截器已存在，刷新状态');
        window.monolithInterceptor.refreshLinkStatus();
        return;
    }
    
    // 动态注入拦截器
    const script = document.createElement('script');
    script.src = window.location.origin + '/api/v2/interceptor/script?lite=1&debug=1';
    script.onload = function(){
        console.log('Monolith拦截器已注入');
    };
    document.head.appendChild(script);
    
    // 显示提示
    const notice = document.createElement('div');
    notice.innerHTML = '🔗 Monolith智能链接拦截器已激活';
    notice.style.cssText = 'position:fixed;top:10px;right:10px;background:#4caf50;color:white;padding:10px;border-radius:5px;z-index:10000;font-family:sans-serif;font-size:14px;';
    document.body.appendChild(notice);
    setTimeout(() => notice.remove(), 3000);
})();"#;

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <title>Monolith增强书签脚本</title>
    <style>
        body {{ font-family: sans-serif; max-width: 800px; margin: 0 auto; padding: 20px; }}
        .bookmarklet {{ background: #f5f5f5; padding: 15px; border-radius: 5px; margin: 20px 0; }}
        .bookmarklet a {{ color: #2196f3; text-decoration: none; word-break: break-all; }}
        .instructions {{ background: #e3f2fd; padding: 15px; border-radius: 5px; }}
    </style>
</head>
<body>
    <h1>🔗 Monolith智能链接拦截器书签</h1>
    
    <div class="instructions">
        <h3>使用说明：</h3>
        <ol>
            <li>将下面的链接拖拽到浏览器书签栏</li>
            <li>访问任何网页后点击该书签</li>
            <li>页面中的外部链接将被智能拦截，显示缓存状态</li>
            <li>点击链接时会智能路由到最佳处理方式</li>
        </ol>
    </div>
    
    <div class="bookmarklet">
        <h3>增强版拦截器书签：</h3>
        <a href="{bookmarklet_code}">📎 Monolith智能拦截器</a>
    </div>
    
    <div class="instructions">
        <h3>功能特性：</h3>
        <ul>
            <li>🎯 自动检测页面链接状态</li>
            <li>⚡ 智能缓存查询和批量处理</li>
            <li>🎨 可视化状态指示器</li>
            <li>🚀 智能路由决策</li>
            <li>💾 本地缓存加速</li>
            <li>🔄 实时状态更新</li>
        </ul>
    </div>
</body>
</html>"#,
        bookmarklet_code = bookmarklet_code
    );

    Ok(Html(html))
}

// 非web feature的占位实现
#[cfg(not(feature = "web"))]
pub async fn get_interceptor_script() {}

#[cfg(not(feature = "web"))]
pub async fn get_interceptor_injection_script() {}

#[cfg(not(feature = "web"))]
pub async fn get_enhanced_bookmarklet_script() {}