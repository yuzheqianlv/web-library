//! # 网络模块
//!
//! 这个模块包含所有与网络通信、缓存和会话管理相关的功能：
//!
//! - HTTP会话管理和资源下载
//! - 本地和Redis缓存系统
//! - Cookie处理和管理
//!
//! # 模块组织
//!
//! - `session` - HTTP会话管理、请求处理、资源下载
//! - `cache` - 本地文件缓存系统
//! - `cookies` - Cookie解析和管理

pub mod cache;
pub mod cookies;
pub mod session;

// Re-export commonly used items for convenience
pub use cache::Cache;
pub use cookies::{parse_cookie_file_contents, Cookie};
pub use session::Session;
