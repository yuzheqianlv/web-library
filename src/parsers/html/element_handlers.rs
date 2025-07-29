//! HTML 元素处理器模块
//!
//! 本模块提供了一个可扩展的HTML元素处理系统，用于在创建单体HTML文件时处理不同类型的HTML元素。
//! 每个元素处理器负责处理特定类型的HTML元素，包括资源嵌入、属性修改和内容处理。
//!
//! # 主要功能
//!
//! - **元素特化处理**：为不同HTML元素（如img、link、script等）提供专门的处理逻辑
//! - **资源嵌入**：将外部资源（CSS、JavaScript、图片等）嵌入为data URL
//! - **属性清理**：移除或修改可能影响页面行为的HTML属性
//! - **URL解析**：将相对URL转换为绝对URL
//! - **条件处理**：根据用户选项（如禁用JavaScript、CSS、图片等）进行条件处理
//!
//! # 架构设计
//!
//! 本模块采用策略模式和注册表模式：
//! - `ElementHandler` trait 定义了处理元素的统一接口
//! - 各种具体处理器实现该trait，处理特定元素类型
//! - `ElementHandlerRegistry` 管理所有处理器，提供统一的调用接口
//!
//! # 使用示例
//!
//! ```rust,ignore
//! use crate::parsers::html::element_handlers::ElementHandlerRegistry;
//!
//! let registry = ElementHandlerRegistry::new();
//! registry.handle_element("img", &mut session, &document_url, &node);
//! ```

use markup5ever_rcdom::{Handle, NodeData};

use crate::network::session::Session;
use crate::parsers::css::embed_css;
use crate::utils::url::{resolve_url, Url, EMPTY_IMAGE_DATA_URL};

use super::assets::{embed_srcset, retrieve_and_embed_asset};
use super::dom::{get_node_attr, set_node_attr};
use super::parser::{parse_link_type, LinkType};

/// HTML 元素处理器特征
///
/// 定义了处理HTML元素的统一接口。每个具体的元素处理器都需要实现此trait，
/// 以提供特定元素类型的处理逻辑。
///
/// # 设计理念
///
/// 该trait采用策略模式，允许为不同类型的HTML元素提供专门的处理逻辑，
/// 同时保持代码的可扩展性和可维护性。
pub trait ElementHandler {
    /// 处理指定的HTML元素节点
    ///
    /// # 参数
    ///
    /// * `session` - 可变的会话对象，包含处理选项和缓存
    /// * `document_url` - 文档的基础URL，用于解析相对路径
    /// * `node` - 要处理的HTML元素节点句柄
    ///
    /// # 处理内容
    ///
    /// 根据元素类型和用户配置，可能包括：
    /// - 嵌入外部资源（图片、CSS、JavaScript等）
    /// - 修改或移除特定属性
    /// - 转换相对URL为绝对URL
    /// - 清理可能影响页面行为的内容
    fn handle(&self, session: &mut Session, document_url: &Url, node: &Handle);

    /// 检查是否能处理指定的元素名称
    ///
    /// # 参数
    ///
    /// * `element_name` - HTML元素的标签名（如"img", "link", "script"等）
    ///
    /// # 返回值
    ///
    /// 如果该处理器能够处理给定的元素类型，返回`true`；否则返回`false`
    fn can_handle(&self, element_name: &str) -> bool;
}

/// META 元素处理器
///
/// 处理HTML中的`<meta>`元素。主要功能是移除可能影响页面行为的meta标签，
/// 特别是那些能够控制页面重定向或刷新的http-equiv属性。
///
/// # 处理逻辑
///
/// - 检查meta元素的`http-equiv`属性
/// - 如果属性值为"refresh"或"location"，则移除该属性
/// - 这样做是为了防止嵌入的HTML文件执行不期望的页面跳转或刷新操作
///
/// # 安全性考虑
///
/// 移除控制类的meta标签有助于确保生成的单体HTML文件不会自动跳转到其他页面，
/// 从而保持文件的独立性和安全性。
pub struct MetaElementHandler;

impl ElementHandler for MetaElementHandler {
    fn can_handle(&self, element_name: &str) -> bool {
        element_name == "meta"
    }

    fn handle(&self, _session: &mut Session, _document_url: &Url, node: &Handle) {
        if let Some(meta_attr_http_equiv_value) = get_node_attr(node, "http-equiv") {
            if meta_attr_http_equiv_value.eq_ignore_ascii_case("refresh")
                || meta_attr_http_equiv_value.eq_ignore_ascii_case("location")
            {
                // 移除META节点中能够控制页面行为的http-equiv属性
                set_node_attr(node, "http-equiv", None);
            }
        }
    }
}

/// LINK 元素处理器  
///
/// 处理HTML中的`<link>`元素，支持多种链接类型的处理，包括样式表、图标、
/// 预加载资源等。根据链接类型采用不同的处理策略。
///
/// # 支持的链接类型
///
/// - **Favicon/AppleTouchIcon**: 网站图标，会被嵌入为data URL
/// - **Stylesheet**: CSS样式表，会被下载并嵌入到文档中
/// - **Preload/DnsPrefetch**: 预加载提示，由于资源已嵌入，这些提示会被移除
/// - **其他类型**: 确保href属性为绝对URL
///
/// # 处理策略
///
/// 根据用户配置（如是否禁用CSS、图片等）决定具体的处理方式，
/// 既能保持功能完整性，又能优化最终文件大小。
pub struct LinkElementHandler;

impl ElementHandler for LinkElementHandler {
    fn can_handle(&self, element_name: &str) -> bool {
        element_name == "link"
    }

    fn handle(&self, session: &mut Session, document_url: &Url, node: &Handle) {
        let link_node_types: Vec<LinkType> =
            parse_link_type(&get_node_attr(node, "rel").unwrap_or_default());

        if link_node_types.contains(&LinkType::Favicon)
            || link_node_types.contains(&LinkType::AppleTouchIcon)
        {
            self.handle_favicon_link(session, document_url, node);
        } else if link_node_types.contains(&LinkType::Stylesheet) {
            self.handle_stylesheet_link(session, document_url, node);
        } else if link_node_types.contains(&LinkType::Preload)
            || link_node_types.contains(&LinkType::DnsPrefetch)
        {
            // 由于所有资源都作为data URL嵌入，预加载和DNS预取不再必要
            set_node_attr(node, "rel", None);
        } else {
            self.handle_other_link(document_url, node);
        }
    }
}

impl LinkElementHandler {
    /// 处理网站图标链接（favicon和Apple touch icon）
    ///
    /// # 参数
    ///
    /// * `session` - 会话对象，包含用户配置选项
    /// * `document_url` - 文档基础URL
    /// * `node` - link元素节点
    ///
    /// # 处理逻辑
    ///
    /// - 如果用户未禁用图片且href不为空，则下载并嵌入图标资源
    /// - 否则移除href属性以避免无效链接
    fn handle_favicon_link(&self, session: &mut Session, document_url: &Url, node: &Handle) {
        if let Some(link_attr_href_value) = get_node_attr(node, "href") {
            if !session.options.no_images && !link_attr_href_value.is_empty() {
                retrieve_and_embed_asset(
                    session,
                    document_url,
                    node,
                    "href",
                    &link_attr_href_value,
                );
            } else {
                set_node_attr(node, "href", None);
            }
        }
    }

    /// 处理样式表链接
    ///
    /// # 参数
    ///
    /// * `session` - 会话对象，包含用户配置选项
    /// * `document_url` - 文档基础URL
    /// * `node` - link元素节点
    ///
    /// # 处理逻辑
    ///
    /// - 如果用户禁用了CSS，则移除href和integrity属性
    /// - 否则下载CSS文件并嵌入到文档中
    /// - 嵌入过程中会同时处理CSS中的相对URL引用
    fn handle_stylesheet_link(&self, session: &mut Session, document_url: &Url, node: &Handle) {
        if let Some(link_attr_href_value) = get_node_attr(node, "href") {
            if session.options.no_css {
                set_node_attr(node, "href", None);
                set_node_attr(node, "integrity", None);
            } else if !link_attr_href_value.is_empty() {
                retrieve_and_embed_asset(
                    session,
                    document_url,
                    node,
                    "href",
                    &link_attr_href_value,
                );
            }
        }
    }

    /// 处理其他类型的链接
    ///
    /// # 参数
    ///
    /// * `document_url` - 文档基础URL
    /// * `node` - link元素节点
    ///
    /// # 处理逻辑
    ///
    /// 确保所有其他类型链接的href属性都是完整的绝对URL，
    /// 这对于保持链接在单体HTML文件中的有效性很重要。
    fn handle_other_link(&self, document_url: &Url, node: &Handle) {
        // 确保所有其他LINK元素的href属性都是完整的URL
        if let Some(link_attr_href_value) = get_node_attr(node, "href") {
            let href_full_url: Url = resolve_url(document_url, &link_attr_href_value);
            set_node_attr(node, "href", Some(href_full_url.to_string()));
        }
    }
}

/// BASE 元素处理器
///
/// 处理HTML中的`<base>`元素，确保其href属性使用绝对URL。
/// BASE元素定义了文档中所有相对URL的基础URL，对于单体HTML文件的
/// 正确显示至关重要。
///
/// # 处理目标
///
/// - 将BASE元素的相对URL转换为绝对URL
/// - 仅处理HTTP/HTTPS协议的文档
/// - 确保单体HTML文件在任何位置都能正确解析相对路径
///
/// # 重要性
///
/// 正确处理BASE元素对于维护嵌入资源的URL解析规则非常重要，
/// 特别是当HTML文件被移动到不同位置时。
pub struct BaseElementHandler;

impl ElementHandler for BaseElementHandler {
    fn can_handle(&self, element_name: &str) -> bool {
        element_name == "base"
    }

    fn handle(&self, _session: &mut Session, document_url: &Url, node: &Handle) {
        if document_url.scheme() == "http" || document_url.scheme() == "https" {
            // 确保BASE节点不包含相对URL
            if let Some(base_attr_href_value) = get_node_attr(node, "href") {
                let href_full_url: Url = resolve_url(document_url, &base_attr_href_value);
                set_node_attr(node, "href", Some(href_full_url.to_string()));
            }
        }
    }
}

/// BODY 元素处理器
///
/// 处理HTML中的`<body>`元素，主要处理背景图片的嵌入。
/// BODY元素的background属性（虽然已被废弃，但仍被一些网站使用）
/// 需要特殊处理以确保背景图片能够正确嵌入。
///
/// # 处理内容
///
/// - 检测并处理`background`属性中的背景图片URL
/// - 根据用户配置决定是否下载并嵌入背景图片
/// - 清理原有的background属性以避免无效引用
///
/// # 注意事项
///
/// 虽然HTML5中background属性已被废弃，推荐使用CSS的background-image，
/// 但为了兼容性，此处理器仍然支持传统的background属性。
pub struct BodyElementHandler;

impl ElementHandler for BodyElementHandler {
    fn can_handle(&self, element_name: &str) -> bool {
        element_name == "body"
    }

    fn handle(&self, session: &mut Session, document_url: &Url, node: &Handle) {
        // 读取并记住此BODY节点的background属性值
        if let Some(body_attr_background_value) = get_node_attr(node, "background") {
            // 默认移除BODY节点的background属性
            set_node_attr(node, "background", None);

            if !session.options.no_images && !body_attr_background_value.is_empty() {
                retrieve_and_embed_asset(
                    session,
                    document_url,
                    node,
                    "background",
                    &body_attr_background_value,
                );
            }
        }
    }
}

/// IMG 元素处理器
///
/// 处理HTML中的`<img>`元素，是最复杂的元素处理器之一。
/// 负责处理图片的下载、嵌入以及各种图片相关属性的处理。
///
/// # 处理功能
///
/// - **多源支持**: 处理`src`和`data-src`属性（支持懒加载）
/// - **响应式图片**: 处理`srcset`属性以支持不同分辨率的图片
/// - **条件嵌入**: 根据用户配置决定是否嵌入图片
/// - **空图片处理**: 为禁用图片模式提供空白图片占位符
///
/// # 处理逻辑
///
/// 1. 检查用户是否禁用了图片显示
/// 2. 处理图片源URL（优先data-src，其次src）
/// 3. 下载并嵌入图片资源为data URL
/// 4. 处理响应式图片的srcset属性
///
/// # 兼容性
///
/// 支持现代网站常用的懒加载模式（data-src属性），
/// 确保即使是动态加载的图片也能被正确嵌入。
pub struct ImgElementHandler;

impl ElementHandler for ImgElementHandler {
    fn can_handle(&self, element_name: &str) -> bool {
        element_name == "img"
    }

    fn handle(&self, session: &mut Session, document_url: &Url, node: &Handle) {
        // 查找src和data-src属性
        let img_attr_src_value: Option<String> = get_node_attr(node, "src");
        let img_attr_data_src_value: Option<String> = get_node_attr(node, "data-src");

        if session.options.no_images {
            self.handle_no_images_mode(&img_attr_src_value, &img_attr_data_src_value, node);
        } else if self.has_empty_sources(&img_attr_src_value, &img_attr_data_src_value) {
            // 添加空的src属性
            set_node_attr(node, "src", Some("".to_string()));
        } else {
            self.handle_image_embedding(
                session,
                document_url,
                node,
                &img_attr_src_value,
                &img_attr_data_src_value,
            );
        }

        // 处理srcset属性
        self.handle_srcset(session, document_url, node);
    }
}

impl ImgElementHandler {
    /// 处理禁用图片模式
    ///
    /// # 参数
    ///
    /// * `img_attr_src_value` - img元素的src属性值
    /// * `img_attr_data_src_value` - img元素的data-src属性值
    /// * `node` - img元素节点
    ///
    /// # 处理逻辑  
    ///
    /// 当用户选择禁用图片时，将所有图片源替换为空白图片的data URL，
    /// 这样可以保持页面布局不变，同时减少文件大小。
    fn handle_no_images_mode(
        &self,
        img_attr_src_value: &Option<String>,
        img_attr_data_src_value: &Option<String>,
        node: &Handle,
    ) {
        // 将空图片放入src和data-src属性中
        if img_attr_src_value.is_some() {
            set_node_attr(node, "src", Some(EMPTY_IMAGE_DATA_URL.to_string()));
        }
        if img_attr_data_src_value.is_some() {
            set_node_attr(node, "data-src", Some(EMPTY_IMAGE_DATA_URL.to_string()));
        }
    }

    /// 检查图片源是否为空
    ///
    /// # 参数
    ///
    /// * `img_attr_src_value` - img元素的src属性值
    /// * `img_attr_data_src_value` - img元素的data-src属性值
    ///
    /// # 返回值
    ///
    /// 如果两个图片源都为空或不存在，返回`true`
    fn has_empty_sources(
        &self,
        img_attr_src_value: &Option<String>,
        img_attr_data_src_value: &Option<String>,
    ) -> bool {
        img_attr_src_value.clone().unwrap_or_default().is_empty()
            && img_attr_data_src_value
                .clone()
                .unwrap_or_default()
                .is_empty()
    }

    /// 处理图片嵌入
    ///
    /// # 参数
    ///
    /// * `session` - 会话对象
    /// * `document_url` - 文档基础URL
    /// * `node` - img元素节点
    /// * `img_attr_src_value` - img元素的src属性值
    /// * `img_attr_data_src_value` - img元素的data-src属性值
    ///
    /// # 处理逻辑
    ///
    /// 优先使用data-src属性（懒加载），如果不存在则使用src属性。
    /// 下载图片并将其嵌入为data URL格式。
    fn handle_image_embedding(
        &self,
        session: &mut Session,
        document_url: &Url,
        node: &Handle,
        img_attr_src_value: &Option<String>,
        img_attr_data_src_value: &Option<String>,
    ) {
        let img_full_url: String = if !img_attr_data_src_value
            .clone()
            .unwrap_or_default()
            .is_empty()
        {
            img_attr_data_src_value.clone().unwrap_or_default()
        } else {
            img_attr_src_value.clone().unwrap_or_default()
        };
        retrieve_and_embed_asset(session, document_url, node, "src", &img_full_url);
    }

    /// 处理响应式图片的srcset属性
    ///
    /// # 参数
    ///
    /// * `session` - 会话对象
    /// * `document_url` - 文档基础URL
    /// * `node` - img元素节点
    ///
    /// # 处理逻辑
    ///
    /// 解析srcset属性中的多个图片URL，下载并嵌入所有图片资源，
    /// 生成新的srcset字符串。这确保了响应式图片在单体HTML文件中
    /// 仍能正常工作。
    fn handle_srcset(&self, session: &mut Session, document_url: &Url, node: &Handle) {
        if let Some(img_srcset) = get_node_attr(node, "srcset") {
            if !img_srcset.is_empty() {
                let resolved_srcset: String = embed_srcset(session, document_url, &img_srcset);
                set_node_attr(node, "srcset", Some(resolved_srcset));
            }
        }
    }
}

/// SCRIPT 元素处理器
///
/// 处理HTML中的`<script>`元素，负责JavaScript代码的嵌入和清理。
/// 支持内联脚本和外部脚本文件的处理，并根据用户配置决定是否保留JavaScript。
///
/// # 处理功能
///
/// - **外部脚本嵌入**: 下载外部JavaScript文件并嵌入到script标签中
/// - **条件清理**: 根据用户配置选择性移除JavaScript代码
/// - **属性清理**: 移除不再需要的src和integrity属性
/// - **内容清理**: 清空内联脚本内容（当禁用JS时）
///
/// # 安全考虑
///
/// 当用户选择禁用JavaScript时，会完全清理所有脚本内容，
/// 这有助于：
/// - 减少文件大小
/// - 提高安全性（避免执行恶意脚本）
/// - 加快页面加载速度
pub struct ScriptElementHandler;

impl ElementHandler for ScriptElementHandler {
    fn can_handle(&self, element_name: &str) -> bool {
        element_name == "script"
    }

    fn handle(&self, session: &mut Session, document_url: &Url, node: &Handle) {
        // 读取integrity和src属性的值
        let script_attr_src: String = get_node_attr(node, "src").unwrap_or_default();

        if session.options.no_js {
            // 清空内部内容
            node.children.borrow_mut().clear();
            // 移除src属性
            if !script_attr_src.is_empty() {
                set_node_attr(node, "src", None);
                // 清除integrity属性
                set_node_attr(node, "integrity", None);
            }
        } else if !script_attr_src.is_empty() {
            retrieve_and_embed_asset(session, document_url, node, "src", &script_attr_src);
        }
    }
}

/// STYLE 元素处理器
///
/// 处理HTML中的`<style>`元素，负责内联CSS样式的处理和资源嵌入。
/// 这是CSS处理的核心组件之一，与外部样式表处理互补。
///
/// # 处理功能
///
/// - **CSS资源嵌入**: 处理CSS中引用的外部资源（如@import、url()等）
/// - **URL解析**: 将CSS中的相对URL转换为绝对URL或嵌入的data URL
/// - **条件清理**: 根据用户配置决定是否保留CSS样式
/// - **内容替换**: 在保持DOM结构的同时更新CSS内容
///
/// # 处理逻辑
///
/// 1. 检查用户是否禁用了CSS
/// 2. 如果禁用，清空style标签的所有内容
/// 3. 如果启用，遍历所有文本节点并处理其中的CSS内容
/// 4. 调用CSS嵌入器处理样式中的资源引用
///
/// # 重要性
///
/// 内联样式通常包含关键的页面样式，正确处理这些样式对于
/// 保持页面原有外观至关重要。
pub struct StyleElementHandler;

impl ElementHandler for StyleElementHandler {
    fn can_handle(&self, element_name: &str) -> bool {
        element_name == "style"
    }

    fn handle(&self, session: &mut Session, document_url: &Url, node: &Handle) {
        if session.options.no_css {
            // 清空STYLE标签的内部内容
            node.children.borrow_mut().clear();
        } else {
            for child_node in node.children.borrow_mut().iter_mut() {
                if let NodeData::Text { ref contents } = child_node.data {
                    let mut tendril = contents.borrow_mut();
                    let replacement = embed_css(session, document_url, tendril.as_ref());
                    tendril.clear();
                    tendril.push_slice(&replacement);
                }
            }
        }
    }
}

/// 元素处理器注册表
///
/// 统一管理所有可用的HTML元素处理器，提供处理器注册和元素处理的统一接口。
/// 采用注册表模式，便于扩展和维护不同类型的元素处理器。
///
/// # 设计模式
///
/// - **注册表模式**: 集中管理所有处理器实例
/// - **策略模式**: 根据元素类型选择合适的处理策略
/// - **责任链模式**: 按顺序查找能够处理指定元素的处理器
///
/// # 预注册的处理器
///
/// 默认注册了以下处理器：
/// - `MetaElementHandler`: 处理meta元素
/// - `LinkElementHandler`: 处理link元素（样式表、图标等）
/// - `BaseElementHandler`: 处理base元素
/// - `BodyElementHandler`: 处理body元素的背景图片
/// - `ImgElementHandler`: 处理img元素和响应式图片
/// - `ScriptElementHandler`: 处理script元素
/// - `StyleElementHandler`: 处理style元素中的内联CSS
///
/// # 扩展性
///
/// 可以通过`register()`方法添加新的元素处理器，支持自定义元素的处理逻辑。
pub struct ElementHandlerRegistry {
    /// 存储所有注册的元素处理器
    handlers: Vec<Box<dyn ElementHandler>>,
}

impl ElementHandlerRegistry {
    /// 创建新的元素处理器注册表
    ///
    /// 自动注册所有内置的元素处理器，包括meta、link、base、body、
    /// img、script、style等常见HTML元素的处理器。
    ///
    /// # 返回值
    ///
    /// 返回已预配置所有标准处理器的注册表实例
    pub fn new() -> Self {
        let mut registry = ElementHandlerRegistry {
            handlers: Vec::new(),
        };

        // 注册所有元素处理器
        registry.register(Box::new(MetaElementHandler));
        registry.register(Box::new(LinkElementHandler));
        registry.register(Box::new(BaseElementHandler));
        registry.register(Box::new(BodyElementHandler));
        registry.register(Box::new(ImgElementHandler));
        registry.register(Box::new(ScriptElementHandler));
        registry.register(Box::new(StyleElementHandler));

        registry
    }

    /// 注册新的元素处理器
    ///
    /// # 参数
    ///
    /// * `handler` - 要注册的元素处理器，必须实现ElementHandler trait
    ///
    /// # 注意事项
    ///
    /// 处理器按注册顺序查找，先注册的处理器具有更高的优先级。
    /// 如果多个处理器声称能处理同一元素类型，只有第一个会被使用。
    pub fn register(&mut self, handler: Box<dyn ElementHandler>) {
        self.handlers.push(handler);
    }

    /// 处理指定的HTML元素
    ///
    /// # 参数
    ///
    /// * `element_name` - HTML元素的标签名（如"img", "link", "script"等）
    /// * `session` - 会话对象，包含处理选项和缓存
    /// * `document_url` - 文档基础URL
    /// * `node` - 要处理的HTML元素节点
    ///
    /// # 处理逻辑
    ///
    /// 按注册顺序遍历所有处理器，找到第一个能够处理指定元素类型的处理器，
    /// 然后调用其handle方法进行处理。如果没有找到合适的处理器，
    /// 元素将保持不变。
    ///
    /// # 性能考虑
    ///
    /// 采用线性查找，对于少量处理器（通常<10个）性能影响可忽略。
    /// 如果需要处理大量不同类型的元素，可以考虑使用HashMap优化查找。
    pub fn handle_element(
        &self,
        element_name: &str,
        session: &mut Session,
        document_url: &Url,
        node: &Handle,
    ) {
        for handler in &self.handlers {
            if handler.can_handle(element_name) {
                handler.handle(session, document_url, node);
                return;
            }
        }
    }
}

impl Default for ElementHandlerRegistry {
    fn default() -> Self {
        Self::new()
    }
}
