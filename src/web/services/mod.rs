//! Web服务层模块
//!
//! 提供各种业务逻辑服务，包括缓存查询、智能路由等功能

#[cfg(feature = "web")]
pub mod cache_query_engine;

#[cfg(feature = "web")]
pub use cache_query_engine::*;

#[cfg(not(feature = "web"))]
pub struct CacheQueryEngine;