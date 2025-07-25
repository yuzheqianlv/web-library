//! 翻译配置管理模块
//!
//! 提供简化的配置管理，支持环境变量、配置文件和默认值

pub mod manager;

// 重新导出主要类型
pub use manager::{ConfigManager, TranslationConfig};

/// 配置常量
pub mod constants {
    use std::time::Duration;

    // 批次处理相关
    pub const MAX_BATCH_SIZE: usize = 9000;
    pub const DEFAULT_MIN_CHARS: usize = 2000;
    pub const BATCH_DELAY_MS: u64 = 100;
    pub const SMALL_BATCH_THRESHOLD: usize = 2;

    // 文本过滤相关
    pub const MIN_TEXT_LENGTH: usize = 2;
    pub const MIN_TRANSLATION_LENGTH: usize = 3;
    pub const CHINESE_CHAR_THRESHOLD: f32 = 0.5;
    pub const SPECIAL_CHAR_THRESHOLD: f32 = 0.33;

    // 默认API设置
    pub const DEFAULT_API_URL: &str = "http://localhost:1188/translate";
    pub const DEFAULT_MAX_REQUESTS_PER_SECOND: f64 = 5.0;
    pub const DEFAULT_MAX_TEXT_LENGTH: usize = 10000;
    pub const DEFAULT_MAX_PARAGRAPHS_PER_REQUEST: usize = 100;
    pub const DEFAULT_MAX_CONCURRENT_REQUESTS: usize = 10;

    // 缓存设置
    pub const DEFAULT_CACHE_TTL: Duration = Duration::from_secs(3600); // 1小时
    pub const DEFAULT_BATCH_TIMEOUT: Duration = Duration::from_secs(30);

    // 可翻译属性
    pub const TRANSLATABLE_ATTRS: &[&str] = &[
        "title", "alt", "placeholder", "aria-label", "aria-description",
    ];

    // 跳过的元素
    pub const SKIP_ELEMENTS: &[&str] = &[
        "script", "style", "code", "pre", "noscript", "meta", "link", "head", 
        "svg", "math", "canvas", "video", "audio", "embed", "object", "iframe", 
        "map", "area", "base", "br", "hr", "img", "input", "source", "track", "wbr",
    ];

    // 功能性词汇
    pub const FUNCTIONAL_WORDS: &[&str] = &[
        "ok", "yes", "no", "on", "off", "go", "up", "x", ">", "▶", "▲", "▼", "◀", "•"
    ];

    // 配置文件搜索路径
    pub const CONFIG_PATHS: &[&str] = &[
        "translation-config.toml",
        "config.toml", 
        ".translation-config.toml",
        "~/.config/monolith/translation.toml",
        "/etc/monolith/translation.toml",
    ];
}

/// 便利函数
pub fn config_file_exists() -> bool {
    constants::CONFIG_PATHS
        .iter()
        .any(|path| std::path::Path::new(path).exists())
}

pub fn get_min_translation_chars() -> usize {
    constants::DEFAULT_MIN_CHARS
}

/// 向后兼容的配置加载函数
pub fn load_translation_config(target_lang: &str, api_url: Option<&str>) -> TranslationConfig {
    match ConfigManager::new() {
        Ok(manager) => match manager.create_simple_config(target_lang, api_url) {
            Ok(config) => config,
            Err(e) => {
                tracing::warn!("配置加载失败，使用默认配置: {}", e);
                TranslationConfig::default_with_lang(target_lang, api_url)
            }
        },
        Err(e) => {
            tracing::warn!("创建配置管理器失败，使用默认配置: {}", e);
            TranslationConfig::default_with_lang(target_lang, api_url)
        }
    }
}