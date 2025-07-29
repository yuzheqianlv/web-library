//! 翻译模块 - 重构后的版本
//!
//! 提供完整的翻译功能，采用清晰的模块化架构：
//! - **core**: 核心翻译服务和引擎
//! - **pipeline**: 文本处理管道（收集、过滤、批次）
//! - **storage**: 缓存和持久化存储
//! - **config**: 配置管理
//! - **error**: 错误处理
//!
//! # 基本用法
//!
//! ```rust,no_run
//! use monolith::translation::{TranslationService, TranslationConfig};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // 创建翻译服务
//! let config = TranslationConfig::default_with_lang("zh", None);
//! let mut service = TranslationService::create_default("zh", None)?;
//!
//! // 翻译DOM内容
//! let dom = /* 获取DOM */;
//! let translated_dom = service.translate_dom(dom).await?;
//! # Ok(())
//! # }
//! ```

// ============================================================================
// 子模块声明
// ============================================================================

/// 配置管理模块 - 处理翻译相关的所有配置
/// 
/// 提供翻译API配置、语言设置、缓存配置等功能
pub mod config;

/// 核心翻译引擎模块 - 主要的翻译服务实现
/// 
/// 包含翻译服务、健康检查、统计信息等核心功能
pub mod core;

/// 错误处理模块 - 统一的错误类型和处理机制
/// 
/// 定义了翻译过程中可能出现的各种错误类型
pub mod error;

/// 文本处理管道模块 - 文本收集、过滤和批次处理
/// 
/// 负责从DOM中收集可翻译文本、进行智能过滤和优化批次处理
pub mod pipeline;

/// 存储管理模块 - 缓存和持久化存储
/// 
/// 提供翻译结果缓存、配置存储等功能
pub mod storage;

// ============================================================================
// 向后兼容的模块别名（已弃用，将在未来版本中移除）
// ============================================================================

/// 批次处理模块（向后兼容别名）
/// 
/// **已弃用**: 请使用 `pipeline::batch` 替代
#[deprecated(since = "2.1.0", note = "请使用 pipeline::batch 替代")]
pub mod batch {
    pub use crate::translation::pipeline::batch::*;
}

/// 缓存管理模块（向后兼容别名）
/// 
/// **已弃用**: 请使用 `storage::cache` 替代
#[deprecated(since = "2.1.0", note = "请使用 storage::cache 替代")]
pub mod cache {
    pub use crate::translation::storage::cache::*;
}

/// 文本收集器模块（向后兼容别名）
/// 
/// **已弃用**: 请使用 `pipeline::collector` 替代
#[deprecated(since = "2.1.0", note = "请使用 pipeline::collector 替代")]
pub mod collector {
    pub use crate::translation::pipeline::collector::*;
}

/// 文本过滤器模块（向后兼容别名）
/// 
/// **已弃用**: 请使用 `pipeline::filters` 替代
#[deprecated(since = "2.1.0", note = "请使用 pipeline::filters 替代")]
pub mod filters {
    pub use crate::translation::pipeline::filters::*;
}

/// 翻译服务模块（向后兼容别名）
/// 
/// **已弃用**: 请使用 `core::service` 替代
#[deprecated(since = "2.1.0", note = "请使用 core::service 替代")]
pub mod service {
    pub use crate::translation::core::service::*;
}

/// 文本处理器模块
/// 
/// 处理不同格式的文本内容，包括HTML、CSS等的翻译处理
/// 
/// **注意**: 此模块仍然存在于根目录，未来可能会重构到pipeline中
pub mod processor;

// ============================================================================
// 核心API导出 - 主要的公共接口
// ============================================================================

/// 翻译服务的主要组件
/// 
/// - `TranslationService`: 主翻译服务，提供完整的翻译功能
/// - `HealthStatus`: 服务健康状态检查
/// - `HealthLevel`: 健康等级枚举
/// - `ServiceStats`: 服务统计信息
pub use core::{HealthLevel, HealthStatus, ServiceStats, TranslationService};

/// 配置管理相关组件
/// 
/// - `TranslationConfig`: 翻译配置结构体
/// - `ConfigManager`: 配置管理器，处理配置文件读写
/// - `constants`: 配置常量模块
pub use config::{constants, ConfigManager, TranslationConfig};

/// 错误处理相关类型
/// 
/// - `TranslationError`: 翻译错误的统一类型
/// - `TranslationResult<T>`: 翻译操作的结果类型
/// - `ErrorCategory`: 错误分类枚举
/// - `ErrorSeverity`: 错误严重程度枚举
pub use error::{ErrorCategory, ErrorSeverity, TranslationError, TranslationResult};

// ============================================================================
// 高级API导出 - 供高级用户和扩展开发使用
// ============================================================================

/// 文本处理管道组件
/// 
/// 这些组件提供细粒度的文本处理控制：
/// - `TextCollector`: 从DOM中收集可翻译文本
/// - `TextFilter`: 过滤和分析文本的可翻译性
/// - `BatchManager`: 管理翻译批次，优化API调用
/// - `TextItem`: 表示单个可翻译文本项
/// - `Batch`: 表示一个翻译批次
/// - `TextAnalysis`: 文本分析结果
/// - `LanguageHint`: 语言提示信息
/// - 各种枚举类型：`BatchType`, `BatchPriority`, `TextType`, `TextPriority`
pub use pipeline::{
    Batch, BatchManager, BatchPriority, BatchType, LanguageHint, TextAnalysis, TextCollector,
    TextFilter, TextItem, TextPriority, TextType,
};

/// 存储和缓存组件
/// 
/// 提供翻译结果的持久化和缓存功能：
/// - `CacheManager`: 缓存管理器，处理翻译结果缓存
/// - `CacheConfig`: 缓存配置结构体
/// - `CacheEntry`: 缓存条目表示
/// - `CacheStats`: 缓存统计信息
pub use storage::{CacheConfig, CacheEntry, CacheManager, CacheStats};

// ============================================================================
// 便利函数导出 - 简化常见操作的高级函数
// ============================================================================

/// 翻译HTML DOM内容（异步版本）
///
/// 这是最常用的翻译API，内部会自动处理文本收集、批次处理和缓存
///
/// # 参数
///
/// * `dom` - 要翻译的HTML DOM结构
/// * `target_lang` - 目标语言代码（如 "zh", "en", "ja"）
/// * `api_url` - 可选的翻译API URL
///
/// # Examples
///
/// ```rust,no_run
/// use markup5ever_rcdom::RcDom;
/// use monolith::translation::translate_dom_content;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let dom: RcDom = /* 从HTML解析得到的DOM */;
/// let translated_dom = translate_dom_content(
///     dom,
///     "zh",
///     Some("http://localhost:1188/translate")
/// ).await?;
/// # Ok(())
/// # }
/// ```
pub async fn translate_dom_content(
    dom: markup5ever_rcdom::RcDom,
    target_lang: &str,
    api_url: Option<&str>,
) -> TranslationResult<markup5ever_rcdom::RcDom> {
    let mut service = TranslationService::create_default(target_lang, api_url)?;
    service.translate_dom(dom).await
}

/// 翻译HTML DOM内容（同步版本）
///
/// 内部创建异步运行时来执行翻译，适用于非异步环境
///
/// # Examples
///
/// ```rust,no_run
/// use markup5ever_rcdom::RcDom;
/// use monolith::translation::translate_dom_content_sync;
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let dom: RcDom = /* 从HTML解析得到的DOM */;
/// let translated_dom = translate_dom_content_sync(
///     dom,
///     "zh",
///     Some("http://localhost:1188/translate")
/// )?;
/// # Ok(())
/// # }
/// ```
pub fn translate_dom_content_sync(
    dom: markup5ever_rcdom::RcDom,
    target_lang: &str,
    api_url: Option<&str>,
) -> Result<markup5ever_rcdom::RcDom, crate::core::MonolithError> {
    #[cfg(feature = "translation")]
    {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| crate::core::MonolithError::new(&format!("创建异步运行时失败: {e}")))?;

        rt.block_on(async {
            translate_dom_content(dom, target_lang, api_url)
                .await
                .map_err(|e| crate::core::MonolithError::new(&e.to_string()))
        })
    }

    #[cfg(not(feature = "translation"))]
    {
        let _ = (dom, target_lang, api_url);
        Err(crate::core::MonolithError::new(
            "Translation feature not enabled",
        ))
    }
}

/// 翻译CSS内容（实验性功能）
/// 
/// **注意**: 此功能目前处于实验阶段，暂时返回原始CSS内容不做翻译。
/// 在未来的版本中，将支持CSS中的文本内容翻译，如：
/// - `content` 属性中的文本
/// - CSS变量中的文本值
/// - 注释中的文本（可选）
/// 
/// # 参数
/// 
/// * `css` - 要翻译的CSS内容
/// * `target_lang` - 目标语言代码（如 "zh", "en", "ja"）
/// * `api_url` - 可选的翻译API URL
/// 
/// # 返回值
/// 
/// 返回翻译后的CSS内容。当前版本返回原始内容。
/// 
/// # Examples
/// 
/// ```rust,no_run
/// use monolith::translation::translate_css_content;
/// 
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let css = ".button::after { content: 'Click me'; }";
/// let translated_css = translate_css_content(
///     css,
///     "zh",
///     Some("http://localhost:1188/translate")
/// ).await?;
/// // 未来版本将返回: ".button::after { content: '点击我'; }"
/// # Ok(())
/// # }
/// ```
#[cfg(feature = "translation")]
pub async fn translate_css_content(
    css: &str,
    target_lang: &str,
    api_url: Option<&str>,
) -> TranslationResult<String> {
    // TODO: 实现CSS翻译功能
    // 1. 解析CSS，识别可翻译的文本内容
    // 2. 提取文本并进行翻译
    // 3. 将翻译结果替换回CSS中
    // 
    // 暂时返回原内容，避免破坏CSS结构
    let _ = (target_lang, api_url);
    Ok(css.to_string())
}

/// 检查文本是否应该翻译（便利函数）
/// 
/// 使用内置的文本过滤器快速判断给定文本是否需要翻译。
/// 此函数会考虑文本长度、内容类型、语言特征等因素。
/// 
/// # 参数
/// 
/// * `text` - 要检查的文本内容
/// 
/// # 返回值
/// 
/// 如果文本应该翻译返回 `true`，否则返回 `false`
/// 
/// # Examples
/// 
/// ```rust
/// use monolith::translation::should_translate;
/// 
/// assert_eq!(should_translate("Hello World"), true);
/// assert_eq!(should_translate("123"), false);  // 纯数字
/// assert_eq!(should_translate(""), false);     // 空文本
/// assert_eq!(should_translate("   "), false);  // 纯空白
/// ```
pub fn should_translate(text: &str) -> bool {
    let filter = pipeline::TextFilter::new();
    filter.should_translate(text)
}

/// 分析文本的可翻译性（便利函数）
/// 
/// 对文本进行深度分析，返回详细的分析结果，包括文本类型、
/// 优先级、语言提示等信息。
/// 
/// # 参数
/// 
/// * `text` - 要分析的文本内容
/// 
/// # 返回值
/// 
/// 返回 `TextAnalysis` 结构体，包含分析结果的详细信息
/// 
/// # Examples
/// 
/// ```rust
/// use monolith::translation::analyze_text;
/// 
/// let analysis = analyze_text("Hello World!");
/// println!("文本类型: {:?}", analysis.text_type);
/// println!("优先级: {:?}", analysis.priority);
/// println!("是否应该翻译: {}", analysis.should_translate);
/// ```
pub fn analyze_text(text: &str) -> pipeline::TextAnalysis {
    let filter = pipeline::TextFilter::new();
    filter.analyze_text(text)
}

/// 检查翻译配置文件是否存在
/// 
/// 在当前工作目录中查找标准的翻译配置文件。
/// 通常查找 `translation-config.toml` 或 `.env` 文件。
/// 
/// # 返回值
/// 
/// 如果找到配置文件返回 `true`，否则返回 `false`
/// 
/// # Examples
/// 
/// ```rust
/// use monolith::translation::config_file_exists;
/// 
/// if config_file_exists() {
///     println!("找到翻译配置文件");
/// } else {
///     println!("未找到配置文件，将使用默认设置");
/// }
/// ```
pub fn config_file_exists() -> bool {
    config::config_file_exists()
}

/// 获取最小翻译字符数配置
/// 
/// 返回系统配置的最小翻译字符数阈值。低于此长度的文本
/// 通常不会被翻译，以避免翻译过短或无意义的内容。
/// 
/// # 返回值
/// 
/// 最小翻译字符数，默认值通常为3-5个字符
/// 
/// # Examples
/// 
/// ```rust
/// use monolith::translation::get_min_translation_chars;
/// 
/// let min_chars = get_min_translation_chars();
/// println!("最小翻译字符数: {}", min_chars);
/// ```
pub fn get_min_translation_chars() -> usize {
    config::get_min_translation_chars()
}

/// 加载翻译配置（向后兼容函数）
/// 
/// 从配置文件和环境变量中加载翻译配置，如果没有找到配置文件，
/// 则使用提供的参数创建默认配置。
/// 
/// **注意**: 此函数主要用于向后兼容，新代码建议使用 `ConfigManager`。
/// 
/// # 参数
/// 
/// * `target_lang` - 目标语言代码（如 "zh", "en", "ja"）
/// * `api_url` - 可选的翻译API URL
/// 
/// # 返回值
/// 
/// 返回配置好的 `TranslationConfig` 实例
/// 
/// # Examples
/// 
/// ```rust
/// use monolith::translation::load_translation_config;
/// 
/// let config = load_translation_config("zh", Some("http://localhost:1188/translate"));
/// println!("目标语言: {}", config.target_language);
/// ```
pub fn load_translation_config(target_lang: &str, api_url: Option<&str>) -> TranslationConfig {
    config::load_translation_config(target_lang, api_url)
}

/// 生成示例配置文件
/// 
/// 在当前目录中创建一个示例的翻译配置文件 `translation-config.toml`，
/// 包含所有可用的配置选项和说明注释。
/// 
/// # 返回值
/// 
/// 成功时返回 `Ok(())`，失败时返回包含错误信息的 `Err`
/// 
/// # Examples
/// 
/// ```rust,no_run
/// use monolith::translation::generate_example_config;
/// 
/// match generate_example_config() {
///     Ok(_) => println!("配置文件生成成功"),
///     Err(e) => eprintln!("生成配置文件失败: {}", e),
/// }
/// ```
pub fn generate_example_config() -> Result<(), crate::core::MonolithError> {
    match ConfigManager::generate_example_config("translation-config.toml") {
        Ok(_) => {
            println!("已生成示例配置文件: translation-config.toml");
            Ok(())
        }
        Err(e) => Err(crate::core::MonolithError::new(&format!(
            "生成配置文件失败: {}",
            e
        ))),
    }
}

// ============================================================================
// 向后兼容导出 - 已弃用的API，将在未来版本中移除
// ============================================================================

/// 增强翻译服务（向后兼容别名）
/// 
/// **已弃用**: 请使用 `core::TranslationService` 替代。
/// 此别名仅为保持向后兼容性而保留，新代码不应使用。
/// 
/// # 迁移指南
/// 
/// ```rust,ignore
/// // 旧代码
/// use monolith::translation::EnhancedTranslationService;
/// 
/// // 新代码
/// use monolith::translation::TranslationService;
/// ```
#[deprecated(since = "2.1.0", note = "请使用 TranslationService 替代")]
pub use core::service::TranslationService as EnhancedTranslationService;

/// 收集可翻译文本函数（向后兼容导出）
/// 
/// **已弃用**: 请使用 `pipeline::TextCollector` 替代。
/// 新的实现提供更好的性能和更丰富的功能。
/// 
/// # 迁移指南
/// 
/// ```rust,ignore
/// // 旧代码
/// use monolith::translation::collect_translatable_texts;
/// let texts = collect_translatable_texts(&dom);
/// 
/// // 新代码
/// use monolith::translation::TextCollector;
/// let collector = TextCollector::new();
/// let texts = collector.collect_texts(&dom);
/// ```
#[deprecated(since = "2.1.0", note = "请使用 pipeline::TextCollector 替代")]
pub use pipeline::collector::collect_translatable_texts;

/// 创建优化批次函数（向后兼容导出）
/// 
/// **已弃用**: 请使用 `pipeline::BatchManager` 替代。
/// 新的批次管理器提供更智能的批次策略和更好的错误处理。
/// 
/// # 迁移指南
/// 
/// ```rust,ignore
/// // 旧代码
/// use monolith::translation::create_optimized_batches;
/// let batches = create_optimized_batches(items);
/// 
/// // 新代码
/// use monolith::translation::BatchManager;
/// let mut manager = BatchManager::default();
/// let batches = manager.create_batches(items);
/// ```
#[deprecated(since = "2.1.0", note = "请使用 pipeline::BatchManager 替代")]
pub use pipeline::batch::create_optimized_batches;

// ============================================================================
// 模块信息和元数据
// ============================================================================

/// 模块版本信息
pub const VERSION: &str = "2.1.0";
pub const MODULE_NAME: &str = "translation";

/// 模块信息
#[derive(Debug, Clone)]
pub struct ModuleInfo {
    pub name: String,
    pub version: String,
    pub enabled: bool,
    pub features: Vec<String>,
}

/// 获取模块信息
pub fn get_module_info() -> ModuleInfo {
    ModuleInfo {
        name: MODULE_NAME.to_string(),
        version: VERSION.to_string(),
        enabled: cfg!(feature = "translation"),
        features: get_enabled_features(),
    }
}

/// 获取启用的功能
fn get_enabled_features() -> Vec<String> {
    let mut features = Vec::new();

    #[cfg(feature = "translation")]
    {
        features.push("core_translation".to_string());
        features.push("pipeline_processing".to_string());
        features.push("storage_cache".to_string());
        features.push("config_management".to_string());
        features.push("error_handling".to_string());
    }

    #[cfg(feature = "web")]
    features.push("mongodb_storage".to_string());

    features
}

/// 运行翻译模块自检
pub async fn self_check() -> TranslationResult<()> {
    #[cfg(feature = "translation")]
    {
        tracing::info!("开始翻译模块自检...");

        // 检查配置管理器
        let _config_manager = ConfigManager::new()?;
        tracing::debug!("✓ 配置管理器正常");

        // 检查文本过滤器
        let filter = pipeline::TextFilter::new();
        let test_result = filter.should_translate("Hello World");
        if test_result {
            tracing::debug!("✓ 文本过滤器正常");
        } else {
            return Err(TranslationError::InternalError(
                "文本过滤器异常".to_string(),
            ));
        }

        // 检查批次管理器
        let mut batch_manager = pipeline::BatchManager::default();
        let test_items = vec![]; // 空列表测试
        let _batches = batch_manager.create_batches(test_items);
        tracing::debug!("✓ 批次管理器正常");

        // 检查缓存管理器
        let cache_config = storage::CacheConfig::default();
        let _cache_manager = storage::CacheManager::new(cache_config);
        tracing::debug!("✓ 缓存管理器正常");

        tracing::info!("翻译模块自检完成，所有组件正常");
        Ok(())
    }

    #[cfg(not(feature = "translation"))]
    {
        Err(TranslationError::new("Translation feature not enabled"))
    }
}

/// 模块初始化
pub fn init() {
    #[cfg(feature = "translation")]
    {
        tracing::info!("翻译模块 v{} 已加载", VERSION);
        tracing::info!("启用的功能: {:?}", get_enabled_features());
    }

    #[cfg(not(feature = "translation"))]
    {
        tracing::info!("翻译模块 v{} 已加载 (功能已禁用)", VERSION);
    }
}
