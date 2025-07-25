//! 翻译引擎核心实现
//!
//! 处理底层翻译逻辑，包括批次处理、索引标记和错误恢复

use std::collections::HashMap;
use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
use std::time::Instant;
use markup5ever_rcdom::{RcDom, NodeData};

#[cfg(feature = "translation")]
use markdown_translator::TranslationService;
#[cfg(feature = "translation")]
use regex::Regex;
#[cfg(feature = "translation")]
use tokio::time::{sleep, Duration};

use crate::html::set_node_attr;
use crate::translation::{
    pipeline::batch::Batch,
    error::{TranslationError, TranslationResult},
};

/// 翻译引擎
/// 
/// 负责底层翻译处理，包括文本索引、批次翻译和DOM更新
#[cfg(feature = "translation")]
pub struct TranslationEngine {
    service: Arc<TranslationService>,
    stats: EngineStats,
    config: EngineConfig,
}

#[cfg(feature = "translation")]
impl TranslationEngine {
    /// 创建新的翻译引擎
    pub fn new(service: Arc<TranslationService>, config: EngineConfig) -> Self {
        Self {
            service,
            stats: EngineStats::default(),
            config,
        }
    }
    
    /// 处理单个批次翻译
    pub async fn process_batch(&mut self, dom: RcDom, batch: Batch) -> TranslationResult<RcDom> {
        let start_time = Instant::now();
        
        tracing::debug!(
            "处理翻译批次: {} 项，预估 {} 字符",
            batch.items.len(),
            batch.estimated_chars
        );
        
        // 1. 准备翻译文本
        let texts = self.prepare_texts(&batch)?;
        if texts.is_empty() {
            return Ok(dom);
        }
        
        // 2. 执行翻译
        let translated_texts = self.translate_texts(texts).await?;
        
        // 3. 更新DOM
        let updated_dom = self.update_dom(dom, &batch, translated_texts)?;
        
        self.stats.inc_batches_processed();
        self.stats.add_processing_time(start_time.elapsed());
        
        Ok(updated_dom)
    }
    
    /// 准备翻译文本（优化版，减少克隆）
    fn prepare_texts(&self, batch: &Batch) -> TranslationResult<Vec<String>> {
        // 预分配容量以减少重分配
        let mut texts = Vec::with_capacity(batch.items.len());
        
        // 所有文本类型都需要翻译，简化匹配
        for item in &batch.items {
            // 直接添加文本，无需按类型分类
            texts.push(item.text.clone());  // 这里仍需要克隆因为需要owned值
        }
        
        // 添加索引标记以便后续匹配
        if self.config.use_indexing {
            texts = self.add_index_markers(texts);
        }
        
        Ok(texts)
    }
    
    /// 添加索引标记
    fn add_index_markers(&self, texts: Vec<String>) -> Vec<String> {
        texts
            .into_iter()
            .enumerate()
            .map(|(i, text)| {
                if text.trim().is_empty() {
                    text
                } else {
                    format!("[{}]{}", i, text)
                }
            })
            .collect()
    }
    
    /// 执行翻译
    async fn translate_texts(&mut self, texts: Vec<String>) -> TranslationResult<Vec<String>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        
        let combined_text = texts.join("\n\n");
        self.stats.add_characters_sent(combined_text.len());
        
        // 执行翻译并处理重试
        let mut attempt = 0;
        let max_attempts = self.config.max_retry_attempts;
        
        while attempt < max_attempts {
            match self.service.translate(&combined_text).await {
                Ok(translated) => {
                    self.stats.add_characters_received(translated.len());
                    return self.parse_translated_response(translated, texts.len());
                }
                Err(e) => {
                    attempt += 1;
                    self.stats.inc_translation_errors();
                    
                    if attempt >= max_attempts {
                        return Err(TranslationError::TranslationServiceError(
                            format!("翻译失败，已重试 {} 次: {}", max_attempts, e)
                        ));
                    }
                    
                    // 指数退避
                    let delay = Duration::from_millis(self.config.retry_delay_ms * (1 << attempt));
                    tracing::warn!("翻译失败，{}ms后重试 (尝试 {}/{}): {}", delay.as_millis(), attempt + 1, max_attempts, e);
                    sleep(delay).await;
                }
            }
        }
        
        Err(TranslationError::TranslationServiceError("翻译重试失败".to_string()))
    }
    
    /// 解析翻译响应
    fn parse_translated_response(&mut self, response: String, expected_count: usize) -> TranslationResult<Vec<String>> {
        let parts: Vec<String> = response
            .split("\n\n")
            .map(|s| s.trim().to_string())
            .collect();
        
        if self.config.use_indexing {
            self.parse_indexed_response(parts, expected_count)
        } else {
            if parts.len() != expected_count {
                tracing::warn!(
                    "翻译结果数量不匹配: 期望 {}，得到 {}",
                    expected_count,
                    parts.len()
                );
            }
            Ok(parts)
        }
    }
    
    /// 解析带索引的翻译响应
    fn parse_indexed_response(&mut self, parts: Vec<String>, expected_count: usize) -> TranslationResult<Vec<String>> {
        let index_regex = Regex::new(r"^\[(\d+)\](.*)$").unwrap();
        let mut indexed_results = HashMap::new();
        
        for part in parts {
            if let Some(captures) = index_regex.captures(&part) {
                if let (Some(index_str), Some(text)) = (captures.get(1), captures.get(2)) {
                    if let Ok(index) = index_str.as_str().parse::<usize>() {
                        indexed_results.insert(index, text.as_str().to_string());
                    }
                }
            }
        }
        
        // 重建结果向量
        let mut results = Vec::with_capacity(expected_count);
        for i in 0..expected_count {
            if let Some(translated) = indexed_results.get(&i) {
                results.push(translated.clone());
            } else {
                // 如果找不到对应索引，使用原文或空字符串
                results.push(String::new());
                self.stats.inc_missing_translations();
            }
        }
        
        Ok(results)
    }
    
    /// 更新DOM
    fn update_dom(
        &self,
        dom: RcDom,
        batch: &Batch,
        translated_texts: Vec<String>
    ) -> TranslationResult<RcDom> {
        if batch.items.len() != translated_texts.len() {
            return Err(TranslationError::InternalError(
                format!(
                    "文本项数量与翻译结果数量不匹配: {} vs {}",
                    batch.items.len(),
                    translated_texts.len()
                )
            ));
        }
        
        for (item, translated) in batch.items.iter().zip(translated_texts.iter()) {
            if translated.trim().is_empty() {
                continue; // 跳过空翻译
            }
            
            match &item.text_type {
                crate::translation::pipeline::collector::TextType::Content |
                crate::translation::pipeline::collector::TextType::Title |
                crate::translation::pipeline::collector::TextType::Link |
                crate::translation::pipeline::collector::TextType::Button |
                crate::translation::pipeline::collector::TextType::FormLabel |
                crate::translation::pipeline::collector::TextType::ImageAlt |
                crate::translation::pipeline::collector::TextType::Tooltip => {
                    self.update_text_content(&item.node, translated)?;
                }
                crate::translation::pipeline::collector::TextType::Attribute(attr_name) => {
                    self.update_attribute_content(&item.node, attr_name, translated)?;
                }
            }
        }
        
        Ok(dom)
    }
    
    /// 更新文本内容
    fn update_text_content(
        &self,
        node: &markup5ever_rcdom::Handle,
        translated: &str
    ) -> TranslationResult<()> {
        if let NodeData::Text { contents } = &node.data {
            let mut contents = contents.borrow_mut();
            *contents = translated.into();
        }
        Ok(())
    }
    
    /// 更新属性内容
    fn update_attribute_content(
        &self,
        node: &markup5ever_rcdom::Handle,
        attr_name: &str,
        translated: &str
    ) -> TranslationResult<()> {
        set_node_attr(node, attr_name, Some(translated.to_string()));
        Ok(())
    }
    
    /// 获取统计信息
    pub fn get_stats(&self) -> &EngineStats {
        &self.stats
    }
    
    /// 重置统计信息
    pub fn reset_stats(&mut self) {
        self.stats.reset();
    }
}

/// 翻译引擎配置
#[derive(Debug, Clone)]
pub struct EngineConfig {
    pub use_indexing: bool,
    pub max_retry_attempts: usize,
    pub retry_delay_ms: u64,
    pub max_concurrent_requests: usize,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            use_indexing: true,
            max_retry_attempts: 3,
            retry_delay_ms: 1000,
            max_concurrent_requests: 5,
        }
    }
}

/// 翻译引擎统计信息（线程安全版本）
#[derive(Debug, Default)]
pub struct EngineStats {
    pub batches_processed: AtomicUsize,
    pub characters_sent: AtomicUsize,
    pub characters_received: AtomicUsize,
    pub translation_errors: AtomicUsize,
    pub missing_translations: AtomicUsize,
    pub total_processing_time: std::sync::atomic::AtomicU64,  // 微秒
}

impl EngineStats {
    /// 增加处理批次计数
    pub fn inc_batches_processed(&self) {
        self.batches_processed.fetch_add(1, Ordering::Relaxed);
    }
    
    /// 添加发送字符数
    pub fn add_characters_sent(&self, count: usize) {
        self.characters_sent.fetch_add(count, Ordering::Relaxed);
    }
    
    /// 添加接收字符数
    pub fn add_characters_received(&self, count: usize) {
        self.characters_received.fetch_add(count, Ordering::Relaxed);
    }
    
    /// 增加翻译错误计数
    pub fn inc_translation_errors(&self) {
        self.translation_errors.fetch_add(1, Ordering::Relaxed);
    }
    
    /// 增加缺失翻译计数
    pub fn inc_missing_translations(&self) {
        self.missing_translations.fetch_add(1, Ordering::Relaxed);
    }
    
    /// 添加处理时间
    pub fn add_processing_time(&self, duration: std::time::Duration) {
        self.total_processing_time.fetch_add(duration.as_micros() as u64, Ordering::Relaxed);
    }
}

impl EngineStats {
    /// 重置统计信息
    pub fn reset(&mut self) {
        *self = Default::default();
    }
}

/// 非translation特性的占位实现
#[cfg(not(feature = "translation"))]
pub struct TranslationEngine;

#[cfg(not(feature = "translation"))]
impl TranslationEngine {
    pub fn new(_service: (), _config: EngineConfig) -> Self {
        Self
    }
}

#[cfg(not(feature = "translation"))]
pub type EngineConfig = ();

#[cfg(not(feature = "translation"))]
pub type EngineStats = ();