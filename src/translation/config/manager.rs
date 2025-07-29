//! 翻译功能配置管理器
//!
//! 这个模块提供了一个统一的配置管理系统，用于处理翻译功能的各种配置选项。
//! 支持多种配置来源的优先级合并：文件配置、环境变量和默认值。
//!
//! ## 配置优先级
//! 
//! 1. 环境变量（最高优先级）
//! 2. 配置文件（TOML/JSON格式）
//! 3. 默认值（最低优先级）
//!
//! ## 支持的配置文件格式
//!
//! - TOML 格式 (推荐): `config.toml`
//! - JSON 格式: `config.json`
//!
//! ## 配置文件搜索路径
//!
//! 配置管理器会按顺序在以下位置搜索配置文件：
//! - `./config.toml`
//! - `~/.config/monolith/config.toml`
//! - `/etc/monolith/config.toml`
//!
//! ## 环境变量支持
//!
//! 通过类型安全的环境变量系统，支持运行时覆盖配置文件中的设置。
//! 环境变量会自动映射到相应的配置字段。
//!
//! ## 使用示例
//!
//! ```rust
//! use crate::translation::config::manager::ConfigManager;
//!
//! // 创建配置管理器，会自动加载配置文件和环境变量
//! let manager = ConfigManager::new()?;
//! let config = manager.get_config();
//!
//! // 创建特定语言的配置
//! let zh_config = manager.create_simple_config("zh", None)?;
//! ```

use std::path::Path;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::constants;
use crate::translation::error::{TranslationError, TranslationResult};

/// 翻译功能配置结构体
/// 
/// 这个结构体包含了翻译系统的所有配置选项，分为以下几个类别：
/// - 基础配置：控制翻译的基本行为
/// - 性能配置：控制请求频率和并发度
/// - 批次配置：控制批量翻译的行为
/// - 缓存配置：控制翻译结果的缓存策略
/// - 功能开关：控制各种功能特性的启用状态
///
/// ## 配置文件示例（TOML格式）
///
/// ```toml
/// # 基础配置
/// enabled = true
/// target_lang = "zh"
/// source_lang = "auto"
/// api_url = "http://localhost:1188/translate"
///
/// # 性能配置
/// max_requests_per_second = 10.0
/// max_text_length = 5000
/// max_paragraphs_per_request = 50
/// max_concurrent_requests = 5
///
/// # 批次配置
/// batch_size = 20
/// min_batch_chars = 100
/// batch_timeout_secs = 30
///
/// # 缓存配置
/// cache_enabled = true
/// cache_ttl_secs = 3600
/// local_cache_size = 1000
///
/// # 功能开关
/// parallel_enabled = true
/// retry_enabled = true
/// max_retry_attempts = 3
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TranslationConfig {
    /// 是否启用翻译功能
    /// 
    /// 当设置为 `false` 时，翻译系统将跳过所有翻译操作，
    /// 直接返回原始文本。这对于调试或临时禁用翻译很有用。
    pub enabled: bool,
    
    /// 目标翻译语言代码
    /// 
    /// 使用 ISO 639-1 标准的语言代码，例如：
    /// - "zh": 中文
    /// - "en": 英文
    /// - "ja": 日文
    /// - "ko": 韩文
    pub target_lang: String,
    
    /// 源语言代码
    /// 
    /// 使用 "auto" 表示自动检测源语言，
    /// 或者指定具体的语言代码如 "en"、"ja" 等。
    pub source_lang: String,
    
    /// 翻译 API 的基础 URL
    /// 
    /// 翻译服务的端点地址，通常是 DeepLX 或其他兼容的翻译服务。
    /// 例如：`http://localhost:1188/translate`
    pub api_url: String,

    /// 每秒最大请求数限制
    /// 
    /// 用于控制对翻译 API 的请求频率，避免触发服务端的限流。
    /// 支持小数值，例如 0.5 表示每 2 秒最多 1 个请求。
    pub max_requests_per_second: f64,
    
    /// 单次翻译文本的最大长度（字符数）
    /// 
    /// 超过此长度的文本将被分割成多个较小的片段进行翻译。
    /// 这有助于避免 API 的文本长度限制。
    pub max_text_length: usize,
    
    /// 单次请求中最大段落数
    /// 
    /// 控制批量翻译时单个请求包含的段落数量上限，
    /// 有助于平衡请求效率和响应时间。
    pub max_paragraphs_per_request: usize,
    
    /// 最大并发请求数
    /// 
    /// 控制同时进行的翻译请求数量，需要根据翻译服务的性能
    /// 和网络状况进行调整。
    pub max_concurrent_requests: usize,

    /// 批处理的批次大小
    /// 
    /// 控制一次批处理操作中包含的文本项目数量。
    /// 较大的批次可以提高效率，但会增加内存使用。
    pub batch_size: usize,
    
    /// 批处理的最小字符数阈值
    /// 
    /// 只有当待翻译文本总字符数超过此阈值时，
    /// 才会触发批处理模式。
    pub min_batch_chars: usize,
    
    /// 批处理超时时间（秒）
    /// 
    /// 批处理操作的最大等待时间，超过此时间未完成的操作将被取消。
    pub batch_timeout_secs: u64,

    /// 是否启用翻译结果缓存
    /// 
    /// 启用后会将翻译结果缓存在内存中，相同文本的后续翻译
    /// 请求将直接返回缓存结果，提高响应速度。
    pub cache_enabled: bool,
    
    /// 缓存项的生存时间（秒）
    /// 
    /// 缓存项在指定时间后会自动过期，避免使用过时的翻译结果。
    pub cache_ttl_secs: u64,
    
    /// 本地缓存的最大条目数
    /// 
    /// 控制内存中缓存的最大条目数量，防止缓存无限增长
    /// 导致内存泄漏。
    pub local_cache_size: usize,

    /// 是否启用并行翻译
    /// 
    /// 启用后多个翻译任务可以并行执行，提高整体翻译速度。
    /// 禁用后翻译任务将串行执行。
    pub parallel_enabled: bool,
    
    /// 是否启用自动重试机制
    /// 
    /// 启用后翻译请求失败时会自动重试，提高翻译的可靠性。
    pub retry_enabled: bool,
    
    /// 最大重试次数
    /// 
    /// 翻译请求失败时的最大重试次数。只有在 `retry_enabled` 
    /// 为 `true` 时才会生效。
    pub max_retry_attempts: usize,
}

impl TranslationConfig {
    /// 创建默认配置实例
    /// 
    /// 返回一个包含所有默认值的配置实例。默认配置适用于大多数使用场景，
    /// 可以作为配置的起始点进行进一步的自定义。
    /// 
    /// ## 默认值说明
    /// 
    /// - 翻译功能：启用
    /// - 目标语言：中文 (`zh`)
    /// - 源语言：自动检测 (`auto`)
    /// - API URL：使用常量中定义的默认地址
    /// - 请求限制：每秒10个请求，最多5个并发
    /// - 缓存：启用，TTL为1小时，最多缓存1000项
    /// 
    /// ## 示例
    /// 
    /// ```rust
    /// let config = TranslationConfig::default();
    /// assert_eq!(config.target_lang, "zh");
    /// assert!(config.enabled);
    /// ```
    /// 
    /// ## 返回值
    /// 
    /// 返回配置了所有默认值的 `TranslationConfig` 实例
    pub fn default() -> Self {
        Self {
            enabled: true,
            target_lang: "zh".to_string(),
            source_lang: "auto".to_string(),
            api_url: constants::DEFAULT_API_URL.to_string(),

            max_requests_per_second: constants::DEFAULT_MAX_REQUESTS_PER_SECOND,
            max_text_length: constants::DEFAULT_MAX_TEXT_LENGTH,
            max_paragraphs_per_request: constants::DEFAULT_MAX_PARAGRAPHS_PER_REQUEST,
            max_concurrent_requests: constants::DEFAULT_MAX_CONCURRENT_REQUESTS,

            batch_size: constants::MAX_BATCH_SIZE,
            min_batch_chars: constants::DEFAULT_MIN_CHARS,
            batch_timeout_secs: constants::DEFAULT_BATCH_TIMEOUT.as_secs(),

            cache_enabled: true,
            cache_ttl_secs: constants::DEFAULT_CACHE_TTL.as_secs(),
            local_cache_size: 1000,

            parallel_enabled: true,
            retry_enabled: true,
            max_retry_attempts: 3,
        }
    }

    /// 创建带指定目标语言的默认配置
    /// 
    /// 基于默认配置创建一个新实例，但使用自定义的目标语言和可选的API URL。
    /// 这是一个便利方法，用于快速创建特定语言的配置。
    /// 
    /// ## 参数
    /// 
    /// - `target_lang`: 目标翻译语言的语言代码（如 "zh", "en", "ja"）
    /// - `api_url`: 可选的翻译API URL，如果为 `None` 则使用默认URL
    /// 
    /// ## 示例
    /// 
    /// ```rust
    /// // 创建日文翻译配置，使用默认API URL
    /// let ja_config = TranslationConfig::default_with_lang("ja", None);
    /// 
    /// // 创建英文翻译配置，使用自定义API URL
    /// let en_config = TranslationConfig::default_with_lang(
    ///     "en", 
    ///     Some("http://custom-api.example.com/translate")
    /// );
    /// ```
    /// 
    /// ## 返回值
    /// 
    /// 返回配置了指定语言和API URL的 `TranslationConfig` 实例
    pub fn default_with_lang(target_lang: &str, api_url: Option<&str>) -> Self {
        let mut config = Self::default();
        config.target_lang = target_lang.to_string();
        if let Some(url) = api_url {
            config.api_url = url.to_string();
        }
        config
    }

    /// 验证配置的有效性
    /// 
    /// 检查配置中的各项参数是否合理和有效。这个方法会验证关键配置项
    /// 的取值范围和逻辑一致性，确保配置能够正常工作。
    /// 
    /// ## 验证规则
    /// 
    /// - `batch_size` 必须大于 0
    /// - `max_concurrent_requests` 必须大于 0
    /// - `max_requests_per_second` 必须大于 0.0
    /// - 当启用缓存时，`local_cache_size` 必须大于 0
    /// 
    /// ## 返回值
    /// 
    /// - `Ok(())`: 配置验证通过
    /// - `Err(TranslationError::ConfigError)`: 配置验证失败，错误信息包含具体的问题描述
    /// 
    /// ## 示例
    /// 
    /// ```rust
    /// let mut config = TranslationConfig::default();
    /// 
    /// // 有效配置
    /// assert!(config.validate().is_ok());
    /// 
    /// // 无效配置
    /// config.batch_size = 0;
    /// assert!(config.validate().is_err());
    /// ```
    /// 
    /// ## 错误
    /// 
    /// 当配置参数不符合要求时，返回 `TranslationError::ConfigError`
    pub fn validate(&self) -> TranslationResult<()> {
        if self.batch_size == 0 {
            return Err(TranslationError::ConfigError("批次大小不能为0".to_string()));
        }

        if self.max_concurrent_requests == 0 {
            return Err(TranslationError::ConfigError(
                "最大并发数不能为0".to_string(),
            ));
        }

        if self.max_requests_per_second <= 0.0 {
            return Err(TranslationError::ConfigError(
                "请求速率必须大于0".to_string(),
            ));
        }

        if self.cache_enabled && self.local_cache_size == 0 {
            return Err(TranslationError::ConfigError(
                "启用缓存时缓存大小不能为0".to_string(),
            ));
        }

        Ok(())
    }

    /// 应用环境变量覆盖配置
    /// 
    /// 使用类型安全的环境变量系统来覆盖当前配置的字段值。
    /// 这个方法会检查所有相关的环境变量，如果存在则用环境变量的值
    /// 覆盖配置文件中的对应设置。
    /// 
    /// ## 支持的环境变量
    /// 
    /// ### 翻译相关
    /// - `TRANSLATION_ENABLED`: 是否启用翻译
    /// - `TRANSLATION_TARGET_LANG`: 目标语言
    /// - `TRANSLATION_SOURCE_LANG`: 源语言
    /// - `TRANSLATION_API_URL`: API URL
    /// - `TRANSLATION_MAX_REQUESTS_PER_SECOND`: 最大请求频率
    /// - `TRANSLATION_MAX_CONCURRENT_REQUESTS`: 最大并发数
    /// - `TRANSLATION_BATCH_SIZE`: 批次大小
    /// - `TRANSLATION_BATCH_TIMEOUT`: 批次超时时间
    /// 
    /// ### 缓存相关
    /// - `CACHE_ENABLED`: 是否启用缓存
    /// - `CACHE_LOCAL_CACHE_SIZE`: 本地缓存大小
    /// - `CACHE_TTL`: 缓存生存时间
    /// 
    /// ## 环境变量优先级
    /// 
    /// 环境变量具有最高优先级，会覆盖配置文件中的对应设置。
    /// 这对于容器化部署和不同环境的配置管理非常有用。
    /// 
    /// ## 示例
    /// 
    /// ```bash
    /// export TRANSLATION_TARGET_LANG=en
    /// export TRANSLATION_API_URL=http://localhost:8080/translate
    /// export CACHE_ENABLED=false
    /// ```
    /// 
    /// ```rust
    /// let mut config = TranslationConfig::default();
    /// config.apply_env_overrides();
    /// // 配置现在会反映环境变量中的设置
    /// ```
    /// 
    /// ## 注意事项
    /// 
    /// - 环境变量值必须能够正确解析为对应的类型
    /// - 无效的环境变量值会被忽略，继续使用原有配置
    /// - API URL 的覆盖会记录到日志中
    pub fn apply_env_overrides(&mut self) {
        use crate::env::{cache, translation, EnvVar};

        // 翻译相关环境变量
        if let Ok(enabled) = translation::Enabled::get() {
            self.enabled = enabled;
        }

        if let Ok(target_lang) = translation::TargetLang::get() {
            self.target_lang = target_lang;
        }

        if let Ok(source_lang) = translation::SourceLang::get() {
            self.source_lang = source_lang;
        }

        if let Ok(api_url) = translation::ApiUrl::get() {
            self.api_url = api_url;
            tracing::info!("环境变量覆盖 API URL: {}", self.api_url);
        }

        if let Ok(rate) = translation::MaxRequestsPerSecond::get() {
            self.max_requests_per_second = rate;
        }

        if let Ok(max_concurrent) = translation::MaxConcurrentRequests::get() {
            self.max_concurrent_requests = max_concurrent;
        }

        if let Ok(batch_size) = translation::BatchSize::get() {
            self.batch_size = batch_size;
        }

        if let Ok(batch_timeout) = translation::BatchTimeout::get() {
            self.batch_timeout_secs = batch_timeout.as_secs();
        }

        // 缓存相关环境变量
        if let Ok(cache_enabled) = cache::Enabled::get() {
            self.cache_enabled = cache_enabled;
        }

        if let Ok(cache_size) = cache::LocalCacheSize::get() {
            self.local_cache_size = cache_size;
        }

        if let Ok(cache_ttl) = cache::Ttl::get() {
            self.cache_ttl_secs = cache_ttl.as_secs();
        }
    }

    /// 获取批处理超时时间的 Duration 表示
    /// 
    /// 将配置中的批处理超时时间（秒数）转换为 `std::time::Duration` 类型，
    /// 便于在需要 Duration 类型的 API 中使用。
    /// 
    /// ## 返回值
    /// 
    /// 返回表示批处理超时时间的 `Duration` 实例
    /// 
    /// ## 示例
    /// 
    /// ```rust
    /// let config = TranslationConfig::default();
    /// let timeout = config.batch_timeout();
    /// 
    /// // 可以直接用于 tokio 的超时操作
    /// // let result = tokio::time::timeout(timeout, some_operation()).await;
    /// ```
    pub fn batch_timeout(&self) -> Duration {
        Duration::from_secs(self.batch_timeout_secs)
    }

    /// 获取缓存生存时间的 Duration 表示
    /// 
    /// 将配置中的缓存TTL时间（秒数）转换为 `std::time::Duration` 类型，
    /// 便于在缓存系统中使用。
    /// 
    /// ## 返回值
    /// 
    /// 返回表示缓存生存时间的 `Duration` 实例
    /// 
    /// ## 示例
    /// 
    /// ```rust
    /// let config = TranslationConfig::default();
    /// let ttl = config.cache_ttl();
    /// 
    /// // 可以用于设置缓存过期时间
    /// // cache.insert_with_ttl(key, value, ttl);
    /// ```
    pub fn cache_ttl(&self) -> Duration {
        Duration::from_secs(self.cache_ttl_secs)
    }
}

impl Default for TranslationConfig {
    fn default() -> Self {
        Self::default()
    }
}

/// 翻译配置管理器
/// 
/// 这个结构体负责管理翻译系统的配置，提供了统一的配置加载、验证和访问接口。
/// 配置管理器支持从多种来源加载配置，并按优先级进行合并。
/// 
/// ## 配置加载顺序
/// 
/// 1. **加载配置文件**：按预定义路径搜索 TOML 或 JSON 格式的配置文件
/// 2. **加载环境变量文件**：自动检测并加载 `.env` 系列文件
/// 3. **应用环境变量覆盖**：使用环境变量覆盖配置文件中的设置
/// 4. **配置验证**：确保所有配置参数的有效性
/// 
/// ## 使用方法
/// 
/// ### 基本用法
/// 
/// ```rust
/// use crate::translation::config::manager::ConfigManager;
/// 
/// // 创建配置管理器，自动加载和验证配置
/// let manager = ConfigManager::new()?;
/// 
/// // 获取当前配置
/// let config = manager.get_config();
/// println!("目标语言: {}", config.target_lang);
/// ```
/// 
/// ### 创建自定义配置
/// 
/// ```rust
/// // 创建特定语言的配置
/// let zh_config = manager.create_simple_config("zh", None)?;
/// let en_config = manager.create_simple_config("en", Some("http://localhost:8080/translate"))?;
/// 
/// // 创建向后兼容的配置
/// let legacy_config = manager.create_legacy_config("zh", Some("http://api.example.com"))?;
/// ```
/// 
/// ### 生成示例配置文件
/// 
/// ```rust
/// // 生成带有所有默认值的示例配置文件
/// ConfigManager::generate_example_config("config.toml")?;
/// ```
/// 
/// ## 特性
/// 
/// - **自动配置发现**：在标准路径中自动搜索配置文件
/// - **环境变量集成**：支持通过环境变量动态覆盖配置
/// - **配置验证**：确保配置参数的合理性和一致性
/// - **多格式支持**：同时支持 TOML 和 JSON 格式的配置文件
/// - **向后兼容**：提供与旧版本配置系统的兼容接口
/// - **错误处理**：提供详细的错误信息和处理机制
/// 
/// ## 配置文件搜索路径
/// 
/// 配置管理器会按以下顺序搜索配置文件：
/// 1. `./config.toml` - 当前目录
/// 2. `~/.config/monolith/config.toml` - 用户配置目录
/// 3. `/etc/monolith/config.toml` - 系统配置目录
/// 
/// ## 环境变量文件加载
/// 
/// 自动检测并加载以下环境变量文件（按优先级）：
/// 1. `.env.local` - 本地开发配置
/// 2. `.env.development` - 开发环境配置
/// 3. `.env.production` - 生产环境配置
/// 4. `.env` - 通用环境配置
pub struct ConfigManager {
    /// 当前有效的翻译配置
    /// 
    /// 这个字段存储经过加载、环境变量覆盖和验证后的最终配置。
    /// 所有配置相关的操作都基于这个字段进行。
    config: TranslationConfig,
}

impl ConfigManager {
    /// 创建新的配置管理器实例
    /// 
    /// 这是配置管理器的主要入口点，执行完整的配置初始化流程：
    /// 
    /// 1. **配置文件加载**：在标准路径中搜索并加载配置文件
    /// 2. **环境变量处理**：加载 `.env` 文件并应用环境变量覆盖
    /// 3. **配置验证**：验证所有配置参数的有效性
    /// 4. **实例创建**：创建包含最终配置的管理器实例
    /// 
    /// ## 配置加载流程
    /// 
    /// ```text
    /// 1. 搜索配置文件 (config.toml/config.json)
    ///    ├─ ./config.toml
    ///    ├─ ~/.config/monolith/config.toml  
    ///    └─ /etc/monolith/config.toml
    /// 
    /// 2. 加载环境变量文件
    ///    ├─ .env.local (最高优先级)
    ///    ├─ .env.development
    ///    ├─ .env.production
    ///    └─ .env
    /// 
    /// 3. 应用环境变量覆盖
    ///    └─ TRANSLATION_*, CACHE_* 等变量
    /// 
    /// 4. 验证配置有效性
    ///    └─ 检查参数范围和逻辑一致性
    /// ```
    /// 
    /// ## 返回值
    /// 
    /// - `Ok(ConfigManager)`: 成功创建配置管理器实例
    /// - `Err(TranslationError)`: 配置加载、验证失败或其他错误
    /// 
    /// ## 错误情况
    /// 
    /// - 配置文件格式错误（TOML/JSON 解析失败）
    /// - 配置参数验证失败（如批次大小为0）
    /// - 文件系统访问错误（权限不足等）
    /// 
    /// ## 示例
    /// 
    /// ```rust
    /// use crate::translation::config::manager::ConfigManager;
    /// 
    /// // 创建配置管理器
    /// match ConfigManager::new() {
    ///     Ok(manager) => {
    ///         println!("配置加载成功");
    ///         let config = manager.get_config();
    ///         println!("目标语言: {}", config.target_lang);
    ///     }
    ///     Err(e) => {
    ///         eprintln!("配置加载失败: {}", e);
    ///     }
    /// }
    /// ```
    /// 
    /// ## 日志输出
    /// 
    /// 此方法会产生以下日志信息：
    /// - 配置文件加载路径
    /// - 环境变量文件加载状态
    /// - 环境变量覆盖情况
    pub fn new() -> TranslationResult<Self> {
        // 1. 加载基础配置（从文件或使用默认值）
        let mut config = Self::load_config()?;
        
        // 2. 应用环境变量覆盖（环境变量优先级最高）
        config.apply_env_overrides();
        
        // 3. 验证最终配置的有效性
        config.validate()?;

        Ok(Self { config })
    }

    /// 获取当前有效的翻译配置
    /// 
    /// 返回经过完整初始化流程（文件加载、环境变量覆盖、验证）后的
    /// 最终配置。这个配置可以直接用于翻译系统的各个组件。
    /// 
    /// ## 返回值
    /// 
    /// 返回 `TranslationConfig` 的不可变引用，包含所有已验证的配置参数。
    /// 
    /// ## 配置内容
    /// 
    /// 返回的配置包含以下主要类别：
    /// - **基础设置**：启用状态、语言配置、API URL
    /// - **性能参数**：请求频率限制、并发控制、文本长度限制
    /// - **批处理设置**：批次大小、超时时间、字符数阈值
    /// - **缓存配置**：缓存开关、TTL、本地缓存大小
    /// - **功能开关**：并行处理、重试机制等
    /// 
    /// ## 使用示例
    /// 
    /// ```rust
    /// let manager = ConfigManager::new()?;
    /// let config = manager.get_config();
    /// 
    /// // 检查翻译功能是否启用
    /// if config.enabled {
    ///     println!("翻译功能已启用，目标语言: {}", config.target_lang);
    ///     println!("API URL: {}", config.api_url);
    ///     println!("最大并发请求数: {}", config.max_concurrent_requests);
    /// }
    /// 
    /// // 获取性能相关配置
    /// println!("请求频率限制: {} 请求/秒", config.max_requests_per_second);
    /// println!("批次大小: {}", config.batch_size);
    /// ```
    /// 
    /// ## 注意事项
    /// 
    /// - 返回的是配置的引用，避免了不必要的复制开销
    /// - 配置在管理器创建后是不可变的，确保了配置的一致性
    /// - 如需修改配置，应创建新的管理器实例
    pub fn get_config(&self) -> &TranslationConfig {
        &self.config
    }

    /// 创建自定义的翻译配置（性能优化版）
    /// 
    /// 基于当前管理器的配置创建一个新的配置实例，允许自定义目标语言和API URL。
    /// 这个方法经过性能优化，当参数与当前配置相同时会避免不必要的克隆操作。
    /// 
    /// ## 参数
    /// 
    /// - `target_lang`: 目标翻译语言代码（如 "zh", "en", "ja", "ko"）
    /// - `api_url`: 可选的翻译API URL，如果为 `None` 则使用当前配置的URL
    /// 
    /// ## 性能优化特性
    /// 
    /// - **智能克隆检测**：只有在参数确实不同时才进行配置克隆
    /// - **选择性修改**：只修改需要变更的字段，保持其他配置不变
    /// - **引用比较**：通过字符串比较避免不必要的对象创建
    /// 
    /// ## 返回值
    /// 
    /// 返回自定义后的 `TranslationConfig` 实例，包含指定的语言和API设置，
    /// 其他配置参数继承自当前管理器的配置。
    /// 
    /// ## 使用场景
    /// 
    /// - 为不同语言创建专用配置
    /// - 在多API环境中切换翻译服务
    /// - 批量处理多语言内容
    /// - 测试不同的翻译配置
    /// 
    /// ## 示例
    /// 
    /// ```rust
    /// let manager = ConfigManager::new()?;
    /// 
    /// // 创建中文翻译配置
    /// let zh_config = manager.create_simple_config("zh", None)?;
    /// 
    /// // 创建英文翻译配置，使用自定义API
    /// let en_config = manager.create_simple_config(
    ///     "en", 
    ///     Some("http://custom-api.example.com/translate")
    /// )?;
    /// 
    /// // 创建日文翻译配置
    /// let ja_config = manager.create_simple_config("ja", None)?;
    /// 
    /// println!("中文配置目标语言: {}", zh_config.target_lang);
    /// println!("英文配置API: {}", en_config.api_url);
    /// ```
    /// 
    /// ## 配置继承
    /// 
    /// 新配置会继承管理器中的所有其他设置：
    /// - 性能参数（请求频率、并发数等）
    /// - 批处理设置（批次大小、超时时间等）
    /// - 缓存配置（TTL、缓存大小等）
    /// - 功能开关（并行处理、重试机制等）
    pub fn create_simple_config(
        &self,
        target_lang: &str,
        api_url: Option<&str>,
    ) -> TranslationResult<TranslationConfig> {
        // 性能优化：检查参数是否与当前配置相同
        // 如果目标语言相同且API URL也相同（或未指定），则直接返回当前配置的克隆
        if self.config.target_lang == target_lang
            && api_url.map_or(true, |url| url == self.config.api_url)
        {
            // 无需修改，直接返回当前配置的克隆
            return Ok(self.config.clone());
        }

        // 需要修改配置，创建新的配置实例
        let mut config = self.config.clone();
        
        // 更新目标语言
        config.target_lang = target_lang.to_string();
        
        // 如果提供了新的API URL，则更新
        if let Some(url) = api_url {
            config.api_url = url.to_string();
        }
        
        Ok(config)
    }

    /// 创建向后兼容的传统配置
    /// 
    /// 为了保持与旧版本 `markdown_translator` 库的兼容性，这个方法创建
    /// 传统格式的配置对象。主要用于需要与旧代码集成的场景。
    /// 
    /// ## 参数
    /// 
    /// - `_target_lang`: 目标语言（当前版本中未使用，保留用于向后兼容）
    /// - `api_url`: 可选的翻译API URL，如果为 `None` 则使用当前配置的URL
    /// 
    /// ## 配置映射
    /// 
    /// 传统配置的字段映射关系：
    /// - `enabled`: 固定为 `true`（启用翻译）
    /// - `source_lang`: 固定为 `"auto"`（自动检测源语言）
    /// - `deeplx_api_url`: 使用提供的URL或当前配置的API URL
    /// - 其他字段：使用 `markdown_translator` 的默认值
    /// 
    /// ## 返回值
    /// 
    /// 返回 `markdown_translator::TranslationConfig` 实例，
    /// 可以直接用于旧版本的翻译系统。
    /// 
    /// ## 使用场景
    /// 
    /// - 与使用 `markdown_translator` 库的旧代码集成
    /// - 渐进式系统升级过程中的过渡方案
    /// - 维持API兼容性的插件系统
    /// 
    /// ## 示例
    /// 
    /// ```rust
    /// let manager = ConfigManager::new()?;
    /// 
    /// // 创建传统格式的配置
    /// let legacy_config = manager.create_legacy_config(
    ///     "zh", 
    ///     Some("http://legacy-api.example.com/translate")
    /// )?;
    /// 
    /// // 可以直接用于旧版本的翻译器
    /// // let translator = markdown_translator::Translator::new(legacy_config);
    /// ```
    /// 
    /// ## 注意事项
    /// 
    /// - 目标语言参数当前未使用，但保留以保持API兼容性
    /// - 传统配置使用固定的默认值，可能与当前管理器配置不完全一致
    /// - 建议在新代码中使用 `create_simple_config()` 方法
    pub fn create_legacy_config(
        &self,
        _target_lang: &str,  // 保留参数以维持API兼容性，但当前版本未使用
        api_url: Option<&str>,
    ) -> TranslationResult<markdown_translator::TranslationConfig> {
        Ok(markdown_translator::TranslationConfig {
            enabled: true,  // 传统配置始终启用翻译功能
            source_lang: "auto".to_string(),  // 自动检测源语言
            deeplx_api_url: api_url.unwrap_or(&self.config.api_url).to_string(),
            ..Default::default()  // 其他字段使用默认值
        })
    }

    /// 从配置文件中加载翻译配置
    /// 
    /// 这个私有方法负责从文件系统中加载配置。它会按照预定义的优先级顺序
    /// 搜索配置文件，并在找不到任何配置文件时返回默认配置。
    /// 
    /// ## 加载流程
    /// 
    /// 1. **环境变量文件**：首先尝试加载 `.env` 系列文件
    /// 2. **配置文件搜索**：在预定义路径中搜索配置文件
    /// 3. **文件加载**：找到第一个存在的配置文件并加载
    /// 4. **默认配置**：如果没找到配置文件，使用默认配置
    /// 
    /// ## 配置文件搜索路径
    /// 
    /// 按以下优先级顺序搜索：
    /// 1. `./config.toml` - 当前工作目录（最高优先级）
    /// 2. `~/.config/monolith/config.toml` - 用户配置目录
    /// 3. `/etc/monolith/config.toml` - 系统配置目录
    /// 
    /// ## 支持的文件格式
    /// 
    /// - **TOML 格式**：`.toml` 扩展名（推荐）
    /// - **JSON 格式**：`.json` 扩展名
    /// 
    /// ## 返回值
    /// 
    /// - `Ok(TranslationConfig)`: 成功加载的配置或默认配置
    /// - `Err(TranslationError)`: 配置文件读取或解析失败
    /// 
    /// ## 日志输出
    /// 
    /// - 成功加载配置文件时输出文件路径
    /// - 未找到配置文件时输出使用默认配置的信息
    /// - 环境变量文件加载状态
    /// 
    /// ## 实现细节
    /// 
    /// - 支持 `~` 符号家目录扩展
    /// - 使用文件扩展名自动识别文件格式
    /// - 在找到第一个有效配置文件后立即停止搜索
    fn load_config() -> TranslationResult<TranslationConfig> {
        // 1. 首先尝试加载环境变量文件（.env 系列）
        Self::load_dotenv();

        // 2. 按优先级顺序搜索配置文件
        for path in constants::CONFIG_PATHS {
            // 支持 ~ 符号家目录扩展
            let expanded_path = shellexpand::tilde(path);
            
            // 检查文件是否存在
            if Path::new(expanded_path.as_ref()).exists() {
                tracing::info!("加载配置文件: {}", expanded_path);
                // 找到第一个有效配置文件，立即加载并返回
                return Self::load_from_file(&expanded_path);
            }
        }

        // 3. 未找到任何配置文件，使用默认配置
        tracing::info!("未找到配置文件，使用默认配置");
        Ok(TranslationConfig::default())
    }

    /// 从指定文件加载配置内容
    /// 
    /// 这个私有方法负责从具体的配置文件路径读取并解析配置内容。
    /// 支持多种文件格式，并根据文件扩展名自动选择解析器。
    /// 
    /// ## 参数
    /// 
    /// - `path`: 配置文件的完整路径（绝对路径或相对路径）
    /// 
    /// ## 支持的文件格式
    /// 
    /// - **TOML 格式**：文件扩展名为 `.toml`
    ///   - 使用 `toml::from_str()` 进行解析
    ///   - 支持完整的 TOML 语法和特性
    /// 
    /// - **JSON 格式**：非 `.toml` 扩展名的文件
    ///   - 使用 `serde_json::from_str()` 进行解析
    ///   - 支持标准 JSON 格式
    /// 
    /// ## 返回值
    /// 
    /// - `Ok(TranslationConfig)`: 成功解析的配置对象
    /// - `Err(TranslationError::ConfigError)`: 文件读取或解析失败
    /// 
    /// ## 错误情况
    /// 
    /// ### 文件访问错误
    /// - 文件不存在或没有读取权限
    /// - 文件被其他进程锁定
    /// - 磁盘空间不足或I/O错误
    /// 
    /// ### 格式解析错误
    /// - TOML 语法错误（如括号不匹配、缩进错误）
    /// - JSON 语法错误（如未闭合的大括号、多余逗号）
    /// - 类型不匹配（如字符串传给整数字段）
    /// 
    /// ## 示例配置文件
    /// 
    /// ### TOML 格式 (config.toml)
    /// ```toml
    /// enabled = true
    /// target_lang = "zh"
    /// api_url = "http://localhost:1188/translate"
    /// max_requests_per_second = 10.0
    /// batch_size = 20
    /// cache_enabled = true
    /// ```
    /// 
    /// ### JSON 格式 (config.json)
    /// ```json
    /// {
    ///   "enabled": true,
    ///   "target_lang": "zh",
    ///   "api_url": "http://localhost:1188/translate",
    ///   "max_requests_per_second": 10.0,
    ///   "batch_size": 20,
    ///   "cache_enabled": true
    /// }
    /// ```
    /// 
    /// ## 实现细节
    /// 
    /// - 使用文件扩展名做格式检测，简单且可靠
    /// - 错误信息包含具体的解析器错误，便于调试
    /// - 文件内容先完整读入内存再解析，适合小型配置文件
    fn load_from_file(path: &str) -> TranslationResult<TranslationConfig> {
        // 1. 读取文件内容到内存
        let content = std::fs::read_to_string(path)
            .map_err(|e| TranslationError::ConfigError(format!("读取配置文件失败: {}", e)))?;

        // 2. 根据文件扩展名选择适当的解析器
        if path.ends_with(".toml") {
            // TOML 格式解析
            toml::from_str(&content)
                .map_err(|e| TranslationError::ConfigError(format!("解析TOML配置失败: {}", e)))
        } else {
            // JSON 格式解析（默认情况）
            serde_json::from_str(&content)
                .map_err(|e| TranslationError::ConfigError(format!("解析JSON配置失败: {}", e)))
        }
    }

    /// 加载 .env 文件
    fn load_dotenv() {
        let env_files = [".env.local", ".env.development", ".env.production", ".env"];

        for env_file in &env_files {
            if Path::new(env_file).exists() {
                if let Ok(_) = dotenv::from_filename(env_file) {
                    tracing::info!("已加载环境变量文件: {}", env_file);
                    break;
                }
            }
        }
    }

    /// 生成示例配置文件
    pub fn generate_example_config(path: &str) -> TranslationResult<()> {
        let config = TranslationConfig::default();
        let content = toml::to_string_pretty(&config)
            .map_err(|e| TranslationError::ConfigError(format!("序列化配置失败: {}", e)))?;

        std::fs::write(path, content)
            .map_err(|e| TranslationError::ConfigError(format!("写入配置文件失败: {}", e)))?;

        Ok(())
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new().expect("无法创建默认配置管理器")
    }
}

impl ConfigManager {
    /// 获取默认配置（向后兼容）
    pub fn default_config() -> TranslationConfig {
        TranslationConfig::default()
    }
}
