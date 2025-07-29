//! 翻译引擎核心实现
//!
//! 这个模块实现了 Monolith 项目的翻译引擎，负责将网页内容从源语言翻译为目标语言。
//! 翻译引擎采用批处理方式来优化性能，支持以下核心功能：
//! 
//! ## 主要功能
//! - **批次处理**: 将多个翻译项目组合成批次，减少API调用次数
//! - **索引标记**: 通过在文本前添加索引标记来确保翻译结果的正确对应
//! - **错误恢复**: 实现重试机制和指数退避策略来处理网络错误
//! - **性能统计**: 跟踪翻译性能指标，包括处理时间、字符数等
//! - **DOM更新**: 将翻译结果正确应用到HTML DOM树中
//!
//! ## 工作流程
//! 1. 接收包含待翻译文本的批次对象
//! 2. 为文本添加索引标记（可选）
//! 3. 调用外部翻译服务进行翻译
//! 4. 解析翻译结果并匹配到原始文本
//! 5. 更新DOM节点中的文本内容或属性
//!
//! ## 使用示例
//! ```rust
//! use translation::core::engine::{TranslationEngine, EngineConfig};
//! 
//! let config = EngineConfig::default();
//! let mut engine = TranslationEngine::new(translation_service, config);
//! let translated_dom = engine.process_batch(dom, batch).await?;
//! ```

use markup5ever_rcdom::{NodeData, RcDom};
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::time::Instant;

#[cfg(feature = "translation")]
use markdown_translator::TranslationService;
#[cfg(feature = "translation")]
use regex::Regex;
#[cfg(feature = "translation")]
use tokio::time::{sleep, Duration};

use crate::html::set_node_attr;
use crate::translation::{
    error::{TranslationError, TranslationResult},
    pipeline::batch::Batch,
};

/// 翻译引擎
///
/// 这是翻译系统的核心组件，负责协调整个翻译流程。引擎维护翻译服务连接、
/// 统计信息和配置参数，提供高效的批量翻译处理能力。
///
/// ## 设计特点
/// - 使用 `Arc<TranslationService>` 支持多线程共享翻译服务
/// - 内置统计系统跟踪翻译性能和错误率
/// - 可配置的重试策略和并发控制
/// - 支持索引标记确保翻译结果正确对应
///
/// ## 线程安全性
/// 引擎本身不是线程安全的（需要 `&mut self`），但内部的统计信息使用原子操作，
/// 翻译服务使用 `Arc` 包装支持多线程访问。
#[cfg(feature = "translation")]
pub struct TranslationEngine {
    /// 翻译服务实例，使用 Arc 支持多线程共享
    service: Arc<TranslationService>,
    /// 翻译统计信息，使用原子操作确保线程安全
    stats: EngineStats,
    /// 引擎配置参数，包括重试策略和并发设置
    config: EngineConfig,
}

#[cfg(feature = "translation")]
impl TranslationEngine {
    /// 创建新的翻译引擎
    ///
    /// # 参数
    /// - `service`: 翻译服务实例，封装在 Arc 中支持多线程共享
    /// - `config`: 引擎配置，包含重试策略、并发限制等参数
    ///
    /// # 返回值
    /// 返回初始化完成的翻译引擎实例，统计信息被重置为默认值
    ///
    /// # 示例
    /// ```rust
    /// let service = Arc::new(translation_service);
    /// let config = EngineConfig::default();
    /// let engine = TranslationEngine::new(service, config);
    /// ```
    pub fn new(service: Arc<TranslationService>, config: EngineConfig) -> Self {
        Self {
            service,
            stats: EngineStats::default(),
            config,
        }
    }

    /// 处理单个批次翻译
    ///
    /// 这是引擎的主要入口方法，处理完整的翻译流程：从文本准备到DOM更新。
    /// 方法会自动记录处理时间和统计信息。
    ///
    /// # 参数
    /// - `dom`: 要处理的HTML DOM树
    /// - `batch`: 包含待翻译文本项的批次对象
    ///
    /// # 返回值
    /// 返回更新后的DOM树，其中包含翻译后的文本内容
    ///
    /// # 错误
    /// - `TranslationError::TranslationServiceError`: 翻译服务调用失败
    /// - `TranslationError::InternalError`: 内部处理错误，如文本数量不匹配
    ///
    /// # 性能考虑
    /// 此方法会记录处理时间和字符统计，用于性能监控和优化
    pub async fn process_batch(&mut self, dom: RcDom, batch: Batch) -> TranslationResult<RcDom> {
        let start_time = Instant::now();

        tracing::debug!(
            "处理翻译批次: {} 项，预估 {} 字符",
            batch.items.len(),
            batch.estimated_chars
        );

        // 1. 准备翻译文本 - 提取并格式化待翻译的文本
        let texts = self.prepare_texts(&batch)?;
        if texts.is_empty() {
            return Ok(dom); // 没有需要翻译的内容，直接返回原DOM
        }

        // 2. 执行翻译 - 调用外部翻译服务
        let translated_texts = self.translate_texts(texts).await?;

        // 3. 更新DOM - 将翻译结果应用到DOM节点
        let updated_dom = self.update_dom(dom, &batch, translated_texts)?;

        // 更新统计信息
        self.stats.inc_batches_processed();
        self.stats.add_processing_time(start_time.elapsed());

        Ok(updated_dom)
    }

    /// 准备翻译文本（优化版，减少克隆）
    ///
    /// 从批次中提取待翻译的文本，并根据配置添加索引标记。
    /// 此方法经过优化以减少内存分配和数据克隆。
    ///
    /// # 参数
    /// - `batch`: 包含待翻译文本项的批次
    ///
    /// # 返回值
    /// 返回准备好的文本向量，如果启用了索引标记，文本会包含形如 "[0]文本内容" 的前缀
    ///
    /// # 性能优化
    /// - 预分配向量容量避免重复分配
    /// - 统一处理所有文本类型，简化逻辑分支
    fn prepare_texts(&self, batch: &Batch) -> TranslationResult<Vec<String>> {
        // 预分配容量以减少重分配，提升性能
        let mut texts = Vec::with_capacity(batch.items.len());

        // 所有文本类型都需要翻译，简化匹配逻辑
        for item in &batch.items {
            // 直接添加文本，无需按类型分类
            // 注意：这里需要克隆因为返回的是 owned 值
            texts.push(item.text.clone());
        }

        // 根据配置决定是否添加索引标记
        // 索引标记用于确保翻译结果能正确对应到原始文本
        if self.config.use_indexing {
            texts = self.add_index_markers(texts);
        }

        Ok(texts)
    }

    /// 添加索引标记
    ///
    /// 为每个文本项添加数字索引前缀，用于在翻译后匹配原文和译文。
    /// 这是解决批量翻译时文本顺序可能被打乱问题的关键机制。
    ///
    /// # 参数
    /// - `texts`: 原始文本向量
    ///
    /// # 返回值
    /// 返回添加索引标记后的文本向量，格式为 "[索引]原文本"
    ///
    /// # 标记规则
    /// - 空文本或仅包含空白字符的文本不添加标记
    /// - 非空文本添加形如 "[0]", "[1]" 的数字索引前缀
    /// - 索引从0开始递增
    fn add_index_markers(&self, texts: Vec<String>) -> Vec<String> {
        texts
            .into_iter()
            .enumerate()
            .map(|(i, text)| {
                // 空文本不需要添加索引标记，避免干扰翻译服务
                if text.trim().is_empty() {
                    text
                } else {
                    // 添加数字索引前缀，格式: [索引]文本内容
                    format!("[{}]{}", i, text)
                }
            })
            .collect()
    }

    /// 执行翻译
    ///
    /// 调用外部翻译服务进行文本翻译，实现自动重试和错误恢复机制。
    /// 使用指数退避策略来处理临时网络错误和服务限流。
    ///
    /// # 参数
    /// - `texts`: 待翻译的文本列表
    ///
    /// # 返回值
    /// 返回翻译后的文本列表，与输入文本一一对应
    ///
    /// # 错误处理策略
    /// - 自动重试：根据配置的最大重试次数进行重试
    /// - 指数退避：每次重试的等待时间按指数递增
    /// - 统计记录：记录错误次数和发送/接收的字符数
    ///
    /// # 性能优化
    /// - 将多个文本合并为单次API调用，减少网络开销
    /// - 使用 "\n\n" 作为分隔符，便于翻译服务识别文本边界
    async fn translate_texts(&mut self, texts: Vec<String>) -> TranslationResult<Vec<String>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        // 将所有文本合并为单个请求，用双换行分隔
        // 这样可以减少API调用次数，提高效率
        let combined_text = texts.join("\n\n");
        self.stats.add_characters_sent(combined_text.len());

        // 实现重试机制，处理网络错误和服务临时不可用
        let mut attempt = 0;
        let max_attempts = self.config.max_retry_attempts;

        while attempt < max_attempts {
            match self.service.translate(&combined_text).await {
                Ok(translated) => {
                    // 翻译成功，记录统计信息并解析响应
                    self.stats.add_characters_received(translated.len());
                    return self.parse_translated_response(translated, texts.len());
                }
                Err(e) => {
                    attempt += 1;
                    self.stats.inc_translation_errors();

                    // 达到最大重试次数，返回错误
                    if attempt >= max_attempts {
                        return Err(TranslationError::TranslationServiceError(format!(
                            "翻译失败，已重试 {} 次: {}",
                            max_attempts, e
                        )));
                    }

                    // 指数退避策略：等待时间随重试次数指数增长
                    // 公式：base_delay * 2^attempt
                    let delay = Duration::from_millis(self.config.retry_delay_ms * (1 << attempt));
                    tracing::warn!(
                        "翻译失败，{}ms后重试 (尝试 {}/{}): {}",
                        delay.as_millis(),
                        attempt + 1,
                        max_attempts,
                        e
                    );
                    sleep(delay).await;
                }
            }
        }

        // 理论上不会到达这里，但为了类型安全提供默认错误
        Err(TranslationError::TranslationServiceError(
            "翻译重试失败".to_string(),
        ))
    }

    /// 解析翻译响应
    ///
    /// 将翻译服务返回的合并文本分解为独立的翻译结果。
    /// 根据配置决定是否使用索引标记进行精确匹配。
    ///
    /// # 参数
    /// - `response`: 翻译服务返回的完整响应文本
    /// - `expected_count`: 期望的文本项数量，用于验证结果完整性
    ///
    /// # 返回值
    /// 返回按顺序排列的翻译结果向量
    ///
    /// # 处理策略
    /// - 使用索引标记时：解析 "[索引]译文" 格式，确保顺序正确
    /// - 不使用索引标记时：按分隔符直接分割，依赖顺序保持
    /// - 数量不匹配时记录警告，但不中断处理
    fn parse_translated_response(
        &mut self,
        response: String,
        expected_count: usize,
    ) -> TranslationResult<Vec<String>> {
        // 按双换行符分割响应，对应发送时的合并方式
        let parts: Vec<String> = response
            .split("\n\n")
            .map(|s| s.trim().to_string()) // 去除每部分的前后空白
            .collect();

        if self.config.use_indexing {
            // 使用索引标记进行精确匹配，处理可能的乱序问题
            self.parse_indexed_response(parts, expected_count)
        } else {
            // 简单模式：依赖顺序保持，直接返回分割结果
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
    ///
    /// 处理包含索引标记的翻译结果，确保每个译文能正确对应到原始文本。
    /// 这个方法是处理批量翻译顺序问题的核心机制。
    ///
    /// # 参数 
    /// - `parts`: 按分隔符分割的翻译结果片段
    /// - `expected_count`: 期望的结果数量
    ///
    /// # 返回值
    /// 返回按原始顺序重排的翻译结果向量
    ///
    /// # 索引格式
    /// 期望的格式为 "[数字]译文内容"，例如 "[0]Hello world"
    ///
    /// # 错误恢复
    /// - 无法解析的索引项将被忽略
    /// - 缺失的索引位置用空字符串填充
    /// - 记录缺失翻译的统计信息用于监控
    fn parse_indexed_response(
        &mut self,
        parts: Vec<String>,
        expected_count: usize,
    ) -> TranslationResult<Vec<String>> {
        // 正则表达式匹配索引标记格式: [数字]内容
        let index_regex = Regex::new(r"^\[(\d+)\](.*)$").unwrap();
        let mut indexed_results = HashMap::new();

        // 解析每个翻译片段，提取索引和对应的译文
        for part in parts {
            if let Some(captures) = index_regex.captures(&part) {
                if let (Some(index_str), Some(text)) = (captures.get(1), captures.get(2)) {
                    if let Ok(index) = index_str.as_str().parse::<usize>() {
                        indexed_results.insert(index, text.as_str().to_string());
                    }
                }
            }
            // 无法匹配索引格式的片段将被静默忽略
            // 这样可以容忍翻译服务返回的格式不完全符合预期的情况
        }

        // 按原始顺序重建结果向量，确保索引与原文本位置一致
        let mut results = Vec::with_capacity(expected_count);
        for i in 0..expected_count {
            if let Some(translated) = indexed_results.get(&i) {
                results.push(translated.clone());
            } else {
                // 如果找不到对应索引的翻译，使用空字符串作为占位
                // 这比跳过更安全，因为保持了向量长度的一致性
                results.push(String::new());
                self.stats.inc_missing_translations(); // 记录缺失翻译用于监控
            }
        }

        Ok(results)
    }

    /// 更新DOM
    ///
    /// 将翻译结果应用到HTML DOM树中，根据不同的文本类型采用相应的更新策略。
    /// 这是翻译流程的最后一步，确保译文正确显示在网页中。
    ///
    /// # 参数
    /// - `dom`: 要更新的HTML DOM树
    /// - `batch`: 包含原始文本信息和DOM节点引用的批次
    /// - `translated_texts`: 与批次项一一对应的翻译结果
    ///
    /// # 返回值
    /// 返回更新后的DOM树
    ///
    /// # 错误
    /// - 当批次项数量与翻译结果数量不匹配时返回内部错误
    ///
    /// # 更新策略
    /// - 文本内容类型：直接替换节点的文本内容
    /// - 属性类型：更新指定属性的值（如 alt、title 等）
    /// - 空翻译：跳过更新，保持原文内容
    fn update_dom(
        &self,
        dom: RcDom,
        batch: &Batch,
        translated_texts: Vec<String>,
    ) -> TranslationResult<RcDom> {
        // 验证数据一致性，确保每个文本项都有对应的翻译结果
        if batch.items.len() != translated_texts.len() {
            return Err(TranslationError::InternalError(format!(
                "文本项数量与翻译结果数量不匹配: {} vs {}",
                batch.items.len(),
                translated_texts.len()
            )));
        }

        // 逐项更新DOM节点
        for (item, translated) in batch.items.iter().zip(translated_texts.iter()) {
            // 跳过空的翻译结果，保持原文不变
            if translated.trim().is_empty() {
                continue;
            }

            // 根据文本类型选择合适的更新方法
            match &item.text_type {
                // 文本内容类型：更新节点的文本内容
                crate::translation::pipeline::collector::TextType::Content
                | crate::translation::pipeline::collector::TextType::Title
                | crate::translation::pipeline::collector::TextType::Link
                | crate::translation::pipeline::collector::TextType::Button
                | crate::translation::pipeline::collector::TextType::FormLabel
                | crate::translation::pipeline::collector::TextType::ImageAlt
                | crate::translation::pipeline::collector::TextType::Tooltip => {
                    self.update_text_content(&item.node, translated)?;
                }
                // 属性类型：更新指定属性的值
                crate::translation::pipeline::collector::TextType::Attribute(attr_name) => {
                    self.update_attribute_content(&item.node, attr_name, translated)?;
                }
            }
        }

        Ok(dom)
    }

    /// 更新文本内容
    ///
    /// 更新DOM文本节点的内容，将原文替换为翻译后的文本。
    ///
    /// # 参数
    /// - `node`: 要更新的DOM节点句柄
    /// - `translated`: 翻译后的文本内容
    ///
    /// # 返回值
    /// 成功时返回 `Ok(())`
    ///
    /// # 实现细节
    /// 只有当节点类型为 `NodeData::Text` 时才会进行更新，
    /// 其他类型的节点会被静默忽略，确保操作的安全性。
    fn update_text_content(
        &self,
        node: &markup5ever_rcdom::Handle,
        translated: &str,
    ) -> TranslationResult<()> {
        if let NodeData::Text { contents } = &node.data {
            let mut contents = contents.borrow_mut();
            *contents = translated.into(); // 使用 Into trait 进行类型转换
        }
        // 非文本节点不需要处理，静默忽略
        Ok(())
    }

    /// 更新属性内容
    ///
    /// 更新DOM元素节点的指定属性值，常用于 alt、title、placeholder 等属性。
    ///
    /// # 参数
    /// - `node`: 要更新的DOM节点句柄
    /// - `attr_name`: 要更新的属性名称
    /// - `translated`: 翻译后的属性值
    ///
    /// # 返回值
    /// 成功时返回 `Ok(())`
    ///
    /// # 使用场景
    /// - 图片的 alt 属性翻译
    /// - 表单元素的 placeholder 翻译
    /// - 链接的 title 属性翻译
    fn update_attribute_content(
        &self,
        node: &markup5ever_rcdom::Handle,
        attr_name: &str,
        translated: &str,
    ) -> TranslationResult<()> {
        // 使用辅助函数设置节点属性，处理底层DOM操作细节
        set_node_attr(node, attr_name, Some(translated.to_string()));
        Ok(())
    }

    /// 获取统计信息
    ///
    /// 返回当前引擎的性能统计信息，用于监控和调试。
    ///
    /// # 返回值
    /// 返回统计信息的不可变引用，包含以下指标：
    /// - 处理的批次数量
    /// - 发送和接收的字符数
    /// - 翻译错误次数
    /// - 缺失翻译次数
    /// - 总处理时间
    pub fn get_stats(&self) -> &EngineStats {
        &self.stats
    }

    /// 重置统计信息
    ///
    /// 将所有统计计数器重置为零，通常在开始新的翻译会话时调用。
    ///
    /// # 使用场景
    /// - 开始新的翻译任务时清零计数器
    /// - 定期重置统计信息进行阶段性分析
    /// - 测试环境中确保统计数据的独立性
    pub fn reset_stats(&mut self) {
        self.stats.reset();
    }
}

/// 翻译引擎配置
///
/// 定义翻译引擎的行为参数，包括重试策略、性能优化和错误处理设置。
/// 配置支持克隆，可以在多个引擎实例间共享。
///
/// # 字段说明
/// - `use_indexing`: 是否启用索引标记系统，用于确保批量翻译的顺序正确性
/// - `max_retry_attempts`: 翻译失败时的最大重试次数
/// - `retry_delay_ms`: 重试的基础延迟时间（毫秒），实际延迟会按指数递增
/// - `max_concurrent_requests`: 最大并发请求数量（当前版本暂未使用）
#[derive(Debug, Clone)]
pub struct EngineConfig {
    /// 是否启用索引标记功能
    /// 
    /// 启用后会在翻译文本前添加 [index] 标记，确保批量翻译时
    /// 译文能正确对应到原文，适用于翻译服务可能改变文本顺序的情况
    pub use_indexing: bool,
    
    /// 翻译失败时的最大重试次数
    /// 
    /// 用于处理网络错误、服务临时不可用等临时性问题
    /// 设置为 0 表示不重试，建议值为 3-5 次
    pub max_retry_attempts: usize,
    
    /// 重试的基础延迟时间（毫秒）
    /// 
    /// 实际延迟时间 = retry_delay_ms * 2^attempt_number
    /// 这实现了指数退避策略，避免过度占用服务资源
    pub retry_delay_ms: u64,
    
    /// 最大并发请求数量
    /// 
    /// 预留字段，用于将来实现并发翻译处理
    /// 当前版本串行处理所有请求
    pub max_concurrent_requests: usize,
}

impl Default for EngineConfig {
    /// 创建默认配置
    ///
    /// 提供经过测试的默认参数值，适用于大多数使用场景：
    /// - 启用索引标记确保翻译准确性
    /// - 3次重试处理临时错误
    /// - 1秒基础延迟实现合理的退避策略
    /// - 5个并发连接为将来的并发处理预留
    fn default() -> Self {
        Self {
            use_indexing: true,         // 默认启用索引标记
            max_retry_attempts: 3,      // 重试3次，平衡可靠性和响应时间
            retry_delay_ms: 1000,       // 1秒基础延迟，避免过于频繁的重试
            max_concurrent_requests: 5, // 预留的并发数量
        }
    }
}

/// 翻译引擎统计信息（线程安全版本）
///
/// 使用原子操作记录翻译引擎的性能指标和错误信息，支持多线程环境下的安全访问。
/// 所有计数器都是累加的，提供引擎运行期间的完整统计数据。
///
/// # 线程安全性
/// 所有字段都使用原子类型，可以在多线程环境中安全地读取和更新，
/// 无需额外的同步机制。
///
/// # 用途
/// - 性能监控：跟踪处理速度和资源使用情况
/// - 错误分析：统计各类错误的发生频率
/// - 质量评估：监控翻译成功率和完整性
#[derive(Debug, Default)]
pub struct EngineStats {
    /// 已处理的翻译批次总数
    ///
    /// 每个批次可能包含多个文本项，此计数器记录调用
    /// `process_batch` 方法成功完成的次数
    pub batches_processed: AtomicUsize,
    
    /// 发送给翻译服务的字符总数
    ///
    /// 包括所有文本内容和索引标记，用于计算翻译成本
    /// 和网络流量统计
    pub characters_sent: AtomicUsize,
    
    /// 从翻译服务接收的字符总数
    /// 
    /// 包括翻译结果中的所有字符，可用于计算翻译质量
    /// （接收字符数与发送字符数的比率）
    pub characters_received: AtomicUsize,
    
    /// 翻译服务调用失败次数
    ///
    /// 包括网络错误、服务错误、超时等各种失败情况，
    /// 不包括重试成功的情况
    pub translation_errors: AtomicUsize,
    
    /// 缺失翻译项的数量
    ///
    /// 当使用索引标记时，如果某些索引在翻译结果中找不到
    /// 对应的译文，会增加此计数器
    pub missing_translations: AtomicUsize,
    
    /// 总处理时间（微秒）
    ///
    /// 记录所有翻译操作的累计耗时，包括网络请求时间、
    /// 文本处理时间和DOM更新时间
    pub total_processing_time: std::sync::atomic::AtomicU64,
}

impl EngineStats {
    /// 增加处理批次计数
    ///
    /// 每当成功完成一个批次的翻译处理时调用。
    /// 使用 Relaxed 内存顺序，因为计数器的精确顺序不重要。
    pub fn inc_batches_processed(&self) {
        self.batches_processed.fetch_add(1, Ordering::Relaxed);
    }

    /// 添加发送字符数
    ///
    /// 记录向翻译服务发送的字符数量，用于流量统计。
    ///
    /// # 参数
    /// - `count`: 本次发送的字符数量
    pub fn add_characters_sent(&self, count: usize) {
        self.characters_sent.fetch_add(count, Ordering::Relaxed);
    }

    /// 添加接收字符数
    ///
    /// 记录从翻译服务接收的字符数量，用于质量分析。
    ///
    /// # 参数
    /// - `count`: 本次接收的字符数量
    pub fn add_characters_received(&self, count: usize) {
        self.characters_received.fetch_add(count, Ordering::Relaxed);
    }

    /// 增加翻译错误计数
    ///
    /// 当翻译服务调用失败时调用，包括网络错误、服务错误等。
    /// 不计算重试成功的情况，只记录最终失败的调用。
    pub fn inc_translation_errors(&self) {
        self.translation_errors.fetch_add(1, Ordering::Relaxed);
    }

    /// 增加缺失翻译计数
    ///
    /// 当使用索引标记时，如果翻译结果中缺少某些索引的译文，
    /// 会调用此方法记录缺失的数量。
    pub fn inc_missing_translations(&self) {
        self.missing_translations.fetch_add(1, Ordering::Relaxed);
    }

    /// 添加处理时间
    ///
    /// 累加翻译操作的耗时，用于性能分析和优化。
    ///
    /// # 参数
    /// - `duration`: 本次操作的持续时间
    ///
    /// # 实现细节
    /// 时间以微秒为单位存储，提供足够的精度用于性能分析
    pub fn add_processing_time(&self, duration: std::time::Duration) {
        self.total_processing_time
            .fetch_add(duration.as_micros() as u64, Ordering::Relaxed);
    }
}

impl EngineStats {
    /// 重置统计信息
    ///
    /// 将所有统计计数器重置为零，创建一个全新的统计状态。
    /// 通常在开始新的翻译会话或需要清零计数器时调用。
    ///
    /// # 使用场景
    /// - 开始新的翻译任务时清理历史数据
    /// - 定期重置统计信息进行阶段性分析
    /// - 测试环境中确保每次测试的独立性
    ///
    /// # 实现细节
    /// 通过赋值默认值来重置所有原子计数器，这比逐个调用 store(0) 更简洁
    pub fn reset(&mut self) {
        *self = Default::default();
    }
}

/// 非translation特性的占位实现
///
/// 当未启用 "translation" 特性时，提供空的占位结构体，
/// 确保代码在不同特性配置下都能正常编译。
#[cfg(not(feature = "translation"))]
pub struct TranslationEngine;

#[cfg(not(feature = "translation"))]
impl TranslationEngine {
    /// 占位构造函数
    ///
    /// 当未启用翻译特性时，创建一个空的引擎实例。
    /// 参数被忽略，因为没有实际的翻译功能需要初始化。
    pub fn new(_service: (), _config: EngineConfig) -> Self {
        Self
    }
}

/// 未启用翻译特性时的配置类型别名
///
/// 使用空元组作为占位类型，保持API的一致性
#[cfg(not(feature = "translation"))]
pub type EngineConfig = ();

/// 未启用翻译特性时的统计类型别名
///
/// 使用空元组作为占位类型，保持API的一致性
#[cfg(not(feature = "translation"))]
pub type EngineStats = ();
