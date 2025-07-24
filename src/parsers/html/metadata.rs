use chrono::{SecondsFormat, Utc};
use html5ever::interface::{Attribute, QualName};
use html5ever::serialize::{serialize, SerializeOpts};
use html5ever::tendril::format_tendril;
use html5ever::tree_builder::create_element;
use html5ever::{namespace_url, ns, LocalName};
use markup5ever_rcdom::{Handle, NodeData, RcDom, SerializableHandle};

use crate::core::parse_content_type;
use crate::utils::url::{clean_url, Url};

use super::dom::{find_nodes, get_node_attr, html_to_dom, set_node_attr};
use super::utils::FAVICON_VALUES;

/// 获取文档的 base URL
pub fn get_base_url(handle: &Handle) -> Option<String> {
    if let Some(base_node) = find_nodes(handle, vec!["html", "head", "base"]).first() {
        // Only the first base tag matters (we ignore the rest, if there's any)
        get_node_attr(base_node, "href")
    } else {
        None
    }
}

/// 获取文档字符编码
pub fn get_charset(node: &Handle) -> Option<String> {
    for meta_node in find_nodes(node, vec!["html", "head", "meta"]).iter() {
        if let Some(meta_charset_node_attr_value) = get_node_attr(meta_node, "charset") {
            // Processing <meta charset="..." />
            return Some(meta_charset_node_attr_value);
        }

        if get_node_attr(meta_node, "http-equiv")
            .unwrap_or_default()
            .eq_ignore_ascii_case("content-type")
        {
            if let Some(meta_content_type_node_attr_value) = get_node_attr(meta_node, "content") {
                // Processing <meta http-equiv="content-type" content="text/html; charset=..." />
                let (_media_type, charset, _is_base64) =
                    parse_content_type(&meta_content_type_node_attr_value);
                return Some(charset);
            }
        }
    }

    None
}

/// 获取 robots meta 标签内容
pub fn get_robots(handle: &Handle) -> Option<String> {
    for meta_node in find_nodes(handle, vec!["html", "head", "meta"]).iter() {
        // Only the first base tag matters (we ignore the rest, if there's any)
        if get_node_attr(meta_node, "name")
            .unwrap_or_default()
            .eq_ignore_ascii_case("robots")
        {
            return get_node_attr(meta_node, "content");
        }
    }

    None
}

/// 获取文档标题
pub fn get_title(node: &Handle) -> Option<String> {
    for title_node in find_nodes(node, vec!["html", "head", "title"]).iter() {
        for child_node in title_node.children.borrow().iter() {
            if let NodeData::Text { ref contents } = child_node.data {
                return Some(contents.borrow().to_string());
            }
        }
    }

    None
}

/// 检查是否有 favicon
pub fn has_favicon(handle: &Handle) -> bool {
    let mut found_favicon: bool = false;

    for link_node in find_nodes(handle, vec!["html", "head", "link"]).iter() {
        if let Some(attr_value) = get_node_attr(link_node, "rel") {
            if FAVICON_VALUES.contains(&attr_value.trim().to_lowercase().as_str()) {
                found_favicon = true;
                break;
            }
        }
    }

    found_favicon
}

/// 设置 base URL
pub fn set_base_url(document: &Handle, base_href_value: String) -> RcDom {
    use super::dom::get_child_node_by_name;

    let mut buf: Vec<u8> = Vec::new();
    serialize(
        &mut buf,
        &SerializableHandle::from(document.clone()),
        SerializeOpts::default(),
    )
    .expect("unable to serialize DOM into buffer");
    let dom = html_to_dom(&buf, "utf-8".to_string());

    if let Some(html_node) = get_child_node_by_name(&dom.document, "html") {
        if let Some(head_node) = get_child_node_by_name(&html_node, "head") {
            // Check if BASE node already exists in the DOM tree
            if let Some(base_node) = get_child_node_by_name(&head_node, "base") {
                set_node_attr(&base_node, "href", Some(base_href_value));
            } else {
                let base_node = create_element(
                    &dom,
                    QualName::new(None, ns!(), LocalName::from("base")),
                    vec![Attribute {
                        name: QualName::new(None, ns!(), LocalName::from("href")),
                        value: format_tendril!("{}", base_href_value),
                    }],
                );

                // Insert newly created BASE node into HEAD
                head_node.children.borrow_mut().push(base_node.clone());
            }
        }
    }

    dom
}

/// 设置字符编码
pub fn set_charset(dom: RcDom, charset: String) -> RcDom {
    for meta_node in find_nodes(&dom.document, vec!["html", "head", "meta"]).iter() {
        if get_node_attr(meta_node, "charset").is_some() {
            set_node_attr(meta_node, "charset", Some(charset));
            return dom;
        }

        if get_node_attr(meta_node, "http-equiv")
            .unwrap_or_default()
            .eq_ignore_ascii_case("content-type")
            && get_node_attr(meta_node, "content").is_some()
        {
            set_node_attr(
                meta_node,
                "content",
                Some(format!("text/html;charset={charset}")),
            );
            return dom;
        }
    }

    // Manually append charset META node to HEAD
    {
        let meta_charset_node: Handle = create_element(
            &dom,
            QualName::new(None, ns!(), LocalName::from("meta")),
            vec![Attribute {
                name: QualName::new(None, ns!(), LocalName::from("charset")),
                value: format_tendril!("{}", charset),
            }],
        );

        // Insert newly created META charset node into HEAD
        if let Some(head_node) = find_nodes(&dom.document, vec!["html", "head"]).first() {
            head_node
                .children
                .borrow_mut()
                .push(meta_charset_node.clone());
        }
    }

    dom
}

/// 设置 robots meta 标签
pub fn set_robots(dom: RcDom, content_value: &str) -> RcDom {
    for meta_node in find_nodes(&dom.document, vec!["html", "head", "meta"]).iter() {
        if get_node_attr(meta_node, "name")
            .unwrap_or_default()
            .eq_ignore_ascii_case("robots")
        {
            set_node_attr(meta_node, "content", Some(content_value.to_string()));
            return dom;
        }
    }

    // Manually append robots META node to HEAD
    {
        let meta_charset_node: Handle = create_element(
            &dom,
            QualName::new(None, ns!(), LocalName::from("meta")),
            vec![
                Attribute {
                    name: QualName::new(None, ns!(), LocalName::from("name")),
                    value: format_tendril!("robots"),
                },
                Attribute {
                    name: QualName::new(None, ns!(), LocalName::from("content")),
                    value: format_tendril!("{}", content_value),
                },
            ],
        );

        // Insert newly created META charset node into HEAD
        if let Some(head_node) = find_nodes(&dom.document, vec!["html", "head"]).first() {
            head_node
                .children
                .borrow_mut()
                .push(meta_charset_node.clone());
        }
    }

    dom
}

/// 创建元数据标签
pub fn create_metadata_tag(url: &Url) -> String {
    let datetime: &str = &Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
    let mut clean_url: Url = clean_url(url.clone());

    // Prevent credentials from getting into metadata
    if clean_url.scheme() == "http" || clean_url.scheme() == "https" {
        // Only HTTP(S) URLs can contain credentials
        clean_url.set_username("").unwrap();
        clean_url.set_password(None).unwrap();
    }

    format!(
        "<!-- Saved from {} at {} using {} v{} -->",
        if clean_url.scheme() == "http" || clean_url.scheme() == "https" {
            clean_url.as_str()
        } else {
            "local source"
        },
        datetime,
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
    )
}
