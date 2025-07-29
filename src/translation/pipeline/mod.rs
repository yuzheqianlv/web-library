//! 翻译管道模块
//!
//! 这个模块实现了一个完整的文本处理管道，用于处理网页内容的翻译需求。
//! 该管道遵循"收集 -> 过滤 -> 批处理 -> 翻译"的工作流程，提供高效、智能的文本处理能力。
//!
//! # 架构设计
//!
//! 翻译管道采用三层架构设计：
//!
//! ## 1. 文本收集层 (`collector`)
//! - **主要职责**: 从DOM树中提取需要翻译的文本内容和属性
//! - **核心组件**: `TextCollector` - 智能文本收集器
//! - **输出产物**: `TextItem` - 包含文本内容、位置信息、优先级等元数据的结构化对象
//! - **特色功能**:
//!   - 支持并行收集提升性能
//!   - 智能识别文本类型和优先级
//!   - 计算文本翻译复杂度权重
//!   - 提供详细的收集统计信息
//!
//! ## 2. 文本过滤层 (`filters`)
//! - **主要职责**: 智能判断文本是否需要翻译，过滤掉不适合翻译的内容
//! - **核心组件**: `TextFilter` - 多规则文本过滤器
//! - **过滤策略**:
//!   - 长度阈值过滤（过短文本）
//!   - 内容类型检测（URL、邮箱、代码片段等）
//!   - 语言识别（已翻译的中文内容）
//!   - 功能性词汇过滤（按钮文本如"OK"、"Cancel"等）
//!   - CSS选择器和技术标识符过滤
//! - **高级功能**:
//!   - 文本可翻译性评分（0.0-1.0）
//!   - 多语言提示检测
//!   - 自定义过滤规则支持
//!
//! ## 3. 批次处理层 (`batch`)
//! - **主要职责**: 将过滤后的文本组织成优化的批次，提高翻译API调用效率
//! - **核心组件**: `BatchManager` - 智能批次管理器
//! - **优化策略**:
//!   - 基于文本优先级的智能分组
//!   - 考虑复杂度权重的批次大小计算
//!   - 自适应批次合并和拆分
//!   - 并发控制和超时管理
//! - **批次类型**:
//!   - `Standard`: 标准批次（平衡大小和效率）
//!   - `Small`: 小批次（优先处理或特殊情况）
//!   - `Large`: 大批次（需要拆分的超大内容）
//!   - `Single`: 单项批次（独立处理的重要内容）
//!   - `Priority`: 优先批次（高优先级内容）
//!
//! # 工作流程
//!
//! ```text
//! DOM Tree → TextCollector → [TextItem...] → TextFilter → [FilteredTextItem...] → BatchManager → [Batch...]
//!     ↓              ↓              ↓              ↓                 ↓                    ↓
//! 网页内容      提取文本      结构化文本项     过滤无需翻译      保留可翻译文本        优化批次分组
//! ```
//!
//! # 使用示例
//!
//! ```rust
//! use crate::translation::pipeline::*;
//!
//! // 1. 创建文本收集器
//! let mut collector = TextCollector::default();
//! let text_items = collector.collect_translatable_texts(&dom_root)?;
//!
//! // 2. 创建文本过滤器
//! let filter = TextFilter::new();
//! let filtered_items: Vec<TextItem> = text_items
//!     .into_iter()
//!     .filter(|item| filter.should_translate(&item.text))
//!     .collect();
//!
//! // 3. 创建批次管理器
//! let mut batch_manager = BatchManager::default();
//! let batches = batch_manager.create_batches(filtered_items);
//!
//! // 4. 处理批次（调用翻译API）
//! for batch in batches {
//!     // 翻译批次中的文本...
//!     println!("处理批次 {}: {} 个文本项", batch.id, batch.items.len());
//! }
//! ```
//!
//! # 性能特性
//!
//! - **内存效率**: 使用引用计数和写时复制策略，最小化内存分配
//! - **并发安全**: 支持多线程环境，使用原子操作和线程安全的数据结构
//! - **可扩展性**: 支持大规模网页内容处理，智能内存管理
//! - **可配置性**: 丰富的配置选项，支持不同场景的性能调优
//!
//! # 扩展性
//!
//! - **自定义过滤规则**: `AdvancedTextFilter` 支持添加自定义过滤逻辑
//! - **自定义处理器**: `AdvancedTextCollector` 支持添加自定义文本处理器
//! - **插件化架构**: 各层组件可独立替换和扩展
//!
//! # 监控和调试
//!
//! - **统计信息**: 每个组件都提供详细的处理统计信息
//! - **性能指标**: 包括处理时间、内存使用、缓存命中率等
//! - **调试支持**: 完整的日志记录和错误追踪

pub mod batch;
pub mod collector;
pub mod filters;

// 重新导出主要类型，提供统一的API接口

/// 批次处理相关类型
/// 
/// 这些类型用于将文本项组织成优化的批次，提高翻译API的调用效率。
/// 批次管理器会根据文本的优先级、大小和复杂度智能地创建批次。
pub use batch::{Batch, BatchManager, BatchPriority, BatchType};

/// 文本收集相关类型
/// 
/// 这些类型用于从DOM树中提取和组织需要翻译的文本内容。
/// 文本收集器提供了智能的文本识别和优先级分配功能。
pub use collector::{TextCollector, TextItem, TextPriority, TextType};

/// 文本过滤相关类型
/// 
/// 这些类型用于智能判断文本是否需要翻译，过滤掉不适合翻译的内容。
/// 过滤器支持多种过滤策略和自定义规则。
pub use filters::{LanguageHint, TextAnalysis, TextFilter};
