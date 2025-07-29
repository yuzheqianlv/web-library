//! 翻译处理器模块
//! 
//! 本模块提供了一个高级的翻译处理器，用于批量处理网页文本翻译任务。
//! 
//! 主要功能：
//! - 支持批量翻译处理，提高翻译效率
//! - 智能索引标记系统，减少API调用次数
//! - 并发处理支持，加速大批量翻译
//! - 完善的错误恢复和重试机制
//! - 详细的统计信息收集
//! 
//! # 使用示例
//! 
//! ```rust
//! use std::sync::Arc;
//! use markdown_translator::TranslationService;
//! use crate::translation::processor::{TranslationProcessor, ProcessorConfig};
//! 
//! async fn process_translation() -> Result<(), Box<dyn std::error::Error>> {
//!     let service = Arc::new(TranslationService::new());
//!     let config = ProcessorConfig::default();
//!     let mut processor = TranslationProcessor::new(service, config);
//!     
//!     // 处理批次...
//!     // processor.process_batches(batches).await?;
//!     
//!     Ok(())
//! }
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

#[cfg(feature = "translation")]
use markdown_translator::TranslationService;
#[cfg(feature = "translation")]
use regex::Regex;
#[cfg(feature = "translation")]
use tokio::time::{sleep, Duration};

use crate::html::set_node_attr;
use crate::translation::batch::{Batch, BatchType};
use crate::translation::collector::TextItem;
use crate::translation::error::{TranslationError, TranslationResult};
use markup5ever_rcdom::NodeData;

/// 翻译处理器
/// 
/// 负责协调和执行文本翻译任务的核心组件。支持多种处理策略：
/// - 顺序处理：适合小规模翻译任务
/// - 并发处理：适合大批量翻译任务，可显著提升性能
/// - 索引标记翻译：将多个文本合并后一次性翻译，减少API调用
/// 
/// # 字段
/// - `service`: 翻译服务的共享引用，用于实际执行翻译
/// - `stats`: 处理统计信息，记录成功率、耗时等数据
/// - `config`: 处理器配置，控制并发数、重试策略等行为
#[cfg(feature = "translation")]
pub struct TranslationProcessor {
    /// 翻译服务实例
    service: Arc<TranslationService>,
    /// 处理统计信息
    stats: ProcessorStats,
    /// 处理器配置参数
    config: ProcessorConfig,
}

#[cfg(feature = "translation")]
impl TranslationProcessor {
    /// 创建新的翻译处理器
    /// 
    /// # 参数
    /// - `service`: 翻译服务的共享引用，包含翻译API的具体实现
    /// - `config`: 处理器配置，定义并发数、重试策略等行为参数
    /// 
    /// # 返回值
    /// 返回一个新的 `TranslationProcessor` 实例，初始化统计信息为默认值
    /// 
    /// # 示例
    /// ```rust
    /// use std::sync::Arc;
    /// use crate::translation::processor::{TranslationProcessor, ProcessorConfig};
    /// 
    /// let service = Arc::new(TranslationService::new());
    /// let config = ProcessorConfig::default();
    /// let processor = TranslationProcessor::new(service, config);
    /// ```
    pub fn new(service: Arc<TranslationService>, config: ProcessorConfig) -> Self {
        Self {
            service,
            stats: ProcessorStats::default(),
            config,
        }
    }

    /// 处理批次列表（并发优化版本）
    /// 
    /// 根据配置自动选择最佳处理策略：
    /// - 当启用并发处理且批次数量大于1时，使用并发处理以提升性能
    /// - 否则使用顺序处理以确保稳定性
    /// 
    /// # 参数
    /// - `batches`: 待处理的翻译批次列表，每个批次包含一组相关的文本项
    /// 
    /// # 返回值
    /// - `Ok(())`: 所有批次处理成功
    /// - `Err(TranslationError)`: 处理过程中发生错误
    /// 
    /// # 特性
    /// - 自动重置统计信息
    /// - 智能策略选择
    /// - 详细的日志输出
    /// - 空批次检查和早期返回
    pub async fn process_batches(&mut self, batches: Vec<Batch>) -> TranslationResult<()> {
        // 重置统计信息，准备新的处理周期
        self.stats.reset();
        self.stats.total_batches = batches.len();

        if batches.is_empty() {
            tracing::info!("没有批次需要处理");
            return Ok(());
        }

        tracing::info!("开始并发处理 {} 个翻译批次", batches.len());

        // 根据配置决定处理策略
        if self.config.enable_concurrent_processing && batches.len() > 1 {
            self.process_batches_concurrently(batches).await
        } else {
            self.process_batches_sequentially(batches).await
        }
    }

    /// 并发处理批次（带超时和重试机制）
    /// 
    /// 使用信号量控制并发数量，每个批次在独立的异步任务中处理。
    /// 包含完整的错误处理、超时控制和重试机制。
    /// 
    /// # 参数
    /// - `batches`: 待处理的批次列表
    /// 
    /// # 返回值
    /// - `Ok(())`: 所有批次处理成功
    /// - `Err(TranslationError)`: 有批次处理失败
    /// 
    /// # 特性
    /// - 使用信号量限制并发数量，避免过载
    /// - 每个批次独立超时控制
    /// - 全局超时保护
    /// - 原子计数器统计成功/失败
    /// - 详细的性能统计
    async fn process_batches_concurrently(&mut self, batches: Vec<Batch>) -> TranslationResult<()> {
        use futures::future::join_all;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;
        use tokio::sync::Semaphore;
        use tokio::time::timeout;

        // 创建信号量控制并发数，防止同时处理过多批次导致资源耗尽
        let semaphore = Arc::new(Semaphore::new(self.config.max_concurrent_batches));
        let successful_count = Arc::new(AtomicUsize::new(0));
        let failed_count = Arc::new(AtomicUsize::new(0));
        let retry_count = Arc::new(AtomicUsize::new(0));

        // 创建带超时和重试的异步任务
        let tasks: Vec<_> = batches
            .into_iter()
            .enumerate()
            .map(|(batch_index, batch)| {
                let semaphore = Arc::clone(&semaphore);
                let service = Arc::clone(&self.service);
                let config = self.config.clone();
                let successful_count = Arc::clone(&successful_count);
                let failed_count = Arc::clone(&failed_count);
                let _retry_count = Arc::clone(&retry_count);

                async move {
                    // 获取信号量许可，控制并发数
                    let _permit = semaphore.acquire().await.map_err(|e| {
                        TranslationError::ConcurrencyError(format!("获取并发许可失败: {}", e))
                    })?;

                    tracing::debug!(
                        "开始并发处理批次 {}: {} 项",
                        batch_index + 1,
                        batch.items.len()
                    );

                    // 带重试的批次处理
                    let result = Self::process_batch_with_retry(
                        batch,
                        service,
                        config.clone(),
                        batch_index + 1,
                    )
                    .await;

                    // 更新原子计数器
                    match &result {
                        Ok(stats) => {
                            successful_count.fetch_add(1, Ordering::Relaxed);
                            tracing::debug!("批次 {} 处理成功", batch_index + 1);
                            Ok((batch_index, stats.clone()))
                        }
                        Err(e) => {
                            failed_count.fetch_add(1, Ordering::Relaxed);
                            tracing::error!("批次 {} 处理失败: {}", batch_index + 1, e);
                            Err(e.clone())
                        }
                    }
                }
            })
            .collect();

        let batches_count = tasks.len(); // 保存批次数量

        // 并发执行所有任务（带全局超时）
        let results = timeout(
            self.config.batch_timeout * (batches_count as u32 + 1),
            join_all(tasks),
        )
        .await
        .map_err(|_| {
            TranslationError::TimeoutError(format!(
                "全局批次处理超时: {:.1}秒",
                (self.config.batch_timeout * (batches_count as u32 + 1)).as_secs_f32()
            ))
        })?;

        // 收集统计信息
        let mut total_translated_items = 0;
        let mut total_processing_time = Duration::from_millis(0);

        for result in results {
            if let Ok((_, stats)) = result {
                total_translated_items += stats.translated_items;
                total_processing_time += stats.processing_time;
            }
        }

        // 更新主统计信息
        let successful = successful_count.load(Ordering::Relaxed);
        let failed = failed_count.load(Ordering::Relaxed);
        let retries = retry_count.load(Ordering::Relaxed);

        self.stats.successful_batches = successful;
        self.stats.failed_batches = failed;
        self.stats.translated_items = total_translated_items;
        self.stats.processing_time = total_processing_time;

        self.stats.success_rate = if self.stats.total_batches > 0 {
            self.stats.successful_batches as f32 / self.stats.total_batches as f32
        } else {
            0.0
        };

        tracing::info!(
            "并发批次处理完成: 成功 {}, 失败 {}, 重试 {}, 成功率 {:.1}%",
            successful,
            failed,
            retries,
            self.stats.success_rate * 100.0
        );

        if failed > 0 {
            Err(TranslationError::ProcessingError(format!(
                "{}个批次处理失败",
                failed
            )))
        } else {
            Ok(())
        }
    }

    /// 带重试机制的批次处理
    /// 
    /// 对单个批次执行处理，包含完整的重试逻辑和超时控制。
    /// 使用指数退避策略逐步增加重试延迟时间。
    /// 
    /// # 参数
    /// - `batch`: 待处理的批次
    /// - `service`: 翻译服务引用
    /// - `config`: 处理器配置
    /// - `batch_number`: 批次编号（用于日志标识）
    /// 
    /// # 返回值
    /// - `Ok(ProcessorStats)`: 处理成功，返回统计信息
    /// - `Err(TranslationError)`: 处理失败或达到最大重试次数
    /// 
    /// # 重试策略
    /// - 对于可重试的错误，使用指数退避算法
    /// - 对于不可重试的错误（如认证失败），立即返回
    /// - 每次重试延迟时间为：retry_delay * 2^attempt
    async fn process_batch_with_retry(
        batch: Batch,
        service: Arc<TranslationService>,
        config: ProcessorConfig,
        batch_number: usize,
    ) -> TranslationResult<ProcessorStats> {
        use tokio::time::{sleep, timeout};

        let mut last_error = None;

        for attempt in 0..=config.max_retries {
            // 创建临时处理器用于本次重试
            let mut temp_processor = TranslationProcessor {
                service: Arc::clone(&service),
                stats: ProcessorStats::default(),
                config: config.clone(),
            };

            // 带超时的批次处理
            let process_future = temp_processor.process_single_batch(batch.clone());
            let result = timeout(config.batch_timeout, process_future).await;

            match result {
                Ok(Ok(())) => {
                    if attempt > 0 {
                        tracing::info!("批次 {} 在第 {} 次重试后成功", batch_number, attempt);
                    }
                    return Ok(temp_processor.stats);
                }
                Ok(Err(e)) => {
                    last_error = Some(e.clone());
                    if !config.enable_retry || !e.is_retryable() {
                        tracing::error!("批次 {} 出现不可重试错误: {}", batch_number, e);
                        return Err(e);
                    }
                }
                Err(_) => {
                    let timeout_error = TranslationError::TimeoutError(format!(
                        "批次 {} 处理超时: {:.1}秒",
                        batch_number,
                        config.batch_timeout.as_secs_f32()
                    ));
                    last_error = Some(timeout_error.clone());
                    if !config.enable_retry {
                        return Err(timeout_error);
                    }
                }
            }

            // 如果不是最后一次尝试，等待后重试
            if attempt < config.max_retries {
                let delay = config.retry_delay * (2_u32.pow(attempt as u32)); // 指数退避
                tracing::warn!(
                    "批次 {} 处理失败，{:.1}秒后进行第 {} 次重试: {}",
                    batch_number,
                    delay.as_secs_f32(),
                    attempt + 1,
                    last_error.as_ref().unwrap()
                );
                sleep(delay).await;
            }
        }

        // 所有重试都失败
        Err(last_error.unwrap_or_else(|| {
            TranslationError::ProcessingError(format!(
                "批次 {} 经 {} 次重试后仍然失败",
                batch_number, config.max_retries
            ))
        }))
    }

    /// 顺序处理批次（原有逻辑保持向后兼容）
    /// 
    /// 按顺序逐个处理批次，适用于小规模翻译任务或需要严格控制资源使用的场景。
    /// 每个批次处理完成后会有配置的延迟时间，避免对翻译服务造成过大压力。
    /// 
    /// # 参数
    /// - `batches`: 待处理的批次列表
    /// 
    /// # 返回值
    /// - `Ok(())`: 处理完成（即使有部分失败）
    /// 
    /// # 特性
    /// - 每个批次间有延迟控制
    /// - 详细的进度日志
    /// - 失败容错，不会因单个批次失败而中断
    /// - 完整的统计信息收集
    async fn process_batches_sequentially(&mut self, batches: Vec<Batch>) -> TranslationResult<()> {
        tracing::info!("开始顺序处理 {} 个翻译批次", batches.len());

        for (i, batch) in batches.into_iter().enumerate() {
            tracing::debug!(
                "处理批次 {}/{}: {}",
                i + 1,
                self.stats.total_batches,
                batch.summary()
            );

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
            "顺序批次处理完成: 成功 {}/{}, 成功率 {:.1}%",
            self.stats.successful_batches,
            self.stats.total_batches,
            self.stats.success_rate * 100.0
        );

        Ok(())
    }

    /// 处理单个批次（支持Clone以便重试）
    /// 
    /// 根据批次类型选择最适合的处理策略：
    /// - Single: 单个项目，直接处理
    /// - Small: 小批次，可选择逐个处理或索引批处理
    /// - 其他: 使用索引批处理以提高效率
    /// 
    /// # 参数
    /// - `batch`: 待处理的批次，必须实现Clone以支持重试机制
    /// 
    /// # 返回值
    /// - `Ok(())`: 批次处理成功
    /// - `Err(TranslationError)`: 处理失败
    /// 
    /// # 处理策略
    /// - 根据批次大小和类型智能选择处理方式
    /// - 记录处理时间统计
    /// - 支持重试机制（通过Clone trait）
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
    /// 
    /// 尝试使用索引标记方式批量翻译文本，这种方式可以显著减少API调用次数。
    /// 如果索引翻译失败，会自动回退到逐个翻译模式。
    /// 
    /// # 参数
    /// - `batch`: 待处理的批次引用
    /// 
    /// # 返回值
    /// - `Ok(())`: 批次处理成功（可能使用了索引翻译或回退翻译）
    /// - `Err(TranslationError)`: 所有翻译方式都失败
    /// 
    /// # 处理流程
    /// 1. 尝试索引标记翻译
    /// 2. 如果成功，更新成功统计
    /// 3. 如果失败，回退到逐个翻译模式
    /// 4. 更新相应的统计信息
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
    /// 
    /// 将批次中的多个文本合并为一个带索引标记的文本，一次性发送给翻译服务。
    /// 这种方式可以利用翻译服务的上下文理解能力，提高翻译质量和效率。
    /// 
    /// # 参数
    /// - `batch`: 包含多个文本项的批次
    /// 
    /// # 返回值
    /// - `Ok(true)`: 索引翻译成功，所有文本都得到了有效翻译
    /// - `Ok(false)`: 索引翻译部分成功，但成功率低于阈值
    /// - `Err(TranslationError)`: 翻译服务调用失败
    /// 
    /// # 索引格式
    /// 输入格式：`[0] 第一段文本\n[1] 第二段文本\n...`
    /// 期望输出格式：`[0] 翻译后的第一段\n[1] 翻译后的第二段\n...`
    async fn try_indexed_translation(&mut self, batch: &Batch) -> TranslationResult<bool> {
        let combined_text = self.combine_texts_with_indices(&batch.items);

        tracing::debug!("索引组合文本长度: {} 字符", combined_text.len());

        match self.service.translate(&combined_text).await {
            Ok(translated) => {
                let success = self.apply_indexed_translation(&batch.items, &translated)?;
                Ok(success)
            }
            Err(e) => Err(TranslationError::TranslationServiceError(format!(
                "索引翻译请求失败: {}",
                e
            ))),
        }
    }

    /// 组合文本并添加索引标记
    /// 
    /// 将多个文本项合并为一个字符串，每个文本前添加索引标记。
    /// 这种格式帮助翻译服务理解文本结构，并在返回时保持对应关系。
    /// 
    /// # 参数
    /// - `items`: 文本项数组
    /// 
    /// # 返回值
    /// 带索引标记的合并文本，格式为：`[索引] 文本内容`
    /// 
    /// # 示例
    /// ```text
    /// 输入: ["Hello", "World", "Test"]
    /// 输出: "[0] Hello\n[1] World\n[2] Test"
    /// ```
    fn combine_texts_with_indices(&self, items: &[TextItem]) -> String {
        items
            .iter()
            .enumerate()
            .map(|(i, item)| format!("[{}] {}", i, item.text.trim()))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// 应用索引翻译结果到DOM节点
    /// 
    /// 解析翻译服务返回的索引标记文本，并将翻译结果应用到对应的DOM节点。
    /// 通过正则表达式匹配索引标记格式，确保翻译结果正确对应到原始文本。
    /// 
    /// # 参数
    /// - `items`: 原始文本项数组
    /// - `translated`: 翻译服务返回的带索引标记的翻译结果
    /// 
    /// # 返回值
    /// - `Ok(true)`: 翻译成功率达到阈值要求
    /// - `Ok(false)`: 翻译成功率低于阈值，建议回退到逐个翻译
    /// - `Err(TranslationError)`: 解析过程中发生错误
    /// 
    /// # 解析规则
    /// - 期望格式：`[数字] 翻译内容`
    /// - 忽略格式不正确的行
    /// - 空翻译内容会被跳过
    /// - 根据成功率判断整体翻译质量
    fn apply_indexed_translation(
        &mut self,
        items: &[TextItem],
        translated: &str,
    ) -> TranslationResult<bool> {
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

    /// 逐个处理文本项目
    /// 
    /// 当索引批处理失败或不适用时的回退策略。
    /// 逐个发送文本到翻译服务，虽然API调用次数增加，但可以确保翻译的准确性。
    /// 
    /// # 参数
    /// - `items`: 待翻译的文本项数组
    /// 
    /// # 返回值
    /// - `Ok(())`: 处理完成（部分项目可能失败，但不中断整体流程）
    /// 
    /// # 特性
    /// - 容错处理：单个项目失败不影响其他项目
    /// - 进度跟踪：详细记录每个项目的处理状态
    /// - 速率控制：项目间有小延迟，避免过度请求
    /// - 统计更新：实时更新成功和失败计数
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

    /// 处理单个文本项目
    /// 
    /// 翻译单个文本项并将结果应用到对应的DOM节点。
    /// 这是最基础的翻译操作，被其他处理方法调用。
    /// 
    /// # 参数
    /// - `item`: 包含文本内容和DOM节点信息的文本项
    /// 
    /// # 返回值
    /// - `Ok(())`: 翻译并应用成功
    /// - `Err(TranslationError)`: 翻译服务调用失败或应用失败
    /// 
    /// # 处理流程
    /// 1. 调用翻译服务获取翻译结果
    /// 2. 清理翻译结果（去除首尾空格）
    /// 3. 将翻译结果应用到DOM节点
    async fn process_single_item(&mut self, item: &TextItem) -> TranslationResult<()> {
        let translated = self
            .service
            .translate(&item.text)
            .await
            .map_err(|e| TranslationError::TranslationServiceError(e.to_string()))?;

        self.apply_translation(item, &translated.trim())?;
        Ok(())
    }

    /// 将翻译结果应用到DOM节点
    /// 
    /// 根据文本项的类型（文本内容或属性值），将翻译结果写入对应的DOM位置。
    /// 包含翻译质量检查和不同节点类型的处理逻辑。
    /// 
    /// # 参数
    /// - `item`: 包含目标DOM节点和位置信息的文本项
    /// - `translated`: 已翻译的文本内容
    /// 
    /// # 返回值
    /// - `Ok(())`: 应用成功
    /// - `Err(TranslationError)`: 应用失败或翻译结果无效
    /// 
    /// # 处理类型
    /// - **属性翻译**: 更新DOM节点的指定属性值
    /// - **文本翻译**: 更新DOM节点的文本内容
    /// 
    /// # 质量检查
    /// - 检查翻译结果是否为空
    /// - 检测可能的翻译失败（结果与原文相同）
    /// - 验证DOM节点类型的正确性
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
                    "节点不是文本类型".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// 获取处理器统计信息
    /// 
    /// 返回当前处理周期的统计信息引用，用于监控和分析处理效果。
    /// 
    /// # 返回值
    /// 包含批次处理、项目翻译、索引处理等各项统计数据的引用
    /// 
    /// # 用途
    /// - 监控翻译处理的成功率和性能
    /// - 分析不同翻译策略的效果
    /// - 优化处理器配置参数
    /// - 生成处理报告
    pub fn get_stats(&self) -> &ProcessorStats {
        &self.stats
    }
}

/// 处理器配置
/// 
/// 用于控制翻译处理器的各种行为参数和性能设置。
/// 可以根据具体使用场景调整这些参数以优化翻译效果和性能。
/// 
/// # 配置分类
/// - **批次处理**: 控制批次大小和成功率阈值
/// - **延迟控制**: 控制批次间和重试间的延迟时间
/// - **重试机制**: 控制重试策略和次数
/// - **并发控制**: 控制并发处理的行为和限制
/// - **超时控制**: 控制各种操作的超时时间
#[derive(Debug, Clone)]
pub struct ProcessorConfig {
    /// 小批次阈值
    /// 
    /// 当批次中的文本项数量小于等于此值时，使用逐个翻译策略而不是索引批处理。
    /// 较小的值会更多使用逐个翻译，提高准确性但降低效率。
    pub small_batch_threshold: usize,
    
    /// 最小成功率
    /// 
    /// 索引批处理的最小成功率阈值（0.0-1.0）。
    /// 当索引翻译的成功率低于此值时，会判定为失败并回退到逐个翻译。
    pub min_success_rate: f32,
    
    /// 批次间延迟时间
    /// 
    /// 在顺序处理模式下，每个批次处理完成后的等待时间。
    /// 用于避免对翻译服务造成过大压力，防止触发限流。
    pub batch_delay: Duration,
    
    /// 是否启用重试机制
    /// 
    /// 当设置为true时，处理失败的批次会自动重试。
    /// 重试次数由max_retries控制，重试间隔由retry_delay控制。
    pub enable_retry: bool,
    
    /// 最大重试次数
    /// 
    /// 每个批次处理失败后的最大重试次数。
    /// 使用指数退避策略，每次重试的延迟时间会逐步增加。
    pub max_retries: usize,
    
    /// 重试基础延迟时间
    /// 
    /// 第一次重试的延迟时间。后续重试使用指数退避策略：
    /// delay = retry_delay * 2^attempt_number
    pub retry_delay: Duration,
    
    /// 是否启用并发处理
    /// 
    /// 当设置为true且批次数量大于1时，会使用并发处理以提升性能。
    /// 并发数由max_concurrent_batches控制。
    pub enable_concurrent_processing: bool,
    
    /// 最大并发批次数
    /// 
    /// 同时处理的批次数量上限。
    /// 过高的值可能导致资源耗尽或触发翻译服务的限流。
    pub max_concurrent_batches: usize,
    
    /// 单个批次超时时间
    /// 
    /// 每个批次处理的最大允许时间。
    /// 超过此时间的批次会被标记为超时并可能触发重试。
    pub batch_timeout: Duration,
}

impl Default for ProcessorConfig {
    /// 创建默认配置
    /// 
    /// 默认配置适用于大多数使用场景，提供了性能和稳定性的平衡：
    /// 
    /// - `small_batch_threshold`: 2 - 小批次使用逐个翻译
    /// - `min_success_rate`: 0.8 - 索引翻译80%成功率阈值
    /// - `batch_delay`: 100ms - 批次间适中延迟
    /// - `enable_retry`: true - 启用重试机制
    /// - `max_retries`: 3 - 最多重试3次
    /// - `retry_delay`: 1000ms - 1秒基础重试延迟
    /// - `enable_concurrent_processing`: true - 启用并发处理
    /// - `max_concurrent_batches`: 5 - 最多5个并发批次
    /// - `batch_timeout`: 30s - 批次超时时间
    fn default() -> Self {
        Self {
            small_batch_threshold: 2,
            min_success_rate: 0.8,
            batch_delay: Duration::from_millis(100),
            enable_retry: true,
            max_retries: 3,
            retry_delay: Duration::from_millis(1000),
            enable_concurrent_processing: true,
            max_concurrent_batches: 5,
            batch_timeout: Duration::from_secs(30),
        }
    }
}

/// 处理器统计信息
/// 
/// 收集和维护翻译处理过程中的各种统计数据，用于监控处理效果和性能。
/// 这些统计信息可以帮助优化配置参数和诊断处理问题。
/// 
/// # 统计类别
/// - **批次统计**: 总数、成功数、失败数和成功率
/// - **项目统计**: 翻译成功和跳过的文本项数量
/// - **索引统计**: 索引批处理的成功和失败次数
/// - **性能统计**: 总处理时间
#[derive(Debug, Clone, Default)]
pub struct ProcessorStats {
    /// 总批次数量
    /// 
    /// 本次处理周期中的批次总数。
    pub total_batches: usize,
    
    /// 成功处理的批次数量
    /// 
    /// 完全成功处理的批次数量，用于计算成功率。
    pub successful_batches: usize,
    
    /// 失败的批次数量
    /// 
    /// 处理失败的批次数量（包括重试后仍失败的批次）。
    pub failed_batches: usize,
    
    /// 批次处理成功率
    /// 
    /// 成功批次数量占总批次数量的比例（0.0-1.0）。
    /// 计算公式：successful_batches / total_batches
    pub success_rate: f32,
    
    /// 成功翻译的文本项数量
    /// 
    /// 实际完成翻译的单个文本项总数。
    pub translated_items: usize,
    
    /// 跳过的文本项数量
    /// 
    /// 由于错误或其他原因未能翻译的文本项数量。
    pub skipped_items: usize,
    
    /// 索引批处理成功次数
    /// 
    /// 使用索引标记方式成功处理的批次数量。
    pub indexed_successes: usize,
    
    /// 索引批处理失败次数
    /// 
    /// 索引标记方式处理失败，需要回退到逐个翻译的批次数量。
    pub indexed_failures: usize,
    
    /// 总处理时间
    /// 
    /// 所有批次处理的累计时间，不包括延迟等待时间。
    pub processing_time: Duration,
}

impl ProcessorStats {
    /// 重置所有统计信息
    /// 
    /// 将所有统计字段重置为默认值，通常在开始新的处理周期时调用。
    pub fn reset(&mut self) {
        *self = Default::default();
    }

    /// 计算索引批处理成功率
    /// 
    /// 返回索引批处理的成功率（0.0-1.0）。
    /// 如果没有进行过索引批处理，返回0.0。
    /// 
    /// # 返回值
    /// 索引成功次数占总索引尝试次数的比例
    /// 
    /// # 计算公式
    /// ```text
    /// success_rate = indexed_successes / (indexed_successes + indexed_failures)
    /// ```
    pub fn indexed_success_rate(&self) -> f32 {
        let total = self.indexed_successes + self.indexed_failures;
        if total > 0 {
            self.indexed_successes as f32 / total as f32
        } else {
            0.0
        }
    }
}

/// 非翻译功能的占位实现
/// 
/// 当未启用 "translation" 功能时的简化实现。
/// 提供相同的API接口，但所有操作都会返回功能未启用的错误。
/// 
/// 这种设计允许代码在不同功能配置下保持一致的API，
/// 避免条件编译对上层代码的影响。
#[cfg(not(feature = "translation"))]
pub struct TranslationProcessor;

#[cfg(not(feature = "translation"))]
impl TranslationProcessor {
    /// 创建占位处理器实例
    /// 
    /// 返回一个空的处理器实例，所有操作都会失败并提示功能未启用。
    pub fn new() -> Self {
        Self
    }

    /// 处理批次的占位实现
    /// 
    /// 始终返回功能未启用的错误，提示用户需要启用translation功能。
    /// 
    /// # 参数
    /// - `_batches`: 被忽略的批次列表
    /// 
    /// # 返回值
    /// 始终返回 `TranslationError`，提示功能未启用
    pub async fn process_batches(&mut self, _batches: Vec<Batch>) -> TranslationResult<()> {
        Err(TranslationError::new("Translation feature not enabled"))
    }
}
