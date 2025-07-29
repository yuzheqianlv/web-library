//! HTML解析和处理模块
//!
//! 这个模块提供了完整的HTML文档解析、处理和序列化功能，专门用于创建单体HTML文件。
//! 模块被重构为多个子模块，提供更好的组织结构和可维护性：
//!
//! # 子模块说明
//!
//! - [`utils`]: 基础工具函数和常量定义，包含CSP策略生成、图标识别等
//! - [`parser`]: HTML解析器类型定义和核心解析功能
//! - [`dom`]: 基础DOM操作，包括节点查找、属性操作等
//! - [`metadata`]: 文档元数据处理，如标题、编码、robots标签等
//! - [`serializer`]: HTML文档序列化功能，将DOM转换回HTML字符串
//! - [`assets`]: 静态资源嵌入和处理，包括图片、CSS、JS等资源的内联化
//! - [`walker`]: DOM遍历核心逻辑，用于递归处理DOM树中的所有节点
//! - [`dom_walker`]: DOM遍历器的实现
//! - [`element_handlers`]: 各种HTML元素的处理器
//! - [`complex_element_handlers`]: 复杂HTML元素的专门处理器
//!
//! # 使用场景
//!
//! 此模块主要用于：
//! - 解析HTML文档并构建DOM树
//! - 嵌入外部资源（CSS、JS、图片等）到HTML中
//! - 处理文档元数据和配置
//! - 生成自包含的单体HTML文件
//!
//! # 示例
//!
//! ```rust
//! use monolith::parsers::html::{html_to_dom, serialize_document, walk};
//!
//! // 解析HTML字符串为DOM
//! let html = "<html><body><h1>Hello</h1></body></html>";
//! let dom = html_to_dom(html, None);
//!
//! // 处理DOM树，嵌入资源等
//! // walk(&mut dom, ...);
//!
//! // 序列化回HTML字符串
//! let result = serialize_document(&dom, false, false);
//! ```

/// 静态资源处理模块
///
/// 提供HTML文档中静态资源（图片、CSS、JS等）的嵌入和处理功能
pub mod assets;

/// 复杂HTML元素处理器模块
///
/// 为特定的复杂HTML元素（如表单、媒体元素等）提供专门的处理逻辑
pub mod complex_element_handlers;

/// DOM基础操作模块
///
/// 提供DOM树的基本操作功能，包括节点查找、属性读写等核心功能
pub mod dom;

/// DOM遍历器模块
///
/// 实现DOM树的遍历算法和遍历器结构
pub mod dom_walker;

/// HTML元素处理器模块
///
/// 为各种HTML元素提供处理逻辑，实现元素转换和资源嵌入
pub mod element_handlers;

/// 文档元数据处理模块
///
/// 处理HTML文档的元数据，如标题、编码、robots设置等
pub mod metadata;

/// HTML解析器模块
///
/// 提供HTML字符串解析功能和相关类型定义
pub mod parser;

/// HTML序列化模块
///
/// 将DOM树序列化回HTML字符串的功能实现
pub mod serializer;

/// 工具函数模块
///
/// 提供各种辅助工具函数和常量定义
pub mod utils;

/// DOM遍历核心模块
///
/// 实现DOM树的递归遍历和节点处理的核心逻辑
pub mod walker;

// 重新导出主要的公共 API 以保持向后兼容性

// ============================================================================
// 资源处理相关API
// ============================================================================

/// 向HTML文档添加favicon图标
///
/// 将指定的favicon资源嵌入到HTML文档的head部分
pub use assets::add_favicon;

/// 检查资源的完整性
///
/// 验证嵌入资源的完整性哈希值，确保资源未被篡改
pub use assets::check_integrity;

/// 嵌入srcset属性中的资源
///
/// 处理响应式图片的srcset属性，将其中引用的所有图片资源嵌入到HTML中
pub use assets::embed_srcset;

/// 检索并嵌入静态资源
///
/// 从网络或本地获取资源（如图片、CSS、JS文件），并将其内容嵌入到HTML中
pub use assets::retrieve_and_embed_asset;

// ============================================================================
// DOM操作相关API
// ============================================================================

/// 根据条件查找DOM节点
///
/// 在DOM树中搜索满足特定条件的节点集合
pub use dom::find_nodes;

/// 根据名称获取子节点
///
/// 在指定父节点下查找具有特定标签名的子节点
pub use dom::get_child_node_by_name;

/// 获取节点属性值
///
/// 从DOM节点中读取指定属性的值
pub use dom::get_node_attr;

/// 获取节点名称
///
/// 返回DOM节点的标签名（如"div"、"img"等）
pub use dom::get_node_name;

/// 获取父节点
///
/// 返回指定节点的父节点引用
pub use dom::get_parent_node;

/// 将HTML字符串解析为DOM树
///
/// 这是核心的HTML解析函数，将HTML字符串转换为可操作的DOM结构
pub use dom::html_to_dom;

/// 设置节点属性
///
/// 为DOM节点设置或更新指定属性的值
pub use dom::set_node_attr;

// ============================================================================
// 元数据处理相关API
// ============================================================================

/// 创建元数据标签
///
/// 生成HTML meta标签，用于设置文档元数据
pub use metadata::create_metadata_tag;

/// 获取文档基础URL
///
/// 从HTML文档中提取base标签的href属性值
pub use metadata::get_base_url;

/// 获取文档字符编码
///
/// 从meta标签中读取文档的字符编码设置
pub use metadata::get_charset;

/// 获取robots元数据
///
/// 读取meta robots标签的内容，用于搜索引擎爬虫指导
pub use metadata::get_robots;

/// 获取文档标题
///
/// 从title标签中提取文档标题文本
pub use metadata::get_title;

/// 检查是否存在favicon
///
/// 判断HTML文档中是否已经包含favicon相关的link标签
pub use metadata::has_favicon;

/// 设置文档基础URL
///
/// 添加或更新HTML文档的base标签
pub use metadata::set_base_url;

/// 设置文档字符编码
///
/// 添加或更新文档的charset meta标签
pub use metadata::set_charset;

/// 设置robots元数据
///
/// 添加或更新meta robots标签
pub use metadata::set_robots;

// ============================================================================
// HTML解析器相关API
// ============================================================================

/// 解析链接类型
///
/// 从link标签的rel属性中解析出链接类型枚举
pub use parser::parse_link_type;

/// 解析srcset属性
///
/// 将图片的srcset属性字符串解析为结构化的源集合
pub use parser::parse_srcset;

/// 链接类型枚举
///
/// 表示HTML link标签的不同rel属性值（如stylesheet、icon等）
pub use parser::LinkType;

/// srcset项目结构
///
/// 表示srcset属性中的单个图片源，包含URL和描述符
pub use parser::SrcSetItem;

// ============================================================================
// 序列化相关API
// ============================================================================

/// 序列化DOM文档
///
/// 将处理后的DOM树转换回HTML字符串形式
pub use serializer::serialize_document;

// ============================================================================
// 工具函数相关API
// ============================================================================

/// 组合CSP策略
///
/// 根据配置生成Content Security Policy头部字符串
pub use utils::compose_csp;

/// 判断是否为favicon
///
/// 检查给定的链接或文件名是否为favicon图标
pub use utils::is_favicon;

/// favicon文件名常量集合
///
/// 包含常见的favicon文件名的字符串数组
pub use utils::FAVICON_VALUES;

/// 空白字符常量集合
///
/// 定义HTML中的空白字符集合，用于解析和清理
pub use utils::WHITESPACES;

// ============================================================================
// DOM遍历相关API
// ============================================================================

/// DOM树遍历处理函数
///
/// 递归遍历DOM树中的所有节点，对每个节点执行相应的处理操作。
/// 这是monolith工具的核心功能，用于：
/// - 嵌入外部资源
/// - 清理和优化HTML结构
/// - 应用各种转换规则
pub use walker::walk;
