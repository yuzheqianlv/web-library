//! 存储模块
//!
//! 提供缓存和持久化存储功能。

pub mod cache;

pub use cache::{TranslationCache, CacheStats, CacheEntry, CacheConfig, CacheManager, create_default_cache};