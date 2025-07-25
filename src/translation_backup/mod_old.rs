//! 翻译模块
//!
//! 提供完整的翻译功能，包括文本收集、批次处理、缓存管理等
//!
//! 这个版本经过了全面重构，提供了更好的性能和可维护性。

pub mod batch;
pub mod cache;
pub mod collector;
pub mod config;
pub mod error;
pub mod filters;
pub mod processor;
pub mod service;

// 重新导出主要类型和函数
pub use config::{
    config_file_exists, constants, get_min_translation_chars, load_translation_config,
    ConfigManager,
};

pub use error::{ErrorCategory, ErrorSeverity, TranslationError, TranslationResult};

pub use filters::{analyze_text, should_translate, LanguageHint, TextAnalysis, TextFilter};

pub use collector::{collect_translatable_texts, TextCollector, TextItem, TextPriority, TextType};

pub use batch::{create_optimized_batches, Batch, BatchManager, BatchPriority, BatchType};

pub use cache::{CacheConfig, CacheEntry, CacheManager, CacheStats};

pub use service::{
    translate_dom_content, translate_dom_content_sync, EnhancedTranslationService, HealthLevel,
    HealthStatus, ServiceStats,
};

#[cfg(feature = "translation")]
pub use service::translate_css_content;

#[cfg(feature = "translation")]
pub use processor::{ProcessorConfig, ProcessorStats, TranslationProcessor};

// 向后兼容的导出
pub use service::translate_dom_content as translate_dom_content_new;

/// 生成示例配置文件
pub fn generate_example_config() -> Result<(), crate::core::MonolithError> {
    #[cfg(feature = "translation")]
    {
        use std::path::Path;

        let config_path = "translation-config.toml";

        if Path::new(config_path).exists() {
            println!("配置文件 {} 已存在，跳过生成", config_path);
            return Ok(());
        }

        match markdown_translator::TranslationLibConfig::generate_example_config(config_path) {
            Ok(_) => {
                println!("已生成示例配置文件: {}", config_path);
                println!("请编辑该文件以配置翻译参数");
                Ok(())
            }
            Err(e) => Err(crate::core::MonolithError::new(&format!(
                "生成配置文件失败: {}",
                e
            ))),
        }
    }

    #[cfg(not(feature = "translation"))]
    {
        Err(crate::core::MonolithError::new(
            "Translation feature not enabled",
        ))
    }
}

/// 模块版本信息
pub const VERSION: &str = "2.0.0";
pub const MODULE_NAME: &str = "translation";

/// 模块初始化
pub fn init() {
    #[cfg(feature = "translation")]
    {
        tracing::info!("翻译模块 v{} 已加载", VERSION);

        // 可以在这里添加模块初始化逻辑
        // 比如检查配置文件、初始化全局缓存等
    }

    #[cfg(not(feature = "translation"))]
    {
        tracing::info!("翻译模块 v{} 已加载 (功能已禁用)", VERSION);
    }
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

/// 模块信息
#[derive(Debug, Clone)]
pub struct ModuleInfo {
    pub name: String,
    pub version: String,
    pub enabled: bool,
    pub features: Vec<String>,
}

/// 获取启用的功能
fn get_enabled_features() -> Vec<String> {
    let mut features = Vec::new();

    #[cfg(feature = "translation")]
    features.push("core".to_string());

    #[cfg(feature = "translation")]
    {
        features.push("batch_processing".to_string());
        features.push("smart_filtering".to_string());
        features.push("local_cache".to_string());
        features.push("parallel_processing".to_string());
        features.push("config_management".to_string());
        features.push("error_recovery".to_string());
    }

    features
}

/// 运行翻译模块自检
pub async fn self_check() -> Result<(), TranslationError> {
    #[cfg(feature = "translation")]
    {
        tracing::info!("开始翻译模块自检...");

        // 检查配置管理器
        let config_manager = ConfigManager::new()?;
        let _config = config_manager.get_config();
        tracing::debug!("✓ 配置管理器正常");

        // 检查文本过滤器
        let filter = TextFilter::new();
        let test_result = filter.should_translate("Hello World");
        if test_result {
            tracing::debug!("✓ 文本过滤器正常");
        } else {
            return Err(TranslationError::InternalError(
                "文本过滤器异常".to_string(),
            ));
        }

        // 检查批次管理器
        let mut batch_manager = BatchManager::default();
        let test_items = vec![]; // 空列表测试
        let _batches = batch_manager.create_batches(test_items);
        tracing::debug!("✓ 批次管理器正常");

        // 检查缓存管理器
        let cache_config = CacheConfig::default();
        let _cache_manager = CacheManager::new(cache_config);
        tracing::debug!("✓ 缓存管理器正常");

        tracing::info!("翻译模块自检完成，所有组件正常");
        Ok(())
    }

    #[cfg(not(feature = "translation"))]
    {
        Err(TranslationError::new("Translation feature not enabled"))
    }
}

/// 性能基准测试
#[cfg(feature = "translation")]
pub async fn benchmark() -> BenchmarkResult {
    use std::time::Instant;

    let start_time = Instant::now();

    // 文本过滤性能测试
    let filter = TextFilter::new();
    let test_texts = vec![
        "Hello World",
        "This is a test",
        "https://example.com",
        "test@example.com",
        "already translated: 已翻译",
        "very very very long text that should be translated because it contains meaningful content",
    ];

    let filter_start = Instant::now();
    let filtered_count = test_texts
        .iter()
        .filter(|text| filter.should_translate(text))
        .count();
    let filter_time = filter_start.elapsed();

    // 批次管理性能测试
    let mut batch_manager = BatchManager::default();
    let batch_start = Instant::now();

    // 创建测试文本项
    let test_items: Vec<collector::TextItem> = (0..1000)
        .map(|i| {
            use markup5ever_rcdom::{Node, NodeData};
            use std::cell::RefCell;

            let node = Node::new(NodeData::Text {
                contents: RefCell::new(format!("Test text {}", i).into()),
            });
            collector::TextItem::content(format!("Test text {}", i), node, 0)
        })
        .collect();

    let batches = batch_manager.create_batches(test_items);
    let batch_time = batch_start.elapsed();

    let total_time = start_time.elapsed();

    BenchmarkResult {
        total_time,
        filter_time,
        filter_throughput: test_texts.len() as f64 / filter_time.as_secs_f64(),
        filtered_count,
        batch_time,
        batch_count: batches.len(),
        batch_throughput: 1000.0 / batch_time.as_secs_f64(),
    }
}

/// 基准测试结果
#[cfg(feature = "translation")]
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    pub total_time: std::time::Duration,
    pub filter_time: std::time::Duration,
    pub filter_throughput: f64, // texts per second
    pub filtered_count: usize,
    pub batch_time: std::time::Duration,
    pub batch_count: usize,
    pub batch_throughput: f64, // items per second
}

#[cfg(feature = "translation")]
impl std::fmt::Display for BenchmarkResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Translation Module Benchmark Results:\n\
             Total Time: {:?}\n\
             Filter Performance: {:.0} texts/sec ({:?})\n\
             Filtered: {} texts\n\
             Batch Performance: {:.0} items/sec ({:?})\n\
             Created: {} batches",
            self.total_time,
            self.filter_throughput,
            self.filter_time,
            self.filtered_count,
            self.batch_throughput,
            self.batch_time,
            self.batch_count
        )
    }
}
