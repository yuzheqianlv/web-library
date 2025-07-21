//! 翻译处理器模块
//!
//! 提供高级翻译处理逻辑，包括索引标记系统和错误恢复

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

#[cfg(feature = "translation")]
use markdown_translator::TranslationService;
#[cfg(feature = "translation")]
use regex::Regex;
#[cfg(feature = "translation")]
use tokio::time::{sleep, Duration};

use crate::html::{set_node_attr};
use crate::translation::batch::{Batch, BatchType};
use crate::translation::collector::TextItem;
use crate::translation::error::{TranslationError, TranslationResult};
use markup5ever_rcdom::NodeData;

/// 翻译处理器
#[cfg(feature = "translation")]
pub struct TranslationProcessor {
    service: Arc<TranslationService>,
    stats: ProcessorStats,
    config: ProcessorConfig,
}

#[cfg(feature = "translation")]
impl TranslationProcessor {
    /// 创建新的翻译处理器
    pub fn new(service: Arc<TranslationService>, config: ProcessorConfig) -> Self {
        Self {
            service,
            stats: ProcessorStats::default(),
            config,
        }
    }

    /// 处理批次列表
    pub async fn process_batches(&mut self, batches: Vec<Batch>) -> TranslationResult<()> {
        self.stats.reset();
        self.stats.total_batches = batches.len();

        tracing::info!("开始处理 {} 个翻译批次", batches.len());

        for (i, batch) in batches.into_iter().enumerate() {
            tracing::debug!("处理批次 {}/{}: {}", i + 1, self.stats.total_batches, batch.summary());

            let result = self.process_single_batch(batch).await;
            
            match result {
                Ok(_) => self.stats.successful_batches += 1,
                Err(e) => {
                    self.stats.failed_batches += 1;
                    tracing::error!("批次处理失败: {}", e);
                }
            }

            // 批次间延迟
            if i < self.stats.total_batches - 1 {
                sleep(self.config.batch_delay).await;
            }
        }

        self.stats.success_rate = if self.stats.total_batches > 0 {
            self.stats.successful_batches as f32 / self.stats.total_batches as f32
        } else {
            0.0
        };

        tracing::info!(
            "批次处理完成: 成功 {}/{}, 成功率 {:.1}%",
            self.stats.successful_batches,
            self.stats.total_batches,
            self.stats.success_rate * 100.0
        );

        Ok(())
    }

    /// 处理单个批次
    async fn process_single_batch(&mut self, batch: Batch) -> TranslationResult<()> {
        let start_time = Instant::now();

        match batch.batch_type {
            BatchType::Single => {
                self.process_single_item(&batch.items[0]).await?;
            }
            BatchType::Small => {
                if batch.items.len() <= self.config.small_batch_threshold {
                    self.process_individual_items(&batch.items).await?;
                } else {
                    self.process_indexed_batch(&batch).await?;
                }
            }
            _ => {
                self.process_indexed_batch(&batch).await?;
            }
        }

        self.stats.processing_time += start_time.elapsed();
        Ok(())
    }

    /// 处理索引批次
    async fn process_indexed_batch(&mut self, batch: &Batch) -> TranslationResult<()> {
        tracing::debug!("索引批次翻译: {} 个文本", batch.items.len());

        // 尝试索引标记翻译
        match self.try_indexed_translation(batch).await {
            Ok(true) => {
                tracing::debug!("索引翻译成功");
                self.stats.indexed_successes += 1;
                Ok(())
            }
            Ok(false) => {
                tracing::debug!("索引翻译失败，回退到逐个翻译");
                self.stats.indexed_failures += 1;
                self.process_individual_items(&batch.items).await
            }
            Err(e) => {
                tracing::warn!("索引翻译出错: {}, 回退到逐个翻译", e);
                self.stats.indexed_failures += 1;
                self.process_individual_items(&batch.items).await
            }
        }
    }

    /// 尝试索引标记翻译
    async fn try_indexed_translation(&mut self, batch: &Batch) -> TranslationResult<bool> {
        let combined_text = self.combine_texts_with_indices(&batch.items);
        
        tracing::debug!("索引组合文本长度: {} 字符", combined_text.len());

        match self.service.translate(&combined_text).await {
            Ok(translated) => {
                let success = self.apply_indexed_translation(&batch.items, &translated)?;
                Ok(success)
            }
            Err(e) => {
                Err(TranslationError::TranslationServiceError(format!(
                    "索引翻译请求失败: {}", e
                )))
            }
        }
    }

    /// 组合文本并添加索引
    fn combine_texts_with_indices(&self, items: &[TextItem]) -> String {
        items
            .iter()
            .enumerate()
            .map(|(i, item)| format!("[{}] {}", i, item.text.trim()))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// 应用索引翻译结果
    fn apply_indexed_translation(&mut self, items: &[TextItem], translated: &str) -> TranslationResult<bool> {
        let index_regex = Regex::new(r"^\[(\d+)\]\s*(.*)$")
            .map_err(|e| TranslationError::ParseError(format!("正则表达式编译失败: {}", e)))?;

        let mut translations = HashMap::new();

        // 解析翻译结果
        for line in translated.lines() {
            if let Some(captures) = index_regex.captures(line.trim()) {
                if let (Some(index_str), Some(text)) = (captures.get(1), captures.get(2)) {
                    if let Ok(index) = index_str.as_str().parse::<usize>() {
                        let translated_text = text.as_str().trim();
                        if !translated_text.is_empty() {
                            translations.insert(index, translated_text);
                        }
                    }
                }
            }
        }

        tracing::debug!(
            "索引翻译解析: 输入 {} 项，解析到 {} 项",
            items.len(),
            translations.len()
        );

        // 应用翻译
        let mut success_count = 0;
        for (i, item) in items.iter().enumerate() {
            if let Some(translated_text) = translations.get(&i) {
                self.apply_translation(item, translated_text)?;
                success_count += 1;
                self.stats.translated_items += 1;
            } else {
                tracing::debug!("索引 {} 没有找到对应翻译", i);
                self.stats.skipped_items += 1;
            }
        }

        let success_rate = success_count as f32 / items.len() as f32;
        let is_success = success_rate >= self.config.min_success_rate;

        tracing::debug!(
            "索引翻译成功率: {:.1}% ({}/{}), 判定: {}",
            success_rate * 100.0,
            success_count,
            items.len(),
            if is_success { "成功" } else { "失败" }
        );

        Ok(is_success)
    }

    /// 逐个处理项目
    async fn process_individual_items(&mut self, items: &[TextItem]) -> TranslationResult<()> {
        tracing::debug!("逐个翻译: {} 个文本", items.len());

        for (i, item) in items.iter().enumerate() {
            match self.process_single_item(item).await {
                Ok(_) => {
                    self.stats.translated_items += 1;
                    tracing::debug!("项目 {}/{} 翻译成功", i + 1, items.len());
                }
                Err(e) => {
                    self.stats.skipped_items += 1;
                    tracing::warn!("项目 {}/{} 翻译失败: {}", i + 1, items.len(), e);
                }
            }

            // 个别项目间的小延迟
            if i < items.len() - 1 {
                sleep(Duration::from_millis(50)).await;
            }
        }

        Ok(())
    }

    /// 处理单个项目
    async fn process_single_item(&mut self, item: &TextItem) -> TranslationResult<()> {
        let translated = self.service
            .translate(&item.text)
            .await
            .map_err(|e| TranslationError::TranslationServiceError(e.to_string()))?;

        self.apply_translation(item, &translated.trim())?;
        Ok(())
    }

    /// 应用翻译到DOM
    fn apply_translation(&self, item: &TextItem, translated: &str) -> TranslationResult<()> {
        if translated.is_empty() {
            return Err(TranslationError::InvalidInput("翻译结果为空".to_string()));
        }

        // 检查翻译质量
        if translated == item.text.trim() && item.text.len() > 5 {
            tracing::debug!("翻译结果与原文相同，可能是专有名词: '{}'", translated);
        }

        // 应用到DOM
        if let Some(attr_name) = &item.attr_name {
            // 属性翻译
            set_node_attr(&item.node, attr_name, Some(translated.to_string()));
        } else {
            // 文本内容翻译
            if let NodeData::Text { ref contents } = item.node.data {
                let mut content_ref = contents.borrow_mut();
                content_ref.clear();
                content_ref.push_slice(translated);
            } else {
                return Err(TranslationError::InternalError(
                    "节点不是文本类型".to_string()
                ));
            }
        }

        Ok(())
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> &ProcessorStats {
        &self.stats
    }
}

/// 处理器配置
#[derive(Debug, Clone)]
pub struct ProcessorConfig {
    /// 小批次阈值
    pub small_batch_threshold: usize,
    /// 最小成功率
    pub min_success_rate: f32,
    /// 批次延迟
    pub batch_delay: Duration,
    /// 启用重试
    pub enable_retry: bool,
    /// 最大重试次数
    pub max_retries: usize,
    /// 重试延迟
    pub retry_delay: Duration,
}

impl Default for ProcessorConfig {
    fn default() -> Self {
        Self {
            small_batch_threshold: 2,
            min_success_rate: 0.8,
            batch_delay: Duration::from_millis(100),
            enable_retry: true,
            max_retries: 3,
            retry_delay: Duration::from_millis(1000),
        }
    }
}

/// 处理器统计信息
#[derive(Debug, Clone, Default)]
pub struct ProcessorStats {
    pub total_batches: usize,
    pub successful_batches: usize,
    pub failed_batches: usize,
    pub success_rate: f32,
    pub translated_items: usize,
    pub skipped_items: usize,
    pub indexed_successes: usize,
    pub indexed_failures: usize,
    pub processing_time: Duration,
}

impl ProcessorStats {
    /// 重置统计
    pub fn reset(&mut self) {
        *self = Default::default();
    }

    /// 获取索引成功率
    pub fn indexed_success_rate(&self) -> f32 {
        let total = self.indexed_successes + self.indexed_failures;
        if total > 0 {
            self.indexed_successes as f32 / total as f32
        } else {
            0.0
        }
    }
}

/// 非 translation feature 的简化实现
#[cfg(not(feature = "translation"))]
pub struct TranslationProcessor;

#[cfg(not(feature = "translation"))]
impl TranslationProcessor {
    pub fn new() -> Self {
        Self
    }

    pub async fn process_batches(&mut self, _batches: Vec<Batch>) -> TranslationResult<()> {
        Err(TranslationError::new("Translation feature not enabled"))
    }
}