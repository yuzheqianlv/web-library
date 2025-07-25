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

// 子模块声明
pub mod core;
pub mod pipeline;
pub mod storage;
pub mod config;
pub mod error;

// 向后兼容的模块别名（deprecated）
pub mod batch {
    pub use crate::translation::pipeline::batch::*;
}

pub mod cache {
    pub use crate::translation::storage::cache::*;
}

pub mod collector {
    pub use crate::translation::pipeline::collector::*;
}

pub mod filters {
    pub use crate::translation::pipeline::filters::*;
}

pub mod service {
    pub use crate::translation::core::service::*;
}

// processor 模块仍然存在于根目录
pub mod processor;

// === 核心API导出 ===

// 主要翻译服务
pub use core::{TranslationService, ServiceStats, HealthStatus, HealthLevel};

// 配置管理
pub use config::{TranslationConfig, ConfigManager, constants};

// 错误处理
pub use error::{TranslationError, TranslationResult, ErrorCategory, ErrorSeverity};

// === 高级API导出（可选使用） ===

// 文本处理管道
pub use pipeline::{
    TextCollector, TextFilter, BatchManager,
    TextItem, TextType, TextPriority, TextAnalysis, LanguageHint,
    Batch, BatchType, BatchPriority,
};

// 存储和缓存
pub use storage::{CacheManager, CacheConfig, CacheEntry, CacheStats};

// === 便利函数导出 ===

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
        Err(crate::core::MonolithError::new("Translation feature not enabled"))
    }
}

/// 翻译CSS内容（如果支持）
#[cfg(feature = "translation")]
pub async fn translate_css_content(
    css: &str,
    target_lang: &str,
    api_url: Option<&str>
) -> TranslationResult<String> {
    // 暂时返回原内容，CSS翻译功能可以在后续版本中实现
    let _ = (target_lang, api_url);
    Ok(css.to_string())
}

/// 检查文本是否应该翻译（便利函数）
pub fn should_translate(text: &str) -> bool {
    let filter = pipeline::TextFilter::new();
    filter.should_translate(text)
}

/// 分析文本的可翻译性（便利函数）
pub fn analyze_text(text: &str) -> pipeline::TextAnalysis {
    let filter = pipeline::TextFilter::new();
    filter.analyze_text(text)
}

/// 检查配置文件是否存在
pub fn config_file_exists() -> bool {
    config::config_file_exists()
}

/// 获取最小翻译字符数配置
pub fn get_min_translation_chars() -> usize {
    config::get_min_translation_chars()
}

/// 加载翻译配置（向后兼容函数）
pub fn load_translation_config(target_lang: &str, api_url: Option<&str>) -> TranslationConfig {
    config::load_translation_config(target_lang, api_url)
}

/// 生成示例配置文件
pub fn generate_example_config() -> Result<(), crate::core::MonolithError> {
    match ConfigManager::generate_example_config("translation-config.toml") {
        Ok(_) => {
            println!("已生成示例配置文件: translation-config.toml");
            Ok(())
        }
        Err(e) => Err(crate::core::MonolithError::new(&format!("生成配置文件失败: {}", e))),
    }
}

// === 向后兼容导出（标记为deprecated，将在未来版本中移除） ===

// 向后兼容的导出（已弃用）
#[deprecated(since = "2.1.0", note = "请使用 core::TranslationService 替代")]
pub use core::service::TranslationService as EnhancedTranslationService;

#[deprecated(since = "2.1.0", note = "请使用 pipeline::TextCollector 替代")]
pub use pipeline::collector::collect_translatable_texts;

#[deprecated(since = "2.1.0", note = "请使用 pipeline::BatchManager 替代")]
pub use pipeline::batch::create_optimized_batches;

// === 模块信息 ===

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
            return Err(TranslationError::InternalError("文本过滤器异常".to_string()));
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