use base64::{prelude::BASE64_STANDARD, Engine};
use encoding_rs::Encoding;
use html5ever::interface::{Attribute, QualName};
use html5ever::serialize::{serialize, SerializeOpts};
use html5ever::tendril::format_tendril;
use html5ever::tree_builder::create_element;
use html5ever::{namespace_url, ns, LocalName};
use markup5ever_rcdom::{Handle, NodeData, RcDom, SerializableHandle};
use sha2::{Digest, Sha256, Sha384, Sha512};

use crate::network::session::Session;
use crate::parsers::css::embed_css;
use crate::utils::url::{create_data_url, resolve_url, Url, EMPTY_IMAGE_DATA_URL};

use super::dom::get_node_attr;
use super::dom::{find_nodes, get_node_name, html_to_dom, set_node_attr};
use super::parser::{parse_link_type, parse_srcset, LinkType, SrcSetItem};
use super::walker::walk;

/// 添加 favicon
pub fn add_favicon(document: &Handle, favicon_data_url: String) -> RcDom {
    let mut buf: Vec<u8> = Vec::new();
    serialize(
        &mut buf,
        &SerializableHandle::from(document.clone()),
        SerializeOpts::default(),
    )
    .expect("unable to serialize DOM into buffer");

    let dom = html_to_dom(&buf, "utf-8".to_string());
    for head in find_nodes(&dom.document, vec!["html", "head"]).iter() {
        let favicon_node = create_element(
            &dom,
            QualName::new(None, ns!(), LocalName::from("link")),
            vec![
                Attribute {
                    name: QualName::new(None, ns!(), LocalName::from("rel")),
                    value: format_tendril!("icon"),
                },
                Attribute {
                    name: QualName::new(None, ns!(), LocalName::from("href")),
                    value: format_tendril!("{}", favicon_data_url),
                },
            ],
        );

        // Insert favicon LINK tag into HEAD
        head.children.borrow_mut().push(favicon_node.clone());
    }

    dom
}

/// 检查资源完整性
pub fn check_integrity(data: &[u8], integrity: &str) -> bool {
    if let Some(hash_value) = integrity.strip_prefix("sha256-") {
        let mut hasher = Sha256::new();
        hasher.update(data);
        BASE64_STANDARD.encode(hasher.finalize()) == hash_value
    } else if let Some(hash_value) = integrity.strip_prefix("sha384-") {
        let mut hasher = Sha384::new();
        hasher.update(data);
        BASE64_STANDARD.encode(hasher.finalize()) == hash_value
    } else if let Some(hash_value) = integrity.strip_prefix("sha512-") {
        let mut hasher = Sha512::new();
        hasher.update(data);
        BASE64_STANDARD.encode(hasher.finalize()) == hash_value
    } else {
        false
    }
}

/// 嵌入 srcset 中的资源
pub fn embed_srcset(session: &mut Session, document_url: &Url, srcset: &str) -> String {
    let srcset_items: Vec<SrcSetItem> = parse_srcset(srcset);

    // Embed assets
    let mut result: String = "".to_string();
    let mut i: usize = srcset_items.len();
    for srcset_item in srcset_items {
        if session.options.no_images {
            result.push_str(EMPTY_IMAGE_DATA_URL);
        } else {
            let image_full_url: Url = resolve_url(document_url, srcset_item.path);
            match session.retrieve_asset(document_url, &image_full_url) {
                Ok((data, final_url, media_type, charset)) => {
                    let image_data_url: Url =
                        create_data_url(&media_type, &charset, &data, &final_url);

                    result.push_str(image_data_url.as_ref());
                }
                Err(_) => {
                    if image_full_url.scheme() == "http" || image_full_url.scheme() == "https" {
                        result.push_str(image_full_url.as_ref());
                    } else {
                        result.push_str(EMPTY_IMAGE_DATA_URL);
                    }
                }
            }
        }

        if !srcset_item.descriptor.is_empty() {
            result.push(' ');
            result.push_str(srcset_item.descriptor);
        }

        if i > 1 {
            result.push_str(", ");
        }

        i -= 1;
    }

    result
}

/// 检索并嵌入资源
pub fn retrieve_and_embed_asset(
    session: &mut Session,
    document_url: &Url,
    node: &Handle,
    attr_name: &str,
    attr_value: &str,
) {
    let resolved_url: Url = resolve_url(document_url, attr_value);

    match session.retrieve_asset(&document_url.clone(), &resolved_url) {
        Ok((data, final_url, media_type, charset)) => {
            let node_name: &str = get_node_name(node).unwrap();

            // Check integrity if it's a LINK or SCRIPT element
            let mut ok_to_include: bool = true;
            if node_name == "link" || node_name == "script" {
                // Check integrity
                if let Some(node_integrity_attr_value) = get_node_attr(node, "integrity") {
                    if !node_integrity_attr_value.is_empty() {
                        ok_to_include = check_integrity(&data, &node_integrity_attr_value);
                    }

                    // Wipe the integrity attribute
                    set_node_attr(node, "integrity", None);
                }
            }

            if ok_to_include {
                if node_name == "link"
                    && parse_link_type(&get_node_attr(node, "rel").unwrap_or(String::from("")))
                        .contains(&LinkType::Stylesheet)
                {
                    let stylesheet: String;
                    if let Some(encoding) = Encoding::for_label(charset.as_bytes()) {
                        let (string, _, _) = encoding.decode(&data);
                        stylesheet = string.to_string();
                    } else {
                        stylesheet = String::from_utf8_lossy(&data).to_string();
                    }

                    // Stylesheet LINK elements require special treatment
                    let css: String = embed_css(session, &final_url, &stylesheet);

                    // Create and embed data URL
                    let css_data_url =
                        create_data_url(&media_type, &charset, css.as_bytes(), &final_url);
                    set_node_attr(node, attr_name, Some(css_data_url.to_string()));
                } else if node_name == "frame" || node_name == "iframe" {
                    // (I)FRAMEs are also quite different from conventional resources
                    let frame_dom = html_to_dom(&data, charset.clone());
                    walk(session, &final_url, &frame_dom.document);

                    let mut frame_data: Vec<u8> = Vec::new();
                    let serializable: SerializableHandle = frame_dom.document.into();
                    serialize(&mut frame_data, &serializable, SerializeOpts::default()).unwrap();

                    // Create and embed data URL
                    let mut frame_data_url =
                        create_data_url(&media_type, &charset, &frame_data, &final_url);
                    frame_data_url.set_fragment(resolved_url.fragment());
                    set_node_attr(node, attr_name, Some(frame_data_url.to_string()));
                } else {
                    // Every other type of element gets processed here

                    // Parse media type for SCRIPT elements
                    if node_name == "script" {
                        let script_media_type =
                            get_node_attr(node, "type").unwrap_or(String::from("text/javascript"));

                        if script_media_type == "text/javascript"
                            || script_media_type == "application/javascript"
                        {
                            // Embed javascript code instead of using data URLs
                            use html5ever::parse_document;
                            use html5ever::tendril::TendrilSink;

                            let script_dom: RcDom =
                                parse_document(RcDom::default(), Default::default())
                                    .one("<script>;</script>");
                            for script_node in
                                find_nodes(&script_dom.document, vec!["html", "head", "script"])
                                    .iter()
                            {
                                let text_node = &script_node.children.borrow()[0];

                                if let NodeData::Text { ref contents } = text_node.data {
                                    let mut tendril = contents.borrow_mut();
                                    tendril.clear();
                                    tendril.push_slice(
                                        &String::from_utf8_lossy(&data)
                                            .replace("</script>", "<\\/script>"),
                                    );
                                }

                                node.children.borrow_mut().push(text_node.clone());
                                set_node_attr(node, attr_name, None);
                            }
                        } else {
                            // Create and embed data URL
                            let mut data_url =
                                create_data_url(&script_media_type, &charset, &data, &final_url);
                            data_url.set_fragment(resolved_url.fragment());
                            set_node_attr(node, attr_name, Some(data_url.to_string()));
                        }
                    } else {
                        // Create and embed data URL
                        let mut data_url =
                            create_data_url(&media_type, &charset, &data, &final_url);
                        data_url.set_fragment(resolved_url.fragment());
                        set_node_attr(node, attr_name, Some(data_url.to_string()));
                    }
                }
            }
        }
        Err(_) => {
            if resolved_url.scheme() == "http" || resolved_url.scheme() == "https" {
                // Keep remote references if unable to retrieve the asset
                set_node_attr(node, attr_name, Some(resolved_url.to_string()));
            } else {
                // Remove local references if they can't be successfully embedded as data URLs
                set_node_attr(node, attr_name, None);
            }
        }
    }
}
