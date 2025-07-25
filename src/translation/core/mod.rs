//! 翻译核心功能模块
//!
//! 提供统一的翻译服务接口和核心翻译逻辑

pub mod service;
pub mod engine;

// 重新导出主要类型
pub use service::{TranslationService, ServiceStats, HealthStatus, HealthLevel};
pub use engine::TranslationEngine;