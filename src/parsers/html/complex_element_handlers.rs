//! 复杂HTML元素处理器模块
//!
//! 本模块提供了针对特定HTML元素的专用处理器实现，这些元素需要特殊的处理逻辑来正确嵌入或处理其资源。
//! 每个处理器都实现了 `ElementHandler` trait，负责处理特定类型的HTML元素，如INPUT、SVG、FORM、AUDIO、VIDEO等。
//!
//! ## 主要功能
//!
//! - **INPUT元素处理**：处理type="image"的input元素，嵌入图片资源
//! - **SVG元素处理**：处理SVG及其子元素（image、use），支持SVG片段引用
//! - **媒体元素处理**：处理audio、video、source元素的资源嵌入
//! - **框架元素处理**：处理frame和iframe元素的内容嵌入
//! - **表单元素处理**：处理form元素的action属性URL解析
//! - **链接元素处理**：处理a和area元素的href属性URL解析
//! - **脚本元素处理**：处理noscript元素的内容解析和资源嵌入
//!
//! ## 设计原则
//!
//! 所有处理器都遵循以下设计原则：
//! - 根据session配置选项（如no_images、no_audio等）决定是否嵌入资源
//! - 将相对URL解析为绝对URL
//! - 安全地处理空值和无效值
//! - 支持data URL嵌入以实现完整的单文件HTML

use html5ever::interface::QualName;
use html5ever::parse_document;
use html5ever::tendril::TendrilSink;
use html5ever::tree_builder::{create_element, TreeSink};
use html5ever::{namespace_url, ns, LocalName};
use markup5ever_rcdom::{Handle, NodeData, RcDom};

use crate::network::session::Session;
use crate::utils::url::{
    create_data_url, is_url_and_has_protocol, resolve_url, Url, EMPTY_IMAGE_DATA_URL,
};

use super::assets::{embed_srcset, retrieve_and_embed_asset};
use super::dom::{
    find_nodes, get_child_node_by_name, get_node_attr, get_node_name, get_parent_node, html_to_dom,
    set_node_attr,
};
use super::element_handlers::ElementHandler;

/// INPUT元素处理器
/// 
/// 专门处理HTML `<input>` 元素，特别是type="image"的输入元素。
/// 当输入元素用作图像按钮时，会嵌入其图像资源或根据配置移除图像。
/// 
/// # 处理逻辑
/// 
/// - 检查input元素的type属性是否为"image"
/// - 如果禁用图像显示，则设置为空图像数据URL或清空src属性
/// - 否则检索并嵌入图像资源为data URL
pub struct InputElementHandler;

impl ElementHandler for InputElementHandler {
    /// 检查是否可以处理指定的元素
    /// 
    /// # 参数
    /// 
    /// * `element_name` - HTML元素名称
    /// 
    /// # 返回值
    /// 
    /// 如果元素名为"input"则返回true，否则返回false
    fn can_handle(&self, element_name: &str) -> bool {
        element_name == "input"
    }

    /// 处理INPUT元素
    /// 
    /// 针对type="image"的input元素，处理其src属性中的图像资源。
    /// 根据session配置决定是否嵌入图像或使用占位符。
    /// 
    /// # 参数
    /// 
    /// * `session` - 当前会话，包含配置选项和缓存
    /// * `document_url` - 当前文档的URL，用于解析相对路径
    /// * `node` - 要处理的DOM节点
    fn handle(&self, session: &mut Session, document_url: &Url, node: &Handle) {
        if let Some(input_attr_type_value) = get_node_attr(node, "type") {
            // 只处理图像类型的input元素
            if input_attr_type_value.eq_ignore_ascii_case("image") {
                if let Some(input_attr_src_value) = get_node_attr(node, "src") {
                    if session.options.no_images || input_attr_src_value.is_empty() {
                        // 根据原始src是否为空决定使用空字符串还是空图像数据URL
                        let value = if input_attr_src_value.is_empty() {
                            ""
                        } else {
                            EMPTY_IMAGE_DATA_URL
                        };
                        set_node_attr(node, "src", Some(value.to_string()));
                    } else {
                        // 检索并嵌入图像资源
                        retrieve_and_embed_asset(
                            session,
                            document_url,
                            node,
                            "src",
                            &input_attr_src_value,
                        );
                    }
                }
            }
        }
    }
}

/// SVG元素处理器
/// 
/// 处理HTML中的 `<svg>` 元素。当禁用图像显示时，会移除SVG的所有子元素，
/// 从而禁用整个SVG图形的显示。
/// 
/// # 处理逻辑
/// 
/// - 如果配置了no_images选项，清空SVG元素的所有子节点
/// - 否则保持SVG元素及其内容不变
pub struct SvgElementHandler;

impl ElementHandler for SvgElementHandler {
    /// 检查是否可以处理指定的元素
    /// 
    /// # 参数
    /// 
    /// * `element_name` - HTML元素名称
    /// 
    /// # 返回值
    /// 
    /// 如果元素名为"svg"则返回true，否则返回false
    fn can_handle(&self, element_name: &str) -> bool {
        element_name == "svg"
    }

    /// 处理SVG元素
    /// 
    /// 根据session配置决定是否保留SVG内容。如果禁用图像，则清空所有子元素。
    /// 
    /// # 参数
    /// 
    /// * `session` - 当前会话，包含配置选项
    /// * `_document_url` - 当前文档的URL（此处未使用）
    /// * `node` - 要处理的SVG DOM节点
    fn handle(&self, session: &mut Session, _document_url: &Url, node: &Handle) {
        if session.options.no_images {
            // 移除所有子元素以禁用SVG显示
            node.children.borrow_mut().clear();
        }
    }
}

/// IMAGE元素处理器（SVG中的image元素）
/// 
/// 处理SVG中的 `<image>` 元素，这些元素用于在SVG中嵌入位图图像。
/// 支持处理传统的href属性和已弃用的xlink:href属性。
/// 
/// # 处理逻辑
/// 
/// - 检查href和xlink:href属性
/// - 如果禁用图像显示，移除这些属性
/// - 否则检索并嵌入图像资源为data URL
/// 
/// # 注意事项
/// 
/// xlink:href属性已在SVG 2.0中被弃用，但为了兼容性仍需支持
pub struct ImageElementHandler;

impl ElementHandler for ImageElementHandler {
    /// 检查是否可以处理指定的元素
    /// 
    /// # 参数
    /// 
    /// * `element_name` - HTML元素名称
    /// 
    /// # 返回值
    /// 
    /// 如果元素名为"image"则返回true，否则返回false
    fn can_handle(&self, element_name: &str) -> bool {
        element_name == "image"
    }

    /// 处理SVG IMAGE元素
    /// 
    /// 处理SVG中的image元素的href和xlink:href属性，根据配置嵌入或移除图像资源。
    /// 
    /// # 参数
    /// 
    /// * `session` - 当前会话，包含配置选项和缓存
    /// * `document_url` - 当前文档的URL，用于解析相对路径
    /// * `node` - 要处理的image DOM节点
    fn handle(&self, session: &mut Session, document_url: &Url, node: &Handle) {
        // SVG image元素支持的href属性名称（包括已弃用的xlink:href）
        let attr_names: [&str; 2] = ["href", "xlink:href"];

        for attr_name in attr_names.into_iter() {
            if let Some(image_attr_href_value) = get_node_attr(node, attr_name) {
                if session.options.no_images {
                    // 如果禁用图像，移除href属性
                    set_node_attr(node, attr_name, None);
                } else {
                    // 检索并嵌入图像资源
                    retrieve_and_embed_asset(
                        session,
                        document_url,
                        node,
                        attr_name,
                        &image_attr_href_value,
                    );
                }
            }
        }
    }
}

/// USE元素处理器（SVG中的use元素）
/// 
/// 处理SVG中的 `<use>` 元素，这些元素用于引用和重用其他SVG元素或外部SVG文件。
/// 支持处理本地片段引用（#id）和外部SVG文件引用，能够智能地处理SVG片段和完整SVG文档。
/// 
/// # 处理逻辑
/// 
/// - 支持href和xlink:href属性
/// - 对于外部SVG文件，检索并解析SVG内容
/// - 对于SVG片段引用（#id），提取特定的symbol元素
/// - 对于完整SVG引用，嵌入整个SVG DOM结构
/// - 处理位图图像时创建data URL
/// 
/// # 特殊功能
/// 
/// 这是最复杂的SVG处理器之一，因为它需要：
/// - 解析外部SVG文档
/// - 处理SVG片段引用
/// - 重新组织DOM结构
/// - 保持SVG引用的语义正确性
pub struct UseElementHandler;

impl ElementHandler for UseElementHandler {
    /// 检查是否可以处理指定的元素
    /// 
    /// # 参数
    /// 
    /// * `element_name` - HTML元素名称
    /// 
    /// # 返回值
    /// 
    /// 如果元素名为"use"则返回true，否则返回false
    fn can_handle(&self, element_name: &str) -> bool {
        element_name == "use"
    }

    /// 处理SVG USE元素
    /// 
    /// 处理use元素的href属性，支持引用外部SVG文件或SVG片段。
    /// 根据配置决定是否嵌入资源或移除引用。
    /// 
    /// # 参数
    /// 
    /// * `session` - 当前会话，包含配置选项和缓存
    /// * `document_url` - 当前文档的URL，用于解析相对路径
    /// * `node` - 要处理的use DOM节点
    fn handle(&self, session: &mut Session, document_url: &Url, node: &Handle) {
        // SVG use元素支持的href属性名称（包括已弃用的xlink:href）
        let attr_names: [&str; 2] = ["href", "xlink:href"];

        for attr_name in attr_names.into_iter() {
            if let Some(use_attr_href_value) = get_node_attr(node, attr_name) {
                if session.options.no_images {
                    // 如果禁用图像，移除href属性
                    set_node_attr(node, attr_name, None);
                } else {
                    // 处理SVG use元素的复杂逻辑
                    self.handle_svg_use(
                        session,
                        document_url,
                        node,
                        attr_name,
                        &use_attr_href_value,
                    );
                }
            }
        }
    }
}

impl UseElementHandler {
    /// 处理SVG use元素的具体逻辑
    /// 
    /// 根据引用的资源类型（SVG或位图）采用不同的处理策略。
    /// 对于SVG资源，进行进一步的解析和嵌入；对于位图资源，创建data URL。
    /// 
    /// # 参数
    /// 
    /// * `session` - 当前会话，用于检索资源
    /// * `document_url` - 当前文档URL，用于解析相对路径
    /// * `node` - use元素的DOM节点
    /// * `attr_name` - 属性名（href或xlink:href）
    /// * `use_attr_href_value` - href属性的值
    fn handle_svg_use(
        &self,
        session: &mut Session,
        document_url: &Url,
        node: &Handle,
        attr_name: &str,
        use_attr_href_value: &str,
    ) {
        // 解析为绝对URL
        let image_asset_url: Url = resolve_url(document_url, use_attr_href_value);

        match session.retrieve_asset(document_url, &image_asset_url) {
            Ok((data, final_url, media_type, charset)) => {
                if media_type == "image/svg+xml" {
                    // 处理SVG资源
                    self.handle_svg_asset(session, node, attr_name, &image_asset_url, &data);
                } else {
                    // 处理位图图像，嵌入为data URL
                    let image_asset_data: Url =
                        create_data_url(&media_type, &charset, &data, &final_url);
                    set_node_attr(node, attr_name, Some(image_asset_data.to_string()));
                }
            }
            Err(_) => {
                // 资源检索失败，保持原始URL
                set_node_attr(node, attr_name, Some(image_asset_url.to_string()));
            }
        }
    }

    /// 处理SVG资源
    /// 
    /// 解析SVG数据并根据URL是否包含片段（#id）决定处理方式。
    /// 对于片段引用，提取特定的symbol；对于完整引用，嵌入整个SVG。
    /// 
    /// # 参数
    /// 
    /// * `_session` - 当前会话（未使用）
    /// * `node` - use元素的DOM节点
    /// * `attr_name` - 属性名（href或xlink:href）
    /// * `image_asset_url` - 解析后的资源URL
    /// * `data` - SVG文件的二进制数据
    fn handle_svg_asset(
        &self,
        _session: &mut Session,
        node: &Handle,
        attr_name: &str,
        image_asset_url: &Url,
        data: &[u8],
    ) {
        // 解析SVG文档
        let svg_dom: RcDom = parse_document(RcDom::default(), Default::default())
            .from_utf8()
            .read_from(&mut &data[..])
            .unwrap();

        if image_asset_url.fragment().is_some() {
            // 处理SVG片段引用（#symbol-id）
            self.handle_svg_fragment(node, attr_name, image_asset_url, &svg_dom);
        } else {
            // 处理完整SVG文档
            self.handle_full_svg(node, &svg_dom);
        }
    }

    /// 处理SVG片段引用
    /// 
    /// 当use元素引用外部SVG文件中的特定片段（通过#id）时，提取该片段的symbol元素
    /// 并将其作为子元素添加到当前文档中，同时更新引用为本地引用。
    /// 
    /// # 参数
    /// 
    /// * `node` - use元素的DOM节点
    /// * `attr_name` - 属性名（href或xlink:href）
    /// * `image_asset_url` - 包含片段ID的资源URL
    /// * `svg_dom` - 解析后的SVG文档DOM
    /// 
    /// # 处理步骤
    /// 
    /// 1. 创建新的symbol元素作为容器
    /// 2. 在SVG文档中查找匹配ID的symbol元素
    /// 3. 将找到的symbol内容重新父化到新symbol中
    /// 4. 设置symbol的ID属性
    /// 5. 更新use元素的href为本地引用
    /// 6. 将新symbol添加为use元素的子元素
    fn handle_svg_fragment(
        &self,
        node: &Handle,
        attr_name: &str,
        image_asset_url: &Url,
        svg_dom: &RcDom,
    ) {
        // 创建一个新的symbol元素来承载引用的片段
        let single_symbol_node = create_element(
            svg_dom,
            QualName::new(None, ns!(), LocalName::from("symbol")),
            vec![],
        );

        // 在SVG文档中查找匹配ID的symbol元素
        for symbol_node in find_nodes(
            &svg_dom.document,
            vec!["html", "body", "svg", "defs", "symbol"],
        )
        .iter()
        {
            if get_node_attr(symbol_node, "id").unwrap_or_default()
                == image_asset_url.fragment().unwrap()
            {
                // 将找到的symbol内容移到新的symbol中
                svg_dom.reparent_children(symbol_node, &single_symbol_node);
                // 设置新symbol的ID
                set_node_attr(
                    &single_symbol_node,
                    "id",
                    Some(image_asset_url.fragment().unwrap().to_string()),
                );
                // 更新use元素的href属性为本地引用
                set_node_attr(
                    node,
                    attr_name,
                    Some(format!("#{}", image_asset_url.fragment().unwrap())),
                );
                break;
            }
        }

        // 将新symbol添加为use元素的子元素
        node.children.borrow_mut().push(single_symbol_node.clone());
    }

    /// 处理完整SVG引用
    /// 
    /// 当use元素引用整个外部SVG文件时，将SVG文档的完整内容嵌入到use元素中。
    /// 这实际上是用整个SVG文档的内容替换use元素。
    /// 
    /// # 参数
    /// 
    /// * `node` - use元素的DOM节点
    /// * `svg_dom` - 解析后的SVG文档DOM
    /// 
    /// # 处理步骤
    /// 
    /// 1. 在SVG文档中查找根svg元素
    /// 2. 将svg元素的所有子元素重新父化到use元素中
    /// 3. 实现完整SVG内容的嵌入
    fn handle_full_svg(&self, node: &Handle, svg_dom: &RcDom) {
        // 查找SVG文档的根svg元素并将其内容移到use元素中
        if let Some(svg_node) = find_nodes(&svg_dom.document, vec!["html", "body", "svg"]).first() {
            svg_dom.reparent_children(svg_node, node);
        }
    }
}

/// SOURCE元素处理器
/// 
/// 处理HTML `<source>` 元素，这些元素通常用于`<audio>`、`<video>`和`<picture>`元素中，
/// 提供多个媒体资源选项。根据父元素类型采用不同的处理策略。
/// 
/// # 处理逻辑
/// 
/// - 对于audio父元素：处理src属性，嵌入音频资源或清空
/// - 对于video父元素：处理src属性，嵌入视频资源或清空
/// - 对于picture父元素：处理srcset属性，嵌入响应式图像集
/// - 根据session配置决定是否嵌入对应类型的资源
pub struct SourceElementHandler;

impl ElementHandler for SourceElementHandler {
    /// 检查是否可以处理指定的元素
    /// 
    /// # 参数
    /// 
    /// * `element_name` - HTML元素名称
    /// 
    /// # 返回值
    /// 
    /// 如果元素名为"source"则返回true，否则返回false
    fn can_handle(&self, element_name: &str) -> bool {
        element_name == "source"
    }

    /// 处理SOURCE元素
    /// 
    /// 根据父元素类型处理source元素的src和srcset属性。
    /// 对于不同的父元素（audio、video、picture）采用相应的处理策略。
    /// 
    /// # 参数
    /// 
    /// * `session` - 当前会话，包含配置选项和缓存
    /// * `document_url` - 当前文档的URL，用于解析相对路径
    /// * `node` - 要处理的source DOM节点
    fn handle(&self, session: &mut Session, document_url: &Url, node: &Handle) {
        // 获取父元素信息来决定处理策略
        let parent_node = get_parent_node(node);
        let parent_node_name: &str = get_node_name(&parent_node).unwrap_or_default();

        // 处理src属性（针对audio和video父元素）
        self.handle_source_src(session, document_url, node, parent_node_name);
        // 处理srcset属性（针对picture父元素）
        self.handle_source_srcset(session, document_url, node, parent_node_name);
    }
}

impl SourceElementHandler {
    /// 处理source元素的src属性
    /// 
    /// 根据父元素类型（audio或video）处理src属性中的媒体资源。
    /// 根据session配置决定是否嵌入对应类型的媒体资源。
    /// 
    /// # 参数
    /// 
    /// * `session` - 当前会话，包含配置选项
    /// * `document_url` - 当前文档URL，用于解析相对路径
    /// * `node` - source元素的DOM节点
    /// * `parent_node_name` - 父元素的名称
    fn handle_source_src(
        &self,
        session: &mut Session,
        document_url: &Url,
        node: &Handle,
        parent_node_name: &str,
    ) {
        if let Some(source_attr_src_value) = get_node_attr(node, "src") {
            match parent_node_name {
                "audio" => {
                    if session.options.no_audio {
                        // 禁用音频时清空src属性
                        set_node_attr(node, "src", None);
                    } else {
                        // 嵌入音频资源
                        retrieve_and_embed_asset(
                            session,
                            document_url,
                            node,
                            "src",
                            &source_attr_src_value,
                        );
                    }
                }
                "video" => {
                    if session.options.no_video {
                        // 禁用视频时清空src属性
                        set_node_attr(node, "src", None);
                    } else {
                        // 嵌入视频资源
                        retrieve_and_embed_asset(
                            session,
                            document_url,
                            node,
                            "src",
                            &source_attr_src_value,
                        );
                    }
                }
                _ => {
                    // 其他父元素不处理
                }
            }
        }
    }

    /// 处理source元素的srcset属性
    /// 
    /// 仅对picture父元素下的source元素处理srcset属性，该属性包含
    /// 多个响应式图像资源。根据session配置决定是否嵌入图像资源。
    /// 
    /// # 参数
    /// 
    /// * `session` - 当前会话，包含配置选项
    /// * `document_url` - 当前文档URL，用于解析相对路径
    /// * `node` - source元素的DOM节点
    /// * `parent_node_name` - 父元素的名称
    /// 
    /// # 注意事项
    /// 
    /// srcset属性包含多个图像资源和对应的尺寸/密度描述符，
    /// 需要特殊的解析和嵌入逻辑。
    fn handle_source_srcset(
        &self,
        session: &mut Session,
        document_url: &Url,
        node: &Handle,
        parent_node_name: &str,
    ) {
        if let Some(source_attr_srcset_value) = get_node_attr(node, "srcset") {
            // 仅处理picture元素下的srcset属性
            if parent_node_name == "picture" && !source_attr_srcset_value.is_empty() {
                if session.options.no_images {
                    // 禁用图像时设置为空图像data URL
                    set_node_attr(node, "srcset", Some(EMPTY_IMAGE_DATA_URL.to_string()));
                } else {
                    // 嵌入srcset中的所有图像资源
                    let resolved_srcset: String =
                        embed_srcset(session, document_url, &source_attr_srcset_value);
                    set_node_attr(node, "srcset", Some(resolved_srcset));
                }
            }
        }
    }
}

/// FORM元素处理器
/// 
/// 处理HTML `<form>` 元素，主要负责将表单的action属性从相对URL转换为绝对URL。
/// 这确保了在单文件HTML中表单提交仍能正确工作。
/// 
/// # 处理逻辑
/// 
/// - 检查form元素的action属性
/// - 将相对URL解析为绝对URL
/// - 更新action属性为解析后的绝对URL
/// 
/// # 示例
/// 
/// ```html
/// <!-- 原始表单 -->
/// <form action="/submit">
/// 
/// <!-- 处理后 -->
/// <form action="https://example.com/submit">
/// ```
pub struct FormElementHandler;

impl ElementHandler for FormElementHandler {
    /// 检查是否可以处理指定的元素
    /// 
    /// # 参数
    /// 
    /// * `element_name` - HTML元素名称
    /// 
    /// # 返回值
    /// 
    /// 如果元素名为"form"则返回true，否则返回false
    fn can_handle(&self, element_name: &str) -> bool {
        element_name == "form"
    }

    /// 处理FORM元素
    /// 
    /// 将表单的action属性从相对URL转换为绝对URL，确保表单提交功能正常。
    /// 
    /// # 参数
    /// 
    /// * `_session` - 当前会话（未使用）
    /// * `document_url` - 当前文档的URL，用于解析相对路径
    /// * `node` - 要处理的form DOM节点
    fn handle(&self, _session: &mut Session, document_url: &Url, node: &Handle) {
        if let Some(form_attr_action_value) = get_node_attr(node, "action") {
            // 将action属性修改为完整的绝对URL
            let form_action_full_url: Url = resolve_url(document_url, &form_attr_action_value);
            set_node_attr(node, "action", Some(form_action_full_url.to_string()));
        }
    }
}

/// FRAME/IFRAME元素处理器
/// 
/// 处理HTML `<frame>` 和 `<iframe>` 元素，这些元素用于嵌入其他HTML文档。
/// 支持嵌入框架内容或根据配置禁用框架功能。
/// 
/// # 处理逻辑
/// 
/// - 检查frame或iframe元素的src属性
/// - 如果配置了no_frames选项，清空src属性
/// - 否则检索并嵌入框架内容（递归处理）
/// - 忽略空的src属性以避免无限循环
/// 
/// # 注意事项
/// 
/// 框架处理可能会导致递归加载，因此需要特别小心处理空源和循环引用。
pub struct FrameElementHandler;

impl ElementHandler for FrameElementHandler {
    /// 检查是否可以处理指定的元素
    /// 
    /// # 参数
    /// 
    /// * `element_name` - HTML元素名称
    /// 
    /// # 返回值
    /// 
    /// 如果元素名为"frame"或"iframe"则返回true，否则返回false
    fn can_handle(&self, element_name: &str) -> bool {
        element_name == "frame" || element_name == "iframe"
    }

    /// 处理FRAME/IFRAME元素
    /// 
    /// 处理框架元素的src属性，根据配置决定是否嵌入框架内容。
    /// 特别处理空源的情况以避免无限循环。
    /// 
    /// # 参数
    /// 
    /// * `session` - 当前会话，包含配置选项和缓存
    /// * `document_url` - 当前文档的URL，用于解析相对路径
    /// * `node` - 要处理的frame/iframe DOM节点
    fn handle(&self, session: &mut Session, document_url: &Url, node: &Handle) {
        if let Some(frame_attr_src_value) = get_node_attr(node, "src") {
            if session.options.no_frames {
                // 禁用框架时清空src属性
                set_node_attr(node, "src", Some("".to_string()));
            } else {
                // 忽略空源的框架（它们会导致无限循环）
                if !frame_attr_src_value.trim().is_empty() {
                    retrieve_and_embed_asset(
                        session,
                        document_url,
                        node,
                        "src",
                        &frame_attr_src_value,
                    );
                }
            }
        }
    }
}

/// AUDIO元素处理器
/// 
/// 处理HTML `<audio>` 元素，负责嵌入音频资源或根据配置移除音频内容。
/// 支持处理直接在audio元素上设置的src属性。
/// 
/// # 处理逻辑
/// 
/// - 检查audio元素的src属性
/// - 如果配置了no_audio选项，移除src属性
/// - 否则检索并嵌入音频资源为data URL
/// 
/// # 注意事项
/// 
/// audio元素还可能包含source子元素，这些由SourceElementHandler处理。
pub struct AudioElementHandler;

impl ElementHandler for AudioElementHandler {
    /// 检查是否可以处理指定的元素
    /// 
    /// # 参数
    /// 
    /// * `element_name` - HTML元素名称
    /// 
    /// # 返回值
    /// 
    /// 如果元素名为"audio"则返回true，否则返回false
    fn can_handle(&self, element_name: &str) -> bool {
        element_name == "audio"
    }

    /// 处理AUDIO元素
    /// 
    /// 处理audio元素的src属性，根据配置决定是否嵌入音频资源。
    /// 
    /// # 参数
    /// 
    /// * `session` - 当前会话，包含配置选项和缓存
    /// * `document_url` - 当前文档的URL，用于解析相对路径
    /// * `node` - 要处理的audio DOM节点
    fn handle(&self, session: &mut Session, document_url: &Url, node: &Handle) {
        // 嵌入音频资源
        if let Some(audio_attr_src_value) = get_node_attr(node, "src") {
            if session.options.no_audio {
                // 禁用音频时移除src属性
                set_node_attr(node, "src", None);
            } else {
                // 检索并嵌入音频资源
                retrieve_and_embed_asset(session, document_url, node, "src", &audio_attr_src_value);
            }
        }
    }
}

/// VIDEO元素处理器
/// 
/// 处理HTML `<video>` 元素，负责嵌入视频资源和视频海报图像。
/// 视频元素比音频元素更复杂，因为它还支持poster属性来显示预览图像。
/// 
/// # 处理逻辑
/// 
/// - 处理src属性：嵌入视频资源或根据配置移除
/// - 处理poster属性：嵌入海报图像或根据配置移除
/// - 支持通过source子元素提供的多个视频源
/// 
/// # 注意事项
/// 
/// video元素可能同时具有src属性和source子元素，source元素由SourceElementHandler处理。
pub struct VideoElementHandler;

impl ElementHandler for VideoElementHandler {
    /// 检查是否可以处理指定的元素
    /// 
    /// # 参数
    /// 
    /// * `element_name` - HTML元素名称
    /// 
    /// # 返回值
    /// 
    /// 如果元素名为"video"则返回true，否则返回false
    fn can_handle(&self, element_name: &str) -> bool {
        element_name == "video"
    }

    /// 处理VIDEO元素
    /// 
    /// 处理video元素的src和poster属性，分别嵌入视频资源和海报图像。
    /// 
    /// # 参数
    /// 
    /// * `session` - 当前会话，包含配置选项和缓存
    /// * `document_url` - 当前文档的URL，用于解析相对路径
    /// * `node` - 要处理的video DOM节点
    fn handle(&self, session: &mut Session, document_url: &Url, node: &Handle) {
        // 处理视频源
        self.handle_video_src(session, document_url, node);
        // 处理海报图像
        self.handle_video_poster(session, document_url, node);
    }
}

impl VideoElementHandler {
    /// 处理video元素的src属性
    /// 
    /// 处理video元素直接设置的src属性中的视频资源。
    /// 根据session配置决定是否嵌入视频资源。
    /// 
    /// # 参数
    /// 
    /// * `session` - 当前会话，包含配置选项
    /// * `document_url` - 当前文档URL，用于解析相对路径
    /// * `node` - video元素的DOM节点
    fn handle_video_src(&self, session: &mut Session, document_url: &Url, node: &Handle) {
        // 嵌入视频资源
        if let Some(video_attr_src_value) = get_node_attr(node, "src") {
            if session.options.no_video {
                // 禁用视频时移除src属性
                set_node_attr(node, "src", None);
            } else {
                // 检索并嵌入视频资源
                retrieve_and_embed_asset(session, document_url, node, "src", &video_attr_src_value);
            }
        }
    }

    /// 处理video元素的poster属性
    /// 
    /// 处理video元素的poster属性中的海报图像资源。
    /// poster属性用于在视频加载或播放前显示一个预览图像。
    /// 
    /// # 参数
    /// 
    /// * `session` - 当前会话，包含配置选项
    /// * `document_url` - 当前文档URL，用于解析相对路径
    /// * `node` - video元素的DOM节点
    /// 
    /// # 注意事项
    /// 
    /// - 跳过空的poster属性以避免不必要的处理
    /// - poster图像受no_images配置影响，而非no_video配置
    fn handle_video_poster(&self, session: &mut Session, document_url: &Url, node: &Handle) {
        // 嵌入海报图像
        if let Some(video_attr_poster_value) = get_node_attr(node, "poster") {
            // 跳过空源的海报
            if !video_attr_poster_value.is_empty() {
                if session.options.no_images {
                    // 禁用图像时设置为空图像data URL
                    set_node_attr(node, "poster", Some(EMPTY_IMAGE_DATA_URL.to_string()));
                } else {
                    // 检索并嵌入海报图像
                    retrieve_and_embed_asset(
                        session,
                        document_url,
                        node,
                        "poster",
                        &video_attr_poster_value,
                    );
                }
            }
        }
    }
}

/// ANCHOR/AREA元素处理器
/// 
/// 处理HTML `<a>` 和 `<area>` 元素，这些元素用于创建超链接和图像映射区域。
/// 主要处理href属性，将相对URL转换为绝对URL，并特殊处理JavaScript链接。
/// 
/// # 处理逻辑
/// 
/// - 对于JavaScript链接（javascript:）：根据no_js配置决定是否替换为空JS调用
/// - 对于锈点链接（#）：保持原样，不做修改
/// - 对于已有协议的绝对URL：保持原样
/// - 对于相对URL：转换为绝对URL
/// - 特殊协议链接（如mailto:）：保持原样
/// 
/// # 示例
/// 
/// ```html
/// <!-- 原始链接 -->
/// <a href="/page">链接</a>
/// <a href="#section">锦点</a>
/// <a href="javascript:alert('hello')">脚本</a>
/// 
/// <!-- 处理后 -->
/// <a href="https://example.com/page">链接</a>
/// <a href="#section">锦点</a>  <!-- 保持不变 -->
/// <a href="javascript:;">脚本</a>  <!-- 如果禁用JS -->
/// ```
pub struct AnchorElementHandler;

impl ElementHandler for AnchorElementHandler {
    /// 检查是否可以处理指定的元素
    /// 
    /// # 参数
    /// 
    /// * `element_name` - HTML元素名称
    /// 
    /// # 返回值
    /// 
    /// 如果元素名为"a"或"area"则返回true，否则返回false
    fn can_handle(&self, element_name: &str) -> bool {
        element_name == "a" || element_name == "area"
    }

    /// 处理ANCHOR/AREA元素
    /// 
    /// 处理超链接元素的href属性，根据链接类型采用不同的处理策略。
    /// 确保在单文件HTML中链接仍能正确指向目标资源。
    /// 
    /// # 参数
    /// 
    /// * `session` - 当前会话，包含配置选项
    /// * `document_url` - 当前文档的URL，用于解析相对路径
    /// * `node` - 要处理的a或area DOM节点
    fn handle(&self, session: &mut Session, document_url: &Url, node: &Handle) {
        if let Some(anchor_attr_href_value) = get_node_attr(node, "href") {
            if anchor_attr_href_value.trim().starts_with("javascript:") {
                // 处理JavaScript链接
                if session.options.no_js {
                    // 替换为空JS调用以保持原始行为
                    set_node_attr(node, "href", Some("javascript:;".to_string()));
                }
            } else {
                // 不触磬mailto:链接或以#开头的锢点链接
                if !anchor_attr_href_value.starts_with('#')
                    && !is_url_and_has_protocol(&anchor_attr_href_value)
                {
                    // 将相对URL转换为绝对URL
                    let href_full_url: Url = resolve_url(document_url, &anchor_attr_href_value);
                    set_node_attr(node, "href", Some(href_full_url.to_string()));
                }
            }
        }
    }
}

/// NOSCRIPT元素处理器
/// 
/// 处理HTML `<noscript>` 元素，这些元素包含在JavaScript禁用时显示的备用HTML内容。
/// 由于noscript元素的内容通常作为文本节点存储，需要特殊处理来解析并嵌入其中的资源。
/// 
/// # 处理逻辑
/// 
/// 1. 遍历noscript元素的所有子节点
/// 2. 对于文本节点，获取其HTML内容
/// 3. 将HTML内容解析为DOM结构
/// 4. 递归地处理DOM中的资源嵌入
/// 5. 将处理后的HTML序列化回文本
/// 6. 替换原始的noscript内容
/// 
/// # 复杂性
/// 
/// 这是最复杂的元素处理器之一，因为它需要：
/// - 解析嵌套的HTML内容
/// - 递归地处理嵌套的DOM结构
/// - HTML序列化和反序列化
/// - 处理可能的编码问题
pub struct NoscriptElementHandler;

impl ElementHandler for NoscriptElementHandler {
    /// 检查是否可以处理指定的元素
    /// 
    /// # 参数
    /// 
    /// * `element_name` - HTML元素名称
    /// 
    /// # 返回值
    /// 
    /// 如果元素名为"noscript"则返回true，否则返回false
    fn can_handle(&self, element_name: &str) -> bool {
        element_name == "noscript"
    }

    /// 处理NOSCRIPT元素
    /// 
    /// 处理noscript元素的内容，解析其中的HTML并递归地嵌入资源。
    /// 这确保了在JavaScript禁用环境下显示的备用内容也能正确地嵌入资源。
    /// 
    /// # 参数
    /// 
    /// * `session` - 当前会话，包含配置选项和缓存
    /// * `document_url` - 当前文档的URL，用于解析相对路径
    /// * `node` - 要处理的noscript DOM节点
    /// 
    /// # 实现细节
    /// 
    /// 该方法使用了walker::walk函数来递归地处理noscript内容中的DOM节点，
    /// 确保所有嵌套的资源都能被正确地嵌入。
    fn handle(&self, session: &mut Session, document_url: &Url, node: &Handle) {
        use super::walker::walk;

        // 遍历noscript元素的所有子节点
        for child_node in node.children.borrow_mut().iter_mut() {
            if let NodeData::Text { ref contents } = child_node.data {
                // 获取NOSCRIPT节点的内容
                let mut noscript_contents = contents.borrow_mut();
                // 将NOSCRIPT节点的内容解析为DOM
                let noscript_contents_dom: RcDom =
                    html_to_dom(noscript_contents.as_bytes(), "".to_string());
                // 嵌入NOSCRIPT节点内容中的资源
                walk(session, document_url, &noscript_contents_dom.document);
                // 清空原始内容
                noscript_contents.clear();
                // 将包含嵌入资源的HTML插入到NOSCRIPT节点中
                if let Some(html) = get_child_node_by_name(&noscript_contents_dom.document, "html")
                {
                    if let Some(body) = get_child_node_by_name(&html, "body") {
                        let mut buf: Vec<u8> = Vec::new();
                        use html5ever::serialize::{serialize, SerializeOpts};
                        use markup5ever_rcdom::SerializableHandle;
                        let serializable: SerializableHandle = body.into();
                        serialize(&mut buf, &serializable, SerializeOpts::default())
                            .expect("Unable to serialize DOM into buffer");
                        let result = String::from_utf8_lossy(&buf);
                        noscript_contents.push_slice(&result);
                    }
                }
            }
        }
    }
}
