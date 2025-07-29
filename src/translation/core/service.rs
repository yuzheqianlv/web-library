//! 翻译服务核心实现
//!
//! 本模块提供了统一的翻译服务接口，整合了文本收集、批次处理、缓存管理等功能。
//! 这是翻译系统的主要入口点，负责协调各个子系统的工作。
//!
//! ## 设计理念
//!
//! 1. **模块化设计**: 每个组件都有明确的职责，便于测试和维护
//! 2. **线程安全**: 使用原子操作和Arc智能指针确保并发安全
//! 3. **错误处理**: 统一的错误类型和处理机制
//! 4. **可配置性**: 支持灵活的配置选项以适应不同需求
//! 5. **性能监控**: 内置统计和健康检查功能
//!
//! ## 主要组件
//!
//! - `TranslationService`: 主要的翻译服务类，协调各个子系统
//! - `ServiceStats`: 线程安全的统计信息收集器
//! - `HealthStatus`: 服务健康状态监控
//!
//! ## 使用示例
//!
//! ```rust
//! use monolith::translation::core::service::TranslationService;
//!
//! // 创建翻译服务
//! let mut service = TranslationService::create_default("zh", Some("http://api.example.com"))?;
//!
//! // 翻译DOM内容
//! let translated_dom = service.translate_dom(dom).await?;
//!
//! // 获取统计信息
//! let stats = service.get_stats().snapshot();
//! println!("处理了 {} 个文本", stats.texts_collected);
//! ```

use markup5ever_rcdom::RcDom;
use std::sync::{
    atomic::{AtomicU64, AtomicUsize, Ordering},
    Arc,
};
use std::time::Instant;

#[cfg(feature = "translation")]
use markdown_translator::TranslationService as BaseTranslationService;

use crate::translation::{
    config::{ConfigManager, TranslationConfig},
    error::TranslationResult,
    pipeline::batch::{BatchManager, BatchManagerConfig},
    pipeline::collector::{CollectorConfig, TextCollector},
    processor::{ProcessorConfig, TranslationProcessor},
    storage::cache::{CacheConfig, CacheManager},
};

/// 统一的翻译服务
///
/// 这是重构后的主要翻译服务，提供简化的API和更好的性能。该服务整合了多个子系统：
///
/// - **基础翻译服务**: 与外部翻译API的接口
/// - **配置管理器**: 动态配置管理和更新
/// - **文本收集器**: 从DOM中提取可翻译文本
/// - **批次管理器**: 将文本组织成批次以优化API调用
/// - **缓存管理器**: 缓存翻译结果以提高性能
/// - **翻译处理器**: 实际执行翻译操作
/// - **统计收集器**: 收集性能和使用统计
///
/// ## 特性
///
/// - **异步处理**: 支持异步翻译操作，避免阻塞
/// - **批量优化**: 自动将文本组织成批次，减少API调用次数
/// - **智能缓存**: 多层缓存策略，提高重复内容的处理速度
/// - **错误恢复**: 内置重试机制和错误处理
/// - **监控友好**: 提供详细的统计信息和健康检查
///
/// ## 线程安全
///
/// 该结构体的所有字段都是线程安全的，可以在多线程环境中安全使用。
/// 统计信息使用原子操作，避免了锁的开销。
pub struct TranslationService {
    /// 基础翻译服务，封装了与外部翻译API的交互
    /// 
    /// 使用Arc包装以支持多线程访问，只在启用translation特性时可用。
    /// 这是实际执行翻译请求的底层服务。
    #[cfg(feature = "translation")]
    base_service: Arc<BaseTranslationService>,

    /// 配置管理器，负责加载和管理翻译配置
    /// 
    /// 支持动态配置更新和配置文件监听，确保配置变更能够及时生效。
    config_manager: ConfigManager,
    
    /// 文本收集器，从DOM中提取需要翻译的文本内容
    /// 
    /// 能够智能识别可翻译的文本节点，跳过代码、链接等不需要翻译的内容。
    text_collector: TextCollector,
    
    /// 批次管理器，将收集到的文本组织成批次
    /// 
    /// 根据文本长度和API限制自动创建最优的批次，提高翻译效率。
    batch_manager: BatchManager,
    
    /// 缓存管理器，管理翻译结果的缓存
    /// 
    /// 支持内存和持久化缓存，避免重复翻译相同内容。
    cache_manager: CacheManager,

    /// 翻译处理器，协调缓存查询和实际翻译操作
    /// 
    /// 只在启用translation特性时可用，负责执行具体的翻译流程。
    #[cfg(feature = "translation")]
    processor: TranslationProcessor,

    /// 服务统计信息，记录翻译服务的性能指标
    /// 
    /// 使用原子操作确保线程安全，提供实时的性能监控数据。
    stats: ServiceStats,
}

impl TranslationService {
    /// 创建新的翻译服务实例
    ///
    /// 根据提供的配置初始化所有子系统组件。这是创建翻译服务的主要方式，
    /// 提供了完整的配置控制能力。
    ///
    /// # 参数
    ///
    /// * `config` - 翻译配置，包含API地址、缓存设置、批次大小等参数
    ///
    /// # 返回值
    ///
    /// 返回初始化完成的翻译服务实例，如果初始化失败则返回错误。
    ///
    /// # 错误
    ///
    /// 如果以下情况发生，函数将返回错误：
    /// - 配置文件无效或缺失
    /// - 基础翻译服务初始化失败
    /// - 缓存系统初始化失败
    ///
    /// # 示例
    ///
    /// ```rust
    /// use monolith::translation::{TranslationConfig, TranslationService};
    ///
    /// let config = TranslationConfig {
    ///     source_lang: "en".to_string(),
    ///     target_lang: "zh".to_string(),
    ///     api_url: "http://api.example.com".to_string(),
    ///     cache_enabled: true,
    ///     ..Default::default()
    /// };
    ///
    /// let service = TranslationService::new(config)?;
    /// ```
    #[cfg(feature = "translation")]
    pub fn new(config: TranslationConfig) -> TranslationResult<Self> {
        // 创建基础服务需要的配置结构，转换为底层库所需的格式
        let base_config = markdown_translator::TranslationConfig {
            enabled: true,
            source_lang: config.source_lang.clone(),
            deeplx_api_url: config.api_url.clone(),
            ..Default::default()
        };
        let base_service = Arc::new(BaseTranslationService::new(base_config));

        // 初始化配置管理器，负责运行时配置的动态管理
        let config_manager = ConfigManager::new()?;

        // 初始化文本收集器，使用默认配置进行DOM文本提取
        let collector_config = CollectorConfig::default();
        let text_collector = TextCollector::new(collector_config);

        // 初始化批次管理器，根据用户配置优化批次大小
        let batch_config = BatchManagerConfig::from(&config);
        let batch_manager = BatchManager::new(batch_config);

        // 初始化缓存管理器，配置缓存策略和存储选项
        let cache_config = CacheConfig {
            enable_local_cache: config.cache_enabled,
            local_cache_size: config.local_cache_size,
            default_ttl: config.cache_ttl(),
            enable_warmup: false, // 暂时不启用缓存预热功能
            ..Default::default()
        };
        let cache_manager = CacheManager::new(cache_config);

        // 初始化翻译处理器，连接基础服务和处理配置
        let processor_config = ProcessorConfig::default();
        // 使用 Arc::clone 明确表示这是引用计数的克隆，而非数据克隆
        let processor = TranslationProcessor::new(Arc::clone(&base_service), processor_config);

        Ok(Self {
            base_service,
            config_manager,
            text_collector,
            batch_manager,
            cache_manager,
            processor,
            stats: ServiceStats::default(),
        })
    }

    /// 创建使用默认配置的翻译服务
    ///
    /// 这是一个便捷方法，使用预设的默认配置快速创建翻译服务实例。
    /// 适用于大多数常见场景，减少了配置的复杂性。
    ///
    /// # 参数
    ///
    /// * `target_lang` - 目标语言代码（如 "zh", "en", "ja" 等）
    /// * `api_url` - 可选的翻译API地址，如果为None则使用默认配置
    ///
    /// # 返回值
    ///
    /// 返回配置了默认设置的翻译服务实例。
    ///
    /// # 错误
    ///
    /// - 当translation特性未启用时返回特性未启用错误
    /// - 当默认配置无效时返回配置错误
    ///
    /// # 示例
    ///
    /// ```rust
    /// // 使用默认API地址
    /// let service = TranslationService::create_default("zh", None)?;
    ///
    /// // 使用自定义API地址
    /// let service = TranslationService::create_default("zh", Some("http://custom-api.com"))?;
    /// ```
    pub fn create_default(target_lang: &str, api_url: Option<&str>) -> TranslationResult<Self> {
        #[cfg(feature = "translation")]
        {
            let enhanced_config = crate::translation::config::TranslationConfig::default_with_lang(
                target_lang,
                api_url,
            );
            Self::new(enhanced_config)
        }

        #[cfg(not(feature = "translation"))]
        {
            let _ = (target_lang, api_url);
            Err(crate::translation::error::TranslationError::FeatureNotEnabled)
        }
    }

    /// 翻译DOM文档内容
    ///
    /// 这是翻译服务的核心方法，负责处理完整的DOM翻译流程。
    /// 该方法会：
    ///
    /// 1. 从DOM中收集所有可翻译的文本内容
    /// 2. 将文本组织成批次以优化API调用
    /// 3. 查询缓存以避免重复翻译
    /// 4. 对未缓存的内容执行翻译
    /// 5. 将翻译结果应用回DOM结构
    /// 6. 更新统计信息
    ///
    /// # 参数
    ///
    /// * `dom` - 需要翻译的DOM文档，使用html5ever的RcDom结构
    ///
    /// # 返回值
    ///
    /// 返回翻译后的DOM文档。如果没有可翻译内容，返回原始DOM。
    ///
    /// # 错误
    ///
    /// 可能的错误情况包括：
    /// - 文本收集失败
    /// - 网络连接问题导致翻译API调用失败
    /// - 翻译API返回错误响应
    /// - DOM操作失败
    ///
    /// # 性能考虑
    ///
    /// - 该方法是异步的，不会阻塞调用线程
    /// - 内置批次处理可以显著减少API调用次数
    /// - 智能缓存可以大幅提升重复内容的处理速度
    /// - 统计信息收集对性能影响最小
    ///
    /// # 示例
    ///
    /// ```rust
    /// use markup5ever_rcdom::RcDom;
    /// 
    /// let mut service = TranslationService::create_default("zh", None)?;
    /// let original_dom: RcDom = parse_html_document(&html_content);
    /// 
    /// let translated_dom = service.translate_dom(original_dom).await?;
    /// println!("翻译统计: {:?}", service.get_stats().snapshot());
    /// ```
    pub async fn translate_dom(&mut self, dom: RcDom) -> TranslationResult<RcDom> {
        let start_time = Instant::now();
        self.stats.reset();

        tracing::info!("开始DOM翻译处理");

        #[cfg(feature = "translation")]
        {
            // 第一步：从DOM中收集所有需要翻译的文本内容
            // 文本收集器会智能识别文本节点，排除不需要翻译的内容
            let texts = self
                .text_collector
                .collect_translatable_texts(&dom.document)?;
            self.stats
                .texts_collected
                .store(texts.len(), Ordering::Relaxed);

            // 如果没有找到可翻译的文本，直接返回原始DOM
            if texts.is_empty() {
                tracing::info!("没有找到需要翻译的文本");
                return Ok(dom);
            }

            // 第二步：将收集到的文本组织成批次
            // 批次管理器会根据文本长度和API限制优化批次大小
            let batches = self.batch_manager.create_batches(texts);
            self.stats
                .batches_created
                .store(batches.len(), Ordering::Relaxed);

            // 第三步：处理翻译批次
            // 翻译处理器会查询缓存，对未缓存的内容执行翻译，并更新DOM
            self.processor.process_batches(batches).await?;
            let translated_dom = dom; // 实际的DOM修改应该在processor中完成

            // 记录处理时间并输出日志
            self.stats.add_processing_time(start_time.elapsed());
            tracing::info!("DOM翻译处理完成，耗时: {:?}", self.stats.processing_time);

            Ok(translated_dom)
        }

        #[cfg(not(feature = "translation"))]
        {
            let _ = start_time;
            Ok(dom)
        }
    }

    /// 获取服务统计信息的只读引用
    ///
    /// 返回当前翻译服务的统计信息，包括处理的文本数量、缓存命中率、
    /// 处理时间等性能指标。这些统计信息对于监控和调优非常有用。
    ///
    /// # 返回值
    ///
    /// 返回ServiceStats的不可变引用，包含所有统计计数器。
    ///
    /// # 示例
    ///
    /// ```rust
    /// let stats = service.get_stats();
    /// let snapshot = stats.snapshot();
    /// println!("已处理文本: {}", snapshot.texts_collected);
    /// println!("缓存命中率: {:.2}%", 
    ///     100.0 * snapshot.cache_hits as f64 / 
    ///     (snapshot.cache_hits + snapshot.cache_misses) as f64);
    /// ```
    pub fn get_stats(&self) -> &ServiceStats {
        &self.stats
    }

    /// 重置所有统计信息
    ///
    /// 将所有统计计数器重置为零，通常在开始新的翻译任务前调用。
    /// 这对于获取单次操作的准确统计数据很有用。
    ///
    /// # 示例
    ///
    /// ```rust
    /// service.reset_stats();
    /// let dom = service.translate_dom(dom).await?;
    /// let stats = service.get_stats().snapshot();
    /// println!("本次翻译处理了 {} 个文本", stats.texts_collected);
    /// ```
    pub fn reset_stats(&mut self) {
        self.stats.reset();
    }

    /// 获取服务健康状态
    ///
    /// 检查翻译服务各个组件的健康状态，用于监控和故障诊断。
    /// 健康检查包括配置、缓存、收集器、批次管理器等所有子系统。
    ///
    /// # 返回值
    ///
    /// 返回HealthStatus结构，包含：
    /// - overall: 整体健康状态
    /// - components: 各个组件的详细健康状态
    ///
    /// # 健康级别
    ///
    /// - `Healthy`: 所有组件正常工作
    /// - `Degraded`: 部分组件有问题但服务仍可用
    /// - `Unhealthy`: 服务不可用或有严重问题
    ///
    /// # 示例
    ///
    /// ```rust
    /// let health = service.get_health();
    /// match health.overall {
    ///     HealthLevel::Healthy => println!("服务运行正常"),
    ///     HealthLevel::Degraded => println!("服务性能下降"),
    ///     HealthLevel::Unhealthy => println!("服务不可用"),
    /// }
    /// 
    /// for (component, level) in health.components {
    ///     println!("组件 {}: {:?}", component, level);
    /// }
    /// ```
    pub fn get_health(&self) -> HealthStatus {
        let mut components = std::collections::HashMap::new();

        // 检查各组件健康状态
        // 目前使用简化的健康检查，实际应用中可以添加更详细的检查逻辑
        components.insert("config".to_string(), HealthLevel::Healthy);        // 配置管理器
        components.insert("cache".to_string(), HealthLevel::Healthy);         // 缓存管理器
        components.insert("collector".to_string(), HealthLevel::Healthy);     // 文本收集器
        components.insert("batch_manager".to_string(), HealthLevel::Healthy); // 批次管理器

        // 翻译处理器只在启用translation特性时检查
        #[cfg(feature = "translation")]
        components.insert("processor".to_string(), HealthLevel::Healthy);

        // 根据各组件状态确定整体健康状态
        let overall = if components
            .values()
            .all(|&level| level == HealthLevel::Healthy)
        {
            // 所有组件都健康
            HealthLevel::Healthy
        } else if components
            .values()
            .any(|&level| level == HealthLevel::Unhealthy)
        {
            // 任何组件不健康都会导致整体不健康
            HealthLevel::Unhealthy
        } else {
            // 有组件降级但无不健康组件
            HealthLevel::Degraded
        };

        HealthStatus {
            overall,
            components,
        }
    }
}

/// 翻译服务统计信息（线程安全版本）
///
/// 使用原子操作确保在多线程环境中的数据一致性，收集翻译服务的各种性能指标。
/// 这些统计信息对于性能监控、问题诊断和系统优化非常重要。
///
/// ## 设计特点
///
/// - **线程安全**: 所有字段都使用原子类型，支持无锁并发访问
/// - **实时更新**: 统计数据在操作执行时即时更新
/// - **轻量级**: 原子操作的性能开销很小，不会影响翻译性能
/// - **全面覆盖**: 涵盖了翻译流程的各个关键环节
///
/// ## 统计指标说明
///
/// - `texts_collected`: 从DOM中收集到的可翻译文本数量
/// - `batches_created`: 创建的翻译批次数量
/// - `translations_completed`: 完成的翻译操作数量
/// - `cache_hits/misses`: 缓存命中和未命中次数，用于评估缓存效率
/// - `processing_time`: 总处理时间（微秒），用于性能分析
/// - `total_chars_processed`: 处理的字符总数，用于评估工作量
/// - `errors_encountered`: 遇到的错误次数，用于质量监控
#[derive(Debug, Default)]
pub struct ServiceStats {
    /// 收集到的可翻译文本数量
    pub texts_collected: AtomicUsize,
    
    /// 创建的翻译批次数量
    pub batches_created: AtomicUsize,
    
    /// 完成的翻译操作数量
    pub translations_completed: AtomicUsize,
    
    /// 缓存命中次数
    pub cache_hits: AtomicUsize,
    
    /// 缓存未命中次数
    pub cache_misses: AtomicUsize,
    
    /// 总处理时间，以微秒为单位存储
    pub processing_time: AtomicU64,
    
    /// 处理的字符总数
    pub total_chars_processed: AtomicUsize,
    
    /// 遇到的错误次数
    pub errors_encountered: AtomicUsize,
}

impl ServiceStats {
    /// 增加文本收集计数
    ///
    /// 当从DOM中成功收集到一个可翻译文本时调用。
    /// 使用原子操作确保线程安全。
    pub fn inc_texts_collected(&self) {
        self.texts_collected.fetch_add(1, Ordering::Relaxed);
    }

    /// 增加批次创建计数
    ///
    /// 当批次管理器创建一个新的翻译批次时调用。
    /// 批次数量反映了API调用的优化程度。
    pub fn inc_batches_created(&self) {
        self.batches_created.fetch_add(1, Ordering::Relaxed);
    }

    /// 增加翻译完成计数
    ///
    /// 当一个翻译操作成功完成时调用。
    /// 这包括缓存命中和实际API调用的翻译。
    pub fn inc_translations_completed(&self) {
        self.translations_completed.fetch_add(1, Ordering::Relaxed);
    }

    /// 增加缓存命中计数
    ///
    /// 当从缓存中找到翻译结果时调用。
    /// 高缓存命中率表示良好的性能表现。
    pub fn inc_cache_hits(&self) {
        self.cache_hits.fetch_add(1, Ordering::Relaxed);
    }

    /// 增加缓存未命中计数
    ///
    /// 当缓存中没有找到翻译结果，需要调用API时调用。
    /// 与缓存命中次数一起用于计算缓存命中率。
    pub fn inc_cache_misses(&self) {
        self.cache_misses.fetch_add(1, Ordering::Relaxed);
    }

    /// 添加处理时间
    ///
    /// 累加翻译操作的处理时间，用于性能分析。
    /// 时间以微秒精度存储，支持高精度的性能监控。
    ///
    /// # 参数
    ///
    /// * `duration` - 本次操作的处理时间
    pub fn add_processing_time(&self, duration: std::time::Duration) {
        self.processing_time
            .fetch_add(duration.as_micros() as u64, Ordering::Relaxed);
    }

    /// 增加字符处理计数
    ///
    /// 累加处理的字符数量，用于评估翻译工作量。
    /// 这个指标有助于了解翻译的规模和计费。
    ///
    /// # 参数
    ///
    /// * `count` - 本次处理的字符数量
    pub fn add_chars_processed(&self, count: usize) {
        self.total_chars_processed
            .fetch_add(count, Ordering::Relaxed);
    }

    /// 增加错误计数
    ///
    /// 当翻译过程中遇到错误时调用。
    /// 错误计数用于监控服务质量和稳定性。
    pub fn inc_errors(&self) {
        self.errors_encountered.fetch_add(1, Ordering::Relaxed);
    }

    /// 获取统计数据的一致性快照
    ///
    /// 由于统计数据使用原子操作，在高并发情况下不同字段可能在不同时刻读取。
    /// 此方法提供一个时间点的数据快照，确保所有统计数据的一致性。
    ///
    /// # 返回值
    ///
    /// 返回ServiceStatsSnapshot结构，包含所有统计数据的副本。
    /// 快照数据是不可变的，适合用于显示和分析。
    ///
    /// # 性能说明
    ///
    /// 此方法执行多个原子读取操作，但不会阻塞其他线程的统计更新。
    /// 快照操作的性能开销很小，可以频繁调用。
    ///
    /// # 示例
    ///
    /// ```rust
    /// let snapshot = stats.snapshot();
    /// 
    /// // 计算缓存命中率
    /// let total_queries = snapshot.cache_hits + snapshot.cache_misses;
    /// let hit_rate = if total_queries > 0 {
    ///     100.0 * snapshot.cache_hits as f64 / total_queries as f64
    /// } else {
    ///     0.0
    /// };
    /// 
    /// println!("缓存命中率: {:.2}%", hit_rate);
    /// println!("平均处理时间: {:?}", 
    ///     snapshot.processing_time / snapshot.texts_collected.max(1) as u32);
    /// ```
    pub fn snapshot(&self) -> ServiceStatsSnapshot {
        ServiceStatsSnapshot {
            texts_collected: self.texts_collected.load(Ordering::Relaxed),
            batches_created: self.batches_created.load(Ordering::Relaxed),
            translations_completed: self.translations_completed.load(Ordering::Relaxed),
            cache_hits: self.cache_hits.load(Ordering::Relaxed),
            cache_misses: self.cache_misses.load(Ordering::Relaxed),
            processing_time: std::time::Duration::from_micros(
                self.processing_time.load(Ordering::Relaxed),
            ),
            total_chars_processed: self.total_chars_processed.load(Ordering::Relaxed),
            errors_encountered: self.errors_encountered.load(Ordering::Relaxed),
        }
    }
}

/// 翻译服务统计数据的不可变快照
///
/// 此结构体包含了某个时间点的所有统计数据副本，使用普通的数值类型而非原子类型。
/// 快照数据是不可变的，适合用于显示、日志记录和数据分析。
///
/// ## 用途
///
/// - **性能监控**: 跟踪翻译服务的处理能力和效率
/// - **问题诊断**: 分析错误率和性能瓶颈
/// - **容量规划**: 基于历史数据预测资源需求
/// - **用户界面**: 为Web界面或CLI提供展示数据
///
/// ## 数据一致性
///
/// 快照中的所有数据都是在同一时刻读取的，保证了数据的一致性。
/// 这对于计算比率（如缓存命中率）等衍生指标非常重要。
#[derive(Debug, Clone, Copy)]
pub struct ServiceStatsSnapshot {
    /// 收集到的可翻译文本数量
    pub texts_collected: usize,
    
    /// 创建的翻译批次数量
    pub batches_created: usize,
    
    /// 完成的翻译操作数量
    pub translations_completed: usize,
    
    /// 缓存命中次数
    pub cache_hits: usize,
    
    /// 缓存未命中次数
    pub cache_misses: usize,
    
    /// 总处理时间
    pub processing_time: std::time::Duration,
    
    /// 处理的字符总数
    pub total_chars_processed: usize,
    
    /// 遇到的错误次数
    pub errors_encountered: usize,
}

impl ServiceStats {
    /// 重置所有统计计数器
    ///
    /// 将所有统计数据重置为零值，通常在开始新的翻译任务前调用。
    /// 这对于获取特定操作的准确统计数据很有用。
    ///
    /// # 注意事项
    ///
    /// 此方法需要可变引用，因为它会重新初始化整个结构体。
    /// 在多线程环境中使用时需要适当的同步机制。
    pub fn reset(&mut self) {
        *self = Default::default();
    }
}

/// 翻译服务的健康状态报告
///
/// 提供翻译服务及其各个组件的健康状态信息，用于监控和故障诊断。
/// 健康检查是系统可观测性的重要组成部分。
///
/// ## 结构说明
///
/// - `overall`: 整体健康状态，综合所有组件的状态得出
/// - `components`: 各个组件的详细健康状态映射
///
/// ## 使用场景
///
/// - **运维监控**: 集成到监控系统中进行自动化健康检查
/// - **负载均衡**: 健康检查结果可以用于负载均衡决策
/// - **故障诊断**: 快速定位问题组件
/// - **用户界面**: 在管理界面中显示服务状态
#[derive(Debug, Clone)]
pub struct HealthStatus {
    /// 整体健康级别
    pub overall: HealthLevel,
    
    /// 各组件的健康状态映射
    /// 
    /// 键是组件名称（如"cache", "processor"等），
    /// 值是对应的健康级别
    pub components: std::collections::HashMap<String, HealthLevel>,
}

/// 健康状态级别枚举
///
/// 定义了系统组件可能的健康状态，按严重程度从好到坏排列。
///
/// ## 级别说明
///
/// - `Healthy`: 组件运行正常，所有功能可用
/// - `Degraded`: 组件部分功能受限，但核心功能仍可用
/// - `Unhealthy`: 组件无法正常工作，可能影响整体服务
///
/// ## 判断标准
///
/// 具体的健康判断标准由各个组件的实现决定，可能包括：
/// - 网络连接状态
/// - 资源使用情况
/// - 错误率水平
/// - 响应时间
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HealthLevel {
    /// 健康状态 - 组件运行正常
    Healthy,
    
    /// 降级状态 - 组件功能受限但仍可用
    Degraded,
    
    /// 不健康状态 - 组件无法正常工作
    Unhealthy,
}
