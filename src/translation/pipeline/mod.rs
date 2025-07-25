//! 翻译管道模块
//!
//! 提供文本处理管道，包括收集、过滤和批次处理

pub mod collector;
pub mod filters;
pub mod batch;

// 重新导出主要类型
pub use collector::{TextCollector, TextItem, TextType, TextPriority};
pub use filters::{TextFilter, TextAnalysis, LanguageHint};
pub use batch::{BatchManager, Batch, BatchType, BatchPriority};