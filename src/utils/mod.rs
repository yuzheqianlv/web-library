//! # 工具模块
//!
//! 这个模块包含各种工具函数和实用程序：
//!
//! - URL处理和解析工具
//! - 数据URL创建和解析
//! - URL清理和规范化
//!
//! # 模块组织
//!
//! - `url` - URL处理、数据URL、域名验证等工具函数

pub mod url;

// Re-export commonly used items for convenience
pub use url::{
    clean_url, create_data_url, domain_is_within_domain, get_referer_url, is_url_and_has_protocol,
    parse_data_url, resolve_url, Url, EMPTY_IMAGE_DATA_URL,
};
