//! # 解析器模块
//!
//! 这个模块包含所有用于解析和处理不同类型web资源的核心功能，是 Monolith 项目的核心组件之一。
//! 它提供了全面的web内容解析、处理和转换能力，支持将复杂的web页面转换为单一的HTML文件。
//!
//! ## 核心功能
//!
//! - **HTML解析和DOM操作** - 基于 html5ever 的完整HTML解析功能
//! - **CSS解析和样式处理** - CSS内容嵌入、URL重写和样式优化
//! - **JavaScript处理** - 事件处理器识别和JavaScript代码处理
//! - **链接重写和URL转换** - 智能链接重写，支持翻译服务集成
//!
//! ## 模块组织
//!
//! - [`html`] - HTML文档解析、DOM操作、元数据处理和资源嵌入
//! - [`css`] - CSS样式表解析、URL处理、资源嵌入和样式优化
//! - [`js`] - JavaScript事件处理器识别和DOM事件处理
//! - [`link_rewriter`] - 链接重写功能，将HTML中的链接转换为翻译服务链接
//!
//! ## 使用示例
//!
//! ```rust
//! use monolith::parsers::{html_to_dom, serialize_document, embed_css};
//! use url::Url;
//!
//! // 解析HTML文档
//! let html = "<html><head><title>测试</title></head><body>内容</body></html>";
//! let document_url = Url::parse("https://example.com").unwrap();
//! let dom = html_to_dom(html, document_url.as_str()).unwrap();
//!
//! // 序列化文档
//! let result = serialize_document(&dom);
//! ```
//!
//! ## 设计理念
//!
//! 本模块采用模块化设计，每个子模块负责特定类型的内容处理：
//! - 通过统一的接口提供不同类型资源的处理能力
//! - 支持递归处理复杂的嵌套结构（如CSS中的@import规则）
//! - 提供灵活的配置选项以适应不同的使用场景
//! - 保持与原始内容的高度兼容性

/// CSS解析和处理模块
///
/// 提供CSS样式表的解析、资源嵌入和URL重写功能。
/// 支持处理CSS中的各种资源引用，包括图片、字体等。
pub mod css;

/// HTML解析和DOM操作模块
///
/// 基于html5ever提供完整的HTML解析和DOM操作功能。
/// 包含子模块用于处理不同方面的HTML操作。
pub mod html;

/// JavaScript事件处理器模块
///
/// 提供JavaScript事件处理器的识别和处理功能。
/// 主要用于识别HTML元素上的事件处理器属性。
pub mod js;

/// 链接重写模块
///
/// 提供HTML文档中链接的智能重写功能。
/// 支持将普通链接转换为翻译服务链接。
pub mod link_rewriter;

// ================================================================================================
// 公共API重新导出
// ================================================================================================
//
// 为了提供更好的用户体验，我们将各个子模块中最常用的功能重新导出到根模块级别。
// 这样用户可以直接使用 `use monolith::parsers::function_name` 而不需要深入到具体的子模块。
//
// 重新导出策略：
// 1. 导出核心处理函数（如 html_to_dom, serialize_document, embed_css）
// 2. 导出常用的元数据操作函数（如 get_title, set_charset 等）
// 3. 导出实用工具函数（如 attr_is_event_handler）
// 4. 保持子模块的完整功能，高级用户仍可直接访问子模块
//
// 这种设计平衡了易用性和功能完整性，既方便日常使用，又保留了高级功能的访问能力。

/// 嵌入CSS样式表到HTML文档中
///
/// 将外部CSS文件的内容嵌入到HTML文档中，并处理其中的资源引用。
///
/// # 参数
///
/// * `session` - 网络会话，用于下载外部资源
/// * `document_url` - 文档的基础URL，用于解析相对路径
/// * `css` - CSS样式表内容
///
/// # 返回值
///
/// 返回处理后的CSS字符串，所有外部资源已嵌入为data URL
pub use css::embed_css;

// HTML模块的重新导出 - 提供DOM操作和元数据处理功能
pub use html::{
    add_favicon,         // 向HTML文档添加favicon图标
    create_metadata_tag, // 创建META标签元素
    get_base_url,        // 获取HTML文档的base URL
    get_charset,         // 获取HTML文档的字符编码
    get_robots,          // 获取HTML文档的robots设置
    get_title,           // 获取HTML文档的标题
    has_favicon,         // 检查HTML文档是否已有favicon
    html_to_dom,         // 将HTML字符串解析为DOM树
    serialize_document,  // 将DOM树序列化为HTML字符串
    set_base_url,        // 设置HTML文档的base URL
    set_charset,         // 设置HTML文档的字符编码
    set_robots,          // 设置HTML文档的robots设置
    walk,                // 遍历DOM树并处理所有节点
};

/// 检查HTML属性是否为事件处理器
///
/// 判断给定的HTML属性名是否为JavaScript事件处理器属性（如onclick、onload等）。
///
/// # 参数
///
/// * `attr_name` - 要检查的属性名
///
/// # 返回值
///
/// 如果是事件处理器属性则返回true，否则返回false
///
/// # 示例
///
/// ```rust
/// use monolith::parsers::attr_is_event_handler;
///
/// assert!(attr_is_event_handler("onclick"));
/// assert!(attr_is_event_handler("onload"));
/// assert!(!attr_is_event_handler("class"));
/// ```
pub use js::attr_is_event_handler;

// 链接重写模块的重新导出 - 提供链接转换功能
pub use link_rewriter::{
    rewrite_links_in_dom,  // 重写DOM树中的所有链接
    rewrite_links_in_html, // 重写HTML字符串中的所有链接
};

// ================================================================================================
// 重新导出函数的详细文档
// ================================================================================================
//
// 由于Rust语法限制，我们无法在 `pub use` 语句中直接添加文档注释。
// 以下是对重新导出函数的详细说明：

// ## HTML相关函数
//
// ### add_favicon(document, favicon_url)
// 向HTML文档添加favicon图标。在文档头部添加favicon相关的link元素。
//
// ### create_metadata_tag(name, content) -> Handle
// 创建一个新的META标签并设置相应的属性。返回创建的META元素节点。
//
// ### get_base_url(document) -> Option<String>
// 从文档的base元素中提取基础URL，如果没有base元素则返回None。
//
// ### get_charset(document) -> Option<String>
// 从META标签中提取文档的字符编码设置。
//
// ### get_robots(document) -> Option<String>
// 从META标签中提取robots的设置值。
//
// ### get_title(document) -> String
// 从title元素中提取文档标题。如果未找到则返回空字符串。
//
// ### has_favicon(document) -> bool
// 检查文档头部是否已经包含favicon相关的link元素。
//
// ### html_to_dom(html, document_url) -> Result<RcDom, MonolithError>
// 使用html5ever解析器将HTML字符串转换为可操作的DOM树结构。
//
// ### serialize_document(document) -> String
// 将DOM树结构转换回HTML字符串格式。
//
// ### set_base_url(document, url)
// 在文档头部设置或更新base元素的href属性。
//
// ### set_charset(document, charset)
// 在文档头部设置或更新字符编码META标签。
//
// ### set_robots(document, robots)
// 在文档头部设置或更新robots META标签。
//
// ### walk(session, document, document_url, options)
// 递归遍历DOM树的所有节点，对每个节点执行相应的处理操作。
// 这是资源嵌入和链接重写的核心功能。

// ## CSS相关函数
//
// ### embed_css(session, document_url, css) -> String
// 将外部CSS文件的内容嵌入到HTML文档中，并处理其中的资源引用。
// 返回处理后的CSS字符串，所有外部资源已嵌入为data URL。

// ## JavaScript相关函数
//
// ### attr_is_event_handler(attr_name) -> bool
// 判断给定的HTML属性名是否为JavaScript事件处理器属性（如onclick、onload等）。

// ## 链接重写相关函数
//
// ### rewrite_links_in_dom(dom, base_url, translation_base_path) -> RcDom
// 遍历DOM树，将所有的链接重写为指向翻译服务的链接。
//
// ### rewrite_links_in_html(html, base_url, translation_base_path) -> String
// 解析HTML字符串，重写其中的链接，然后返回处理后的HTML。
