//! 主题API处理器
//!
//! 提供主题相关的REST API接口

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Html,
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::web::theme::{ThemeConfig, UserThemePreference};

use crate::web::types::AppState;

/// 设置主题请求
#[derive(Debug, Deserialize)]
pub struct SetThemeRequest {
    pub theme: String,
}

/// 主题列表响应
#[derive(Debug, Serialize)]
pub struct ThemeListResponse {
    pub themes: Vec<ThemeInfo>,
    pub current: String,
}

/// 主题信息
#[derive(Debug, Serialize)]
pub struct ThemeInfo {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub is_dark: bool,
}

/// API响应结构
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T, message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: message.into(),
            data: Some(data),
        }
    }

    pub fn error(message: impl Into<String>) -> ApiResponse<()> {
        ApiResponse {
            success: false,
            message: message.into(),
            data: None,
        }
    }
}

/// 获取所有可用主题列表
pub async fn get_themes(
    State(app_state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<ThemeListResponse>>, StatusCode> {
    let manager = app_state
        .theme_manager
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let themes = manager
        .get_themes()
        .into_iter()
        .map(|theme| ThemeInfo {
            name: theme.name.clone(),
            display_name: theme.display_name.clone(),
            description: theme.description.clone(),
            is_dark: theme.is_dark,
        })
        .collect();

    let current_theme = manager
        .get_current_theme()
        .map(|t| t.name.clone())
        .unwrap_or_else(|| "light".to_string());

    let response = ThemeListResponse {
        themes,
        current: current_theme,
    };

    Ok(Json(ApiResponse::success(response, "获取主题列表成功")))
}

/// 获取特定主题配置
pub async fn get_theme(
    Path(theme_name): Path<String>,
    State(app_state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<ThemeConfig>>, StatusCode> {
    let manager = app_state
        .theme_manager
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match manager.get_theme(&theme_name) {
        Some(theme) => Ok(Json(ApiResponse::success(
            theme.clone(),
            "获取主题配置成功",
        ))),
        None => {
            let error: ApiResponse<ThemeConfig> = ApiResponse {
                success: false,
                message: format!("主题 '{}' 不存在", theme_name),
                data: None,
            };
            Ok(Json(error))
        }
    }
}

/// 设置当前主题
pub async fn set_theme(
    State(app_state): State<Arc<AppState>>,
    Json(request): Json<SetThemeRequest>,
) -> Result<Json<ApiResponse<()>>, StatusCode> {
    let mut manager = app_state
        .theme_manager
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match manager.set_current_theme(&request.theme) {
        Ok(()) => Ok(Json(ApiResponse::success(
            (),
            format!("主题已切换为 '{}'", request.theme),
        ))),
        Err(err) => {
            let error: ApiResponse<()> = ApiResponse {
                success: false,
                message: err,
                data: None,
            };
            Ok(Json(error))
        }
    }
}

/// 获取主题CSS变量
pub async fn get_theme_css(
    Path(theme_name): Path<String>,
    State(app_state): State<Arc<AppState>>,
) -> Result<Html<String>, StatusCode> {
    let manager = app_state
        .theme_manager
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if manager.get_theme(&theme_name).is_none() {
        return Err(StatusCode::NOT_FOUND);
    }

    let css = manager.generate_css_variables(Some(&theme_name));
    Ok(Html(css))
}

/// 获取当前主题CSS变量
pub async fn get_current_theme_css(
    State(app_state): State<Arc<AppState>>,
) -> Result<Html<String>, StatusCode> {
    let manager = app_state
        .theme_manager
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let css = manager.generate_css_variables(None);
    Ok(Html(css))
}

/// 获取主题选择器HTML
pub async fn get_theme_selector(
    State(app_state): State<Arc<AppState>>,
) -> Result<Html<String>, StatusCode> {
    let manager = app_state
        .theme_manager
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let html = manager.generate_theme_selector();
    Ok(Html(html))
}

/// 获取主题切换JavaScript
pub async fn get_theme_script(
    State(app_state): State<Arc<AppState>>,
) -> Result<Html<String>, StatusCode> {
    let manager = app_state
        .theme_manager
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let script = format!(
        r#"<script type="text/javascript">
{}
</script>"#,
        manager.generate_theme_script()
    );
    Ok(Html(script))
}

/// 注册自定义主题
pub async fn register_theme(
    State(app_state): State<Arc<AppState>>,
    Json(theme_config): Json<ThemeConfig>,
) -> Result<Json<ApiResponse<()>>, StatusCode> {
    let mut manager = app_state
        .theme_manager
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // 验证主题配置
    if theme_config.name.is_empty() {
        let error: ApiResponse<()> = ApiResponse {
            success: false,
            message: "主题名称不能为空".to_string(),
            data: None,
        };
        return Ok(Json(error));
    }

    manager.register_theme(theme_config.clone());
    Ok(Json(ApiResponse::success(
        (),
        format!("主题 '{}' 注册成功", theme_config.name),
    )))
}

/// 删除自定义主题
pub async fn delete_theme(
    Path(theme_name): Path<String>,
    State(app_state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<()>>, StatusCode> {
    let mut manager = app_state
        .theme_manager
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match manager.remove_theme(&theme_name) {
        Ok(()) => Ok(Json(ApiResponse::success(
            (),
            format!("主题 '{}' 删除成功", theme_name),
        ))),
        Err(err) => {
            let error: ApiResponse<()> = ApiResponse {
                success: false,
                message: err,
                data: None,
            };
            Ok(Json(error))
        }
    }
}

/// 获取用户主题偏好设置
pub async fn get_user_preference() -> Result<Json<ApiResponse<UserThemePreference>>, StatusCode> {
    // 这里可以从数据库或缓存中获取用户偏好
    // 目前返回默认设置
    let preference = UserThemePreference::default();
    Ok(Json(ApiResponse::success(preference, "获取用户偏好成功")))
}

/// 设置用户主题偏好
pub async fn set_user_preference(
    Json(preference): Json<UserThemePreference>,
) -> Result<Json<ApiResponse<()>>, StatusCode> {
    // 这里可以将用户偏好保存到数据库或缓存
    // 目前只是验证并返回成功

    if preference.preferred_theme.is_empty() {
        let error: ApiResponse<()> = ApiResponse {
            success: false,
            message: "首选主题不能为空".to_string(),
            data: None,
        };
        return Ok(Json(error));
    }

    Ok(Json(ApiResponse::success((), "用户偏好设置成功")))
}

/// 根据系统主题自动切换
pub async fn auto_switch_theme(
    State(app_state): State<Arc<AppState>>,
    Json(request): Json<AutoSwitchRequest>,
) -> Result<Json<ApiResponse<()>>, StatusCode> {
    let mut manager = app_state
        .theme_manager
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let target_theme = if request.is_dark_mode {
        "dark"
    } else {
        "light"
    };

    match manager.set_current_theme(target_theme) {
        Ok(()) => Ok(Json(ApiResponse::success(
            (),
            format!(
                "已自动切换到{}主题",
                if request.is_dark_mode {
                    "暗色"
                } else {
                    "明亮"
                }
            ),
        ))),
        Err(err) => {
            let error: ApiResponse<()> = ApiResponse {
                success: false,
                message: err,
                data: None,
            };
            Ok(Json(error))
        }
    }
}

/// 自动切换主题请求
#[derive(Debug, Deserialize)]
pub struct AutoSwitchRequest {
    pub is_dark_mode: bool,
}

/// 主题预览
pub async fn preview_theme(
    Path(theme_name): Path<String>,
    State(app_state): State<Arc<AppState>>,
) -> Result<Html<String>, StatusCode> {
    let manager = app_state
        .theme_manager
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let theme = match manager.get_theme(&theme_name) {
        Some(t) => t,
        None => return Err(StatusCode::NOT_FOUND),
    };

    let css_variables = manager.generate_css_variables(Some(&theme_name));

    let preview_html = format!(
        r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>主题预览 - {}</title>
    <style>
        {}
        
        body {{
            font-family: var(--font-family);
            font-size: var(--font-base-size);
            line-height: var(--font-line-height);
            color: var(--color-text-primary);
            background: var(--color-background);
            margin: 0;
            padding: var(--spacing-large);
        }}
        
        .preview-container {{
            max-width: 800px;
            margin: 0 auto;
        }}
        
        .card {{
            background: var(--color-surface);
            border: var(--border-width) var(--border-style) var(--color-border);
            border-radius: var(--border-radius);
            padding: var(--spacing-large);
            margin-bottom: var(--spacing-medium);
            box-shadow: var(--shadow-medium);
        }}
        
        .btn {{
            background: var(--color-primary);
            color: white;
            border: none;
            padding: var(--spacing-small) var(--spacing-medium);
            border-radius: var(--border-radius);
            cursor: pointer;
            transition: background-color var(--animation-duration) var(--animation-easing);
            margin-right: var(--spacing-small);
        }}
        
        .btn:hover {{
            background: var(--color-primary-hover);
        }}
        
        .btn-secondary {{
            background: var(--color-secondary);
            color: var(--color-text-primary);
        }}
        
        .btn-secondary:hover {{
            background: var(--color-secondary-hover);
        }}
        
        .status {{
            padding: var(--spacing-small);
            border-radius: var(--border-radius);
            margin: var(--spacing-small) 0;
        }}
        
        .status.success {{
            background: var(--color-success);
            color: white;
        }}
        
        .status.warning {{
            background: var(--color-warning);
            color: white;
        }}
        
        .status.error {{
            background: var(--color-error);
            color: white;
        }}
        
        .status.info {{
            background: var(--color-info);
            color: white;
        }}
        
        .text-secondary {{
            color: var(--color-text-secondary);
        }}
    </style>
</head>
<body>
    <div class="preview-container">
        <h1>主题预览: {}</h1>
        <p class="text-secondary">{}</p>
        
        <div class="card">
            <h2>按钮组件</h2>
            <button class="btn">主要按钮</button>
            <button class="btn btn-secondary">次要按钮</button>
        </div>
        
        <div class="card">
            <h2>状态提示</h2>
            <div class="status success">成功状态</div>
            <div class="status warning">警告状态</div>
            <div class="status error">错误状态</div>
            <div class="status info">信息状态</div>
        </div>
        
        <div class="card">
            <h2>文本样式</h2>
            <p>这是主要文本颜色的示例段落。</p>
            <p class="text-secondary">这是次要文本颜色的示例段落。</p>
        </div>
        
        <div class="card">
            <h2>主题信息</h2>
            <p><strong>主题名称:</strong> {}</p>
            <p><strong>主题类型:</strong> {}</p>
            <p><strong>主要颜色:</strong> <span style="background: var(--color-primary); color: white; padding: 2px 8px; border-radius: 4px;">{}</span></p>
        </div>
    </div>
</body>
</html>"#,
        theme.display_name,
        css_variables,
        theme.display_name,
        theme.description,
        theme.name,
        if theme.is_dark {
            "暗色主题"
        } else {
            "明亮主题"
        },
        theme.colors.primary
    );

    Ok(Html(preview_html))
}
