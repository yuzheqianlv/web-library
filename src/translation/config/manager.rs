//! 简化的配置管理器
//!
//! 提供统一的配置接口，支持文件配置、环境变量和默认值

use std::path::Path;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::translation::error::{TranslationError, TranslationResult};
use super::constants;

/// 简化的翻译配置
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TranslationConfig {
    // 基础配置
    pub enabled: bool,
    pub target_lang: String,
    pub source_lang: String,
    pub api_url: String,
    
    // 性能配置
    pub max_requests_per_second: f64,
    pub max_text_length: usize,
    pub max_paragraphs_per_request: usize,
    pub max_concurrent_requests: usize,
    
    // 批次配置
    pub batch_size: usize,
    pub min_batch_chars: usize,
    pub batch_timeout_secs: u64,
    
    // 缓存配置
    pub cache_enabled: bool,
    pub cache_ttl_secs: u64,
    pub local_cache_size: usize,
    
    // 功能开关
    pub parallel_enabled: bool,
    pub retry_enabled: bool,
    pub max_retry_attempts: usize,
}

impl TranslationConfig {
    /// 创建默认配置
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
    
    /// 创建带指定语言的默认配置
    pub fn default_with_lang(target_lang: &str, api_url: Option<&str>) -> Self {
        let mut config = Self::default();
        config.target_lang = target_lang.to_string();
        if let Some(url) = api_url {
            config.api_url = url.to_string();
        }
        config
    }
    
    /// 验证配置
    pub fn validate(&self) -> TranslationResult<()> {
        if self.batch_size == 0 {
            return Err(TranslationError::ConfigError("批次大小不能为0".to_string()));
        }
        
        if self.max_concurrent_requests == 0 {
            return Err(TranslationError::ConfigError("最大并发数不能为0".to_string()));
        }
        
        if self.max_requests_per_second <= 0.0 {
            return Err(TranslationError::ConfigError("请求速率必须大于0".to_string()));
        }
        
        if self.cache_enabled && self.local_cache_size == 0 {
            return Err(TranslationError::ConfigError("启用缓存时缓存大小不能为0".to_string()));
        }
        
        Ok(())
    }
    
    /// 应用环境变量覆盖（使用新的类型安全环境变量系统）
    pub fn apply_env_overrides(&mut self) {
        use crate::env::{translation, cache, EnvVar};
        
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
    
    /// 转换为Duration类型
    pub fn batch_timeout(&self) -> Duration {
        Duration::from_secs(self.batch_timeout_secs)
    }
    
    pub fn cache_ttl(&self) -> Duration {
        Duration::from_secs(self.cache_ttl_secs)
    }
}

impl Default for TranslationConfig {
    fn default() -> Self {
        Self::default()
    }
}

/// 简化的配置管理器
pub struct ConfigManager {
    config: TranslationConfig,
}

impl ConfigManager {
    /// 创建新的配置管理器
    pub fn new() -> TranslationResult<Self> {
        let mut config = Self::load_config()?;
        config.apply_env_overrides();
        config.validate()?;
        
        Ok(Self { config })
    }
    
    /// 获取配置
    pub fn get_config(&self) -> &TranslationConfig {
        &self.config
    }
    
    /// 创建简单配置（优化版，减少克隆）
    pub fn create_simple_config(&self, target_lang: &str, api_url: Option<&str>) -> TranslationResult<TranslationConfig> {
        // 只在必要的字段上进行修改，减少完整的结构体克隆
        if self.config.target_lang == target_lang && 
           api_url.map_or(true, |url| url == self.config.api_url) {
            // 如果没有变化，直接返回引用配置的克隆（但这里仍需要克隆以符合API）
            return Ok(self.config.clone());
        }
        
        let mut config = self.config.clone();
        config.target_lang = target_lang.to_string();
        if let Some(url) = api_url {
            config.api_url = url.to_string();
        }
        Ok(config)
    }
    
    /// 创建传统配置（向后兼容）
    pub fn create_legacy_config(&self, _target_lang: &str, api_url: Option<&str>) -> TranslationResult<markdown_translator::TranslationConfig> {
        Ok(markdown_translator::TranslationConfig {
            enabled: true,
            source_lang: "auto".to_string(),
            deeplx_api_url: api_url.unwrap_or(&self.config.api_url).to_string(),
            ..Default::default()
        })
    }
    
    /// 从文件加载配置
    fn load_config() -> TranslationResult<TranslationConfig> {
        // 首先尝试加载 .env 文件
        Self::load_dotenv();
        
        // 查找配置文件
        for path in constants::CONFIG_PATHS {
            let expanded_path = shellexpand::tilde(path);
            if Path::new(expanded_path.as_ref()).exists() {
                tracing::info!("加载配置文件: {}", expanded_path);
                return Self::load_from_file(&expanded_path);
            }
        }
        
        tracing::info!("未找到配置文件，使用默认配置");
        Ok(TranslationConfig::default())
    }
    
    /// 从指定文件加载配置
    fn load_from_file(path: &str) -> TranslationResult<TranslationConfig> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| TranslationError::ConfigError(format!("读取配置文件失败: {}", e)))?;
        
        // 尝试TOML格式
        if path.ends_with(".toml") {
            toml::from_str(&content)
                .map_err(|e| TranslationError::ConfigError(format!("解析TOML配置失败: {}", e)))
        } else {
            // 尝试JSON格式
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