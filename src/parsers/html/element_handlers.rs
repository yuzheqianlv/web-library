use markup5ever_rcdom::{Handle, NodeData};

use crate::network::session::Session;
use crate::parsers::css::embed_css;
use crate::utils::url::{resolve_url, Url, EMPTY_IMAGE_DATA_URL};

use super::assets::{embed_srcset, retrieve_and_embed_asset};
use super::dom::{get_node_attr, set_node_attr};
use super::parser::{parse_link_type, LinkType};

/// 元素处理器特征，定义了处理不同HTML元素的接口
pub trait ElementHandler {
    /// 处理特定HTML元素
    fn handle(&self, session: &mut Session, document_url: &Url, node: &Handle);

    /// 检查是否能处理指定的元素名称
    fn can_handle(&self, element_name: &str) -> bool;
}

/// META元素处理器
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
                // Remove http-equiv attributes from META nodes if they're able to control the page
                set_node_attr(node, "http-equiv", None);
            }
        }
    }
}

/// LINK元素处理器
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
            // Since all resources are embedded as data URLs, preloading and prefetching are not necessary
            set_node_attr(node, "rel", None);
        } else {
            self.handle_other_link(document_url, node);
        }
    }
}

impl LinkElementHandler {
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

    fn handle_other_link(&self, document_url: &Url, node: &Handle) {
        // Make sure that all other LINKs' href attributes are full URLs
        if let Some(link_attr_href_value) = get_node_attr(node, "href") {
            let href_full_url: Url = resolve_url(document_url, &link_attr_href_value);
            set_node_attr(node, "href", Some(href_full_url.to_string()));
        }
    }
}

/// BASE元素处理器
pub struct BaseElementHandler;

impl ElementHandler for BaseElementHandler {
    fn can_handle(&self, element_name: &str) -> bool {
        element_name == "base"
    }

    fn handle(&self, _session: &mut Session, document_url: &Url, node: &Handle) {
        if document_url.scheme() == "http" || document_url.scheme() == "https" {
            // Ensure the BASE node doesn't have a relative URL
            if let Some(base_attr_href_value) = get_node_attr(node, "href") {
                let href_full_url: Url = resolve_url(document_url, &base_attr_href_value);
                set_node_attr(node, "href", Some(href_full_url.to_string()));
            }
        }
    }
}

/// BODY元素处理器
pub struct BodyElementHandler;

impl ElementHandler for BodyElementHandler {
    fn can_handle(&self, element_name: &str) -> bool {
        element_name == "body"
    }

    fn handle(&self, session: &mut Session, document_url: &Url, node: &Handle) {
        // Read and remember background attribute value of this BODY node
        if let Some(body_attr_background_value) = get_node_attr(node, "background") {
            // Remove background BODY node attribute by default
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

/// IMG元素处理器
pub struct ImgElementHandler;

impl ElementHandler for ImgElementHandler {
    fn can_handle(&self, element_name: &str) -> bool {
        element_name == "img"
    }

    fn handle(&self, session: &mut Session, document_url: &Url, node: &Handle) {
        // Find src and data-src attribute(s)
        let img_attr_src_value: Option<String> = get_node_attr(node, "src");
        let img_attr_data_src_value: Option<String> = get_node_attr(node, "data-src");

        if session.options.no_images {
            self.handle_no_images_mode(&img_attr_src_value, &img_attr_data_src_value, node);
        } else if self.has_empty_sources(&img_attr_src_value, &img_attr_data_src_value) {
            // Add empty src attribute
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

        // Resolve srcset attribute
        self.handle_srcset(session, document_url, node);
    }
}

impl ImgElementHandler {
    fn handle_no_images_mode(
        &self,
        img_attr_src_value: &Option<String>,
        img_attr_data_src_value: &Option<String>,
        node: &Handle,
    ) {
        // Put empty images into src and data-src attributes
        if img_attr_src_value.is_some() {
            set_node_attr(node, "src", Some(EMPTY_IMAGE_DATA_URL.to_string()));
        }
        if img_attr_data_src_value.is_some() {
            set_node_attr(node, "data-src", Some(EMPTY_IMAGE_DATA_URL.to_string()));
        }
    }

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

    fn handle_srcset(&self, session: &mut Session, document_url: &Url, node: &Handle) {
        if let Some(img_srcset) = get_node_attr(node, "srcset") {
            if !img_srcset.is_empty() {
                let resolved_srcset: String = embed_srcset(session, document_url, &img_srcset);
                set_node_attr(node, "srcset", Some(resolved_srcset));
            }
        }
    }
}

/// SCRIPT元素处理器
pub struct ScriptElementHandler;

impl ElementHandler for ScriptElementHandler {
    fn can_handle(&self, element_name: &str) -> bool {
        element_name == "script"
    }

    fn handle(&self, session: &mut Session, document_url: &Url, node: &Handle) {
        // Read values of integrity and src attributes
        let script_attr_src: String = get_node_attr(node, "src").unwrap_or_default();

        if session.options.no_js {
            // Empty inner content
            node.children.borrow_mut().clear();
            // Remove src attribute
            if !script_attr_src.is_empty() {
                set_node_attr(node, "src", None);
                // Wipe integrity attribute
                set_node_attr(node, "integrity", None);
            }
        } else if !script_attr_src.is_empty() {
            retrieve_and_embed_asset(session, document_url, node, "src", &script_attr_src);
        }
    }
}

/// STYLE元素处理器
pub struct StyleElementHandler;

impl ElementHandler for StyleElementHandler {
    fn can_handle(&self, element_name: &str) -> bool {
        element_name == "style"
    }

    fn handle(&self, session: &mut Session, document_url: &Url, node: &Handle) {
        if session.options.no_css {
            // Empty inner content of STYLE tags
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

/// 元素处理器注册表，管理所有可用的元素处理器
pub struct ElementHandlerRegistry {
    handlers: Vec<Box<dyn ElementHandler>>,
}

impl ElementHandlerRegistry {
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

    pub fn register(&mut self, handler: Box<dyn ElementHandler>) {
        self.handlers.push(handler);
    }

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
