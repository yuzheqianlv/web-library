use html5ever::interface::QualName;
use html5ever::parse_document;
use html5ever::tendril::TendrilSink;
use html5ever::tree_builder::{create_element, TreeSink};
use html5ever::{namespace_url, ns, LocalName};
use markup5ever_rcdom::{Handle, NodeData, RcDom};

use crate::network::session::Session;
use crate::parsers::css::embed_css;
use crate::parsers::js::attr_is_event_handler;
use crate::utils::url::{
    create_data_url, is_url_and_has_protocol, resolve_url, Url, EMPTY_IMAGE_DATA_URL,
};

use super::assets::{embed_srcset, retrieve_and_embed_asset};
use super::dom::get_node_attr;
use super::dom::{find_nodes, get_node_name, get_parent_node, set_node_attr};
use super::parser::{parse_link_type, LinkType};

/// DOM 遍历和处理的核心函数
pub fn walk(session: &mut Session, document_url: &Url, node: &Handle) {
    match node.data {
        NodeData::Document => {
            // Dig deeper
            for child_node in node.children.borrow().iter() {
                walk(session, document_url, child_node);
            }
        }
        NodeData::Element {
            ref name,
            ref attrs,
            ..
        } => {
            match name.local.as_ref() {
                "meta" => {
                    if let Some(meta_attr_http_equiv_value) = get_node_attr(node, "http-equiv") {
                        if meta_attr_http_equiv_value.eq_ignore_ascii_case("refresh")
                            || meta_attr_http_equiv_value.eq_ignore_ascii_case("location")
                        {
                            // Remove http-equiv attributes from META nodes if they're able to control the page
                            set_node_attr(node, "http-equiv", None);
                        }
                    }
                }
                "link" => {
                    let link_node_types: Vec<LinkType> =
                        parse_link_type(&get_node_attr(node, "rel").unwrap_or(String::from("")));

                    if link_node_types.contains(&LinkType::Favicon)
                        || link_node_types.contains(&LinkType::AppleTouchIcon)
                    {
                        // Find and resolve LINK's href attribute
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
                    } else if link_node_types.contains(&LinkType::Stylesheet) {
                        // Resolve LINK's href attribute
                        if let Some(link_attr_href_value) = get_node_attr(node, "href") {
                            if session.options.no_css {
                                set_node_attr(node, "href", None);
                                // Wipe integrity attribute
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
                    } else if link_node_types.contains(&LinkType::Preload)
                        || link_node_types.contains(&LinkType::DnsPrefetch)
                    {
                        // Since all resources are embedded as data URLs, preloading and prefetching are not necessary
                        set_node_attr(node, "rel", None);
                    } else {
                        // Make sure that all other LINKs' href attributes are full URLs
                        if let Some(link_attr_href_value) = get_node_attr(node, "href") {
                            let href_full_url: Url =
                                resolve_url(document_url, &link_attr_href_value);
                            set_node_attr(node, "href", Some(href_full_url.to_string()));
                        }
                    }
                }
                "base" => {
                    if document_url.scheme() == "http" || document_url.scheme() == "https" {
                        // Ensure the BASE node doesn't have a relative URL
                        if let Some(base_attr_href_value) = get_node_attr(node, "href") {
                            let href_full_url: Url =
                                resolve_url(document_url, &base_attr_href_value);
                            set_node_attr(node, "href", Some(href_full_url.to_string()));
                        }
                    }
                }
                "body" => {
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
                "img" => {
                    // Find src and data-src attribute(s)
                    let img_attr_src_value: Option<String> = get_node_attr(node, "src");
                    let img_attr_data_src_value: Option<String> = get_node_attr(node, "data-src");

                    if session.options.no_images {
                        // Put empty images into src and data-src attributes
                        if img_attr_src_value.is_some() {
                            set_node_attr(node, "src", Some(EMPTY_IMAGE_DATA_URL.to_string()));
                        }
                        if img_attr_data_src_value.is_some() {
                            set_node_attr(node, "data-src", Some(EMPTY_IMAGE_DATA_URL.to_string()));
                        }
                    } else if img_attr_src_value.clone().unwrap_or_default().is_empty()
                        && img_attr_data_src_value
                            .clone()
                            .unwrap_or_default()
                            .is_empty()
                    {
                        // Add empty src attribute
                        set_node_attr(node, "src", Some("".to_string()));
                    } else {
                        // Add data URL src attribute
                        let img_full_url: String = if !img_attr_data_src_value
                            .clone()
                            .unwrap_or_default()
                            .is_empty()
                        {
                            img_attr_data_src_value.unwrap_or_default()
                        } else {
                            img_attr_src_value.unwrap_or_default()
                        };
                        retrieve_and_embed_asset(session, document_url, node, "src", &img_full_url);
                    }

                    // Resolve srcset attribute
                    if let Some(img_srcset) = get_node_attr(node, "srcset") {
                        if !img_srcset.is_empty() {
                            let resolved_srcset: String =
                                embed_srcset(session, document_url, &img_srcset);
                            set_node_attr(node, "srcset", Some(resolved_srcset));
                        }
                    }
                }
                "input" => {
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
                "svg" => {
                    if session.options.no_images {
                        // Remove all children
                        node.children.borrow_mut().clear();
                    }
                }
                "image" => {
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
                "use" => {
                    let attr_names: [&str; 2] = ["href", "xlink:href"];

                    for attr_name in attr_names.into_iter() {
                        if let Some(use_attr_href_value) = get_node_attr(node, attr_name) {
                            if session.options.no_images {
                                set_node_attr(node, attr_name, None);
                            } else {
                                let image_asset_url: Url =
                                    resolve_url(document_url, &use_attr_href_value);

                                match session.retrieve_asset(document_url, &image_asset_url) {
                                    Ok((data, final_url, media_type, charset)) => {
                                        if media_type == "image/svg+xml" {
                                            // Parse SVG
                                            let svg_dom: RcDom = parse_document(
                                                RcDom::default(),
                                                Default::default(),
                                            )
                                            .from_utf8()
                                            .read_from(&mut data.as_slice())
                                            .unwrap();

                                            if image_asset_url.fragment().is_some() {
                                                // Take only that one #fragment symbol from SVG and replace this image|use with that node
                                                let single_symbol_node = create_element(
                                                    &svg_dom,
                                                    QualName::new(
                                                        None,
                                                        ns!(),
                                                        LocalName::from("symbol"),
                                                    ),
                                                    vec![],
                                                );
                                                for symbol_node in find_nodes(
                                                    &svg_dom.document,
                                                    vec!["html", "body", "svg", "defs", "symbol"],
                                                )
                                                .iter()
                                                {
                                                    if get_node_attr(symbol_node, "id")
                                                        .unwrap_or_default()
                                                        == image_asset_url.fragment().unwrap()
                                                    {
                                                        svg_dom.reparent_children(
                                                            symbol_node,
                                                            &single_symbol_node,
                                                        );
                                                        set_node_attr(
                                                            &single_symbol_node,
                                                            "id",
                                                            Some(
                                                                image_asset_url
                                                                    .fragment()
                                                                    .unwrap()
                                                                    .to_string(),
                                                            ),
                                                        );

                                                        set_node_attr(
                                                            node,
                                                            attr_name,
                                                            Some(format!(
                                                                "#{}",
                                                                image_asset_url.fragment().unwrap()
                                                            )),
                                                        );

                                                        break;
                                                    }
                                                }

                                                node.children
                                                    .borrow_mut()
                                                    .push(single_symbol_node.clone());
                                            } else {
                                                // Replace this image|use with whole DOM of that SVG file
                                                if let Some(svg_node) = find_nodes(
                                                    &svg_dom.document,
                                                    vec!["html", "body", "svg"],
                                                )
                                                .first()
                                                {
                                                    svg_dom.reparent_children(svg_node, node);
                                                }
                                                // TODO: decide if we resort to using data URL here or stick with embedding the DOM
                                            }
                                        } else {
                                            // It's likely a raster image; embed it as data URL
                                            let image_asset_data: Url = create_data_url(
                                                &media_type,
                                                &charset,
                                                &data,
                                                &final_url,
                                            );
                                            set_node_attr(
                                                node,
                                                attr_name,
                                                Some(image_asset_data.to_string()),
                                            );
                                        }
                                    }
                                    Err(_) => {
                                        set_node_attr(
                                            node,
                                            attr_name,
                                            Some(image_asset_url.to_string()),
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
                "source" => {
                    let parent_node = get_parent_node(node);
                    let parent_node_name: &str = get_node_name(&parent_node).unwrap_or_default();

                    if let Some(source_attr_src_value) = get_node_attr(node, "src") {
                        if parent_node_name == "audio" {
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
                        } else if parent_node_name == "video" {
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
                    }

                    if let Some(source_attr_srcset_value) = get_node_attr(node, "srcset") {
                        if parent_node_name == "picture" && !source_attr_srcset_value.is_empty() {
                            if session.options.no_images {
                                set_node_attr(
                                    node,
                                    "srcset",
                                    Some(EMPTY_IMAGE_DATA_URL.to_string()),
                                );
                            } else {
                                let resolved_srcset: String =
                                    embed_srcset(session, document_url, &source_attr_srcset_value);
                                set_node_attr(node, "srcset", Some(resolved_srcset));
                            }
                        }
                    }
                }
                "a" | "area" => {
                    if let Some(anchor_attr_href_value) = get_node_attr(node, "href") {
                        if anchor_attr_href_value
                            .clone()
                            .trim()
                            .starts_with("javascript:")
                        {
                            if session.options.no_js {
                                // Replace with empty JS call to preserve original behavior
                                set_node_attr(node, "href", Some("javascript:;".to_string()));
                            }
                        } else {
                            // Don't touch mailto: links or hrefs which begin with a hash sign
                            if !anchor_attr_href_value.clone().starts_with('#')
                                && !is_url_and_has_protocol(&anchor_attr_href_value.clone())
                            {
                                let href_full_url: Url =
                                    resolve_url(document_url, &anchor_attr_href_value);
                                set_node_attr(node, "href", Some(href_full_url.to_string()));
                            }
                        }
                    }
                }
                "script" => {
                    // Read values of integrity and src attributes
                    let script_attr_src: &str = &get_node_attr(node, "src").unwrap_or_default();

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
                        retrieve_and_embed_asset(
                            session,
                            document_url,
                            node,
                            "src",
                            script_attr_src,
                        );
                    }
                }
                "style" => {
                    if session.options.no_css {
                        // Empty inner content of STYLE tags
                        node.children.borrow_mut().clear();
                    } else {
                        for child_node in node.children.borrow_mut().iter_mut() {
                            if let NodeData::Text { ref contents } = child_node.data {
                                let mut tendril = contents.borrow_mut();
                                let replacement =
                                    embed_css(session, document_url, tendril.as_ref());
                                tendril.clear();
                                tendril.push_slice(&replacement);
                            }
                        }
                    }
                }
                "form" => {
                    if let Some(form_attr_action_value) = get_node_attr(node, "action") {
                        // Modify action property to ensure it's a full URL
                        let form_action_full_url: Url =
                            resolve_url(document_url, &form_attr_action_value);
                        set_node_attr(node, "action", Some(form_action_full_url.to_string()));
                    }
                }
                "frame" | "iframe" => {
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
                "audio" => {
                    // Embed audio source
                    if let Some(audio_attr_src_value) = get_node_attr(node, "src") {
                        if session.options.no_audio {
                            set_node_attr(node, "src", None);
                        } else {
                            retrieve_and_embed_asset(
                                session,
                                document_url,
                                node,
                                "src",
                                &audio_attr_src_value,
                            );
                        }
                    }
                }
                "video" => {
                    // Embed video source
                    if let Some(video_attr_src_value) = get_node_attr(node, "src") {
                        if session.options.no_video {
                            set_node_attr(node, "src", None);
                        } else {
                            retrieve_and_embed_asset(
                                session,
                                document_url,
                                node,
                                "src",
                                &video_attr_src_value,
                            );
                        }
                    }

                    // Embed poster images
                    if let Some(video_attr_poster_value) = get_node_attr(node, "poster") {
                        // Skip posters with empty source
                        if !video_attr_poster_value.is_empty() {
                            if session.options.no_images {
                                set_node_attr(
                                    node,
                                    "poster",
                                    Some(EMPTY_IMAGE_DATA_URL.to_string()),
                                );
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
                "noscript" => {
                    use super::dom::{get_child_node_by_name, html_to_dom};

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
                            if let Some(html) =
                                get_child_node_by_name(&noscript_contents_dom.document, "html")
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
                _ => {}
            }

            // Process style attributes
            if session.options.no_css {
                // Get rid of style attributes
                set_node_attr(node, "style", None);
            } else {
                // Embed URLs found within the style attribute of this node
                if let Some(node_attr_style_value) = get_node_attr(node, "style") {
                    let embedded_style = embed_css(session, document_url, &node_attr_style_value);
                    set_node_attr(node, "style", Some(embedded_style));
                }
            }

            // Strip all JS from document
            if session.options.no_js {
                let attrs_mut = &mut attrs.borrow_mut();
                // Get rid of JS event attributes
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

            // Dig deeper
            for child_node in node.children.borrow().iter() {
                walk(session, document_url, child_node);
            }
        }
        _ => {
            // Note: in case of options.no_js being set to true, there's no need to worry about
            //       getting rid of comments that may contain scripts, e.g. <!--[if IE]><script>...
            //       since that's not part of W3C standard and therefore gets ignored
            //       by browsers other than IE [5, 9]
        }
    }
}
