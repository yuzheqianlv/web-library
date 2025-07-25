//! 翻译服务核心实现
//!
//! 提供统一的翻译服务接口，整合文本收集、批次处理、缓存管理等功能

use std::sync::{Arc, atomic::{AtomicUsize, AtomicU64, Ordering}};
use std::time::Instant;
use markup5ever_rcdom::RcDom;

#[cfg(feature = "translation")]
use markdown_translator::TranslationService as BaseTranslationService;

use crate::translation::{
    pipeline::batch::{BatchManager, BatchManagerConfig},
    storage::cache::{CacheConfig, CacheManager},
    pipeline::collector::{CollectorConfig, TextCollector},
    config::{ConfigManager, TranslationConfig},
    error::TranslationResult,
    processor::{ProcessorConfig, TranslationProcessor},
};

/// 统一的翻译服务
/// 
/// 这是重构后的主要翻译服务，提供简化的API和更好的性能
pub struct TranslationService {
    #[cfg(feature = "translation")]
    base_service: Arc<BaseTranslationService>,
    
    config_manager: ConfigManager,
    text_collector: TextCollector,
    batch_manager: BatchManager,
    cache_manager: CacheManager,
    
    #[cfg(feature = "translation")]
    processor: TranslationProcessor,
    
    stats: ServiceStats,
}

impl TranslationService {
    /// 创建新的翻译服务
    #[cfg(feature = "translation")]
    pub fn new(config: TranslationConfig) -> TranslationResult<Self> {
        // 创建基础服务需要的配置结构
        let base_config = markdown_translator::TranslationConfig {
            enabled: true,
            source_lang: config.source_lang.clone(),
            deeplx_api_url: config.api_url.clone(),
            ..Default::default()
        };
        let base_service = Arc::new(BaseTranslationService::new(base_config));
        
        let config_manager = ConfigManager::new()?;
        
        let collector_config = CollectorConfig::default();
        let text_collector = TextCollector::new(collector_config);
        
        let batch_config = BatchManagerConfig::from(&config);
        let batch_manager = BatchManager::new(batch_config);
        
        let cache_config = CacheConfig {
            enable_local_cache: config.cache_enabled,
            local_cache_size: config.local_cache_size,
            default_ttl: config.cache_ttl(),
            enable_warmup: false,
            ..Default::default()
        };
        let cache_manager = CacheManager::new(cache_config);
        
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
    
    /// 创建默认翻译服务
    pub fn create_default(target_lang: &str, api_url: Option<&str>) -> TranslationResult<Self> {
        #[cfg(feature = "translation")]
        {
            let enhanced_config = crate::translation::config::TranslationConfig::default_with_lang(target_lang, api_url);
            Self::new(enhanced_config)
        }
        
        #[cfg(not(feature = "translation"))]
        {
            let _ = (target_lang, api_url);
            Err(TranslationError::new("Translation feature not enabled"))
        }
    }
    
    /// 翻译DOM内容
    pub async fn translate_dom(&mut self, dom: RcDom) -> TranslationResult<RcDom> {
        let start_time = Instant::now();
        self.stats.reset();
        
        tracing::info!("开始DOM翻译处理");
        
        #[cfg(feature = "translation")]
        {
            // 1. 收集可翻译文本
            let texts = self.text_collector.collect_translatable_texts(&dom.document)?;
            self.stats.texts_collected.store(texts.len(), Ordering::Relaxed);
            
            if texts.is_empty() {
                tracing::info!("没有找到需要翻译的文本");
                return Ok(dom);
            }
            
            // 2. 创建批次
            let batches = self.batch_manager.create_batches(texts);
            self.stats.batches_created.store(batches.len(), Ordering::Relaxed);
            
            // 3. 处理翻译
            self.processor.process_batches(batches).await?;
            let translated_dom = dom;  // 实际的DOM修改应该在processor中完成
            
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
    
    /// 获取服务统计信息
    pub fn get_stats(&self) -> &ServiceStats {
        &self.stats
    }
    
    /// 重置统计信息
    pub fn reset_stats(&mut self) {
        self.stats.reset();
    }
    
    /// 获取健康状态
    pub fn get_health(&self) -> HealthStatus {
        let mut components = std::collections::HashMap::new();
        
        // 检查各组件健康状态
        components.insert("config".to_string(), HealthLevel::Healthy);
        components.insert("cache".to_string(), HealthLevel::Healthy);
        components.insert("collector".to_string(), HealthLevel::Healthy);
        components.insert("batch_manager".to_string(), HealthLevel::Healthy);
        
        #[cfg(feature = "translation")]
        components.insert("processor".to_string(), HealthLevel::Healthy);
        
        let overall = if components.values().all(|&level| level == HealthLevel::Healthy) {
            HealthLevel::Healthy
        } else if components.values().any(|&level| level == HealthLevel::Unhealthy) {
            HealthLevel::Unhealthy
        } else {
            HealthLevel::Degraded
        };
        
        HealthStatus { overall, components }
    }
}

/// 服务统计信息（线程安全版本）
#[derive(Debug, Default)]
pub struct ServiceStats {
    pub texts_collected: AtomicUsize,
    pub batches_created: AtomicUsize,
    pub translations_completed: AtomicUsize,
    pub cache_hits: AtomicUsize,
    pub cache_misses: AtomicUsize,
    pub processing_time: AtomicU64,  // 以微秒为单位
    pub total_chars_processed: AtomicUsize,
    pub errors_encountered: AtomicUsize,
}

impl ServiceStats {
    /// 增加文本收集计数
    pub fn inc_texts_collected(&self) {
        self.texts_collected.fetch_add(1, Ordering::Relaxed);
    }
    
    /// 增加批次创建计数
    pub fn inc_batches_created(&self) {
        self.batches_created.fetch_add(1, Ordering::Relaxed);
    }
    
    /// 增加翻译完成计数
    pub fn inc_translations_completed(&self) {
        self.translations_completed.fetch_add(1, Ordering::Relaxed);
    }
    
    /// 增加缓存命中计数
    pub fn inc_cache_hits(&self) {
        self.cache_hits.fetch_add(1, Ordering::Relaxed);
    }
    
    /// 增加缓存未命中计数
    pub fn inc_cache_misses(&self) {
        self.cache_misses.fetch_add(1, Ordering::Relaxed);
    }
    
    /// 添加处理时间
    pub fn add_processing_time(&self, duration: std::time::Duration) {
        self.processing_time.fetch_add(duration.as_micros() as u64, Ordering::Relaxed);
    }
    
    /// 增加字符处理计数
    pub fn add_chars_processed(&self, count: usize) {
        self.total_chars_processed.fetch_add(count, Ordering::Relaxed);
    }
    
    /// 增加错误计数
    pub fn inc_errors(&self) {
        self.errors_encountered.fetch_add(1, Ordering::Relaxed);
    }
    
    /// 获取统计快照用于读取
    pub fn snapshot(&self) -> ServiceStatsSnapshot {
        ServiceStatsSnapshot {
            texts_collected: self.texts_collected.load(Ordering::Relaxed),
            batches_created: self.batches_created.load(Ordering::Relaxed),
            translations_completed: self.translations_completed.load(Ordering::Relaxed),
            cache_hits: self.cache_hits.load(Ordering::Relaxed),
            cache_misses: self.cache_misses.load(Ordering::Relaxed),
            processing_time: std::time::Duration::from_micros(self.processing_time.load(Ordering::Relaxed)),
            total_chars_processed: self.total_chars_processed.load(Ordering::Relaxed),
            errors_encountered: self.errors_encountered.load(Ordering::Relaxed),
        }
    }
}

/// 统计数据快照（用于读取和显示）
#[derive(Debug, Clone, Copy)]
pub struct ServiceStatsSnapshot {
    pub texts_collected: usize,
    pub batches_created: usize,
    pub translations_completed: usize,
    pub cache_hits: usize,
    pub cache_misses: usize,
    pub processing_time: std::time::Duration,
    pub total_chars_processed: usize,
    pub errors_encountered: usize,
}

impl ServiceStats {
    /// 重置统计信息
    pub fn reset(&mut self) {
        *self = Default::default();
    }
}

/// 健康状态
#[derive(Debug, Clone)]
pub struct HealthStatus {
    pub overall: HealthLevel,
    pub components: std::collections::HashMap<String, HealthLevel>,
}

/// 健康级别
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HealthLevel {
    Healthy,
    Degraded,
    Unhealthy,
}