//! 统一的环境变量管理系统
//!
//! 基于翻译组件重构的经验，提供类型安全、可验证的环境变量管理

use std::env;
use std::fmt;
use std::time::Duration;

/// 环境变量解析错误
#[derive(Debug, Clone)]
pub struct EnvError {
    pub variable: String,
    pub message: String,
}

impl fmt::Display for EnvError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Environment variable '{}': {}", self.variable, self.message)
    }
}

impl std::error::Error for EnvError {}

pub type EnvResult<T> = Result<T, EnvError>;

/// 环境变量访问器特性
pub trait EnvVar<T> {
    const NAME: &'static str;
    const DEFAULT: Option<T>;
    const DESCRIPTION: &'static str;
    
    fn parse(value: &str) -> EnvResult<T>;
    
    fn get() -> EnvResult<T> {
        match env::var(Self::NAME) {
            Ok(value) => Self::parse(&value),
            Err(_) => {
                if let Some(default) = Self::DEFAULT {
                    Ok(default)
                } else {
                    Err(EnvError {
                        variable: Self::NAME.to_string(),
                        message: "Required environment variable not set".to_string(),
                    })
                }
            }
        }
    }
    
    fn get_or_default(default: T) -> T {
        Self::get().unwrap_or(default)
    }
}

/// 核心环境变量定义
pub mod core {
    use super::*;
    
    /// 应用运行模式
    pub struct Mode;
    impl EnvVar<String> for Mode {
        const NAME: &'static str = "MONOLITH_MODE";
        const DEFAULT: Option<String> = None;
        
        fn get() -> EnvResult<String> {
            match env::var(Self::NAME) {
                Ok(value) => Self::parse(&value),
                Err(_) => Ok("production".to_string()),
            }
        }
        const DESCRIPTION: &'static str = "Application mode: development, staging, production";
        
        fn parse(value: &str) -> EnvResult<String> {
            match value.to_lowercase().as_str() {
                "development" | "dev" => Ok("development".to_string()),
                "staging" | "stage" => Ok("staging".to_string()),
                "production" | "prod" => Ok("production".to_string()),
                _ => Err(EnvError {
                    variable: Self::NAME.to_string(),
                    message: format!("Invalid mode '{}'. Use: development, staging, production", value),
                })
            }
        }
    }
    
    /// 日志级别
    pub struct LogLevel;
    impl EnvVar<String> for LogLevel {
        const NAME: &'static str = "MONOLITH_LOG_LEVEL";
        const DEFAULT: Option<String> = None;
        
        fn get() -> EnvResult<String> {
            match env::var(Self::NAME) {
                Ok(value) => Self::parse(&value),
                Err(_) => Ok("info".to_string()),
            }
        }
        const DESCRIPTION: &'static str = "Log level: trace, debug, info, warn, error";
        
        fn parse(value: &str) -> EnvResult<String> {
            match value.to_lowercase().as_str() {
                "trace" | "debug" | "info" | "warn" | "error" => Ok(value.to_lowercase()),
                _ => Err(EnvError {
                    variable: Self::NAME.to_string(),
                    message: format!("Invalid log level '{}'. Use: trace, debug, info, warn, error", value),
                })
            }
        }
    }
    
    /// 启用颜色输出
    pub struct NoColor;
    impl EnvVar<bool> for NoColor {
        const NAME: &'static str = "NO_COLOR";
        const DEFAULT: Option<bool> = Some(false);
        const DESCRIPTION: &'static str = "Disable colored output when set to any value";
        
        fn parse(value: &str) -> EnvResult<bool> {
            // NO_COLOR 遵循标准：任何值都表示禁用颜色
            Ok(!value.is_empty())
        }
    }
}

/// 翻译相关环境变量
pub mod translation {
    use super::*;
    
    /// 翻译功能启用状态
    pub struct Enabled;
    impl EnvVar<bool> for Enabled {
        const NAME: &'static str = "MONOLITH_TRANSLATION_ENABLED";
        const DEFAULT: Option<bool> = Some(true);
        const DESCRIPTION: &'static str = "Enable translation functionality";
        
        fn parse(value: &str) -> EnvResult<bool> {
            parse_bool(value, Self::NAME)
        }
    }
    
    /// 目标语言
    pub struct TargetLang;
    impl EnvVar<String> for TargetLang {
        const NAME: &'static str = "MONOLITH_TRANSLATION_TARGET_LANG";
        const DEFAULT: Option<String> = None;
        
        fn get() -> EnvResult<String> {
            match env::var(Self::NAME) {
                Ok(value) => Self::parse(&value),
                Err(_) => Ok("zh".to_string()),
            }
        }
        const DESCRIPTION: &'static str = "Target language for translation (ISO 639-1 code)";
        
        fn parse(value: &str) -> EnvResult<String> {
            let lang = value.trim().to_lowercase();
            if lang.len() != 2 {
                return Err(EnvError {
                    variable: Self::NAME.to_string(),
                    message: "Language code must be 2 characters (ISO 639-1)".to_string(),
                });
            }
            Ok(lang)
        }
    }
    
    /// 源语言
    pub struct SourceLang;
    impl EnvVar<String> for SourceLang {
        const NAME: &'static str = "MONOLITH_TRANSLATION_SOURCE_LANG";
        const DEFAULT: Option<String> = None;
        
        fn get() -> EnvResult<String> {
            match env::var(Self::NAME) {
                Ok(value) => Self::parse(&value),
                Err(_) => Ok("auto".to_string()),
            }
        }
        const DESCRIPTION: &'static str = "Source language for translation ('auto' for detection)";
        
        fn parse(value: &str) -> EnvResult<String> {
            let lang = value.trim().to_lowercase();
            if lang == "auto" || lang.len() == 2 {
                Ok(lang)
            } else {
                Err(EnvError {
                    variable: Self::NAME.to_string(),
                    message: "Language code must be 'auto' or 2 characters (ISO 639-1)".to_string(),
                })
            }
        }
    }
    
    /// API URL
    pub struct ApiUrl;
    impl EnvVar<String> for ApiUrl {
        const NAME: &'static str = "MONOLITH_TRANSLATION_API_URL";
        const DEFAULT: Option<String> = None;
        
        fn get() -> EnvResult<String> {
            match env::var(Self::NAME) {
                Ok(value) => Self::parse(&value),
                Err(_) => Ok("http://localhost:1188/translate".to_string()),
            }
        }
        const DESCRIPTION: &'static str = "Translation API endpoint URL";
        
        fn parse(value: &str) -> EnvResult<String> {
            let url = value.trim();
            if url.starts_with("http://") || url.starts_with("https://") {
                Ok(url.to_string())
            } else {
                Err(EnvError {
                    variable: Self::NAME.to_string(),
                    message: "API URL must start with http:// or https://".to_string(),
                })
            }
        }
    }
    
    /// 每秒最大请求数
    pub struct MaxRequestsPerSecond;
    impl EnvVar<f64> for MaxRequestsPerSecond {
        const NAME: &'static str = "MONOLITH_TRANSLATION_MAX_REQUESTS_PER_SECOND";
        const DEFAULT: Option<f64> = Some(5.0);
        const DESCRIPTION: &'static str = "Maximum requests per second to translation API";
        
        fn parse(value: &str) -> EnvResult<f64> {
            let rate: f64 = value.parse().map_err(|_| EnvError {
                variable: Self::NAME.to_string(),
                message: "Must be a valid number".to_string(),
            })?;
            
            if rate <= 0.0 {
                return Err(EnvError {
                    variable: Self::NAME.to_string(),
                    message: "Rate must be greater than 0".to_string(),
                });
            }
            
            if rate > 1000.0 {
                return Err(EnvError {
                    variable: Self::NAME.to_string(),
                    message: "Rate too high (max 1000 requests/second)".to_string(),
                });
            }
            
            Ok(rate)
        }
    }
    
    /// 最大并发请求数
    pub struct MaxConcurrentRequests;
    impl EnvVar<usize> for MaxConcurrentRequests {
        const NAME: &'static str = "MONOLITH_TRANSLATION_MAX_CONCURRENT_REQUESTS";
        const DEFAULT: Option<usize> = Some(10);
        const DESCRIPTION: &'static str = "Maximum concurrent requests to translation API";
        
        fn parse(value: &str) -> EnvResult<usize> {
            parse_positive_usize(value, Self::NAME, 1, 100)
        }
    }
    
    /// 批次大小
    pub struct BatchSize;
    impl EnvVar<usize> for BatchSize {
        const NAME: &'static str = "MONOLITH_TRANSLATION_BATCH_SIZE";
        const DEFAULT: Option<usize> = Some(9000);
        const DESCRIPTION: &'static str = "Maximum characters per translation batch";
        
        fn parse(value: &str) -> EnvResult<usize> {
            parse_positive_usize(value, Self::NAME, 100, 50000)
        }
    }
    
    /// 批次超时
    pub struct BatchTimeout;
    impl EnvVar<Duration> for BatchTimeout {
        const NAME: &'static str = "MONOLITH_TRANSLATION_BATCH_TIMEOUT";
        const DEFAULT: Option<Duration> = Some(Duration::from_secs(30));
        const DESCRIPTION: &'static str = "Batch processing timeout in seconds";
        
        fn parse(value: &str) -> EnvResult<Duration> {
            let seconds: u64 = value.parse().map_err(|_| EnvError {
                variable: Self::NAME.to_string(),
                message: "Must be a valid number of seconds".to_string(),
            })?;
            
            if seconds == 0 {
                return Err(EnvError {
                    variable: Self::NAME.to_string(),
                    message: "Timeout must be greater than 0".to_string(),
                });
            }
            
            if seconds > 300 {
                return Err(EnvError {
                    variable: Self::NAME.to_string(),
                    message: "Timeout too long (max 300 seconds)".to_string(),
                });
            }
            
            Ok(Duration::from_secs(seconds))
        }
    }
}

/// 缓存相关环境变量
pub mod cache {
    use super::*;
    
    /// 缓存启用状态
    pub struct Enabled;
    impl EnvVar<bool> for Enabled {
        const NAME: &'static str = "MONOLITH_CACHE_ENABLED";
        const DEFAULT: Option<bool> = Some(true);
        const DESCRIPTION: &'static str = "Enable caching functionality";
        
        fn parse(value: &str) -> EnvResult<bool> {
            parse_bool(value, Self::NAME)
        }
    }
    
    /// 本地缓存大小
    pub struct LocalCacheSize;
    impl EnvVar<usize> for LocalCacheSize {
        const NAME: &'static str = "MONOLITH_CACHE_LOCAL_SIZE";
        const DEFAULT: Option<usize> = Some(1000);
        const DESCRIPTION: &'static str = "Local cache size (number of entries)";
        
        fn parse(value: &str) -> EnvResult<usize> {
            parse_positive_usize(value, Self::NAME, 10, 100000)
        }
    }
    
    /// 缓存TTL
    pub struct Ttl;
    impl EnvVar<Duration> for Ttl {
        const NAME: &'static str = "MONOLITH_CACHE_TTL";
        const DEFAULT: Option<Duration> = Some(Duration::from_secs(3600));
        const DESCRIPTION: &'static str = "Cache TTL in seconds";
        
        fn parse(value: &str) -> EnvResult<Duration> {
            let seconds: u64 = value.parse().map_err(|_| EnvError {
                variable: Self::NAME.to_string(),
                message: "Must be a valid number of seconds".to_string(),
            })?;
            
            if seconds < 60 {
                return Err(EnvError {
                    variable: Self::NAME.to_string(),
                    message: "TTL too short (minimum 60 seconds)".to_string(),
                });
            }
            
            if seconds > 86400 * 7 { // 7 days
                return Err(EnvError {
                    variable: Self::NAME.to_string(),
                    message: "TTL too long (maximum 7 days)".to_string(),
                });
            }
            
            Ok(Duration::from_secs(seconds))
        }
    }
    
    /// 缓存预热启用
    pub struct WarmupEnabled;
    impl EnvVar<bool> for WarmupEnabled {
        const NAME: &'static str = "MONOLITH_CACHE_WARMUP_ENABLED";
        const DEFAULT: Option<bool> = Some(false);
        const DESCRIPTION: &'static str = "Enable cache warmup on startup";
        
        fn parse(value: &str) -> EnvResult<bool> {
            parse_bool(value, Self::NAME)
        }
    }
}

/// Web服务器相关环境变量
pub mod web {
    use super::*;
    
    /// 绑定地址
    pub struct BindAddress;
    impl EnvVar<String> for BindAddress {
        const NAME: &'static str = "MONOLITH_WEB_BIND_ADDRESS";
        const DEFAULT: Option<String> = None;
        
        fn get() -> EnvResult<String> {
            match env::var(Self::NAME) {
                Ok(value) => Self::parse(&value),
                Err(_) => Ok("127.0.0.1".to_string()),
            }
        }
        const DESCRIPTION: &'static str = "Web server bind address";
        
        fn parse(value: &str) -> EnvResult<String> {
            let addr = value.trim();
            if addr.is_empty() {
                return Err(EnvError {
                    variable: Self::NAME.to_string(),
                    message: "Address cannot be empty".to_string(),
                });
            }
            Ok(addr.to_string())
        }
    }
    
    /// 端口
    pub struct Port;
    impl EnvVar<u16> for Port {
        const NAME: &'static str = "MONOLITH_WEB_PORT";
        const DEFAULT: Option<u16> = Some(7080);
        const DESCRIPTION: &'static str = "Web server port";
        
        fn parse(value: &str) -> EnvResult<u16> {
            let port: u16 = value.parse().map_err(|_| EnvError {
                variable: Self::NAME.to_string(),
                message: "Must be a valid port number (1-65535)".to_string(),
            })?;
            
            if port < 1024 && !is_privileged() {
                return Err(EnvError {
                    variable: Self::NAME.to_string(),
                    message: "Ports below 1024 require root privileges".to_string(),
                });
            }
            
            Ok(port)
        }
    }
    
    /// 静态文件目录
    pub struct StaticDir;
    impl EnvVar<String> for StaticDir {
        const NAME: &'static str = "MONOLITH_WEB_STATIC_DIR";
        const DEFAULT: Option<String> = None;
        
        fn get() -> EnvResult<String> {
            match env::var(Self::NAME) {
                Ok(value) => Self::parse(&value),
                Err(_) => Ok("static".to_string()),
            }
        }
        const DESCRIPTION: &'static str = "Static files directory";
        
        fn parse(value: &str) -> EnvResult<String> {
            Ok(value.trim().to_string())
        }
    }
}

/// MongoDB相关环境变量
pub mod mongodb {
    use super::*;
    
    /// MongoDB连接字符串
    pub struct ConnectionString;
    impl EnvVar<String> for ConnectionString {
        const NAME: &'static str = "MONGODB_URL";
        const DEFAULT: Option<String> = None;
        
        fn get() -> EnvResult<String> {
            match env::var(Self::NAME) {
                Ok(value) => Self::parse(&value),
                Err(_) => Ok("mongodb://localhost:27017".to_string()),
            }
        }
        const DESCRIPTION: &'static str = "MongoDB connection string";
        
        fn parse(value: &str) -> EnvResult<String> {
            let url = value.trim();
            if url.starts_with("mongodb://") || url.starts_with("mongodb+srv://") {
                Ok(url.to_string())
            } else {
                Err(EnvError {
                    variable: Self::NAME.to_string(),
                    message: "MongoDB URL must start with mongodb:// or mongodb+srv://".to_string(),
                })
            }
        }
    }
    
    /// 数据库名称
    pub struct DatabaseName;
    impl EnvVar<String> for DatabaseName {
        const NAME: &'static str = "MONGODB_DATABASE";
        const DEFAULT: Option<String> = None;
        
        fn get() -> EnvResult<String> {
            match env::var(Self::NAME) {
                Ok(value) => Self::parse(&value),
                Err(_) => Ok("monolith".to_string()),
            }
        }
        const DESCRIPTION: &'static str = "MongoDB database name";
        
        fn parse(value: &str) -> EnvResult<String> {
            let name = value.trim();
            if name.is_empty() {
                return Err(EnvError {
                    variable: Self::NAME.to_string(),
                    message: "Database name cannot be empty".to_string(),
                });
            }
            Ok(name.to_string())
        }
    }
    
    /// 集合名称
    pub struct CollectionName;
    impl EnvVar<String> for CollectionName {
        const NAME: &'static str = "MONGODB_COLLECTION";
        const DEFAULT: Option<String> = None;
        
        fn get() -> EnvResult<String> {
            match env::var(Self::NAME) {
                Ok(value) => Self::parse(&value),
                Err(_) => Ok("html_cache".to_string()),
            }
        }
        const DESCRIPTION: &'static str = "MongoDB collection name";
        
        fn parse(value: &str) -> EnvResult<String> {
            let name = value.trim();
            if name.is_empty() {
                return Err(EnvError {
                    variable: Self::NAME.to_string(),
                    message: "Collection name cannot be empty".to_string(),
                });
            }
            Ok(name.to_string())
        }
    }
}

/// 性能相关环境变量
pub mod performance {
    use super::*;
    
    /// 工作线程数
    pub struct WorkerThreads;
    impl EnvVar<usize> for WorkerThreads {
        const NAME: &'static str = "MONOLITH_WORKER_THREADS";
        const DEFAULT: Option<usize> = None; // 使用系统默认
        const DESCRIPTION: &'static str = "Number of worker threads (default: CPU cores)";
        
        fn parse(value: &str) -> EnvResult<usize> {
            parse_positive_usize(value, Self::NAME, 1, 128)
        }
    }
    
    /// 最大内存使用
    pub struct MaxMemoryMb;
    impl EnvVar<usize> for MaxMemoryMb {
        const NAME: &'static str = "MONOLITH_MAX_MEMORY_MB";
        const DEFAULT: Option<usize> = Some(512);
        const DESCRIPTION: &'static str = "Maximum memory usage in MB";
        
        fn parse(value: &str) -> EnvResult<usize> {
            parse_positive_usize(value, Self::NAME, 64, 32768)
        }
    }
    
    /// 启用并行处理
    pub struct ParallelEnabled;
    impl EnvVar<bool> for ParallelEnabled {
        const NAME: &'static str = "MONOLITH_PARALLEL_ENABLED";
        const DEFAULT: Option<bool> = Some(true);
        const DESCRIPTION: &'static str = "Enable parallel processing";
        
        fn parse(value: &str) -> EnvResult<bool> {
            parse_bool(value, Self::NAME)
        }
    }
}

/// 安全相关环境变量
pub mod security {
    use super::*;
    
    /// API密钥
    pub struct ApiKey;
    impl EnvVar<String> for ApiKey {
        const NAME: &'static str = "MONOLITH_API_KEY";
        const DEFAULT: Option<String> = None; // 无默认值，必须设置
        const DESCRIPTION: &'static str = "API authentication key";
        
        fn parse(value: &str) -> EnvResult<String> {
            let key = value.trim();
            if key.len() < 8 {
                return Err(EnvError {
                    variable: Self::NAME.to_string(),
                    message: "API key must be at least 8 characters".to_string(),
                });
            }
            Ok(key.to_string())
        }
    }
    
    /// CORS允许的源
    pub struct CorsOrigins;
    impl EnvVar<Vec<String>> for CorsOrigins {
        const NAME: &'static str = "MONOLITH_CORS_ORIGINS";
        const DEFAULT: Option<Vec<String>> = None;
        
        fn get() -> EnvResult<Vec<String>> {
            match env::var(Self::NAME) {
                Ok(value) => Self::parse(&value),
                Err(_) => Ok(vec!["*".to_string()]),
            }
        }
        const DESCRIPTION: &'static str = "CORS allowed origins (comma-separated)";
        
        fn parse(value: &str) -> EnvResult<Vec<String>> {
            Ok(value.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect())
        }
    }
}

/// 辅助函数
fn parse_bool(value: &str, var_name: &str) -> EnvResult<bool> {
    match value.to_lowercase().as_str() {
        "true" | "1" | "yes" | "on" | "enabled" => Ok(true),
        "false" | "0" | "no" | "off" | "disabled" => Ok(false),
        _ => Err(EnvError {
            variable: var_name.to_string(),
            message: format!("Invalid boolean value '{}'. Use: true/false, 1/0, yes/no, on/off, enabled/disabled", value),
        })
    }
}

fn parse_positive_usize(value: &str, var_name: &str, min: usize, max: usize) -> EnvResult<usize> {
    let num: usize = value.parse().map_err(|_| EnvError {
        variable: var_name.to_string(),
        message: "Must be a valid positive number".to_string(),
    })?;
    
    if num < min {
        return Err(EnvError {
            variable: var_name.to_string(),
            message: format!("Value {} is below minimum {}", num, min),
        });
    }
    
    if num > max {
        return Err(EnvError {
            variable: var_name.to_string(),
            message: format!("Value {} exceeds maximum {}", num, max),
        });
    }
    
    Ok(num)
}

fn is_privileged() -> bool {
    // 简化的权限检查
    false // 实际项目中可以添加更精确的权限检查
}

/// 环境变量配置汇总
#[derive(Debug, Clone)]
pub struct EnvConfig {
    // 核心配置
    pub mode: String,
    pub log_level: String,
    pub no_color: bool,
    
    // 翻译配置
    pub translation_enabled: bool,
    pub translation_target_lang: String,
    pub translation_source_lang: String,
    pub translation_api_url: String,
    pub translation_max_requests_per_second: f64,
    pub translation_max_concurrent_requests: usize,
    pub translation_batch_size: usize,
    pub translation_batch_timeout: Duration,
    
    // 缓存配置
    pub cache_enabled: bool,
    pub cache_local_size: usize,
    pub cache_ttl: Duration,
    pub cache_warmup_enabled: bool,
    
    // Web配置
    pub web_bind_address: String,
    pub web_port: u16,
    pub web_static_dir: String,
    
    // MongoDB配置
    pub mongodb_connection_string: String,
    pub mongodb_database_name: String,
    pub mongodb_collection_name: String,
    
    // 性能配置
    pub worker_threads: Option<usize>,
    pub max_memory_mb: usize,
    pub parallel_enabled: bool,
    
    // 安全配置
    pub api_key: Option<String>,
    pub cors_origins: Vec<String>,
}

impl EnvConfig {
    /// 从环境变量加载配置
    pub fn from_env() -> EnvResult<Self> {
        Ok(Self {
            // 核心配置
            mode: core::Mode::get()?,
            log_level: core::LogLevel::get()?,
            no_color: core::NoColor::get()?,
            
            // 翻译配置
            translation_enabled: translation::Enabled::get()?,
            translation_target_lang: translation::TargetLang::get()?,
            translation_source_lang: translation::SourceLang::get()?,
            translation_api_url: translation::ApiUrl::get()?,
            translation_max_requests_per_second: translation::MaxRequestsPerSecond::get()?,
            translation_max_concurrent_requests: translation::MaxConcurrentRequests::get()?,
            translation_batch_size: translation::BatchSize::get()?,
            translation_batch_timeout: translation::BatchTimeout::get()?,
            
            // 缓存配置
            cache_enabled: cache::Enabled::get()?,
            cache_local_size: cache::LocalCacheSize::get()?,
            cache_ttl: cache::Ttl::get()?,
            cache_warmup_enabled: cache::WarmupEnabled::get()?,
            
            // Web配置
            web_bind_address: web::BindAddress::get()?,
            web_port: web::Port::get()?,
            web_static_dir: web::StaticDir::get()?,
            
            // MongoDB配置
            mongodb_connection_string: mongodb::ConnectionString::get()?,
            mongodb_database_name: mongodb::DatabaseName::get()?,
            mongodb_collection_name: mongodb::CollectionName::get()?,
            
            // 性能配置
            worker_threads: performance::WorkerThreads::get().ok(),
            max_memory_mb: performance::MaxMemoryMb::get()?,
            parallel_enabled: performance::ParallelEnabled::get()?,
            
            // 安全配置
            api_key: security::ApiKey::get().ok(),
            cors_origins: security::CorsOrigins::get()?,
        })
    }
    
    /// 验证配置
    pub fn validate(&self) -> EnvResult<()> {
        // 可以添加跨字段验证逻辑
        Ok(())
    }
    
    /// 打印配置摘要（隐藏敏感信息）
    pub fn print_summary(&self) {
        println!("Environment Configuration Summary:");
        println!("  Mode: {}", self.mode);
        println!("  Log Level: {}", self.log_level);
        println!("  Translation: {}", if self.translation_enabled { "enabled" } else { "disabled" });
        println!("  Cache: {}", if self.cache_enabled { "enabled" } else { "disabled" });
        println!("  Web Server: {}:{}", self.web_bind_address, self.web_port);
        if self.api_key.is_some() {
            println!("  API Key: [configured]");
        }
    }
}

/// 环境变量文档生成器
pub fn generate_env_docs() -> String {
    let mut docs = String::new();
    docs.push_str("# Environment Variables Documentation\n\n");
    
    // 核心变量
    docs.push_str("## Core Configuration\n\n");
    docs.push_str(&format!("- `{}`: {} (default: {:?})\n", 
        core::Mode::NAME, core::Mode::DESCRIPTION, core::Mode::DEFAULT));
    docs.push_str(&format!("- `{}`: {} (default: {:?})\n", 
        core::LogLevel::NAME, core::LogLevel::DESCRIPTION, core::LogLevel::DEFAULT));
    docs.push_str(&format!("- `{}`: {} (default: {:?})\n", 
        core::NoColor::NAME, core::NoColor::DESCRIPTION, core::NoColor::DEFAULT));
    
    // 翻译变量
    docs.push_str("\n## Translation Configuration\n\n");
    docs.push_str(&format!("- `{}`: {} (default: {:?})\n", 
        translation::Enabled::NAME, translation::Enabled::DESCRIPTION, translation::Enabled::DEFAULT));
    docs.push_str(&format!("- `{}`: {} (default: {:?})\n", 
        translation::TargetLang::NAME, translation::TargetLang::DESCRIPTION, translation::TargetLang::DEFAULT));
    docs.push_str(&format!("- `{}`: {} (default: {:?})\n", 
        translation::SourceLang::NAME, translation::SourceLang::DESCRIPTION, translation::SourceLang::DEFAULT));
    docs.push_str(&format!("- `{}`: {} (default: {:?})\n", 
        translation::ApiUrl::NAME, translation::ApiUrl::DESCRIPTION, translation::ApiUrl::DEFAULT));
    
    // 更多部分...
    docs.push_str("\n## Cache Configuration\n\n");
    docs.push_str(&format!("- `{}`: {} (default: {:?})\n", 
        cache::Enabled::NAME, cache::Enabled::DESCRIPTION, cache::Enabled::DEFAULT));
    
    docs.push_str("\n## Web Server Configuration\n\n");
    docs.push_str(&format!("- `{}`: {} (default: {:?})\n", 
        web::BindAddress::NAME, web::BindAddress::DESCRIPTION, web::BindAddress::DEFAULT));
    
    docs.push_str("\n## MongoDB Configuration\n\n");
    docs.push_str(&format!("- `{}`: {} (default: {:?})\n", 
        mongodb::ConnectionString::NAME, mongodb::ConnectionString::DESCRIPTION, mongodb::ConnectionString::DEFAULT));
    
    docs
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    
    #[test]
    fn test_core_mode_parsing() {
        // 测试有效值
        assert_eq!(core::Mode::parse("development").unwrap(), "development");
        assert_eq!(core::Mode::parse("PRODUCTION").unwrap(), "production");
        assert_eq!(core::Mode::parse("staging").unwrap(), "staging");
        
        // 测试无效值
        assert!(core::Mode::parse("invalid").is_err());
    }
    
    #[test]
    fn test_boolean_parsing() {
        // 测试各种布尔值格式
        assert!(translation::Enabled::parse("true").unwrap());
        assert!(translation::Enabled::parse("1").unwrap());
        assert!(translation::Enabled::parse("YES").unwrap());
        assert!(translation::Enabled::parse("on").unwrap());
        
        assert!(!translation::Enabled::parse("false").unwrap());
        assert!(!translation::Enabled::parse("0").unwrap());
        assert!(!translation::Enabled::parse("NO").unwrap());
        assert!(!translation::Enabled::parse("off").unwrap());
        
        // 测试无效值
        assert!(translation::Enabled::parse("maybe").is_err());
    }
    
    #[test]
    fn test_url_validation() {
        // 测试有效URL
        assert!(translation::ApiUrl::parse("http://localhost:1188").is_ok());
        assert!(translation::ApiUrl::parse("https://api.example.com").is_ok());
        
        // 测试无效URL
        assert!(translation::ApiUrl::parse("ftp://example.com").is_err());
        assert!(translation::ApiUrl::parse("not-a-url").is_err());
    }
    
    #[test]
    fn test_numeric_validation() {
        // 测试数值范围
        assert!(translation::MaxRequestsPerSecond::parse("5.0").is_ok());
        assert!(translation::MaxRequestsPerSecond::parse("0.1").is_ok());
        
        // 测试超出范围
        assert!(translation::MaxRequestsPerSecond::parse("0").is_err());
        assert!(translation::MaxRequestsPerSecond::parse("2000").is_err());
        assert!(translation::MaxRequestsPerSecond::parse("invalid").is_err());
    }
    
    #[test]
    fn test_env_config_loading() {
        // 设置测试环境变量
        env::set_var("MONOLITH_MODE", "development");
        env::set_var("MONOLITH_TRANSLATION_ENABLED", "true");
        env::set_var("MONOLITH_WEB_PORT", "8080");
        
        let config = EnvConfig::from_env().unwrap();
        assert_eq!(config.mode, "development");
        assert!(config.translation_enabled);
        assert_eq!(config.web_port, 8080);
        
        // 清理测试环境变量
        env::remove_var("MONOLITH_MODE");
        env::remove_var("MONOLITH_TRANSLATION_ENABLED");
        env::remove_var("MONOLITH_WEB_PORT");
    }
}