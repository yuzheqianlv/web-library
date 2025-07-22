//! 翻译服务核心模块
//!
//! 整合所有翻译功能，提供统一的翻译服务接口

use std::sync::Arc;
use std::time::Instant;

use markup5ever_rcdom::RcDom;

#[cfg(feature = "translation")]
use markdown_translator::TranslationService as BaseTranslationService;

use crate::translation::{
    batch::{BatchManager, BatchManagerConfig},
    cache::{CacheConfig, CacheManager},
    collector::{CollectorConfig, TextCollector},
    config::{ConfigManager, EnhancedTranslationConfig},
    error::{TranslationError, TranslationResult},
    filters::TextFilter,
    processor::{ProcessorConfig, TranslationProcessor},
};

/// 增强的翻译服务
pub struct EnhancedTranslationService {
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

impl EnhancedTranslationService {
    /// 创建新的翻译服务
    #[cfg(feature = "translation")]
    pub fn new(config: EnhancedTranslationConfig) -> TranslationResult<Self> {
        let base_service = Arc::new(BaseTranslationService::new(config.base.clone()));

        let config_manager = ConfigManager::new()?;

        let collector_config = CollectorConfig::default();
        let text_collector = TextCollector::new(collector_config);

        let batch_config = BatchManagerConfig::from(&config);
        let batch_manager = BatchManager::new(batch_config);

        let cache_config = CacheConfig {
            enable_local_cache: config.cache.enabled,
            local_cache_size: config.cache.local_cache_size,
            default_ttl: config.cache.ttl,
            enable_warmup: config.cache.enable_warmup,
            ..Default::default()
        };
        let cache_manager = CacheManager::new(cache_config);

        let processor_config = ProcessorConfig::default();
        let processor = TranslationProcessor::new(base_service.clone(), processor_config);

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
            let config = ConfigManager::new()?.create_legacy_config(target_lang, api_url)?;
            let enhanced_config = crate::translation::config::EnhancedTranslationConfig {
                base: config,
                ..crate::translation::config::ConfigManager::default_config()
            };
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

        // 1. 收集可翻译文本
        let texts = self
            .text_collector
            .collect_translatable_texts(&dom.document)?;

        if texts.is_empty() {
            tracing::info!("没有找到需要翻译的文本");
            return Ok(dom);
        }

        tracing::info!("收集到 {} 个待翻译文本", texts.len());
        self.stats.collected_texts = texts.len();

        // 2. 创建翻译批次
        let batches = self.batch_manager.create_batches(texts);
        tracing::info!("创建了 {} 个翻译批次", batches.len());
        self.stats.created_batches = batches.len();

        // 3. 处理翻译批次
        #[cfg(feature = "translation")]
        {
            self.processor.process_batches(batches).await?;

            // 获取处理器统计
            let processor_stats = self.processor.get_stats();
            self.stats.successful_batches = processor_stats.successful_batches;
            self.stats.failed_batches = processor_stats.failed_batches;
            self.stats.translated_items = processor_stats.translated_items;
        }

        #[cfg(not(feature = "translation"))]
        {
            let _ = batches;
            return Err(TranslationError::new("Translation feature not enabled"));
        }

        self.stats.processing_time = start_time.elapsed();
        self.stats.success_rate = if self.stats.created_batches > 0 {
            self.stats.successful_batches as f32 / self.stats.created_batches as f32
        } else {
            0.0
        };

        tracing::info!(
            "DOM翻译完成: 耗时 {:?}, 成功率 {:.1}%",
            self.stats.processing_time,
            self.stats.success_rate * 100.0
        );

        Ok(dom)
    }

    /// 翻译CSS内容
    #[cfg(feature = "translation")]
    pub async fn translate_css(&mut self, css: &str) -> TranslationResult<String> {
        use regex::Regex;

        let content_re = Regex::new(r#"content\s*:\s*["']([^"']+)["']"#)
            .map_err(|e| TranslationError::ParseError(format!("CSS正则表达式错误: {}", e)))?;

        let mut result = css.to_string();
        let filter = TextFilter::new();

        for cap in content_re.captures_iter(css) {
            if let Some(text_match) = cap.get(1) {
                let text = text_match.as_str();

                if filter.should_translate(text) {
                    // 尝试从缓存获取
                    if let Ok(Some(cached)) = self
                        .cache_manager
                        .get_translation(text, "auto", &self.get_target_language().await?)
                        .await
                    {
                        let full_match = cap.get(0).unwrap().as_str();
                        let translated_rule = full_match.replace(text, &cached);
                        result = result.replace(full_match, &translated_rule);
                        continue;
                    }

                    // 翻译文本
                    match self.base_service.translate(text).await {
                        Ok(translated) => {
                            // 缓存结果
                            let _ = self
                                .cache_manager
                                .set_translation(
                                    text,
                                    "auto",
                                    &self.get_target_language().await?,
                                    &translated,
                                )
                                .await;

                            let full_match = cap.get(0).unwrap().as_str();
                            let translated_rule = full_match.replace(text, &translated);
                            result = result.replace(full_match, &translated_rule);
                        }
                        Err(e) => {
                            tracing::warn!("CSS文本翻译失败: {}", e);
                        }
                    }
                }
            }
        }

        Ok(result)
    }

    /// 翻译单个文本
    pub async fn translate_text(&mut self, text: &str) -> TranslationResult<String> {
        let filter = TextFilter::new();

        if !filter.should_translate(text) {
            return Ok(text.to_string());
        }

        #[cfg(feature = "translation")]
        {
            let target_lang = self.get_target_language().await?;

            // 尝试从缓存获取
            if let Ok(Some(cached)) = self
                .cache_manager
                .get_translation(text, "auto", &target_lang)
                .await
            {
                return Ok(cached);
            }

            // 执行翻译
            match self.base_service.translate(text).await {
                Ok(translated) => {
                    // 缓存结果
                    let _ = self
                        .cache_manager
                        .set_translation(text, "auto", &target_lang, &translated)
                        .await;
                    Ok(translated)
                }
                Err(e) => Err(TranslationError::TranslationServiceError(e.to_string())),
            }
        }

        #[cfg(not(feature = "translation"))]
        {
            Err(TranslationError::new("Translation feature not enabled"))
        }
    }

    /// 获取目标语言
    async fn get_target_language(&self) -> TranslationResult<String> {
        let config = self.config_manager.get_config()?;
        Ok(config.base.target_lang)
    }

    // Redis缓存功能已被移除

    /// 重新加载配置
    pub async fn reload_config(&mut self) -> TranslationResult<bool> {
        self.config_manager.reload_if_changed()
    }

    /// 清理过期缓存
    pub async fn cleanup_cache(&mut self) -> TranslationResult<usize> {
        self.cache_manager.cleanup_expired().await
    }

    /// 获取服务统计信息
    pub fn get_stats(&self) -> &ServiceStats {
        &self.stats
    }

    /// 获取详细统计信息
    pub async fn get_detailed_stats(&self) -> DetailedStats {
        let collector_stats = self.text_collector.get_stats().clone();
        let batch_stats = self.batch_manager.get_stats().clone();
        let cache_stats = self.cache_manager.get_stats().clone();
        let cache_info = self.cache_manager.get_cache_info().await;

        #[cfg(feature = "translation")]
        let processor_stats = Some(self.processor.get_stats().clone());

        #[cfg(not(feature = "translation"))]
        let processor_stats = None;

        DetailedStats {
            service: self.stats.clone(),
            collector: collector_stats,
            batch: batch_stats,
            cache: cache_stats,
            cache_info,
            processor: processor_stats,
        }
    }

    /// 健康检查
    pub async fn health_check(&self) -> HealthStatus {
        let mut status = HealthStatus {
            overall: HealthLevel::Healthy,
            components: std::collections::HashMap::new(),
        };

        // 检查配置
        match self.config_manager.get_config() {
            Ok(_) => {
                status
                    .components
                    .insert("config".to_string(), HealthLevel::Healthy);
            }
            Err(_) => {
                status
                    .components
                    .insert("config".to_string(), HealthLevel::Unhealthy);
                status.overall = HealthLevel::Degraded;
            }
        }

        // 检查缓存
        let cache_info = self.cache_manager.get_cache_info().await;
        if cache_info.hit_rate < 0.1 && cache_info.total_entries > 100 {
            status
                .components
                .insert("cache".to_string(), HealthLevel::Degraded);
            if status.overall == HealthLevel::Healthy {
                status.overall = HealthLevel::Degraded;
            }
        } else {
            status
                .components
                .insert("cache".to_string(), HealthLevel::Healthy);
        }

        // 检查翻译服务
        #[cfg(feature = "translation")]
        {
            // 这里可以添加对基础翻译服务的健康检查
            status
                .components
                .insert("translation".to_string(), HealthLevel::Healthy);
        }

        #[cfg(not(feature = "translation"))]
        {
            status
                .components
                .insert("translation".to_string(), HealthLevel::Unhealthy);
            status.overall = HealthLevel::Unhealthy;
        }

        status
    }
}

/// 服务统计信息
#[derive(Debug, Clone, Default)]
pub struct ServiceStats {
    pub collected_texts: usize,
    pub created_batches: usize,
    pub successful_batches: usize,
    pub failed_batches: usize,
    pub translated_items: usize,
    pub success_rate: f32,
    pub processing_time: std::time::Duration,
}

impl ServiceStats {
    pub fn reset(&mut self) {
        *self = Default::default();
    }
}

/// 详细统计信息
pub struct DetailedStats {
    pub service: ServiceStats,
    pub collector: crate::translation::collector::CollectionStats,
    pub batch: crate::translation::batch::BatchStats,
    pub cache: crate::translation::cache::CacheStats,
    pub cache_info: crate::translation::cache::CacheInfo,
    #[cfg(feature = "translation")]
    pub processor: Option<crate::translation::processor::ProcessorStats>,
    #[cfg(not(feature = "translation"))]
    pub processor: Option<()>,
}

/// 健康状态
#[derive(Debug, Clone)]
pub struct HealthStatus {
    pub overall: HealthLevel,
    pub components: std::collections::HashMap<String, HealthLevel>,
}

/// 健康级别
#[derive(Debug, Clone, PartialEq)]
pub enum HealthLevel {
    Healthy,
    Degraded,
    Unhealthy,
}

/// 向后兼容的翻译函数
pub async fn translate_dom_content(
    dom: RcDom,
    target_lang: &str,
    api_url: Option<&str>,
) -> TranslationResult<RcDom> {
    let mut service = EnhancedTranslationService::create_default(target_lang, api_url)?;
    service.translate_dom(dom).await
}

/// 同步翻译接口（向后兼容）
pub fn translate_dom_content_sync(
    dom: RcDom,
    target_lang: &str,
    api_url: Option<&str>,
) -> Result<RcDom, crate::core::MonolithError> {
    #[cfg(feature = "translation")]
    {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| crate::core::MonolithError::new(&format!("创建异步运行时失败: {}", e)))?;

        rt.block_on(translate_dom_content(dom, target_lang, api_url))
            .map_err(|e| e.into())
    }

    #[cfg(not(feature = "translation"))]
    {
        let _ = (target_lang, api_url);
        Ok(dom)
    }
}

/// CSS翻译函数（向后兼容）
#[cfg(feature = "translation")]
pub async fn translate_css_content(
    css: &str,
    _translator: &BaseTranslationService,
) -> Result<String, crate::core::MonolithError> {
    // 创建临时服务来处理CSS翻译
    let config = crate::translation::config::ConfigManager::default_config();
    let mut service = EnhancedTranslationService::new(config)
        .map_err(|e| crate::core::MonolithError::new(&e.to_string()))?;

    service
        .translate_css(css)
        .await
        .map_err(|e| crate::core::MonolithError::new(&e.to_string()))
}

#[cfg(not(feature = "translation"))]
pub async fn translate_css_content(
    css: &str,
    _translator: &(),
) -> Result<String, crate::core::MonolithError> {
    Ok(css.to_string())
}
