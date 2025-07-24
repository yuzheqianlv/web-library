//! # Library模块
//!
//! 提供完整的库管理功能，包括：
//!
//! - 域名和URL的缓存管理
//! - 库数据的查询、分页和排序
//! - 缓存操作（删除、刷新、清理）
//! - 统计信息和分析
//!
//! ## 模块组织
//!
//! - `types` - 数据类型定义
//! - `service` - 业务逻辑服务层
//! - `operations` - 库操作处理
//! - `handlers` - HTTP路由处理器
//!
//! ## 主要功能
//!
//! ### 数据查询
//! - 分页获取域名列表
//! - 按域名、缓存数量、更新时间等排序
//! - 域名详情查看
//! - 库统计信息
//!
//! ### 缓存管理
//! - 删除整个域名的缓存
//! - 删除单个URL的缓存
//! - 批量删除操作
//! - 清理过期缓存
//!
//! ### 数据分析
//! - 缓存大小统计
//! - 域名使用分析
//! - 过期数据统计
//!
//! ## 使用示例
//!
//! ```rust,no_run
//! use monolith::web::library::{LibraryService, LibraryRequest};
//!
//! // 获取库数据
//! let request = LibraryRequest {
//!     domain: None,
//!     page: Some(1),
//!     limit: Some(20),
//!     sort_by: Some("cache_count".to_string()),
//!     sort_order: Some("desc".to_string()),
//! };
//!
//! // let service = LibraryService::new(redis_cache);
//! // let response = service.get_library_data(&request)?;
//! ```

pub mod handlers;
pub mod operations;
pub mod service;
pub mod types;

// V2 API modules
pub mod cache;
pub mod stats_handlers;
pub mod v2_handlers;
pub mod v2_service;
pub mod v2_types;

// Re-export常用类型和函数以便使用
pub use handlers::*;
pub use operations::LibraryOperations;
pub use service::LibraryService;
pub use types::*;

// V2 API re-exports
pub use stats_handlers::*;
pub use v2_handlers::*;
pub use v2_service::LibraryServiceV2;
pub use v2_types::*;
