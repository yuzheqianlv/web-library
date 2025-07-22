//! HTML 构建器模块
//!
//! 负责动态构建 HTML 模板，支持内联 CSS/JS 或外部引用

use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct HtmlBuilderConfig {
    /// 模板目录路径
    pub template_dir: String,
    /// 是否内联 CSS/JS（用于单文件部署）
    pub inline_assets: bool,
    /// 静态资源基础路径
    pub asset_base_path: String,
}

impl Default for HtmlBuilderConfig {
    fn default() -> Self {
        Self {
            template_dir: "templates".to_string(),
            inline_assets: true,
            asset_base_path: "/".to_string(),
        }
    }
}

pub struct HtmlBuilder {
    config: HtmlBuilderConfig,
}

impl HtmlBuilder {
    pub fn new(config: HtmlBuilderConfig) -> Self {
        Self { config }
    }

    /// 构建完整的 HTML 页面
    pub fn build_index_page(&self) -> Result<String, Box<dyn std::error::Error>> {
        self.build_index_page_with_url("")
    }

    /// 构建带预加载 URL 的 HTML 页面
    pub fn build_index_page_with_url(
        &self,
        preload_url: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let template_path = Path::new(&self.config.template_dir);

        // 读取 CSS 内容
        let css_content = if self.config.inline_assets {
            self.read_css_inline(template_path)?
        } else {
            self.get_css_links()
        };

        // 读取 JavaScript 内容
        let js_content = if self.config.inline_assets {
            self.read_js_inline(template_path)?
        } else {
            self.get_js_scripts()
        };

        // 如果有预加载 URL，添加自动加载脚本
        let preload_script = if !preload_url.is_empty() {
            format!(
                r#"
    <script>
        // 预加载 URL 自动填充和翻译
        document.addEventListener('DOMContentLoaded', function() {{
            const urlInput = document.getElementById('url-input');
            const translateBtn = document.getElementById('translate-btn');
            
            if (urlInput && translateBtn) {{
                urlInput.value = "{}";
                // 延迟 500ms 自动开始翻译，确保页面完全加载
                setTimeout(function() {{
                    translateBtn.click();
                }}, 500);
            }}
        }});
    </script>"#,
                preload_url.replace('"', "\\\"") // 转义引号
            )
        } else {
            String::new()
        };

        // 构建完整的 HTML
        let html = format!(
            r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Monolith 网页翻译器</title>
    {}{}
</head>
<body>
    {}
    {}
</body>
</html>"#,
            css_content,
            preload_script,
            self.get_body_content()?,
            js_content
        );

        Ok(html)
    }

    /// 读取并内联 CSS 样式
    fn read_css_inline(&self, template_path: &Path) -> Result<String, Box<dyn std::error::Error>> {
        let css_path = template_path.join("assets/css/main.css");
        let css_content = fs::read_to_string(css_path)?;
        Ok(format!("<style>\n{}\n</style>", css_content))
    }

    /// 生成 CSS 外部链接
    fn get_css_links(&self) -> String {
        format!(
            r#"<link rel="stylesheet" href="{}assets/css/main.css">"#,
            self.config.asset_base_path
        )
    }

    /// 读取并内联 JavaScript
    fn read_js_inline(&self, template_path: &Path) -> Result<String, Box<dyn std::error::Error>> {
        let js_path = template_path.join("assets/js/monolith-translator.js");
        let js_content = fs::read_to_string(js_path)?;
        Ok(format!("<script>\n{}\n</script>", js_content))
    }

    /// 生成 JavaScript 外部引用
    fn get_js_scripts(&self) -> String {
        format!(
            r#"<script src="{}assets/js/monolith-translator.js"></script>"#,
            self.config.asset_base_path
        )
    }

    /// 获取页面主体内容
    fn get_body_content(&self) -> Result<String, Box<dyn std::error::Error>> {
        Ok(r#"
    <!-- 置顶导航栏 -->
    <nav class="navbar" id="navbar">
        <!-- 隐藏/显示按钮 -->
        <button class="toggle-nav-btn" id="toggle-nav-btn" title="隐藏导航栏">
            <svg viewBox="0 0 24 24" fill="currentColor">
                <path d="M3 18h18v-2H3v2zm0-5h18v-2H3v2zm0-7v2h18V6H3z"/>
            </svg>
        </button>

        <!-- URL 输入框 -->
        <input type="url" class="url-input" id="url-input" placeholder="请输入要翻译的网页 URL...">

        <!-- 翻译按钮 -->
        <button class="translate-btn" id="translate-btn">翻译</button>

        <!-- 模式切换 -->
        <div class="mode-toggle">
            <button class="mode-btn active" data-mode="translated">译文</button>
            <button class="mode-btn" data-mode="original">原文</button>
            <button class="mode-btn" data-mode="bilingual">双语</button>
        </div>

        <!-- 书签脚本链接 -->
        <a href="/bookmarklet" class="bookmarklet-link" title="获取一键翻译书签脚本" target="_blank">
            <svg viewBox="0 0 24 24" fill="currentColor" width="20" height="20">
                <path d="M17,7H22V9H19V12H17V9A2,2 0 0,0 15,7H13V5H15A4,4 0 0,1 19,9V12H22V14H17V12C17,11.31 17.17,10.65 17.46,10.1L17,9.6C17,8.26 15.74,7 14.4,7H9.6C8.26,7 7,8.26 7,9.6V14.4C7,15.74 8.26,17 9.6,17H14.4C15.74,17 17,15.74 17,14.4V12Z"/>
            </svg>
        </a>
    </nav>

    <!-- 悬浮显示按钮 -->
    <button class="floating-toggle" id="floating-toggle" title="显示导航栏">
        <svg viewBox="0 0 24 24" fill="currentColor">
            <path d="M3 18h18v-2H3v2zm0-5h18v-2H3v2zm0-7v2h18V6H3z"/>
        </svg>
    </button>

    <!-- 主内容区域 -->
    <main class="main-content" id="main-content">
        <!-- 加载状态 -->
        <div class="loading" id="loading">
            <div class="spinner"></div>
            <p>正在处理网页，请稍候...</p>
        </div>

        <!-- 内容预览区域 -->
        <div class="content-viewer">
            <!-- 空状态 -->
            <div class="empty-state" id="empty-state">
                <svg viewBox="0 0 24 24" fill="currentColor">
                    <path d="M14,2H6A2,2 0 0,0 4,4V20A2,2 0 0,0 6,22H18A2,2 0 0,0 20,20V8L14,2M18,20H6V4H13V9H18V20Z"/>
                </svg>
                <h3>输入网页 URL 开始翻译</h3>
                <p>支持自动检测语言并翻译为中文</p>
            </div>

            <!-- 译文 iframe -->
            <iframe id="translated-frame" class="content-frame"></iframe>

            <!-- 原文 iframe -->
            <iframe id="original-frame" class="content-frame"></iframe>

            <!-- 双语对照 -->
            <div class="bilingual-container" id="bilingual-container">
                <!-- 双语模式标签 -->
                <div class="bilingual-labels">
                    <div class="bilingual-label">译文</div>
                    <div class="bilingual-label">原文</div>
                </div>
                
                <!-- 同步状态指示器 -->
                <div class="sync-indicator" id="sync-indicator">
                    <div class="sync-dot"></div>
                    <span id="sync-status">同步滚动已启用</span>
                </div>
                
                <iframe id="bilingual-translated" class="bilingual-pane"></iframe>
                <iframe id="bilingual-original" class="bilingual-pane"></iframe>
            </div>
        </div>
    </main>

    <!-- 错误提示 -->
    <div class="error-toast" id="error-toast"></div>"#.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_html_builder_inline() {
        let temp_dir = TempDir::new().unwrap();
        let template_dir = temp_dir.path().join("templates");
        let css_dir = template_dir.join("assets/css");
        let js_dir = template_dir.join("assets/js");

        fs::create_dir_all(&css_dir).unwrap();
        fs::create_dir_all(&js_dir).unwrap();

        fs::write(css_dir.join("main.css"), "body { margin: 0; }").unwrap();
        fs::write(
            js_dir.join("monolith-translator.js"),
            "console.log('test');",
        )
        .unwrap();

        let config = HtmlBuilderConfig {
            template_dir: template_dir.to_string_lossy().to_string(),
            inline_assets: true,
            asset_base_path: "/".to_string(),
        };

        let builder = HtmlBuilder::new(config);
        let html = builder.build_index_page().unwrap();

        assert!(html.contains("<style>"));
        assert!(html.contains("body { margin: 0; }"));
        assert!(html.contains("<script>"));
        assert!(html.contains("console.log('test');"));
    }

    #[test]
    fn test_html_builder_external() {
        let config = HtmlBuilderConfig {
            template_dir: "templates".to_string(),
            inline_assets: false,
            asset_base_path: "/static/".to_string(),
        };

        let builder = HtmlBuilder::new(config);
        let html = builder.build_index_page().unwrap();

        assert!(html.contains(r#"<link rel="stylesheet" href="/static/assets/css/main.css">"#));
        assert!(
            html.contains(r#"<script src="/static/assets/js/monolith-translator.js"></script>"#)
        );
    }
}
