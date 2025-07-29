//! DOM遍历器模块
//!
//! 本模块提供了DOM树遍历功能，负责递归遍历HTML文档的DOM结构，
//! 并为每个元素调用相应的处理器。这是Monolith处理HTML文档的核心组件之一。
//!
//! # 主要功能
//!
//! - **DOM树遍历**：递归遍历整个DOM树结构
//! - **元素处理**：为不同类型的HTML元素调用相应的处理器
//! - **样式处理**：处理元素的内联样式属性，嵌入外部CSS资源
//! - **JavaScript处理**：根据配置选择性移除JavaScript事件处理器
//! - **资源嵌入**：协调各种资源（图片、样式表、脚本等）的嵌入操作
//!
//! # 工作流程
//!
//! 1. 从文档根节点开始遍历
//! 2. 对每个元素节点，调用注册的元素处理器
//! 3. 处理元素的样式和JavaScript属性
//! 4. 递归处理所有子节点
//!
//! # 使用示例
//!
//! ```rust
//! use crate::parsers::html::dom_walker::DomWalker;
//! use crate::network::session::Session;
//! use crate::utils::url::Url;
//!
//! let walker = DomWalker::new();
//! let mut session = Session::new();
//! let document_url = Url::parse("https://example.com").unwrap();
//!
//! // 遍历DOM树
//! walker.walk(&mut session, &document_url, &document_root);
//! ```

use markup5ever_rcdom::{Handle, NodeData};

use crate::network::session::Session;
use crate::parsers::css::embed_css;
use crate::parsers::js::attr_is_event_handler;
use crate::utils::url::Url;

use super::complex_element_handlers::*;
use super::dom::{get_node_attr, set_node_attr};
use super::element_handlers::ElementHandlerRegistry;

/// DOM遍历器
///
/// DOM遍历器是处理HTML文档的核心组件，负责递归遍历DOM树中的每个节点，
/// 并为不同类型的HTML元素调用相应的处理器来处理资源嵌入、属性修改等操作。
///
/// # 结构
///
/// - `element_registry`: 元素处理器注册表，包含了所有已注册的元素处理器
///
/// # 功能特性
///
/// - 支持所有标准HTML元素的处理
/// - 自动处理样式属性中的外部资源引用
/// - 根据配置选择性移除JavaScript代码
/// - 可扩展的元素处理器架构
///
/// # 线程安全性
///
/// 该结构体不是线程安全的，不应在多线程环境中共享使用。
pub struct DomWalker {
    /// 元素处理器注册表，用于存储和管理各种HTML元素的处理器
    element_registry: ElementHandlerRegistry,
}

impl DomWalker {
    /// 创建新的DOM遍历器实例
    ///
    /// 创建一个新的DOM遍历器，并自动注册所有内置的元素处理器。
    /// 这些处理器包括图片、音频、视频、表单、SVG等各种HTML元素的处理器。
    ///
    /// # 返回值
    ///
    /// 返回一个已经配置好所有内置处理器的`DomWalker`实例。
    ///
    /// # 示例
    ///
    /// ```rust
    /// use crate::parsers::html::dom_walker::DomWalker;
    ///
    /// let walker = DomWalker::new();
    /// // walker现在已经可以处理所有支持的HTML元素
    /// ```
    pub fn new() -> Self {
        let mut walker = DomWalker {
            element_registry: ElementHandlerRegistry::new(),
        };

        // 注册复杂元素处理器
        walker.register_complex_handlers();

        walker
    }

    /// 注册复杂元素处理器
    ///
    /// 注册所有内置的复杂元素处理器到元素注册表中。
    /// 这些处理器负责处理特定HTML元素的资源嵌入和属性修改。
    ///
    /// # 注册的处理器包括：
    ///
    /// - `InputElementHandler`: 处理`<input>`元素
    /// - `SvgElementHandler`: 处理`<svg>`元素
    /// - `ImageElementHandler`: 处理`<img>`元素
    /// - `UseElementHandler`: 处理SVG `<use>`元素
    /// - `SourceElementHandler`: 处理`<source>`元素
    /// - `FormElementHandler`: 处理`<form>`元素
    /// - `FrameElementHandler`: 处理`<iframe>`和`<frame>`元素
    /// - `AudioElementHandler`: 处理`<audio>`元素
    /// - `VideoElementHandler`: 处理`<video>`元素
    /// - `AnchorElementHandler`: 处理`<a>`元素
    /// - `NoscriptElementHandler`: 处理`<noscript>`元素
    fn register_complex_handlers(&mut self) {
        // 注册输入元素处理器
        self.element_registry
            .register(Box::new(InputElementHandler));
        // 注册SVG元素处理器
        self.element_registry.register(Box::new(SvgElementHandler));
        // 注册图片元素处理器
        self.element_registry
            .register(Box::new(ImageElementHandler));
        // 注册SVG use元素处理器
        self.element_registry.register(Box::new(UseElementHandler));
        // 注册source元素处理器
        self.element_registry
            .register(Box::new(SourceElementHandler));
        // 注册表单元素处理器
        self.element_registry.register(Box::new(FormElementHandler));
        // 注册框架元素处理器
        self.element_registry
            .register(Box::new(FrameElementHandler));
        // 注册音频元素处理器
        self.element_registry
            .register(Box::new(AudioElementHandler));
        // 注册视频元素处理器
        self.element_registry
            .register(Box::new(VideoElementHandler));
        // 注册锚点元素处理器
        self.element_registry
            .register(Box::new(AnchorElementHandler));
        // 注册noscript元素处理器
        self.element_registry
            .register(Box::new(NoscriptElementHandler));
    }

    /// 遍历DOM节点并处理所有子节点
    ///
    /// 这是DOM遍历器的核心方法，负责递归遍历DOM树中的每个节点。
    /// 对于不同类型的节点，该方法会执行相应的处理逻辑：
    ///
    /// - **文档节点**：直接遍历子节点
    /// - **元素节点**：调用元素处理器，处理样式和JavaScript属性，然后遍历子节点
    /// - **其他节点**：跳过处理（如文本节点、注释节点等）
    ///
    /// # 参数
    ///
    /// * `session` - 可变的会话对象，包含处理配置和状态信息
    /// * `document_url` - 文档的基础URL，用于解析相对路径
    /// * `node` - 要遍历的DOM节点句柄
    ///
    /// # 处理流程
    ///
    /// 1. 根据节点类型进行分类处理
    /// 2. 对于元素节点，依次执行：
    ///    - 调用注册的元素处理器
    ///    - 处理样式属性中的外部资源
    ///    - 根据配置处理JavaScript事件属性
    ///    - 递归处理所有子节点
    ///
    /// # 注意事项
    ///
    /// - 该方法会修改DOM树的结构和内容
    /// - 处理过程中可能会发起网络请求来获取外部资源
    /// - JavaScript相关的处理取决于`session.options.no_js`配置
    ///
    /// # 示例
    ///
    /// ```rust
    /// use crate::parsers::html::dom_walker::DomWalker;
    /// use crate::network::session::Session;
    /// use crate::utils::url::Url;
    ///
    /// let walker = DomWalker::new();
    /// let mut session = Session::new();
    /// let document_url = Url::parse("https://example.com").unwrap();
    ///
    /// // 从文档根节点开始遍历整个DOM树
    /// walker.walk(&mut session, &document_url, &document_root);
    /// ```
    pub fn walk(&self, session: &mut Session, document_url: &Url, node: &Handle) {
        match node.data {
            NodeData::Document => {
                // 文档节点：直接遍历所有子节点
                self.walk_children(session, document_url, node);
            }
            NodeData::Element {
                ref name,
                ref attrs,
                ..
            } => {
                let element_name = name.local.as_ref();

                // 使用元素处理器处理当前元素
                self.element_registry
                    .handle_element(element_name, session, document_url, node);

                // 处理通用样式属性
                self.process_style_attributes(session, document_url, node);

                // 处理JavaScript事件属性
                self.process_js_attributes(session, attrs);

                // 递归处理子节点
                self.walk_children(session, document_url, node);
            }
            _ => {
                // 其他节点类型（如注释节点、文本节点）不需要特殊处理
                // 注意：对于options.no_js设置为true的情况，不需要担心
                // 包含脚本的注释，例如<!--[if IE]><script>...
                // 因为这不是W3C标准的一部分，会被除IE [5, 9]之外的浏览器忽略
            }
        }
    }

    /// 遍历所有子节点
    ///
    /// 递归遍历指定节点的所有直接子节点。这是一个辅助方法，
    /// 用于简化在不同节点类型中重复使用的子节点遍历逻辑。
    ///
    /// # 参数
    ///
    /// * `session` - 可变的会话对象，包含处理配置和状态信息
    /// * `document_url` - 文档的基础URL，用于解析相对路径
    /// * `node` - 父节点的句柄，其子节点将被遍历
    ///
    /// # 实现细节
    ///
    /// 该方法通过借用父节点的子节点列表，然后对每个子节点递归调用`walk`方法。
    /// 使用`borrow()`来安全地访问RefCell包装的子节点集合。
    fn walk_children(&self, session: &mut Session, document_url: &Url, node: &Handle) {
        // 遍历当前节点的所有直接子节点
        for child_node in node.children.borrow().iter() {
            // 递归处理每个子节点
            self.walk(session, document_url, child_node);
        }
    }

    /// 处理样式属性
    ///
    /// 处理HTML元素的内联样式属性（style属性），根据会话配置决定是移除样式
    /// 还是嵌入样式中引用的外部资源。
    ///
    /// # 参数
    ///
    /// * `session` - 可变的会话对象，包含CSS处理配置
    /// * `document_url` - 文档的基础URL，用于解析样式中的相对路径
    /// * `node` - 要处理样式属性的DOM节点
    ///
    /// # 处理逻辑
    ///
    /// - 如果`session.options.no_css`为true，则完全移除style属性
    /// - 否则，解析style属性中的CSS内容，嵌入其中引用的外部资源（如背景图片、字体等）
    ///
    /// # CSS资源嵌入
    ///
    /// 当保留CSS时，该方法会：
    /// 1. 提取style属性的值
    /// 2. 调用`embed_css`函数处理其中的URL引用
    /// 3. 将处理后的CSS内容重新设置到style属性
    ///
    /// # 示例
    ///
    /// ```html
    /// <!-- 处理前 -->
    /// <div style="background-image: url('image.png'); font-face: url('font.woff');">
    ///
    /// <!-- 处理后（如果no_css=false） -->
    /// <div style="background-image: url('data:image/png;base64,...'); font-face: url('data:font/woff;base64,...');">
    ///
    /// <!-- 处理后（如果no_css=true） -->
    /// <div>
    /// ```
    fn process_style_attributes(&self, session: &mut Session, document_url: &Url, node: &Handle) {
        if session.options.no_css {
            // 当配置为移除CSS时，删除样式属性
            set_node_attr(node, "style", None);
        } else {
            // 嵌入在此节点样式属性中找到的外部资源URL
            if let Some(node_attr_style_value) = get_node_attr(node, "style") {
                // 处理CSS内容，嵌入外部资源
                let embedded_style = embed_css(session, document_url, &node_attr_style_value);
                // 将处理后的CSS内容重新设置到节点
                set_node_attr(node, "style", Some(embedded_style));
            }
        }
    }

    /// 处理JavaScript事件属性
    ///
    /// 根据会话配置决定是否移除HTML元素的JavaScript事件处理属性。
    /// 当配置为禁用JavaScript时，会移除所有以"on"开头的事件处理属性。
    ///
    /// # 参数
    ///
    /// * `session` - 可变的会话对象，包含JavaScript处理配置
    /// * `attrs` - 元素属性的可变引用，包装在RefCell中用于内部可变性
    ///
    /// # JavaScript事件属性
    ///
    /// 该方法会识别并移除以下类型的JavaScript事件属性：
    /// - `onclick`, `onload`, `onsubmit` 等标准事件属性
    /// - 任何以"on"开头的自定义事件属性
    ///
    /// # 处理流程
    ///
    /// 1. 检查`session.options.no_js`配置
    /// 2. 如果需要移除JavaScript：
    ///    - 遍历所有属性，识别事件处理属性
    ///    - 收集需要删除的属性索引
    ///    - 从后往前删除属性（避免索引移位问题）
    ///
    /// # 性能考虑
    ///
    /// - 使用反向索引删除来避免在删除过程中重新计算索引
    /// - 只在需要移除JavaScript时才执行属性遍历
    ///
    /// # 示例
    ///
    /// ```html
    /// <!-- 处理前 -->
    /// <button onclick="alert('hello')" onmouseover="highlight(this)">Click me</button>
    ///
    /// <!-- 处理后（如果no_js=true） -->
    /// <button>Click me</button>
    /// ```
    ///
    /// # 注意事项
    ///
    /// - 该方法直接修改传入的属性列表
    /// - 只有在`session.options.no_js`为true时才会执行移除操作
    /// - 属性识别通过`attr_is_event_handler`函数完成
    fn process_js_attributes(
        &self,
        session: &mut Session,
        attrs: &std::cell::RefCell<Vec<html5ever::interface::Attribute>>,
    ) {
        // 根据配置决定是否从文档中剥离所有JavaScript
        if session.options.no_js {
            let attrs_mut = &mut attrs.borrow_mut();

            // 收集需要删除的JavaScript事件属性索引
            let mut js_attr_indexes = Vec::new();
            for (i, attr) in attrs_mut.iter().enumerate() {
                // 检查属性名是否为JavaScript事件处理器
                if attr_is_event_handler(&attr.name.local) {
                    js_attr_indexes.push(i);
                }
            }

            // 从后往前删除属性，避免索引移位问题
            js_attr_indexes.reverse();
            for attr_index in js_attr_indexes {
                attrs_mut.remove(attr_index);
            }
        }
    }
}

/// 为DomWalker实现Default trait
///
/// 提供DomWalker的默认构造方式，等同于调用`DomWalker::new()`。
/// 这使得可以使用`DomWalker::default()`或在需要Default实现的上下文中使用。
impl Default for DomWalker {
    /// 创建默认的DomWalker实例
    ///
    /// # 返回值
    ///
    /// 返回一个新的DomWalker实例，已注册所有内置的元素处理器。
    ///
    /// # 示例
    ///
    /// ```rust
    /// use crate::parsers::html::dom_walker::DomWalker;
    ///
    /// // 两种方式等价
    /// let walker1 = DomWalker::new();
    /// let walker2 = DomWalker::default();
    /// ```
    fn default() -> Self {
        Self::new()
    }
}
