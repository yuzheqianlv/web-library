//! 翻译模块统一错误处理
//!
//! 提供结构化错误类型和错误处理机制

use std::fmt;

#[cfg(feature = "translation")]
use thiserror::Error;

use crate::core::MonolithError;

/// 翻译错误类型
#[cfg(feature = "translation")]
#[derive(Error, Debug, Clone)]
pub enum TranslationError {
    /// 配置错误
    #[error("配置错误: {0}")]
    ConfigError(String),

    /// 网络错误
    #[error("网络错误: {0}")]
    NetworkError(String),

    /// 速率限制错误
    #[error("请求速率过快，已达到限制")]
    RateLimitExceeded,

    /// 输入验证错误
    #[error("输入无效: {0}")]
    InvalidInput(String),

    /// 缓存错误
    #[error("缓存错误: {0}")]
    CacheError(String),

    /// 批次处理错误
    #[error("批次处理错误: {0}")]
    BatchProcessingError(String),

    /// 文本收集错误
    #[error("文本收集错误: {0}")]
    TextCollectionError(String),

    /// 翻译服务错误
    #[error("翻译服务错误: {0}")]
    TranslationServiceError(String),

    /// 超时错误
    #[error("操作超时: {0}")]
    TimeoutError(String),

    /// 解析错误
    #[error("解析错误: {0}")]
    ParseError(String),

    /// 序列化错误
    #[error("序列化错误: {0}")]
    SerializationError(String),

    /// 并发错误
    #[error("并发操作错误: {0}")]
    ConcurrencyError(String),

    /// 资源不足错误
    #[error("资源不足: {0}")]
    ResourceExhausted(String),

    /// 处理错误
    #[error("处理错误: {0}")]
    ProcessingError(String),

    /// 内部错误
    #[error("内部错误: {0}")]
    InternalError(String),
}

#[cfg(feature = "translation")]
impl TranslationError {
    /// 检查错误是否可重试
    pub fn is_retryable(&self) -> bool {
        match self {
            TranslationError::NetworkError(_) => true,
            TranslationError::TimeoutError(_) => true,
            TranslationError::TranslationServiceError(_) => true,
            TranslationError::ResourceExhausted(_) => true,
            TranslationError::RateLimitExceeded => false, // 需要等待
            TranslationError::ConfigError(_) => false,
            TranslationError::InvalidInput(_) => false,
            TranslationError::ParseError(_) => false,
            TranslationError::SerializationError(_) => false,
            TranslationError::CacheError(_) => true,
            TranslationError::BatchProcessingError(_) => true,
            TranslationError::TextCollectionError(_) => false,
            TranslationError::ConcurrencyError(_) => true,
            TranslationError::ProcessingError(_) => true,
            TranslationError::InternalError(_) => false,
        }
    }

    /// 获取错误的严重程度
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            TranslationError::ConfigError(_) => ErrorSeverity::Critical,
            TranslationError::NetworkError(_) => ErrorSeverity::Warning,
            TranslationError::RateLimitExceeded => ErrorSeverity::Warning,
            TranslationError::InvalidInput(_) => ErrorSeverity::Info,
            TranslationError::CacheError(_) => ErrorSeverity::Warning,
            TranslationError::BatchProcessingError(_) => ErrorSeverity::Error,
            TranslationError::TextCollectionError(_) => ErrorSeverity::Error,
            TranslationError::TranslationServiceError(_) => ErrorSeverity::Error,
            TranslationError::TimeoutError(_) => ErrorSeverity::Warning,
            TranslationError::ParseError(_) => ErrorSeverity::Error,
            TranslationError::SerializationError(_) => ErrorSeverity::Error,
            TranslationError::ConcurrencyError(_) => ErrorSeverity::Warning,
            TranslationError::ResourceExhausted(_) => ErrorSeverity::Warning,
            TranslationError::ProcessingError(_) => ErrorSeverity::Error,
            TranslationError::InternalError(_) => ErrorSeverity::Critical,
        }
    }

    /// 获取错误类别
    pub fn category(&self) -> ErrorCategory {
        match self {
            TranslationError::ConfigError(_) => ErrorCategory::Configuration,
            TranslationError::NetworkError(_) => ErrorCategory::Network,
            TranslationError::RateLimitExceeded => ErrorCategory::RateLimit,
            TranslationError::InvalidInput(_) => ErrorCategory::Input,
            TranslationError::CacheError(_) => ErrorCategory::Cache,
            TranslationError::BatchProcessingError(_) => ErrorCategory::Processing,
            TranslationError::TextCollectionError(_) => ErrorCategory::Processing,
            TranslationError::TranslationServiceError(_) => ErrorCategory::Service,
            TranslationError::TimeoutError(_) => ErrorCategory::Timeout,
            TranslationError::ParseError(_) => ErrorCategory::Parsing,
            TranslationError::SerializationError(_) => ErrorCategory::Serialization,
            TranslationError::ConcurrencyError(_) => ErrorCategory::Concurrency,
            TranslationError::ResourceExhausted(_) => ErrorCategory::Resource,
            TranslationError::ProcessingError(_) => ErrorCategory::Processing,
            TranslationError::InternalError(_) => ErrorCategory::Internal,
        }
    }

    /// 创建带上下文的错误
    pub fn with_context<T: fmt::Display>(mut self, context: T) -> Self {
        let current_msg = self.to_string();
        let new_msg = format!("{} (上下文: {})", current_msg, context);

        match &mut self {
            TranslationError::ConfigError(ref mut msg) => *msg = new_msg,
            TranslationError::NetworkError(ref mut msg) => *msg = new_msg,
            TranslationError::InvalidInput(ref mut msg) => *msg = new_msg,
            TranslationError::CacheError(ref mut msg) => *msg = new_msg,
            TranslationError::BatchProcessingError(ref mut msg) => *msg = new_msg,
            TranslationError::TextCollectionError(ref mut msg) => *msg = new_msg,
            TranslationError::TranslationServiceError(ref mut msg) => *msg = new_msg,
            TranslationError::TimeoutError(ref mut msg) => *msg = new_msg,
            TranslationError::ParseError(ref mut msg) => *msg = new_msg,
            TranslationError::SerializationError(ref mut msg) => *msg = new_msg,
            TranslationError::ConcurrencyError(ref mut msg) => *msg = new_msg,
            TranslationError::ResourceExhausted(ref mut msg) => *msg = new_msg,
            TranslationError::ProcessingError(ref mut msg) => *msg = new_msg,
            TranslationError::InternalError(ref mut msg) => *msg = new_msg,
            TranslationError::RateLimitExceeded => {
                return TranslationError::RateLimitExceeded;
            }
        }

        self
    }
}

/// 错误严重程度
#[cfg(feature = "translation")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ErrorSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// 错误类别
#[cfg(feature = "translation")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorCategory {
    Configuration,
    Network,
    RateLimit,
    Input,
    Cache,
    Processing,
    Service,
    Timeout,
    Parsing,
    Serialization,
    Concurrency,
    Resource,
    Internal,
}

/// 从MonolithError转换
#[cfg(feature = "translation")]
impl From<MonolithError> for TranslationError {
    fn from(error: MonolithError) -> Self {
        let msg = error.to_string();

        // 根据错误消息内容判断错误类型
        if msg.contains("timeout") || msg.contains("超时") {
            TranslationError::TimeoutError(msg)
        } else if msg.contains("network") || msg.contains("网络") {
            TranslationError::NetworkError(msg)
        } else if msg.contains("config") || msg.contains("配置") {
            TranslationError::ConfigError(msg)
        } else if msg.contains("cache") || msg.contains("缓存") {
            TranslationError::CacheError(msg)
        } else {
            TranslationError::InternalError(msg)
        }
    }
}

/// 转换为MonolithError（向后兼容）
#[cfg(feature = "translation")]
impl From<TranslationError> for MonolithError {
    fn from(error: TranslationError) -> Self {
        MonolithError::new(&error.to_string())
    }
}

/// 标准错误转换
#[cfg(feature = "translation")]
impl From<std::io::Error> for TranslationError {
    fn from(error: std::io::Error) -> Self {
        TranslationError::NetworkError(format!("IO错误: {}", error))
    }
}

#[cfg(feature = "translation")]
impl From<serde_json::Error> for TranslationError {
    fn from(error: serde_json::Error) -> Self {
        TranslationError::SerializationError(format!("JSON序列化错误: {}", error))
    }
}

#[cfg(feature = "translation")]
impl From<toml::de::Error> for TranslationError {
    fn from(error: toml::de::Error) -> Self {
        TranslationError::ParseError(format!("TOML解析错误: {}", error))
    }
}

#[cfg(feature = "translation")]
impl From<config::ConfigError> for TranslationError {
    fn from(error: config::ConfigError) -> Self {
        TranslationError::ConfigError(format!("配置错误: {}", error))
    }
}

#[cfg(feature = "translation")]
impl From<tokio::time::error::Elapsed> for TranslationError {
    fn from(error: tokio::time::error::Elapsed) -> Self {
        TranslationError::TimeoutError(format!("异步操作超时: {}", error))
    }
}

/// 错误结果类型别名
#[cfg(feature = "translation")]
pub type TranslationResult<T> = Result<T, TranslationError>;

/// 错误统计信息
#[cfg(feature = "translation")]
#[derive(Debug, Clone, Default)]
pub struct ErrorStats {
    pub total_errors: usize,
    pub by_category: std::collections::HashMap<ErrorCategory, usize>,
    pub by_severity: std::collections::HashMap<ErrorSeverity, usize>,
    pub retryable_errors: usize,
    pub critical_errors: usize,
}

#[cfg(feature = "translation")]
impl ErrorStats {
    /// 记录错误
    pub fn record_error(&mut self, error: &TranslationError) {
        self.total_errors += 1;

        let category = error.category();
        *self.by_category.entry(category).or_insert(0) += 1;

        let severity = error.severity();
        *self.by_severity.entry(severity).or_insert(0) += 1;

        if error.is_retryable() {
            self.retryable_errors += 1;
        }

        if severity == ErrorSeverity::Critical {
            self.critical_errors += 1;
        }
    }

    /// 重置统计
    pub fn reset(&mut self) {
        *self = Default::default();
    }

    /// 获取错误率
    pub fn error_rate(&self, total_operations: usize) -> f64 {
        if total_operations == 0 {
            0.0
        } else {
            self.total_errors as f64 / total_operations as f64
        }
    }
}

/// 错误处理助手函数
#[cfg(feature = "translation")]
pub mod helpers {
    use super::*;

    /// 记录并返回错误
    pub fn log_error<T>(error: TranslationError) -> TranslationResult<T> {
        match error.severity() {
            ErrorSeverity::Info => tracing::info!("翻译信息: {}", error),
            ErrorSeverity::Warning => tracing::warn!("翻译警告: {}", error),
            ErrorSeverity::Error => tracing::error!("翻译错误: {}", error),
            ErrorSeverity::Critical => tracing::error!("翻译严重错误: {}", error),
        }

        Err(error)
    }

    /// 创建网络错误
    pub fn network_error<T: fmt::Display>(msg: T) -> TranslationError {
        TranslationError::NetworkError(msg.to_string())
    }

    /// 创建配置错误
    pub fn config_error<T: fmt::Display>(msg: T) -> TranslationError {
        TranslationError::ConfigError(msg.to_string())
    }

    /// 创建缓存错误
    pub fn cache_error<T: fmt::Display>(msg: T) -> TranslationError {
        TranslationError::CacheError(msg.to_string())
    }

    /// 创建输入验证错误
    pub fn validation_error<T: fmt::Display>(msg: T) -> TranslationError {
        TranslationError::InvalidInput(msg.to_string())
    }

    /// 创建超时错误
    pub fn timeout_error<T: fmt::Display>(msg: T) -> TranslationError {
        TranslationError::TimeoutError(msg.to_string())
    }

    /// 创建内部错误
    pub fn internal_error<T: fmt::Display>(msg: T) -> TranslationError {
        TranslationError::InternalError(msg.to_string())
    }
}

/// 非translation feature时的简化错误类型
#[cfg(not(feature = "translation"))]
#[derive(Debug, Clone)]
pub struct TranslationError {
    message: String,
}

#[cfg(not(feature = "translation"))]
impl TranslationError {
    pub fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
        }
    }
}

#[cfg(not(feature = "translation"))]
impl fmt::Display for TranslationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

#[cfg(not(feature = "translation"))]
impl std::error::Error for TranslationError {}

#[cfg(not(feature = "translation"))]
impl From<MonolithError> for TranslationError {
    fn from(error: MonolithError) -> Self {
        Self::new(&error.to_string())
    }
}

#[cfg(not(feature = "translation"))]
impl From<TranslationError> for MonolithError {
    fn from(error: TranslationError) -> Self {
        MonolithError::new(&error.to_string())
    }
}

#[cfg(not(feature = "translation"))]
pub type TranslationResult<T> = Result<T, TranslationError>;
