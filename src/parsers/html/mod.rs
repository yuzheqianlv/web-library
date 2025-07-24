//! HTML解析和处理模块
//!
//! 这个模块被重构为多个子模块，提供更好的组织结构和可维护性：
//!
//! - `utils`: 基础工具函数和常量
//! - `parser`: 类型定义和解析功能
//! - `dom`: 基础DOM操作
//! - `metadata`: 文档元数据处理
//! - `serializer`: 序列化功能
//! - `assets`: 资源嵌入和处理
//! - `walker`: DOM遍历核心逻辑

pub mod assets;
pub mod complex_element_handlers;
pub mod dom;
pub mod dom_walker;
pub mod element_handlers;
pub mod metadata;
pub mod parser;
pub mod serializer;
pub mod utils;
pub mod walker;

// 重新导出主要的公共 API 以保持向后兼容性
pub use assets::{add_favicon, check_integrity, embed_srcset, retrieve_and_embed_asset};
pub use dom::{
    find_nodes, get_child_node_by_name, get_node_attr, get_node_name, get_parent_node, html_to_dom,
    set_node_attr,
};
pub use metadata::{
    create_metadata_tag, get_base_url, get_charset, get_robots, get_title, has_favicon,
    set_base_url, set_charset, set_robots,
};
pub use parser::{parse_link_type, parse_srcset, LinkType, SrcSetItem};
pub use serializer::serialize_document;
pub use utils::{compose_csp, is_favicon, FAVICON_VALUES, WHITESPACES};
pub use walker::walk;
