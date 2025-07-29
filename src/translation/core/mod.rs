//! 翻译系统核心模块
//!
//! 本模块是整个翻译系统的核心部分，提供了完整的DOM翻译解决方案。
//! 它整合了文本收集、批次处理、翻译执行、错误恢复等功能，为上层应用
//! 提供简洁易用的翻译服务接口。
//!
//! ## 架构设计
//!
//! 核心模块采用分层架构设计：
//!
//! - **服务层** (`service.rs`): 提供统一的翻译服务接口，协调各个子系统
//! - **引擎层** (`engine.rs`): 处理底层翻译逻辑，包括批次处理和DOM更新
//!
//! ## 主要特性
//!
//! - **批量处理**: 智能文本分组，优化翻译API调用效率
//! - **错误恢复**: 内建重试机制和错误处理策略
//! - **索引标记**: 确保翻译结果与原文本的正确对应
//! - **性能统计**: 详细的性能指标和运行时统计
//! - **健康监控**: 系统组件健康状态检查
//! - **线程安全**: 原子操作保证多线程环境下的数据一致性
//!
//! ## 使用示例
//!
//! ```rust
//! use crate::translation::core::TranslationService;
//! use crate::translation::config::TranslationConfig;
//!
//! // 创建翻译服务
//! let config = TranslationConfig::default_with_lang("zh", Some("http://localhost:1188"));
//! let mut service = TranslationService::new(config)?;
//!
//! // 翻译DOM内容
//! let translated_dom = service.translate_dom(dom).await?;
//!
//! // 获取统计信息
//! let stats = service.get_stats();
//! println!("处理了 {} 个文本", stats.texts_collected.load(Ordering::Relaxed));
//! ```
//!
//! ## 模块依赖关系
//!
//! ```text
//! TranslationService (service.rs)
//!     ├── TextCollector (pipeline/collector.rs)
//!     ├── BatchManager (pipeline/batch.rs)  
//!     ├── CacheManager (storage/cache.rs)
//!     ├── ConfigManager (config/manager.rs)
//!     └── TranslationProcessor (processor.rs)
//!             └── TranslationEngine (engine.rs)
//! ```

pub mod engine;
pub mod service;

// 重新导出核心类型和接口
// 这些是外部模块最常用的类型，通过重新导出简化使用

/// 翻译引擎 - 处理底层翻译逻辑
pub use engine::TranslationEngine;

/// 统一翻译服务 - 主要的对外接口
/// 
/// 这是使用翻译功能的主要入口点，提供了完整的DOM翻译能力
pub use service::TranslationService;

/// 服务运行统计信息
/// 
/// 提供翻译过程中的各项性能指标和运行数据
pub use service::ServiceStats;

/// 系统健康状态检查结果
/// 
/// 用于监控翻译系统各组件的运行状态
pub use service::HealthStatus;

/// 健康状态级别枚举
/// 
/// 定义了系统组件的三种健康状态：健康、降级、不健康
pub use service::HealthLevel;
