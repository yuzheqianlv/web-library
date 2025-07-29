//! 翻译配置管理模块
//!
//! 这个模块提供了一个简化但功能完整的配置管理系统，用于控制翻译功能的各个方面。
//! 它支持多种配置源的优先级合并：环境变量、配置文件和内置默认值。
//!
//! ## 功能特性
//!
//! - **多源配置**: 支持从 TOML/JSON 配置文件、环境变量和默认值加载配置
//! - **配置验证**: 自动验证配置参数的有效性和一致性
//! - **性能调优**: 提供批处理、并发控制、缓存等性能相关配置
//! - **安全性**: 支持请求限速、文本长度限制等安全机制
//! - **向后兼容**: 提供与旧版配置系统的兼容接口
//!
//! ## 配置优先级
//!
//! 1. 环境变量（最高优先级）
//! 2. 配置文件
//! 3. 默认值（最低优先级）
//!
//! ## 使用示例
//!
//! ```rust
//! use crate::translation::config::{ConfigManager, TranslationConfig};
//!
//! // 创建配置管理器，自动加载和验证配置
//! let manager = ConfigManager::new()?;
//! let config = manager.get_config();
//!
//! // 或者使用便利函数创建简单配置
//! let config = load_translation_config("zh", Some("http://localhost:1188/translate"));
//! ```

pub mod manager;

/// 重新导出主要类型，提供模块的公共接口
///
/// - `ConfigManager`: 配置管理器，负责加载、验证和管理配置
/// - `TranslationConfig`: 翻译配置结构体，包含所有配置参数
pub use manager::{ConfigManager, TranslationConfig};

/// 翻译系统配置常量
///
/// 这个模块包含了翻译系统使用的所有默认值和限制常量。
/// 这些常量被分为几个类别：批处理参数、文本过滤规则、API设置、
/// 缓存配置、HTML处理规则等。
pub mod constants {
    use std::time::Duration;

    // 批次处理相关常量
    
    /// 单个批次的最大文本长度（字符数）
    /// 
    /// 用于限制发送到翻译API的单次请求文本量，避免超出API限制
    pub const MAX_BATCH_SIZE: usize = 9000;
    
    /// 批次处理的最小字符数阈值
    /// 
    /// 只有当文本累积超过此阈值时才会触发批次处理，
    /// 可以减少对短文本的不必要API调用
    pub const DEFAULT_MIN_CHARS: usize = 2000;
    
    /// 批次之间的延迟时间（毫秒）
    /// 
    /// 用于控制连续批次请求之间的间隔，避免对API服务器造成过大压力
    pub const BATCH_DELAY_MS: u64 = 100;
    
    /// 小批次阈值
    /// 
    /// 当批次中的段落数少于此值时，被认为是小批次，
    /// 可能会应用不同的处理策略
    pub const SMALL_BATCH_THRESHOLD: usize = 2;

    // 文本过滤相关常量
    
    /// 文本的最小长度要求
    /// 
    /// 短于此长度的文本将被跳过，不进行翻译处理
    pub const MIN_TEXT_LENGTH: usize = 2;
    
    /// 翻译结果的最小长度要求
    /// 
    /// 翻译结果短于此长度时可能被认为是无效翻译
    pub const MIN_TRANSLATION_LENGTH: usize = 3;
    
    /// 中文字符比例阈值
    /// 
    /// 当文本中中文字符比例超过此值时，可能跳过翻译
    /// （因为原文本已经是中文）
    pub const CHINESE_CHAR_THRESHOLD: f32 = 0.5;
    
    /// 特殊字符比例阈值
    /// 
    /// 当文本中特殊字符（非字母、数字、汉字）比例超过此值时，
    /// 可能跳过翻译（认为是代码、符号等不需要翻译的内容）
    pub const SPECIAL_CHAR_THRESHOLD: f32 = 0.33;

    // 默认API设置
    
    /// 默认的翻译API服务器地址
    /// 
    /// 指向本地运行的翻译服务，用户可以通过配置文件或环境变量覆盖
    pub const DEFAULT_API_URL: &str = "http://localhost:1188/translate";
    
    /// 默认的每秒最大请求数
    /// 
    /// 用于限制API请求频率，避免触发服务器的限流机制
    pub const DEFAULT_MAX_REQUESTS_PER_SECOND: f64 = 5.0;
    
    /// 单次请求的最大文本长度
    /// 
    /// 超过此长度的文本将被分割成多个请求处理
    pub const DEFAULT_MAX_TEXT_LENGTH: usize = 10000;
    
    /// 单次请求的最大段落数
    /// 
    /// 限制每个请求中包含的段落数量，平衡处理效率和API限制
    pub const DEFAULT_MAX_PARAGRAPHS_PER_REQUEST: usize = 100;
    
    /// 默认的最大并发请求数
    /// 
    /// 控制同时进行的翻译请求数量，避免过多并发导致资源耗尽
    pub const DEFAULT_MAX_CONCURRENT_REQUESTS: usize = 10;

    // 缓存设置
    
    /// 默认的缓存生存时间（1小时）
    /// 
    /// 翻译结果在缓存中的保存时间，过期后需要重新翻译
    pub const DEFAULT_CACHE_TTL: Duration = Duration::from_secs(3600);
    
    /// 默认的批次处理超时时间
    /// 
    /// 批次处理操作的最长等待时间，超时后将终止处理
    pub const DEFAULT_BATCH_TIMEOUT: Duration = Duration::from_secs(30);

    // HTML处理相关常量
    
    /// 可翻译的HTML属性列表
    /// 
    /// 这些属性的值通常包含用户可见的文本，需要进行翻译
    /// 包括标题、替代文本、占位符、无障碍标签等
    pub const TRANSLATABLE_ATTRS: &[&str] = &[
        "title",           // 元素标题
        "alt",             // 图片替代文本
        "placeholder",     // 输入框占位符
        "aria-label",      // 无障碍标签
        "aria-description", // 无障碍描述
    ];

    /// 需要跳过翻译的HTML元素列表
    /// 
    /// 这些元素通常包含代码、样式、元数据或功能性内容，
    /// 不应该进行文本翻译处理
    pub const SKIP_ELEMENTS: &[&str] = &[
        "script", "style", "code", "pre",      // 代码和样式相关
        "noscript", "meta", "link", "head",    // 元数据相关
        "svg", "math",                         // 图形和数学公式
        "canvas", "video", "audio",            // 媒体元素
        "embed", "object", "iframe",           // 嵌入内容
        "map", "area", "base",                 // 映射和基础元素
        "br", "hr", "img", "input",            // 空元素和表单
        "source", "track", "wbr",              // 媒体资源和换行
    ];

    /// 功能性词汇列表
    /// 
    /// 这些词汇通常是界面控制元素的文本，具有功能性含义，
    /// 可能不需要翻译或需要特殊处理
    pub const FUNCTIONAL_WORDS: &[&str] = &[
        "ok", "yes", "no",                     // 确认/取消类
        "on", "off", "go", "up",              // 状态/动作类
        "x", ">", "▶", "▲", "▼", "◀", "•",    // 符号类
    ];

    /// 配置文件搜索路径列表
    /// 
    /// 系统将按顺序在这些路径中查找配置文件，
    /// 找到第一个存在的文件就使用它
    /// 支持相对路径、用户目录（~）和系统目录
    pub const CONFIG_PATHS: &[&str] = &[
        "translation-config.toml",              // 当前目录下的专用配置文件
        "config.toml",                          // 当前目录下的通用配置文件
        ".translation-config.toml",             // 隐藏的配置文件
        "~/.config/monolith/translation.toml",  // 用户配置目录
        "/etc/monolith/translation.toml",       // 系统配置目录
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
