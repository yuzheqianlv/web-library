//! 书签脚本相关API处理器

use std::sync::Arc;

#[cfg(feature = "web")]
use axum::{extract::State, http::StatusCode, response::Json};

use crate::web::types::{AppState, BookmarkletResponse};

/// 生成书签脚本
#[cfg(feature = "web")]
pub async fn generate_bookmarklet(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<BookmarkletResponse>, (StatusCode, Json<serde_json::Value>)> {
    // 获取当前服务器地址（这里简化处理，实际应用中可能需要从请求头获取）
    let server_url = "http://localhost:7080"; // 可以从配置中获取

    // 增强的书签脚本，支持翻译选项配置
    let bookmarklet_code = format!(
        r#"javascript:(function(){{
    var currentUrl = window.location.href;
    var serverUrl = '{}';
    
    // 创建选项对话框
    var enableTranslation = confirm('是否启用翻译功能？\n\n点击"确定"启用翻译，点击"取消"仅处理页面');
    var targetLang = 'zh';
    
    if (enableTranslation) {{
        var langChoice = prompt('请选择目标语言：\n\nzh - 中文 (默认)\nen - English\nja - 日本語\nko - 한국어\nes - Español\nfr - Français\nde - Deutsch\nru - Русский', 'zh');
        if (langChoice && langChoice.trim()) {{
            targetLang = langChoice.trim();
        }}
    }}
    
    // 构建处理请求
    var requestData = {{
        url: currentUrl,
        translate: enableTranslation,
        target_lang: targetLang,
        options: {{
            no_css: false,
            no_js: true,
            no_images: false,
            no_fonts: false,
            timeout: 30
        }}
    }};
    
    // 显示处理状态
    var statusDiv = document.createElement('div');
    statusDiv.style.cssText = 'position:fixed;top:20px;right:20px;background:#667eea;color:white;padding:15px 20px;border-radius:8px;z-index:999999;font-family:Arial,sans-serif;font-size:14px;box-shadow:0 4px 12px rgba(0,0,0,0.3);';
    statusDiv.innerHTML = '🚀 Monolith 正在处理页面...';
    document.body.appendChild(statusDiv);
    
    // 发送处理请求
    fetch(serverUrl + '/api/process', {{
        method: 'POST',
        headers: {{
            'Content-Type': 'application/json'
        }},
        body: JSON.stringify(requestData)
    }})
    .then(response => response.json())
    .then(result => {{
        document.body.removeChild(statusDiv);
        if (result.status === 'success') {{
            // 成功处理，打开预览页面
            var previewUrl = serverUrl + result.preview_url;
            window.open(previewUrl, '_blank');
        }} else {{
            alert('处理失败: ' + (result.error || '未知错误'));
        }}
    }})
    .catch(error => {{
        document.body.removeChild(statusDiv);
        console.error('Monolith 处理失败:', error);
        alert('请求失败，请检查网络连接或服务器状态');
    }});
}})();"#,
        server_url
    );

    let installation_guide = r#"
## 增强书签脚本安装指南

### Chrome/Edge 浏览器:
1. 右键点击书签栏，选择"添加页面"
2. 名称填写: "🚀 Monolith 智能处理器"
3. 网址填写下方的书签脚本代码
4. 点击保存

### Firefox 浏览器:
1. 右键点击书签栏，选择"新书签"
2. 名称填写: "🚀 Monolith 智能处理器"
3. 位置填写下方的书签脚本代码
4. 点击添加

### Safari 浏览器:
1. 将下方代码复制到剪贴板
2. 添加任意页面为书签
3. 编辑书签，将 URL 替换为书签脚本代码

### 移动端浏览器:
1. 复制书签脚本代码
2. 添加当前页面为书签
3. 编辑书签，将URL替换为脚本代码
4. 重命名为"Monolith处理器"

## 新功能特点
- ✨ 智能翻译选项：可选择是否启用翻译
- 🌍 多语言支持：支持8种目标语言
- 🎯 处理状态显示：实时显示处理进度
- 🔒 安全处理：默认禁用JavaScript防止安全风险
- 📱 跨平台兼容：支持所有主流浏览器
"#
    .to_string();

    let example_usage = r#"
## 智能使用方法

### 基本流程:
1. 在任意网页上点击"🚀 Monolith 智能处理器"书签
2. 选择是否启用翻译功能（确定=启用，取消=仅处理）
3. 如启用翻译，选择目标语言（默认中文）
4. 系统显示处理状态并自动处理页面
5. 处理完成后在新标签页中打开结果

### 翻译选项:
- **zh** - 中文（简体）
- **en** - English
- **ja** - 日本語  
- **ko** - 한국어
- **es** - Español
- **fr** - Français
- **de** - Deutsch
- **ru** - Русский

### 高级功能:
- 🎨 支持原文/译文/双语对照模式切换
- 🎯 智能资源处理，移除广告和跟踪脚本
- 💾 本地化处理，将所有资源嵌入单一文件
- 🔄 实时处理状态显示
- ⚡ 快速响应，优化的处理流程

### 技术特性:
- 📡 RESTful API 架构
- 🛡️ 默认禁用JavaScript提高安全性
- 📱 响应式设计，支持移动端
- 🌐 跨域处理，支持所有网站
- 🎭 主题切换，多种界面风格
"#
    .to_string();

    let response = BookmarkletResponse {
        bookmarklet_code,
        installation_guide,
        example_usage,
    };

    Ok(Json(response))
}

/// 非 web feature 的占位函数
#[cfg(not(feature = "web"))]
pub async fn generate_bookmarklet() -> Result<String, String> {
    Err("Web feature not enabled".to_string())
}
