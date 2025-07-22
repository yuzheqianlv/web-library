//! # 解析器模块
//!
//! 这个模块包含所有用于解析和处理不同类型web资源的功能：
//!
//! - HTML解析和DOM操作
//! - CSS解析和样式处理
//! - JavaScript处理和事件处理器
//!
//! # 模块组织
//!
//! - `html` - HTML文档解析、DOM操作、元数据处理
//! - `css` - CSS样式表解析、URL处理、样式嵌入
//! - `js` - JavaScript事件处理器识别和处理

pub mod css;
pub mod html;
pub mod js;

// Re-export commonly used items for convenience
pub use css::embed_css;
pub use html::{
    add_favicon, create_metadata_tag, get_base_url, get_charset, get_robots, get_title,
    has_favicon, html_to_dom, serialize_document, set_base_url, set_charset, set_robots, walk,
};
pub use js::attr_is_event_handler;
