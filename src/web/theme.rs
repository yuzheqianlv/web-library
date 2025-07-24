//! Web端主题管理模块
//!
//! 提供统一的主题切换和管理功能，包括：
//! - 多主题配置管理
//! - 主题资源生成
//! - 用户偏好存储
//! - 主题API接口

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 主题配置结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    /// 主题名称
    pub name: String,
    /// 主题显示名称
    pub display_name: String,
    /// 主题描述
    pub description: String,
    /// 是否为暗色主题
    pub is_dark: bool,
    /// 颜色变量配置
    pub colors: ThemeColors,
    /// 字体配置
    pub fonts: ThemeFonts,
    /// 间距配置
    pub spacing: ThemeSpacing,
    /// 阴影配置
    pub shadows: ThemeShadows,
    /// 边框配置
    pub borders: ThemeBorders,
    /// 动画配置
    pub animations: ThemeAnimations,
}

/// 主题颜色配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeColors {
    /// 主要颜色
    pub primary: String,
    /// 主要颜色悬停状态
    pub primary_hover: String,
    /// 次要颜色
    pub secondary: String,
    /// 次要颜色悬停状态
    pub secondary_hover: String,
    /// 背景色
    pub background: String,
    /// 内容背景色
    pub surface: String,
    /// 文本主色
    pub text_primary: String,
    /// 文本次色
    pub text_secondary: String,
    /// 边框色
    pub border: String,
    /// 成功色
    pub success: String,
    /// 警告色
    pub warning: String,
    /// 错误色
    pub error: String,
    /// 信息色
    pub info: String,
}

/// 主题字体配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeFonts {
    /// 主字体族
    pub family: String,
    /// 代码字体族
    pub mono_family: String,
    /// 基础字体大小
    pub base_size: String,
    /// 行高
    pub line_height: String,
}

/// 主题间距配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeSpacing {
    /// 基础间距单位
    pub unit: String,
    /// 小间距
    pub small: String,
    /// 中等间距
    pub medium: String,
    /// 大间距
    pub large: String,
    /// 超大间距
    pub xlarge: String,
}

/// 主题阴影配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeShadows {
    /// 小阴影
    pub small: String,
    /// 中等阴影
    pub medium: String,
    /// 大阴影
    pub large: String,
    /// 超大阴影
    pub xlarge: String,
}

/// 主题边框配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeBorders {
    /// 边框宽度
    pub width: String,
    /// 边框样式
    pub style: String,
    /// 圆角半径
    pub radius: String,
    /// 大圆角半径
    pub radius_large: String,
}

/// 主题动画配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeAnimations {
    /// 过渡持续时间
    pub duration: String,
    /// 过渡缓动函数
    pub easing: String,
    /// 快速过渡持续时间
    pub duration_fast: String,
    /// 慢速过渡持续时间
    pub duration_slow: String,
}

/// 主题管理器
#[derive(Debug)]
pub struct ThemeManager {
    /// 所有可用主题
    themes: HashMap<String, ThemeConfig>,
    /// 当前激活的主题
    current_theme: String,
}

impl Default for ThemeManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ThemeManager {
    /// 创建新的主题管理器
    pub fn new() -> Self {
        let mut manager = Self {
            themes: HashMap::new(),
            current_theme: "light".to_string(),
        };

        // 注册默认主题
        manager.register_default_themes();
        manager
    }

    /// 注册默认主题
    fn register_default_themes(&mut self) {
        // 明亮主题
        let light_theme = ThemeConfig {
            name: "light".to_string(),
            display_name: "明亮主题".to_string(),
            description: "清新明亮的默认主题".to_string(),
            is_dark: false,
            colors: ThemeColors {
                primary: "#667eea".to_string(),
                primary_hover: "#5a6fd8".to_string(),
                secondary: "#f8f9fa".to_string(),
                secondary_hover: "#e9ecef".to_string(),
                background: "#f5f5f5".to_string(),
                surface: "#ffffff".to_string(),
                text_primary: "#333333".to_string(),
                text_secondary: "#666666".to_string(),
                border: "#e1e5e9".to_string(),
                success: "#10b981".to_string(),
                warning: "#f59e0b".to_string(),
                error: "#ef4444".to_string(),
                info: "#3b82f6".to_string(),
            },
            fonts: ThemeFonts {
                family: "-apple-system, BlinkMacSystemFont, 'Segoe UI', 'Roboto', sans-serif"
                    .to_string(),
                mono_family: "'SF Mono', 'Monaco', 'Consolas', monospace".to_string(),
                base_size: "16px".to_string(),
                line_height: "1.6".to_string(),
            },
            spacing: ThemeSpacing {
                unit: "1rem".to_string(),
                small: "0.5rem".to_string(),
                medium: "1rem".to_string(),
                large: "1.5rem".to_string(),
                xlarge: "2rem".to_string(),
            },
            shadows: ThemeShadows {
                small: "0 1px 3px rgba(0, 0, 0, 0.1)".to_string(),
                medium: "0 2px 8px rgba(0, 0, 0, 0.1)".to_string(),
                large: "0 4px 16px rgba(0, 0, 0, 0.1)".to_string(),
                xlarge: "0 8px 32px rgba(0, 0, 0, 0.15)".to_string(),
            },
            borders: ThemeBorders {
                width: "1px".to_string(),
                style: "solid".to_string(),
                radius: "8px".to_string(),
                radius_large: "12px".to_string(),
            },
            animations: ThemeAnimations {
                duration: "0.3s".to_string(),
                easing: "ease".to_string(),
                duration_fast: "0.15s".to_string(),
                duration_slow: "0.5s".to_string(),
            },
        };

        // 暗色主题
        let dark_theme = ThemeConfig {
            name: "dark".to_string(),
            display_name: "暗色主题".to_string(),
            description: "护眼的暗色主题".to_string(),
            is_dark: true,
            colors: ThemeColors {
                primary: "#7c3aed".to_string(),
                primary_hover: "#8b5cf6".to_string(),
                secondary: "#374151".to_string(),
                secondary_hover: "#4b5563".to_string(),
                background: "#111827".to_string(),
                surface: "#1f2937".to_string(),
                text_primary: "#f9fafb".to_string(),
                text_secondary: "#d1d5db".to_string(),
                border: "#374151".to_string(),
                success: "#10b981".to_string(),
                warning: "#f59e0b".to_string(),
                error: "#ef4444".to_string(),
                info: "#3b82f6".to_string(),
            },
            fonts: light_theme.fonts.clone(),
            spacing: light_theme.spacing.clone(),
            shadows: ThemeShadows {
                small: "0 1px 3px rgba(0, 0, 0, 0.3)".to_string(),
                medium: "0 2px 8px rgba(0, 0, 0, 0.3)".to_string(),
                large: "0 4px 16px rgba(0, 0, 0, 0.4)".to_string(),
                xlarge: "0 8px 32px rgba(0, 0, 0, 0.5)".to_string(),
            },
            borders: light_theme.borders.clone(),
            animations: light_theme.animations.clone(),
        };

        // 蓝色主题
        let blue_theme = ThemeConfig {
            name: "blue".to_string(),
            display_name: "海洋蓝".to_string(),
            description: "清新的海洋蓝主题".to_string(),
            is_dark: false,
            colors: ThemeColors {
                primary: "#0ea5e9".to_string(),
                primary_hover: "#0284c7".to_string(),
                secondary: "#f0f9ff".to_string(),
                secondary_hover: "#e0f2fe".to_string(),
                background: "#f8fafc".to_string(),
                surface: "#ffffff".to_string(),
                text_primary: "#0f172a".to_string(),
                text_secondary: "#475569".to_string(),
                border: "#cbd5e1".to_string(),
                success: "#10b981".to_string(),
                warning: "#f59e0b".to_string(),
                error: "#ef4444".to_string(),
                info: "#0ea5e9".to_string(),
            },
            fonts: light_theme.fonts.clone(),
            spacing: light_theme.spacing.clone(),
            shadows: light_theme.shadows.clone(),
            borders: light_theme.borders.clone(),
            animations: light_theme.animations.clone(),
        };

        // 绿色主题
        let green_theme = ThemeConfig {
            name: "green".to_string(),
            display_name: "自然绿".to_string(),
            description: "清新的自然绿主题".to_string(),
            is_dark: false,
            colors: ThemeColors {
                primary: "#059669".to_string(),
                primary_hover: "#047857".to_string(),
                secondary: "#f0fdf4".to_string(),
                secondary_hover: "#dcfce7".to_string(),
                background: "#f9fafb".to_string(),
                surface: "#ffffff".to_string(),
                text_primary: "#111827".to_string(),
                text_secondary: "#4b5563".to_string(),
                border: "#d1d5db".to_string(),
                success: "#10b981".to_string(),
                warning: "#f59e0b".to_string(),
                error: "#ef4444".to_string(),
                info: "#3b82f6".to_string(),
            },
            fonts: light_theme.fonts.clone(),
            spacing: light_theme.spacing.clone(),
            shadows: light_theme.shadows.clone(),
            borders: light_theme.borders.clone(),
            animations: light_theme.animations.clone(),
        };

        self.themes.insert("light".to_string(), light_theme);
        self.themes.insert("dark".to_string(), dark_theme);
        self.themes.insert("blue".to_string(), blue_theme);
        self.themes.insert("green".to_string(), green_theme);
    }

    /// 获取所有可用主题列表
    pub fn get_themes(&self) -> Vec<&ThemeConfig> {
        self.themes.values().collect()
    }

    /// 获取主题配置
    pub fn get_theme(&self, name: &str) -> Option<&ThemeConfig> {
        self.themes.get(name)
    }

    /// 获取当前主题
    pub fn get_current_theme(&self) -> Option<&ThemeConfig> {
        self.themes.get(&self.current_theme)
    }

    /// 设置当前主题
    pub fn set_current_theme(&mut self, name: &str) -> Result<(), String> {
        if self.themes.contains_key(name) {
            self.current_theme = name.to_string();
            Ok(())
        } else {
            Err(format!("主题 '{}' 不存在", name))
        }
    }

    /// 注册自定义主题
    pub fn register_theme(&mut self, theme: ThemeConfig) {
        self.themes.insert(theme.name.clone(), theme);
    }

    /// 移除主题
    pub fn remove_theme(&mut self, name: &str) -> Result<(), String> {
        if name == "light" || name == "dark" {
            return Err("不能删除默认主题".to_string());
        }

        if self.current_theme == name {
            self.current_theme = "light".to_string();
        }

        self.themes.remove(name);
        Ok(())
    }

    /// 生成主题CSS变量
    pub fn generate_css_variables(&self, theme_name: Option<&str>) -> String {
        let theme = match theme_name {
            Some(name) => self.get_theme(name),
            None => self.get_current_theme(),
        };

        let theme = match theme {
            Some(t) => t,
            None => return String::new(),
        };

        format!(
            r#":root {{
  /* 颜色变量 */
  --color-primary: {};
  --color-primary-hover: {};
  --color-secondary: {};
  --color-secondary-hover: {};
  --color-background: {};
  --color-surface: {};
  --color-text-primary: {};
  --color-text-secondary: {};
  --color-border: {};
  --color-success: {};
  --color-warning: {};
  --color-error: {};
  --color-info: {};

  /* 字体变量 */
  --font-family: {};
  --font-mono-family: {};
  --font-base-size: {};
  --font-line-height: {};

  /* 间距变量 */
  --spacing-unit: {};
  --spacing-small: {};
  --spacing-medium: {};
  --spacing-large: {};
  --spacing-xlarge: {};

  /* 阴影变量 */
  --shadow-small: {};
  --shadow-medium: {};
  --shadow-large: {};
  --shadow-xlarge: {};

  /* 边框变量 */
  --border-width: {};
  --border-style: {};
  --border-radius: {};
  --border-radius-large: {};

  /* 动画变量 */
  --animation-duration: {};
  --animation-easing: {};
  --animation-duration-fast: {};
  --animation-duration-slow: {};

  /* 主题标识 */
  --theme-name: "{}";
  --theme-is-dark: {};
}}"#,
            theme.colors.primary,
            theme.colors.primary_hover,
            theme.colors.secondary,
            theme.colors.secondary_hover,
            theme.colors.background,
            theme.colors.surface,
            theme.colors.text_primary,
            theme.colors.text_secondary,
            theme.colors.border,
            theme.colors.success,
            theme.colors.warning,
            theme.colors.error,
            theme.colors.info,
            theme.fonts.family,
            theme.fonts.mono_family,
            theme.fonts.base_size,
            theme.fonts.line_height,
            theme.spacing.unit,
            theme.spacing.small,
            theme.spacing.medium,
            theme.spacing.large,
            theme.spacing.xlarge,
            theme.shadows.small,
            theme.shadows.medium,
            theme.shadows.large,
            theme.shadows.xlarge,
            theme.borders.width,
            theme.borders.style,
            theme.borders.radius,
            theme.borders.radius_large,
            theme.animations.duration,
            theme.animations.easing,
            theme.animations.duration_fast,
            theme.animations.duration_slow,
            theme.name,
            if theme.is_dark { 1 } else { 0 }
        )
    }

    /// 生成主题选择器HTML
    pub fn generate_theme_selector(&self) -> String {
        let options = self
            .themes
            .values()
            .map(|theme| {
                let selected = if theme.name == self.current_theme {
                    " selected"
                } else {
                    ""
                };
                format!(
                    r#"<option value="{}" data-is-dark="{}"{}>{}</option>"#,
                    theme.name, theme.is_dark, selected, theme.display_name
                )
            })
            .collect::<Vec<_>>()
            .join("");

        format!(
            r#"<div class="theme-selector">
  <label for="theme-select">主题:</label>
  <select id="theme-select" class="theme-select">
    {}
  </select>
</div>"#,
            options
        )
    }

    /// 生成主题切换JavaScript
    pub fn generate_theme_script(&self) -> String {
        r#"
class ThemeManager {
    constructor() {
        this.currentTheme = localStorage.getItem('monolith-theme') || 'light';
        this.initThemeSelector();
        this.applyTheme(this.currentTheme);
    }

    initThemeSelector() {
        const selector = document.getElementById('theme-select');
        if (selector) {
            selector.value = this.currentTheme;
            selector.addEventListener('change', (e) => {
                this.setTheme(e.target.value);
            });
        }
    }

    async setTheme(themeName) {
        try {
            const response = await fetch('/api/theme/set', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({ theme: themeName })
            });

            if (response.ok) {
                this.currentTheme = themeName;
                localStorage.setItem('monolith-theme', themeName);
                this.applyTheme(themeName);
            }
        } catch (error) {
            console.error('Failed to set theme:', error);
        }
    }

    async applyTheme(themeName) {
        try {
            const response = await fetch(`/api/theme/css/${themeName}`);
            if (response.ok) {
                const css = await response.text();
                this.updateStyleSheet(css);
                
                // 更新 body 类名以便其他样式响应
                document.body.className = document.body.className
                    .replace(/theme-\w+/g, '')
                    .trim() + ` theme-${themeName}`;
            }
        } catch (error) {
            console.error('Failed to apply theme:', error);
        }
    }

    updateStyleSheet(css) {
        let styleElement = document.getElementById('theme-variables');
        if (!styleElement) {
            styleElement = document.createElement('style');
            styleElement.id = 'theme-variables';
            document.head.appendChild(styleElement);
        }
        styleElement.textContent = css;
    }

    toggleTheme() {
        const isDark = this.currentTheme === 'dark';
        this.setTheme(isDark ? 'light' : 'dark');
    }

    isCurrentThemeDark() {
        const selector = document.getElementById('theme-select');
        if (selector) {
            const option = selector.querySelector(`option[value="${this.currentTheme}"]`);
            return option && option.dataset.isDark === 'true';
        }
        return this.currentTheme === 'dark';
    }
}

// 初始化主题管理器
document.addEventListener('DOMContentLoaded', () => {
    window.themeManager = new ThemeManager();
});
"#
        .to_string()
    }
}

/// 用户主题偏好设置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserThemePreference {
    /// 用户ID（可选）
    pub user_id: Option<String>,
    /// 首选主题
    pub preferred_theme: String,
    /// 是否自动根据系统主题切换
    pub auto_switch: bool,
    /// 最后更新时间戳（Unix时间戳）
    pub updated_at: i64,
}

impl Default for UserThemePreference {
    fn default() -> Self {
        Self {
            user_id: None,
            preferred_theme: "light".to_string(),
            auto_switch: true,
            updated_at: chrono::Utc::now().timestamp(),
        }
    }
}
