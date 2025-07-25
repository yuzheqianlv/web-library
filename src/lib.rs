//! # Monolith Library
//!
//! 一个强大的工具库，用于将网页保存为单一的HTML文件，包含所有必要的资源。
//!
//! ## 模块组织
//!
//! - `core` - 核心功能和主要处理逻辑
//! - `parsers` - 资源解析器（HTML、CSS、JavaScript）
//! - `network` - 网络通信和缓存系统
//! - `utils` - 工具函数和实用程序
//! - `builders` - 输出格式构建器
//! - `translation` - 翻译功能（可选）
//! - `web` - Web服务器功能（可选）

#[cfg(feature = "web")]
pub mod builders;
pub mod core;
pub mod env;
pub mod network;
pub mod parsers;
#[cfg(feature = "translation")]
pub mod translation;
pub mod utils;
#[cfg(feature = "web")]
pub mod web;

// Re-export commonly used items for convenience
pub use core::*;
pub use network::*;
pub use parsers::*;
pub use utils::*;
