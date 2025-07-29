//! 翻译模块统一错误处理系统
//!
//! 本模块为 Monolith 项目的翻译功能提供完整的错误处理机制，包括：
//! 
//! ## 核心功能
//! 
//! - **结构化错误类型**：定义了涵盖翻译流程各个环节的详细错误类型
//! - **错误分类系统**：按严重程度和类别对错误进行分类管理
//! - **重试机制支持**：标识哪些错误可以重试，哪些不能重试
//! - **错误统计功能**：提供错误收集和分析工具
//! - **便捷的转换机制**：支持与其他错误类型的自动转换
//! 
//! ## 设计理念
//! 
//! 该错误处理系统采用分层设计：
//! 1. **基础错误类型**：`TranslationError` 枚举覆盖所有可能的错误情况
//! 2. **错误元数据**：每个错误都有严重程度、类别和重试性标记
//! 3. **辅助工具**：提供统计、日志记录和快速创建错误的助手函数
//! 4. **兼容性保证**：在未启用 translation 特性时提供简化实现
//! 
//! ## 使用场景
//! 
//! - 翻译 API 调用失败处理
//! - 网络连接和超时错误管理
//! - 文本解析和处理错误
//! - 缓存操作错误处理
//! - 并发操作错误管理
//! 
//! ## 特性支持
//! 
//! 本模块通过 `#[cfg(feature = "translation")]` 条件编译，
//! 在未启用翻译功能时提供简化的错误类型以减少依赖。

use std::fmt;

#[cfg(feature = "translation")]
use thiserror::Error;

use crate::core::MonolithError;

/// 翻译模块统一错误类型枚举
/// 
/// 此枚举定义了翻译过程中可能出现的所有错误类型，每种错误都有明确的语义和处理策略。
/// 错误类型按功能域划分，便于错误处理和调试。
/// 
/// # 错误分类
/// 
/// - **配置类**：`ConfigError` - 系统配置相关错误
/// - **网络类**：`NetworkError`, `TimeoutError`, `RateLimitExceeded` - 网络通信错误
/// - **数据类**：`InvalidInput`, `ParseError`, `SerializationError` - 数据处理错误
/// - **系统类**：`CacheError`, `ConcurrencyError`, `ResourceExhausted` - 系统资源错误
/// - **业务类**：`TranslationServiceError`, `BatchProcessingError`, `TextCollectionError` - 业务逻辑错误
/// - **通用类**：`ProcessingError`, `InternalError` - 通用处理错误
/// 
/// # 使用示例
/// 
/// ```rust
/// use crate::translation::error::TranslationError;
/// 
/// // 创建网络错误
/// let error = TranslationError::NetworkError("连接超时".to_string());
/// 
/// // 检查是否可重试
/// if error.is_retryable() {
///     println!("此错误可以重试");
/// }
/// 
/// // 获取错误严重程度
/// match error.severity() {
///     ErrorSeverity::Critical => println!("严重错误，需要立即处理"),
///     _ => println!("一般错误"),
/// }
/// ```
#[cfg(feature = "translation")]
#[derive(Error, Debug, Clone)]
pub enum TranslationError {
    /// 配置相关错误
    /// 
    /// 当系统配置文件无效、缺失必要配置项、或配置值不正确时触发。
    /// 此类错误通常是致命的，需要修复配置后重启系统。
    /// 
    /// **示例场景**：
    /// - 翻译 API 密钥未设置或无效
    /// - 配置文件格式错误
    /// - 必需的配置参数缺失
    #[error("配置错误: {0}")]
    ConfigError(String),

    /// 网络通信错误
    /// 
    /// 当与外部服务（如翻译 API）通信失败时触发。
    /// 此类错误通常是临时的，可以通过重试解决。
    /// 
    /// **示例场景**：
    /// - DNS 解析失败
    /// - 服务器连接被拒绝
    /// - 网络连接中断
    #[error("网络错误: {0}")]
    NetworkError(String),

    /// API 调用速率限制错误
    /// 
    /// 当请求频率超过外部服务的限制时触发。
    /// 此错误不应立即重试，需要等待一段时间后再尝试。
    /// 
    /// **处理策略**：实现指数退避算法，逐渐增加重试间隔
    #[error("请求速率过快，已达到限制")]
    RateLimitExceeded,

    /// 输入验证错误
    /// 
    /// 当用户输入或系统输入不符合预期格式或要求时触发。
    /// 此类错误不可重试，需要修正输入数据。
    /// 
    /// **示例场景**：
    /// - 文本内容为空或过长
    /// - 不支持的语言代码
    /// - 格式不正确的 URL 或路径
    #[error("输入无效: {0}")]
    InvalidInput(String),

    /// 缓存操作错误
    /// 
    /// 当缓存读写操作失败时触发。
    /// 通常可以通过重试或降级到非缓存模式解决。
    /// 
    /// **示例场景**：
    /// - 缓存文件损坏
    /// - 磁盘空间不足
    /// - 缓存键冲突
    #[error("缓存错误: {0}")]
    CacheError(String),

    /// 批次处理错误
    /// 
    /// 当批量处理翻译任务时出现错误。
    /// 可能影响整个批次或部分任务的处理。
    /// 
    /// **示例场景**：
    /// - 批次中某些任务失败
    /// - 批次大小超出限制
    /// - 批次处理超时
    #[error("批次处理错误: {0}")]
    BatchProcessingError(String),

    /// 文本收集错误
    /// 
    /// 当从 HTML 文档中提取或收集文本内容时出现错误。
    /// 通常由文档结构异常或解析器问题引起。
    /// 
    /// **示例场景**：
    /// - HTML 结构损坏
    /// - 文本节点访问失败
    /// - 编码转换错误
    #[error("文本收集错误: {0}")]
    TextCollectionError(String),

    /// 翻译服务错误
    /// 
    /// 当外部翻译服务返回错误响应时触发。
    /// 包括 API 错误、服务不可用、认证失败等。
    /// 
    /// **示例场景**：
    /// - API 密钥无效或过期
    /// - 翻译服务暂时不可用
    /// - 请求格式不被服务支持
    #[error("翻译服务错误: {0}")]
    TranslationServiceError(String),

    /// 操作超时错误
    /// 
    /// 当操作执行时间超过预设限制时触发。
    /// 通常可以通过调整超时设置或重试解决。
    /// 
    /// **示例场景**：
    /// - API 请求响应慢
    /// - 大文件处理耗时过长
    /// - 网络延迟过高
    #[error("操作超时: {0}")]
    TimeoutError(String),

    /// 数据解析错误
    /// 
    /// 当解析结构化数据（JSON、XML、TOML 等）失败时触发。
    /// 通常由数据格式错误或不兼容的版本引起。
    /// 
    /// **示例场景**：
    /// - JSON 响应格式错误
    /// - 配置文件语法错误
    /// - API 响应结构变更
    #[error("解析错误: {0}")]
    ParseError(String),

    /// 序列化错误
    /// 
    /// 当将数据结构转换为字符串格式时失败。
    /// 可能由数据类型不兼容或序列化库问题引起。
    /// 
    /// **示例场景**：
    /// - 数据结构包含不可序列化的字段
    /// - 序列化库版本不兼容
    /// - 内存不足导致序列化失败
    #[error("序列化错误: {0}")]
    SerializationError(String),

    /// 并发操作错误
    /// 
    /// 当多线程或异步操作出现竞态条件或死锁时触发。
    /// 通常需要检查并发控制逻辑。
    /// 
    /// **示例场景**：
    /// - 锁竞争导致的死锁
    /// - 异步任务取消
    /// - 共享资源访问冲突
    #[error("并发操作错误: {0}")]
    ConcurrencyError(String),

    /// 系统资源不足错误
    /// 
    /// 当系统资源（内存、磁盘、文件句柄等）不足时触发。
    /// 需要等待资源释放或优化资源使用。
    /// 
    /// **示例场景**：
    /// - 内存不足
    /// - 磁盘空间不足
    /// - 文件句柄数量达到限制
    #[error("资源不足: {0}")]
    ResourceExhausted(String),

    /// 通用处理错误
    /// 
    /// 当业务逻辑处理过程中出现未分类的错误时触发。
    /// 作为其他具体错误类型的后备选项。
    #[error("处理错误: {0}")]
    ProcessingError(String),

    /// 系统内部错误
    /// 
    /// 当出现不应该发生的内部错误时触发。
    /// 通常表示代码逻辑错误或系统异常状态。
    /// 
    /// **处理策略**：记录详细日志，通知开发人员修复
    #[error("内部错误: {0}")]
    InternalError(String),
}

#[cfg(feature = "translation")]
impl TranslationError {
    /// 判断错误是否可以重试
    /// 
    /// 基于错误类型的特性，判断该错误是否适合进行重试操作。
    /// 这对于实现错误恢复和重试机制非常重要。
    /// 
    /// # 重试策略
    /// 
    /// **可重试的错误类型**：
    /// - `NetworkError` - 网络问题通常是临时的
    /// - `TimeoutError` - 超时可能是网络拥塞导致的
    /// - `TranslationServiceError` - 服务可能临时不可用
    /// - `ResourceExhausted` - 资源可能很快被释放
    /// - `CacheError` - 缓存问题可以降级处理
    /// - `BatchProcessingError` - 可以重新组织批次
    /// - `ConcurrencyError` - 并发冲突可能是临时的
    /// - `ProcessingError` - 处理问题可能是临时的
    /// 
    /// **不可重试的错误类型**：
    /// - `RateLimitExceeded` - 需要等待，不是立即重试
    /// - `ConfigError` - 配置错误需要人工修复
    /// - `InvalidInput` - 输入错误需要修正数据
    /// - `ParseError` - 解析错误通常是数据格式问题
    /// - `SerializationError` - 序列化错误通常是代码问题
    /// - `TextCollectionError` - 文本收集失败通常是结构问题
    /// - `InternalError` - 内部错误需要代码修复
    /// 
    /// # 返回值
    /// 
    /// - `true` - 该错误可以重试
    /// - `false` - 该错误不应该重试
    /// 
    /// # 使用示例
    /// 
    /// ```rust
    /// use crate::translation::error::TranslationError;
    /// 
    /// let error = TranslationError::NetworkError("连接失败".to_string());
    /// if error.is_retryable() {
    ///     // 实现重试逻辑
    ///     println!("准备重试操作");
    /// } else {
    ///     println!("错误不可重试，需要人工处理");
    /// }
    /// ```
    pub fn is_retryable(&self) -> bool {
        match self {
            // 网络相关错误通常可以重试
            TranslationError::NetworkError(_) => true,
            TranslationError::TimeoutError(_) => true,
            TranslationError::TranslationServiceError(_) => true,
            
            // 资源相关错误可以重试
            TranslationError::ResourceExhausted(_) => true,
            TranslationError::CacheError(_) => true,
            
            // 处理相关错误可以重试
            TranslationError::BatchProcessingError(_) => true,
            TranslationError::ConcurrencyError(_) => true,
            TranslationError::ProcessingError(_) => true,
            
            // 速率限制需要等待，不是立即重试
            TranslationError::RateLimitExceeded => false,
            
            // 配置和输入错误需要人工修复
            TranslationError::ConfigError(_) => false,
            TranslationError::InvalidInput(_) => false,
            
            // 数据格式错误通常不可重试
            TranslationError::ParseError(_) => false,
            TranslationError::SerializationError(_) => false,
            TranslationError::TextCollectionError(_) => false,
            
            // 内部错误需要代码修复
            TranslationError::InternalError(_) => false,
        }
    }

    /// 获取错误的严重程度级别
    /// 
    /// 根据错误对系统运行的影响程度，返回相应的严重程度级别。
    /// 这有助于错误日志记录、告警系统和错误处理优先级的确定。
    /// 
    /// # 严重程度分级
    /// 
    /// - **Critical（严重）**：系统无法正常运行，需要立即处理
    ///   - `ConfigError` - 配置错误导致功能无法使用
    ///   - `InternalError` - 内部错误可能影响系统稳定性
    /// 
    /// - **Error（错误）**：功能受影响，但系统仍可运行
    ///   - `BatchProcessingError` - 批处理失败影响效率
    ///   - `TextCollectionError` - 文本收集失败影响功能
    ///   - `TranslationServiceError` - 翻译服务不可用
    ///   - `ParseError` - 解析错误影响数据处理
    ///   - `SerializationError` - 序列化错误影响数据传输
    ///   - `ProcessingError` - 处理错误影响业务逻辑
    /// 
    /// - **Warning（警告）**：有问题但可以降级处理
    ///   - `NetworkError` - 网络问题可能是临时的
    ///   - `RateLimitExceeded` - 速率限制需要调整策略
    ///   - `CacheError` - 缓存问题可以绕过
    ///   - `TimeoutError` - 超时可以增加等待时间
    ///   - `ConcurrencyError` - 并发问题可以重试
    ///   - `ResourceExhausted` - 资源不足可以等待
    /// 
    /// - **Info（信息）**：轻微问题，不影响主要功能
    ///   - `InvalidInput` - 输入错误可以提示用户修正
    /// 
    /// # 返回值
    /// 
    /// 返回 `ErrorSeverity` 枚举值，表示错误的严重程度
    /// 
    /// # 使用示例
    /// 
    /// ```rust
    /// use crate::translation::error::{TranslationError, ErrorSeverity};
    /// 
    /// let error = TranslationError::ConfigError("API密钥无效".to_string());
    /// match error.severity() {
    ///     ErrorSeverity::Critical => println!("严重错误！需要立即处理"),
    ///     ErrorSeverity::Error => println!("错误：功能受影响"),
    ///     ErrorSeverity::Warning => println!("警告：存在问题但可继续"),
    ///     ErrorSeverity::Info => println!("信息：轻微问题"),
    /// }
    /// ```
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            // 严重错误：影响系统核心功能
            TranslationError::ConfigError(_) => ErrorSeverity::Critical,
            TranslationError::InternalError(_) => ErrorSeverity::Critical,
            
            // 一般错误：影响功能但系统仍可运行
            TranslationError::BatchProcessingError(_) => ErrorSeverity::Error,
            TranslationError::TextCollectionError(_) => ErrorSeverity::Error,
            TranslationError::TranslationServiceError(_) => ErrorSeverity::Error,
            TranslationError::ParseError(_) => ErrorSeverity::Error,
            TranslationError::SerializationError(_) => ErrorSeverity::Error,
            TranslationError::ProcessingError(_) => ErrorSeverity::Error,
            
            // 警告：有问题但可以处理
            TranslationError::NetworkError(_) => ErrorSeverity::Warning,
            TranslationError::RateLimitExceeded => ErrorSeverity::Warning,
            TranslationError::CacheError(_) => ErrorSeverity::Warning,
            TranslationError::TimeoutError(_) => ErrorSeverity::Warning,
            TranslationError::ConcurrencyError(_) => ErrorSeverity::Warning,
            TranslationError::ResourceExhausted(_) => ErrorSeverity::Warning,
            
            // 信息：轻微问题
            TranslationError::InvalidInput(_) => ErrorSeverity::Info,
        }
    }

    /// 获取错误的功能类别
    /// 
    /// 将错误按功能领域进行分类，便于错误统计、监控和专门处理。
    /// 每个类别代表系统中的一个功能模块或处理阶段。
    /// 
    /// # 错误类别说明
    /// 
    /// - **Configuration**：配置相关错误
    /// - **Network**：网络通信错误  
    /// - **RateLimit**：API调用速率限制
    /// - **Input**：输入验证错误
    /// - **Cache**：缓存操作错误
    /// - **Processing**：业务处理错误
    /// - **Service**：外部服务错误
    /// - **Timeout**：超时错误
    /// - **Parsing**：数据解析错误
    /// - **Serialization**：数据序列化错误
    /// - **Concurrency**：并发操作错误
    /// - **Resource**：系统资源错误
    /// - **Internal**：内部系统错误
    /// 
    /// # 返回值
    /// 
    /// 返回 `ErrorCategory` 枚举值，表示错误所属的功能类别
    /// 
    /// # 使用场景
    /// 
    /// - **错误监控**：按类别统计错误发生频率
    /// - **告警配置**：针对不同类别设置不同的告警策略
    /// - **错误分析**：识别系统中最常出现问题的功能模块
    /// - **专门处理**：为不同类别的错误实现专门的处理逻辑
    /// 
    /// # 使用示例
    /// 
    /// ```rust
    /// use crate::translation::error::{TranslationError, ErrorCategory};
    /// 
    /// let error = TranslationError::NetworkError("连接超时".to_string());
    /// match error.category() {
    ///     ErrorCategory::Network => {
    ///         println!("网络错误，检查网络连接");
    ///         // 实现网络错误专门处理逻辑
    ///     },
    ///     ErrorCategory::Configuration => {
    ///         println!("配置错误，检查配置文件");
    ///     },
    ///     _ => println!("其他类别错误"),
    /// }
    /// ```
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

    /// 为错误添加上下文信息
    /// 
    /// 在现有错误信息的基础上添加额外的上下文信息，
    /// 帮助更好地理解错误发生的环境和原因。
    /// 
    /// # 参数
    /// 
    /// - `context` - 要添加的上下文信息，任何实现了 `fmt::Display` 的类型
    /// 
    /// # 返回值
    /// 
    /// 返回带有上下文信息的新错误实例。原始错误的类型保持不变，
    /// 只是错误消息中会包含上下文信息。
    /// 
    /// # 上下文格式
    /// 
    /// 新的错误消息格式为：`原始错误消息 (上下文: 上下文信息)`
    /// 
    /// # 特殊处理
    /// 
    /// 对于 `RateLimitExceeded` 错误，由于它不包含自定义消息，
    /// 直接返回原错误而不添加上下文。
    /// 
    /// # 使用场景
    /// 
    /// - **调试信息**：添加函数名、文件位置等调试信息
    /// - **业务上下文**：添加用户ID、请求ID等业务信息  
    /// - **操作上下文**：添加正在执行的操作描述
    /// - **数据上下文**：添加相关的数据标识符
    /// 
    /// # 使用示例
    /// 
    /// ```rust
    /// use crate::translation::error::TranslationError;
    /// 
    /// let error = TranslationError::NetworkError("连接失败".to_string());
    /// let error_with_context = error.with_context("翻译API调用");
    /// 
    /// println!("{}", error_with_context);
    /// // 输出：网络错误: 连接失败 (上下文: 翻译API调用)
    /// 
    /// // 链式调用添加多层上下文
    /// let detailed_error = TranslationError::CacheError("写入失败".to_string())
    ///     .with_context("用户123")
    ///     .with_context("批量翻译任务");
    /// ```
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
