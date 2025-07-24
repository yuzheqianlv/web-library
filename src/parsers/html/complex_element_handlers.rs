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
pub struct InputElementHandler;

impl ElementHandler for InputElementHandler {
    fn can_handle(&self, element_name: &str) -> bool {
        element_name == "input"
    }

    fn handle(&self, session: &mut Session, document_url: &Url, node: &Handle) {
        if let Some(input_attr_type_value) = get_node_attr(node, "type") {
            if input_attr_type_value.eq_ignore_ascii_case("image") {
                if let Some(input_attr_src_value) = get_node_attr(node, "src") {
                    if session.options.no_images || input_attr_src_value.is_empty() {
                        let value = if input_attr_src_value.is_empty() {
                            ""
                        } else {
                            EMPTY_IMAGE_DATA_URL
                        };
                        set_node_attr(node, "src", Some(value.to_string()));
                    } else {
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
pub struct SvgElementHandler;

impl ElementHandler for SvgElementHandler {
    fn can_handle(&self, element_name: &str) -> bool {
        element_name == "svg"
    }

    fn handle(&self, session: &mut Session, _document_url: &Url, node: &Handle) {
        if session.options.no_images {
            // Remove all children
            node.children.borrow_mut().clear();
        }
    }
}

/// IMAGE元素处理器（SVG中的image元素）
pub struct ImageElementHandler;

impl ElementHandler for ImageElementHandler {
    fn can_handle(&self, element_name: &str) -> bool {
        element_name == "image"
    }

    fn handle(&self, session: &mut Session, document_url: &Url, node: &Handle) {
        let attr_names: [&str; 2] = ["href", "xlink:href"];

        for attr_name in attr_names.into_iter() {
            if let Some(image_attr_href_value) = get_node_attr(node, attr_name) {
                if session.options.no_images {
                    set_node_attr(node, attr_name, None);
                } else {
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
pub struct UseElementHandler;

impl ElementHandler for UseElementHandler {
    fn can_handle(&self, element_name: &str) -> bool {
        element_name == "use"
    }

    fn handle(&self, session: &mut Session, document_url: &Url, node: &Handle) {
        let attr_names: [&str; 2] = ["href", "xlink:href"];

        for attr_name in attr_names.into_iter() {
            if let Some(use_attr_href_value) = get_node_attr(node, attr_name) {
                if session.options.no_images {
                    set_node_attr(node, attr_name, None);
                } else {
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
    fn handle_svg_use(
        &self,
        session: &mut Session,
        document_url: &Url,
        node: &Handle,
        attr_name: &str,
        use_attr_href_value: &str,
    ) {
        let image_asset_url: Url = resolve_url(document_url, use_attr_href_value);

        match session.retrieve_asset(document_url, &image_asset_url) {
            Ok((data, final_url, media_type, charset)) => {
                if media_type == "image/svg+xml" {
                    self.handle_svg_asset(session, node, attr_name, &image_asset_url, &data);
                } else {
                    // It's likely a raster image; embed it as data URL
                    let image_asset_data: Url =
                        create_data_url(&media_type, &charset, &data, &final_url);
                    set_node_attr(node, attr_name, Some(image_asset_data.to_string()));
                }
            }
            Err(_) => {
                set_node_attr(node, attr_name, Some(image_asset_url.to_string()));
            }
        }
    }

    fn handle_svg_asset(
        &self,
        _session: &mut Session,
        node: &Handle,
        attr_name: &str,
        image_asset_url: &Url,
        data: &[u8],
    ) {
        // Parse SVG
        let svg_dom: RcDom = parse_document(RcDom::default(), Default::default())
            .from_utf8()
            .read_from(&mut &data[..])
            .unwrap();

        if image_asset_url.fragment().is_some() {
            self.handle_svg_fragment(node, attr_name, image_asset_url, &svg_dom);
        } else {
            self.handle_full_svg(node, &svg_dom);
        }
    }

    fn handle_svg_fragment(
        &self,
        node: &Handle,
        attr_name: &str,
        image_asset_url: &Url,
        svg_dom: &RcDom,
    ) {
        // Take only that one #fragment symbol from SVG and replace this image|use with that node
        let single_symbol_node = create_element(
            svg_dom,
            QualName::new(None, ns!(), LocalName::from("symbol")),
            vec![],
        );

        for symbol_node in find_nodes(
            &svg_dom.document,
            vec!["html", "body", "svg", "defs", "symbol"],
        )
        .iter()
        {
            if get_node_attr(symbol_node, "id").unwrap_or_default()
                == image_asset_url.fragment().unwrap()
            {
                svg_dom.reparent_children(symbol_node, &single_symbol_node);
                set_node_attr(
                    &single_symbol_node,
                    "id",
                    Some(image_asset_url.fragment().unwrap().to_string()),
                );
                set_node_attr(
                    node,
                    attr_name,
                    Some(format!("#{}", image_asset_url.fragment().unwrap())),
                );
                break;
            }
        }

        node.children.borrow_mut().push(single_symbol_node.clone());
    }

    fn handle_full_svg(&self, node: &Handle, svg_dom: &RcDom) {
        // Replace this image|use with whole DOM of that SVG file
        if let Some(svg_node) = find_nodes(&svg_dom.document, vec!["html", "body", "svg"]).first() {
            svg_dom.reparent_children(svg_node, node);
        }
    }
}

/// SOURCE元素处理器
pub struct SourceElementHandler;

impl ElementHandler for SourceElementHandler {
    fn can_handle(&self, element_name: &str) -> bool {
        element_name == "source"
    }

    fn handle(&self, session: &mut Session, document_url: &Url, node: &Handle) {
        let parent_node = get_parent_node(node);
        let parent_node_name: &str = get_node_name(&parent_node).unwrap_or_default();

        self.handle_source_src(session, document_url, node, parent_node_name);
        self.handle_source_srcset(session, document_url, node, parent_node_name);
    }
}

impl SourceElementHandler {
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
                        set_node_attr(node, "src", None);
                    } else {
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
                        set_node_attr(node, "src", None);
                    } else {
                        retrieve_and_embed_asset(
                            session,
                            document_url,
                            node,
                            "src",
                            &source_attr_src_value,
                        );
                    }
                }
                _ => {}
            }
        }
    }

    fn handle_source_srcset(
        &self,
        session: &mut Session,
        document_url: &Url,
        node: &Handle,
        parent_node_name: &str,
    ) {
        if let Some(source_attr_srcset_value) = get_node_attr(node, "srcset") {
            if parent_node_name == "picture" && !source_attr_srcset_value.is_empty() {
                if session.options.no_images {
                    set_node_attr(node, "srcset", Some(EMPTY_IMAGE_DATA_URL.to_string()));
                } else {
                    let resolved_srcset: String =
                        embed_srcset(session, document_url, &source_attr_srcset_value);
                    set_node_attr(node, "srcset", Some(resolved_srcset));
                }
            }
        }
    }
}

/// FORM元素处理器
pub struct FormElementHandler;

impl ElementHandler for FormElementHandler {
    fn can_handle(&self, element_name: &str) -> bool {
        element_name == "form"
    }

    fn handle(&self, _session: &mut Session, document_url: &Url, node: &Handle) {
        if let Some(form_attr_action_value) = get_node_attr(node, "action") {
            // Modify action property to ensure it's a full URL
            let form_action_full_url: Url = resolve_url(document_url, &form_attr_action_value);
            set_node_attr(node, "action", Some(form_action_full_url.to_string()));
        }
    }
}

/// FRAME/IFRAME元素处理器
pub struct FrameElementHandler;

impl ElementHandler for FrameElementHandler {
    fn can_handle(&self, element_name: &str) -> bool {
        element_name == "frame" || element_name == "iframe"
    }

    fn handle(&self, session: &mut Session, document_url: &Url, node: &Handle) {
        if let Some(frame_attr_src_value) = get_node_attr(node, "src") {
            if session.options.no_frames {
                // Empty the src attribute
                set_node_attr(node, "src", Some("".to_string()));
            } else {
                // Ignore (i)frames with empty source (they cause infinite loops)
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
pub struct AudioElementHandler;

impl ElementHandler for AudioElementHandler {
    fn can_handle(&self, element_name: &str) -> bool {
        element_name == "audio"
    }

    fn handle(&self, session: &mut Session, document_url: &Url, node: &Handle) {
        // Embed audio source
        if let Some(audio_attr_src_value) = get_node_attr(node, "src") {
            if session.options.no_audio {
                set_node_attr(node, "src", None);
            } else {
                retrieve_and_embed_asset(session, document_url, node, "src", &audio_attr_src_value);
            }
        }
    }
}

/// VIDEO元素处理器
pub struct VideoElementHandler;

impl ElementHandler for VideoElementHandler {
    fn can_handle(&self, element_name: &str) -> bool {
        element_name == "video"
    }

    fn handle(&self, session: &mut Session, document_url: &Url, node: &Handle) {
        self.handle_video_src(session, document_url, node);
        self.handle_video_poster(session, document_url, node);
    }
}

impl VideoElementHandler {
    fn handle_video_src(&self, session: &mut Session, document_url: &Url, node: &Handle) {
        // Embed video source
        if let Some(video_attr_src_value) = get_node_attr(node, "src") {
            if session.options.no_video {
                set_node_attr(node, "src", None);
            } else {
                retrieve_and_embed_asset(session, document_url, node, "src", &video_attr_src_value);
            }
        }
    }

    fn handle_video_poster(&self, session: &mut Session, document_url: &Url, node: &Handle) {
        // Embed poster images
        if let Some(video_attr_poster_value) = get_node_attr(node, "poster") {
            // Skip posters with empty source
            if !video_attr_poster_value.is_empty() {
                if session.options.no_images {
                    set_node_attr(node, "poster", Some(EMPTY_IMAGE_DATA_URL.to_string()));
                } else {
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
pub struct AnchorElementHandler;

impl ElementHandler for AnchorElementHandler {
    fn can_handle(&self, element_name: &str) -> bool {
        element_name == "a" || element_name == "area"
    }

    fn handle(&self, session: &mut Session, document_url: &Url, node: &Handle) {
        if let Some(anchor_attr_href_value) = get_node_attr(node, "href") {
            if anchor_attr_href_value.trim().starts_with("javascript:") {
                if session.options.no_js {
                    // Replace with empty JS call to preserve original behavior
                    set_node_attr(node, "href", Some("javascript:;".to_string()));
                }
            } else {
                // Don't touch mailto: links or hrefs which begin with a hash sign
                if !anchor_attr_href_value.starts_with('#')
                    && !is_url_and_has_protocol(&anchor_attr_href_value)
                {
                    let href_full_url: Url = resolve_url(document_url, &anchor_attr_href_value);
                    set_node_attr(node, "href", Some(href_full_url.to_string()));
                }
            }
        }
    }
}

/// NOSCRIPT元素处理器
pub struct NoscriptElementHandler;

impl ElementHandler for NoscriptElementHandler {
    fn can_handle(&self, element_name: &str) -> bool {
        element_name == "noscript"
    }

    fn handle(&self, session: &mut Session, document_url: &Url, node: &Handle) {
        use super::walker::walk;

        for child_node in node.children.borrow_mut().iter_mut() {
            if let NodeData::Text { ref contents } = child_node.data {
                // Get contents of NOSCRIPT node
                let mut noscript_contents = contents.borrow_mut();
                // Parse contents of NOSCRIPT node as DOM
                let noscript_contents_dom: RcDom =
                    html_to_dom(noscript_contents.as_bytes(), "".to_string());
                // Embed assets of NOSCRIPT node contents
                walk(session, document_url, &noscript_contents_dom.document);
                // Get rid of original contents
                noscript_contents.clear();
                // Insert HTML containing embedded assets into NOSCRIPT node
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
