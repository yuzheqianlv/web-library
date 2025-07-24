use encoding_rs::Encoding;
use html5ever::parse_document;
use html5ever::tendril::TendrilSink;
use html5ever::{namespace_url, ns};
use markup5ever_rcdom::{Handle, NodeData, RcDom};

/// 将 HTML 字节转换为 DOM
pub fn html_to_dom(data: &[u8], document_encoding: String) -> RcDom {
    let s: String;

    if let Some(encoding) = Encoding::for_label(document_encoding.as_bytes()) {
        let (string, _, _) = encoding.decode(data);
        s = string.to_string();
    } else {
        s = String::from_utf8_lossy(data).to_string();
    }

    parse_document(RcDom::default(), Default::default())
        .from_utf8()
        .read_from(&mut s.as_bytes())
        .unwrap()
}

/// 查找指定路径的DOM节点
pub fn find_nodes(node: &Handle, node_names: Vec<&str>) -> Vec<Handle> {
    assert!(!node_names.is_empty());

    let mut found_nodes = Vec::new();
    let node_name = node_names[0];

    if node_names.len() == 1 {
        if let NodeData::Element { ref name, .. } = node.data {
            if &*name.local == node_name {
                found_nodes.push(node.clone());
            }
        }

        for child_node in node.children.borrow().iter() {
            found_nodes.append(&mut find_nodes(child_node, node_names.clone()));
        }
    } else if let NodeData::Element { ref name, .. } = node.data {
        if &*name.local == node_name {
            let mut new_node_names = node_names;
            new_node_names.remove(0);
            found_nodes.append(&mut find_nodes(node, new_node_names));
        } else {
            for child_node in node.children.borrow().iter() {
                found_nodes.append(&mut find_nodes(child_node, node_names.clone()));
            }
        }
    } else {
        for child_node in node.children.borrow().iter() {
            found_nodes.append(&mut find_nodes(child_node, node_names.clone()));
        }
    }

    found_nodes
}

/// 根据名称获取子节点
pub fn get_child_node_by_name(parent: &Handle, node_name: &str) -> Option<Handle> {
    let children = parent.children.borrow();
    let matching_children = children.iter().find(|child| match child.data {
        NodeData::Element { ref name, .. } => &*name.local == node_name,
        _ => false,
    });
    matching_children.cloned()
}

/// 获取节点属性值
pub fn get_node_attr(node: &Handle, attr_name: &str) -> Option<String> {
    match &node.data {
        NodeData::Element { attrs, .. } => {
            for attr in attrs.borrow().iter() {
                if &*attr.name.local == attr_name {
                    return Some(attr.value.to_string());
                }
            }
            None
        }
        _ => None,
    }
}

/// 获取节点名称
pub fn get_node_name(node: &Handle) -> Option<&'_ str> {
    match &node.data {
        NodeData::Element { name, .. } => Some(name.local.as_ref()),
        _ => None,
    }
}

/// 获取父节点
pub fn get_parent_node(child: &Handle) -> Handle {
    let parent = child.parent.take().clone();
    parent.and_then(|node| node.upgrade()).unwrap()
}

/// 设置节点属性
pub fn set_node_attr(node: &Handle, attr_name: &str, attr_value: Option<String>) {
    use html5ever::interface::{Attribute, QualName};
    use html5ever::tendril::format_tendril;
    use html5ever::{ns, LocalName};

    if let NodeData::Element { attrs, .. } = &node.data {
        let attrs_mut = &mut attrs.borrow_mut();
        let mut i = 0;
        let mut found_existing_attr: bool = false;

        while i < attrs_mut.len() {
            if &attrs_mut[i].name.local == attr_name {
                found_existing_attr = true;

                if let Some(attr_value) = attr_value.clone() {
                    let _ = &attrs_mut[i].value.clear();
                    let _ = &attrs_mut[i].value.push_slice(attr_value.as_str());
                } else {
                    // Remove attr completely if attr_value is not defined
                    attrs_mut.remove(i);
                    continue;
                }
            }

            i += 1;
        }

        if !found_existing_attr {
            // Add new attribute (since originally the target node didn't have it)
            if let Some(attr_value) = attr_value.clone() {
                let name = LocalName::from(attr_name);

                attrs_mut.push(Attribute {
                    name: QualName::new(None, ns!(), name),
                    value: format_tendril!("{}", attr_value),
                });
            }
        }
    };
}
