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
"#
    .to_string();

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
