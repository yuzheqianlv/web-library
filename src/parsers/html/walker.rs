//! DOM遍历器模块
//!
//! 这个模块提供了 DOM 树遍历的主要入口点，用于处理 HTML 文档中的所有节点。
//! 它使用模块化的架构，将复杂的 DOM 处理逻辑委托给专门的处理器系统。
//!
//! # 主要功能
//!
//! - **DOM 遍历**: 递归遍历整个 DOM 树结构
//! - **元素处理**: 针对不同 HTML 元素类型调用相应的处理器
//! - **资源嵌入**: 处理和嵌入各种类型的外部资源（CSS、JS、图片等）
//! - **属性处理**: 处理元素的样式属性和 JavaScript 事件属性
//! - **配置选项**: 根据会话配置决定是否保留 CSS 和 JavaScript
//!
//! # 架构设计
//!
//! 该模块采用了分层架构设计：
//! - `walk()` 函数作为公共 API 入口
//! - `DomWalker` 提供实际的遍历实现
//! - 元素处理器注册表管理不同类型元素的处理逻辑
//!
//! # 使用示例
//!
//! ```rust
//! use crate::parsers::html::walker::walk;
//! use crate::network::session::Session;
//! use crate::utils::url::Url;
//! use markup5ever_rcdom::Handle;
//!
//! // 假设你有一个 Session、URL 和 DOM 节点
//! let mut session = Session::new();
//! let document_url = Url::parse("https://example.com").unwrap();
//! // let document_node = ...; // DOM 根节点
//!
//! // 遍历并处理整个 DOM 树
//! walk(&mut session, &document_url, &document_node);
//! ```

use markup5ever_rcdom::Handle;

use crate::network::session::Session;
use crate::utils::url::Url;

use super::dom_walker::DomWalker;

/// DOM遍历和处理的入口函数
///
/// 这是 HTML 解析器模块的核心入口点，负责遍历整个 DOM 树并处理所有节点。
/// 该函数使用模块化的元素处理器系统，相比原来的巨型实现（576行）大大简化了代码结构。
///
/// # 功能特性
///
/// - **递归遍历**: 深度优先遍历整个 DOM 树结构
/// - **元素处理**: 根据元素类型调用相应的处理器（如图片、链接、表单等）
/// - **资源嵌入**: 自动下载并嵌入外部资源（CSS、JavaScript、图片等）
/// - **属性处理**: 处理样式属性和事件处理器属性
/// - **配置驱动**: 根据会话配置选择性地保留或移除 CSS/JavaScript
///
/// # 参数
///
/// * `session` - 可变的会话对象，包含：
///   - 网络配置和缓存
///   - 处理选项（如是否禁用JS/CSS）
///   - 资源下载状态和统计信息
/// * `document_url` - 当前文档的基础URL，用于解析相对路径
/// * `node` - DOM树的根节点句柄，通常是文档节点
///
/// # 处理流程
///
/// 1. 创建新的 `DomWalker` 实例
/// 2. 自动注册所有元素处理器（图片、链接、表单等）
/// 3. 开始递归遍历DOM树
/// 4. 对每个元素节点调用相应的处理器
/// 5. 处理样式属性和JavaScript事件
/// 6. 继续遍历子节点
///
/// # 注意事项
///
/// - 该函数会修改传入的 DOM 树结构
/// - 网络请求可能会阻塞执行
/// - 大型文档的处理可能需要较长时间
/// - 错误会通过会话对象的错误处理机制传播
///
/// # 示例
///
/// ```rust
/// use crate::parsers::html::walker::walk;
/// use crate::network::session::Session;
/// use crate::utils::url::Url;
/// use markup5ever_rcdom::Handle;
///
/// // 创建会话和解析文档URL
/// let mut session = Session::new();
/// let document_url = Url::parse("https://example.com/page.html")?;
///
/// // 假设已经解析了HTML文档获得根节点
/// // let document_node: Handle = parse_html_document(html_content);
///
/// // 遍历并处理整个DOM树
/// walk(&mut session, &document_url, &document_node);
///
/// // 处理完成后，DOM树中的所有外部资源都已被嵌入
/// ```
pub fn walk(session: &mut Session, document_url: &Url, node: &Handle) {
    // 创建DOM遍历器实例，自动注册所有元素处理器
    let walker = DomWalker::new();

    // 开始遍历DOM树，处理所有节点和资源
    walker.walk(session, document_url, node);
}
