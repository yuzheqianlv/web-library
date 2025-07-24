use markup5ever_rcdom::{Handle, NodeData};

use crate::network::session::Session;
use crate::parsers::css::embed_css;
use crate::parsers::js::attr_is_event_handler;
use crate::utils::url::Url;

use super::complex_element_handlers::*;
use super::dom::{get_node_attr, set_node_attr};
use super::element_handlers::ElementHandlerRegistry;

/// DOM遍历器，负责遍历DOM树并调用相应的元素处理器
pub struct DomWalker {
    element_registry: ElementHandlerRegistry,
}

impl DomWalker {
    pub fn new() -> Self {
        let mut walker = DomWalker {
            element_registry: ElementHandlerRegistry::new(),
        };

        // 注册复杂元素处理器
        walker.register_complex_handlers();

        walker
    }

    fn register_complex_handlers(&mut self) {
        self.element_registry
            .register(Box::new(InputElementHandler));
        self.element_registry.register(Box::new(SvgElementHandler));
        self.element_registry
            .register(Box::new(ImageElementHandler));
        self.element_registry.register(Box::new(UseElementHandler));
        self.element_registry
            .register(Box::new(SourceElementHandler));
        self.element_registry.register(Box::new(FormElementHandler));
        self.element_registry
            .register(Box::new(FrameElementHandler));
        self.element_registry
            .register(Box::new(AudioElementHandler));
        self.element_registry
            .register(Box::new(VideoElementHandler));
        self.element_registry
            .register(Box::new(AnchorElementHandler));
        self.element_registry
            .register(Box::new(NoscriptElementHandler));
    }

    /// 遍历DOM节点并处理所有子节点
    pub fn walk(&self, session: &mut Session, document_url: &Url, node: &Handle) {
        match node.data {
            NodeData::Document => {
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
                // 其他节点类型（如注释节点）不需要特殊处理
                // 注意：对于options.no_js设置为true的情况，不需要担心
                // 包含脚本的注释，例如<!--[if IE]><script>...
                // 因为这不是W3C标准的一部分，会被除IE [5, 9]之外的浏览器忽略
            }
        }
    }

    /// 遍历所有子节点
    fn walk_children(&self, session: &mut Session, document_url: &Url, node: &Handle) {
        for child_node in node.children.borrow().iter() {
            self.walk(session, document_url, child_node);
        }
    }

    /// 处理样式属性
    fn process_style_attributes(&self, session: &mut Session, document_url: &Url, node: &Handle) {
        if session.options.no_css {
            // 删除样式属性
            set_node_attr(node, "style", None);
        } else {
            // 嵌入在此节点样式属性中找到的URL
            if let Some(node_attr_style_value) = get_node_attr(node, "style") {
                let embedded_style = embed_css(session, document_url, &node_attr_style_value);
                set_node_attr(node, "style", Some(embedded_style));
            }
        }
    }

    /// 处理JavaScript事件属性
    fn process_js_attributes(
        &self,
        session: &mut Session,
        attrs: &std::cell::RefCell<Vec<html5ever::interface::Attribute>>,
    ) {
        // 从文档中剥离所有JS
        if session.options.no_js {
            let attrs_mut = &mut attrs.borrow_mut();
            // 删除JS事件属性
            let mut js_attr_indexes = Vec::new();
            for (i, attr) in attrs_mut.iter().enumerate() {
                if attr_is_event_handler(&attr.name.local) {
                    js_attr_indexes.push(i);
                }
            }
            js_attr_indexes.reverse();
            for attr_index in js_attr_indexes {
                attrs_mut.remove(attr_index);
            }
        }
    }
}

impl Default for DomWalker {
    fn default() -> Self {
        Self::new()
    }
}
