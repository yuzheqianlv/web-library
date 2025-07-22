//! 翻译配置管理模块
//!
//! 提供配置加载、验证和热重载功能，支持多种配置源

use std::path::Path;
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime};

#[cfg(feature = "translation")]
use config::{Config, Environment, File};
#[cfg(feature = "translation")]
use dotenv;
#[cfg(feature = "translation")]
use markdown_translator::{TranslationConfig, TranslationLibConfig};
#[cfg(feature = "translation")]
use serde::{Deserialize, Serialize};

use crate::translation::error::TranslationError;

/// 翻译配置常量
pub mod constants {
    pub const MAX_BATCH_SIZE: usize = 9000;
    pub const DEFAULT_MIN_CHARS: usize = 2000;
    pub const BATCH_DELAY_MS: u64 = 100;
    pub const SMALL_BATCH_THRESHOLD: usize = 2;
    pub const MIN_TEXT_LENGTH: usize = 2;
    pub const MIN_TRANSLATION_LENGTH: usize = 3;
    pub const CHINESE_CHAR_THRESHOLD: f32 = 0.5;
    pub const SPECIAL_CHAR_THRESHOLD: f32 = 0.33;

    pub const TRANSLATABLE_ATTRS: &[&str] = &[
        "title",
        "alt",
        "placeholder",
        "aria-label",
        "aria-description",
    ];

    pub const SKIP_ELEMENTS: &[&str] = &[
        "script", "style", "code", "pre", "noscript", "meta", "link", "head", "svg", "math",
        "canvas", "video", "audio", "embed", "object", "iframe", "map", "area", "base", "br", "hr",
        "img", "input", "source", "track", "wbr",
    ];

    pub const FUNCTIONAL_WORDS: &[&str] = &["ok", "yes", "no", "on", "off", "go", "up", "x", ">"];

    pub const CONFIG_PATHS: &[&str] = &[
        "translation-config.toml",
        "config.toml",
        ".translation-config.toml",
        "~/.config/monolith/translation.toml",
        "/etc/monolith/translation.toml",
    ];

    // 默认配置值
    pub const DEFAULT_MAX_REQUESTS_PER_SECOND: f64 = 5.0;
    pub const DEFAULT_MAX_TEXT_LENGTH: usize = 10000;
    pub const DEFAULT_MAX_PARAGRAPHS_PER_REQUEST: usize = 100;
    pub const DEFAULT_API_URL: &str = "http://localhost:1188/translate";
    pub const DEFAULT_CACHE_TTL_SECS: u64 = 3600; // 1小时
    pub const DEFAULT_BATCH_TIMEOUT_SECS: u64 = 30;
    pub const DEFAULT_MAX_CONCURRENT_REQUESTS: usize = 10;
}

/// 增强的翻译配置结构
#[cfg(feature = "translation")]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EnhancedTranslationConfig {
    /// 基础翻译配置
    #[serde(flatten)]
    pub base: TranslationConfig,

    /// 批次处理配置
    pub batch: BatchConfig,

    /// 缓存配置
    pub cache: CacheConfig,

    /// 性能配置
    pub performance: PerformanceConfig,

    /// 监控配置
    pub monitoring: MonitoringConfig,
}

#[cfg(feature = "translation")]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BatchConfig {
    /// 最大批次大小（字符数）
    pub max_batch_size: usize,

    /// 最小批次字符数
    pub min_batch_chars: usize,

    /// 批次处理延迟（毫秒）
    pub batch_delay_ms: u64,

    /// 小批次阈值
    pub small_batch_threshold: usize,

    /// 批次超时时间
    #[serde(with = "duration_serde")]
    pub batch_timeout: Duration,
}

#[cfg(feature = "translation")]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CacheConfig {
    /// 启用缓存
    pub enabled: bool,

    /// 本地缓存大小
    pub local_cache_size: usize,

    /// 缓存TTL
    #[serde(with = "duration_serde")]
    pub ttl: Duration,

    /// 缓存预热
    pub enable_warmup: bool,
}

#[cfg(feature = "translation")]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PerformanceConfig {
    /// 最大并发请求数
    pub max_concurrent_requests: usize,

    /// 启用并行处理
    pub enable_parallel_processing: bool,

    /// 工作线程数
    pub worker_threads: Option<usize>,

    /// 连接池大小
    pub connection_pool_size: usize,
}

#[cfg(feature = "translation")]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MonitoringConfig {
    /// 启用指标收集
    pub enable_metrics: bool,

    /// 启用详细日志
    pub enable_verbose_logging: bool,

    /// 指标收集间隔
    #[serde(with = "duration_serde")]
    pub metrics_interval: Duration,
}

/// Duration的序列化/反序列化模块
#[cfg(feature = "translation")]
mod duration_serde {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(duration.as_secs())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(secs))
    }
}

/// 配置管理器
#[cfg(feature = "translation")]
pub struct ConfigManager {
    config: Arc<RwLock<EnhancedTranslationConfig>>,
    last_modified: Arc<RwLock<Option<SystemTime>>>,
    config_path: Option<String>,
}

#[cfg(feature = "translation")]
impl ConfigManager {
    /// 创建新的配置管理器
    pub fn new() -> Result<Self, TranslationError> {
        let (config, config_path) = Self::load_config()?;

        let manager = Self {
            config: Arc::new(RwLock::new(config)),
            last_modified: Arc::new(RwLock::new(None)),
            config_path,
        };

        // 更新最后修改时间
        manager.update_last_modified()?;

        Ok(manager)
    }

    /// 获取当前配置
    pub fn get_config(&self) -> Result<EnhancedTranslationConfig, TranslationError> {
        self.config
            .read()
            .map_err(|e| TranslationError::ConfigError(format!("读取配置失败: {}", e)))
            .map(|config| config.clone())
    }

    /// 检查并重新加载配置（如果有更改）
    pub fn reload_if_changed(&self) -> Result<bool, TranslationError> {
        if let Some(ref path) = self.config_path {
            let metadata = std::fs::metadata(path).map_err(|e| {
                TranslationError::ConfigError(format!("无法读取配置文件元数据: {}", e))
            })?;

            let modified = metadata.modified().map_err(|e| {
                TranslationError::ConfigError(format!("无法获取文件修改时间: {}", e))
            })?;

            let last_modified = self
                .last_modified
                .read()
                .map_err(|e| TranslationError::ConfigError(format!("读取锁失败: {}", e)))?;

            if last_modified.map_or(true, |last| modified > last) {
                drop(last_modified);

                let (new_config, _) = Self::load_config()?;

                *self
                    .config
                    .write()
                    .map_err(|e| TranslationError::ConfigError(format!("写入锁失败: {}", e)))? =
                    new_config;

                self.update_last_modified()?;

                tracing::info!("配置文件已重新加载: {}", path);
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// 加载翻译配置
    fn load_config() -> Result<(EnhancedTranslationConfig, Option<String>), TranslationError> {
        // 首先尝试加载 .env 文件
        Self::load_dotenv();

        // 尝试使用config crate加载配置
        let mut builder = Config::builder();

        // 添加默认配置
        builder = builder.add_source(
            Config::try_from(&Self::default_config())
                .map_err(|e| TranslationError::ConfigError(format!("默认配置错误: {}", e)))?,
        );

        // 查找并加载配置文件
        let mut config_path = None;
        for path in constants::CONFIG_PATHS {
            let expanded_path = shellexpand::tilde(path);
            if Path::new(expanded_path.as_ref()).exists() {
                builder = builder.add_source(File::with_name(&expanded_path));
                config_path = Some(expanded_path.to_string());
                tracing::info!("加载配置文件: {}", expanded_path);
                break;
            }
        }

        // 添加环境变量覆盖（启用类型转换）
        builder = builder.add_source(
            Environment::with_prefix("MONOLITH_TRANSLATION")
                .prefix_separator("_")
                .separator("_")
                .try_parsing(true), // 启用类型转换
        );

        let config = builder
            .build()
            .map_err(|e| TranslationError::ConfigError(format!("构建配置失败: {}", e)))?;

        let mut enhanced_config: EnhancedTranslationConfig = config
            .try_deserialize()
            .map_err(|e| TranslationError::ConfigError(format!("反序列化配置失败: {}", e)))?;

        // 手动处理扁平化字段的环境变量覆盖
        Self::apply_env_overrides(&mut enhanced_config);

        // 添加调试日志
        tracing::info!(
            "加载的配置 - API URL: {}",
            enhanced_config.base.deeplx_api_url
        );
        tracing::info!(
            "加载的配置 - 目标语言: {}",
            enhanced_config.base.target_lang
        );

        // 验证配置
        Self::validate_config(&enhanced_config)?;

        Ok((enhanced_config, config_path))
    }

    /// 手动应用环境变量覆盖（处理扁平化字段）
    fn apply_env_overrides(config: &mut EnhancedTranslationConfig) {
        // 基础配置字段覆盖
        if let Ok(val) = std::env::var("MONOLITH_TRANSLATION_ENABLED") {
            if let Ok(enabled) = val.parse::<bool>() {
                config.base.enabled = enabled;
            }
        }

        if let Ok(val) = std::env::var("MONOLITH_TRANSLATION_TARGET_LANG") {
            config.base.target_lang = val;
        }

        if let Ok(val) = std::env::var("MONOLITH_TRANSLATION_SOURCE_LANG") {
            config.base.source_lang = val;
        }

        if let Ok(val) = std::env::var("MONOLITH_TRANSLATION_DEEPLX_API_URL") {
            config.base.deeplx_api_url = val;
            tracing::info!("环境变量覆盖 API URL: {}", config.base.deeplx_api_url);
        }

        if let Ok(val) = std::env::var("MONOLITH_TRANSLATION_MAX_REQUESTS_PER_SECOND") {
            if let Ok(rate) = val.parse::<f64>() {
                config.base.max_requests_per_second = rate;
            }
        }

        if let Ok(val) = std::env::var("MONOLITH_TRANSLATION_MAX_TEXT_LENGTH") {
            if let Ok(length) = val.parse::<usize>() {
                config.base.max_text_length = length;
            }
        }

        if let Ok(val) = std::env::var("MONOLITH_TRANSLATION_MAX_PARAGRAPHS_PER_REQUEST") {
            if let Ok(paragraphs) = val.parse::<usize>() {
                config.base.max_paragraphs_per_request = paragraphs;
            }
        }
    }

    /// 加载 .env 文件
    fn load_dotenv() {
        // 按优先级加载 .env 文件
        let env_files = [
            ".env.local",       // 本地环境，最高优先级
            ".env.development", // 开发环境
            ".env.production",  // 生产环境
            ".env",             // 默认 .env 文件
        ];

        for env_file in &env_files {
            if Path::new(env_file).exists() {
                match dotenv::from_filename(env_file) {
                    Ok(_) => {
                        tracing::info!("已加载环境变量文件: {}", env_file);
                        // 输出关键环境变量用于调试
                        if let Ok(api_url) = std::env::var("MONOLITH_TRANSLATION_DEEPLX_API_URL") {
                            tracing::info!(
                                "环境变量 MONOLITH_TRANSLATION_DEEPLX_API_URL: {}",
                                api_url
                            );
                        }
                        break; // 找到第一个存在的文件就停止
                    }
                    Err(e) => {
                        tracing::warn!("无法加载环境变量文件 {}: {}", env_file, e);
                    }
                }
            }
        }

        // 如果没有找到任何 .env 文件，尝试默认位置
        if !env_files.iter().any(|f| Path::new(f).exists()) {
            if let Err(e) = dotenv::dotenv() {
                tracing::debug!("未找到 .env 文件或加载失败: {}", e);
            }
        }
    }

    /// 创建默认配置
    pub fn default_config() -> EnhancedTranslationConfig {
        EnhancedTranslationConfig {
            base: TranslationConfig {
                enabled: true,
                target_lang: "zh".to_string(),
                source_lang: "auto".to_string(),
                deeplx_api_url: constants::DEFAULT_API_URL.to_string(),
                max_requests_per_second: constants::DEFAULT_MAX_REQUESTS_PER_SECOND,
                max_text_length: constants::DEFAULT_MAX_TEXT_LENGTH,
                max_paragraphs_per_request: constants::DEFAULT_MAX_PARAGRAPHS_PER_REQUEST,
            },
            batch: BatchConfig {
                max_batch_size: constants::MAX_BATCH_SIZE,
                min_batch_chars: constants::DEFAULT_MIN_CHARS,
                batch_delay_ms: constants::BATCH_DELAY_MS,
                small_batch_threshold: constants::SMALL_BATCH_THRESHOLD,
                batch_timeout: Duration::from_secs(constants::DEFAULT_BATCH_TIMEOUT_SECS),
            },
            cache: CacheConfig {
                enabled: true,
                local_cache_size: 1000,
                ttl: Duration::from_secs(constants::DEFAULT_CACHE_TTL_SECS),
                enable_warmup: false,
            },
            performance: PerformanceConfig {
                max_concurrent_requests: constants::DEFAULT_MAX_CONCURRENT_REQUESTS,
                enable_parallel_processing: true,
                worker_threads: None, // 使用系统默认
                connection_pool_size: 10,
            },
            monitoring: MonitoringConfig {
                enable_metrics: true,
                enable_verbose_logging: false,
                metrics_interval: Duration::from_secs(60),
            },
        }
    }

    /// 验证配置
    fn validate_config(config: &EnhancedTranslationConfig) -> Result<(), TranslationError> {
        if config.batch.max_batch_size == 0 {
            return Err(TranslationError::ConfigError(
                "最大批次大小不能为0".to_string(),
            ));
        }

        if config.batch.min_batch_chars > config.batch.max_batch_size {
            return Err(TranslationError::ConfigError(
                "最小批次字符数不能大于最大批次大小".to_string(),
            ));
        }

        if config.performance.max_concurrent_requests == 0 {
            return Err(TranslationError::ConfigError(
                "最大并发请求数不能为0".to_string(),
            ));
        }

        if config.cache.local_cache_size == 0 && config.cache.enabled {
            return Err(TranslationError::ConfigError(
                "启用缓存时本地缓存大小不能为0".to_string(),
            ));
        }

        Ok(())
    }

    /// 更新最后修改时间
    fn update_last_modified(&self) -> Result<(), TranslationError> {
        if let Some(ref path) = self.config_path {
            let metadata = std::fs::metadata(path).map_err(|e| {
                TranslationError::ConfigError(format!("无法读取配置文件元数据: {}", e))
            })?;

            let modified = metadata.modified().map_err(|e| {
                TranslationError::ConfigError(format!("无法获取文件修改时间: {}", e))
            })?;

            *self
                .last_modified
                .write()
                .map_err(|e| TranslationError::ConfigError(format!("写入锁失败: {}", e)))? =
                Some(modified);
        }

        Ok(())
    }

    /// 创建兼容的TranslationConfig（用于向后兼容）
    pub fn create_legacy_config(
        &self,
        target_lang: &str,
        api_url: Option<&str>,
    ) -> Result<TranslationConfig, TranslationError> {
        let enhanced_config = self.get_config()?;
        let mut config = enhanced_config.base;

        // 应用参数覆盖
        config.target_lang = target_lang.to_string();

        if let Some(url) = api_url {
            config.deeplx_api_url = url.to_string();
        }

        Ok(config)
    }
}

#[cfg(feature = "translation")]
impl Default for ConfigManager {
    fn default() -> Self {
        Self::new().expect("无法创建默认配置管理器")
    }
}

/// 向后兼容的配置加载函数
#[cfg(feature = "translation")]
pub fn load_translation_config(target_lang: &str, api_url: Option<&str>) -> TranslationConfig {
    match ConfigManager::new() {
        Ok(manager) => match manager.create_legacy_config(target_lang, api_url) {
            Ok(config) => config,
            Err(e) => {
                tracing::warn!("使用增强配置失败，回退到简单配置: {}", e);
                create_fallback_config(target_lang, api_url)
            }
        },
        Err(e) => {
            tracing::warn!("创建配置管理器失败，使用后备配置: {}", e);
            create_fallback_config(target_lang, api_url)
        }
    }
}

/// 创建后备配置（兼容原有逻辑）
#[cfg(feature = "translation")]
fn create_fallback_config(target_lang: &str, api_url: Option<&str>) -> TranslationConfig {
    // 尝试从传统配置文件加载
    let lib_config = load_legacy_config();
    let mut config = lib_config.translation;

    config.target_lang = target_lang.to_string();
    config.enabled = true;

    if let Some(url) = api_url {
        config.deeplx_api_url = url.to_string();
    }

    // 如果没有配置文件，使用优化的默认参数
    if !config_file_exists() {
        tracing::info!("未找到配置文件，使用优化默认参数");
        config.max_requests_per_second = constants::DEFAULT_MAX_REQUESTS_PER_SECOND;
        config.max_text_length = constants::DEFAULT_MAX_TEXT_LENGTH;
        config.max_paragraphs_per_request = constants::DEFAULT_MAX_PARAGRAPHS_PER_REQUEST;

        if api_url.is_none() {
            config.deeplx_api_url = constants::DEFAULT_API_URL.to_string();
        }
    }

    config
}

/// 加载传统配置格式
#[cfg(feature = "translation")]
fn load_legacy_config() -> TranslationLibConfig {
    for path in constants::CONFIG_PATHS {
        if Path::new(path).exists() {
            match TranslationLibConfig::from_file(path) {
                Ok(config) => {
                    tracing::info!("从传统配置文件加载: {}", path);
                    return config;
                }
                Err(e) => {
                    tracing::warn!("无法从 {} 加载传统配置: {}", path, e);
                }
            }
        }
    }
    TranslationLibConfig::default()
}

/// 检查配置文件是否存在
#[cfg(feature = "translation")]
pub fn config_file_exists() -> bool {
    constants::CONFIG_PATHS
        .iter()
        .any(|path| Path::new(path).exists())
}

/// 获取最小翻译字符数配置
#[cfg(feature = "translation")]
pub fn get_min_translation_chars() -> usize {
    match ConfigManager::new() {
        Ok(manager) => match manager.get_config() {
            Ok(config) => config.batch.min_batch_chars,
            Err(_) => get_min_chars_from_legacy_config(),
        },
        Err(_) => get_min_chars_from_legacy_config(),
    }
}

/// 从传统配置文件获取最小字符数
#[cfg(feature = "translation")]
fn get_min_chars_from_legacy_config() -> usize {
    for path in constants::CONFIG_PATHS {
        if Path::new(path).exists() {
            if let Ok(content) = std::fs::read_to_string(path) {
                if let Some(value) = parse_min_chars_from_config(&content) {
                    tracing::debug!("从传统配置文件 {} 读取到最小翻译字符数: {}", path, value);
                    return value;
                }
            }
        }
    }

    constants::DEFAULT_MIN_CHARS
}

/// 从配置内容解析最小字符数
#[cfg(feature = "translation")]
fn parse_min_chars_from_config(content: &str) -> Option<usize> {
    for line in content.lines() {
        if line.trim().starts_with("min_translation_chars") {
            if let Some(value_part) = line.split('=').nth(1) {
                if let Ok(value) = value_part.trim().parse::<usize>() {
                    return Some(value);
                }
            }
        }
    }
    None
}

/// 非translation feature时的占位实现
#[cfg(not(feature = "translation"))]
pub mod constants {
    pub const DEFAULT_MIN_CHARS: usize = 2000;
}

#[cfg(not(feature = "translation"))]
pub fn get_min_translation_chars() -> usize {
    constants::DEFAULT_MIN_CHARS
}

#[cfg(not(feature = "translation"))]
pub fn config_file_exists() -> bool {
    false
}
