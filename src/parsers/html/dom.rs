//! DOM 操作工具模块
//!
//! 本模块提供了一套完整的 HTML DOM 操作工具，主要功能包括：
//! - HTML 字节流解析为 DOM 树结构
//! - DOM 节点查找和遍历
//! - 节点属性的读取和修改
//! - 节点关系操作（父子关系等）
//!
//! 基于 html5ever 和 markup5ever_rcdom 构建，提供了高效的 HTML 解析能力
//! 和灵活的 DOM 操作接口。

use encoding_rs::Encoding;
use html5ever::parse_document;
use html5ever::tendril::TendrilSink;
use html5ever::{namespace_url, ns};
use markup5ever_rcdom::{Handle, NodeData, RcDom};

/// 将 HTML 字节数据转换为 DOM 树结构
///
/// 此函数接收原始的 HTML 字节数据和文档编码信息，返回解析后的 DOM 树。
/// 支持多种字符编码格式，如果指定的编码无效，则回退到 UTF-8 处理。
///
/// # 参数
/// - `data`: HTML 文档的字节数据
/// - `document_encoding`: 文档编码名称（如 "UTF-8", "GBK" 等）
///
/// # 返回值
/// 返回 `RcDom` 类型的 DOM 树，可用于后续的节点查找和操作
///
/// # 示例
/// ```rust
/// let html_bytes = b"<html><body><h1>Hello</h1></body></html>";
/// let dom = html_to_dom(html_bytes, "UTF-8".to_string());
/// ```
pub fn html_to_dom(data: &[u8], document_encoding: String) -> RcDom {
    let s: String;

    // 根据指定的编码格式解码HTML字节数据
    if let Some(encoding) = Encoding::for_label(document_encoding.as_bytes()) {
        let (string, _, _) = encoding.decode(data);
        s = string.to_string();
    } else {
        // 如果编码格式无效，使用UTF-8进行lossy转换
        s = String::from_utf8_lossy(data).to_string();
    }

    // 使用html5ever解析器将HTML字符串解析为DOM树
    parse_document(RcDom::default(), Default::default())
        .from_utf8()
        .read_from(&mut s.as_bytes())
        .unwrap()
}

/// 根据节点名称路径查找DOM节点
///
/// 此函数支持深度优先搜索，可以查找指定路径的DOM节点。
/// 支持单级查找和多级路径查找（如 ["html", "body", "div"]）。
///
/// # 参数
/// - `node`: 开始搜索的根节点
/// - `node_names`: 要查找的节点名称路径，按从父到子的顺序
///
/// # 返回值
/// 返回所有匹配路径的节点句柄向量
///
/// # 示例
/// ```rust
/// // 查找所有div节点
/// let divs = find_nodes(&root, vec!["div"]);
///
/// // 查找body下的所有p节点
/// let paragraphs = find_nodes(&root, vec!["body", "p"]);
/// ```
///
/// # Panics
/// 如果 `node_names` 为空则会panic
pub fn find_nodes(node: &Handle, node_names: Vec<&str>) -> Vec<Handle> {
    assert!(!node_names.is_empty());

    let mut found_nodes = Vec::new();
    let node_name = node_names[0];

    if node_names.len() == 1 {
        // 单级查找：检查当前节点是否匹配
        if let NodeData::Element { ref name, .. } = node.data {
            if &*name.local == node_name {
                found_nodes.push(node.clone());
            }
        }

        // 递归搜索所有子节点
        for child_node in node.children.borrow().iter() {
            found_nodes.append(&mut find_nodes(child_node, node_names.clone()));
        }
    } else if let NodeData::Element { ref name, .. } = node.data {
        // 多级查找：检查当前节点是否匹配路径的第一个节点
        if &*name.local == node_name {
            // 匹配成功，继续在子节点中查找剩余路径
            let mut new_node_names = node_names;
            new_node_names.remove(0);
            found_nodes.append(&mut find_nodes(node, new_node_names));
        } else {
            // 当前节点不匹配，在子节点中继续搜索完整路径
            for child_node in node.children.borrow().iter() {
                found_nodes.append(&mut find_nodes(child_node, node_names.clone()));
            }
        }
    } else {
        // 非元素节点，直接搜索子节点
        for child_node in node.children.borrow().iter() {
            found_nodes.append(&mut find_nodes(child_node, node_names.clone()));
        }
    }

    found_nodes
}

/// 根据名称获取直接子节点
///
/// 在父节点的直接子节点中查找指定名称的第一个元素节点。
/// 只查找直接子节点，不进行递归搜索。
///
/// # 参数
/// - `parent`: 父节点句柄
/// - `node_name`: 要查找的子节点名称
///
/// # 返回值
/// 返回第一个匹配的子节点，如果未找到则返回 `None`
///
/// # 示例
/// ```rust
/// // 在body节点中查找第一个div子节点
/// if let Some(div) = get_child_node_by_name(&body_node, "div") {
///     // 找到了div节点
/// }
/// ```
pub fn get_child_node_by_name(parent: &Handle, node_name: &str) -> Option<Handle> {
    let children = parent.children.borrow();
    let matching_children = children.iter().find(|child| match child.data {
        NodeData::Element { ref name, .. } => &*name.local == node_name,
        _ => false,
    });
    matching_children.cloned()
}

/// 获取节点的指定属性值
///
/// 从元素节点中获取指定属性的值。如果节点不是元素节点或属性不存在，
/// 则返回 `None`。
///
/// # 参数
/// - `node`: 要查询的节点句柄
/// - `attr_name`: 属性名称
///
/// # 返回值
/// 返回属性值的字符串，如果属性不存在则返回 `None`
///
/// # 示例
/// ```rust
/// // 获取链接的href属性
/// if let Some(href) = get_node_attr(&link_node, "href") {
///     println!("链接地址: {}", href);
/// }
///
/// // 获取图片的src属性
/// let src = get_node_attr(&img_node, "src");
/// ```
pub fn get_node_attr(node: &Handle, attr_name: &str) -> Option<String> {
    match &node.data {
        NodeData::Element { attrs, .. } => {
            // 遍历节点的所有属性
            for attr in attrs.borrow().iter() {
                if &*attr.name.local == attr_name {
                    return Some(attr.value.to_string());
                }
            }
            None
        }
        _ => None, // 非元素节点没有属性
    }
}

/// 获取节点的标签名称
///
/// 返回元素节点的标签名称（如 "div", "p", "a" 等）。
/// 如果节点不是元素节点，则返回 `None`。
///
/// # 参数
/// - `node`: 要查询的节点句柄
///
/// # 返回值
/// 返回节点的标签名称字符串引用，如果不是元素节点则返回 `None`
///
/// # 示例
/// ```rust
/// if let Some(tag_name) = get_node_name(&node) {
///     match tag_name {
///         "div" => println!("这是一个div元素"),
///         "p" => println!("这是一个段落元素"),
///         _ => println!("其他元素: {}", tag_name),
///     }
/// }
/// ```
pub fn get_node_name(node: &Handle) -> Option<&'_ str> {
    match &node.data {
        NodeData::Element { name, .. } => Some(name.local.as_ref()),
        _ => None, // 非元素节点没有标签名称
    }
}

/// 获取节点的父节点
///
/// 返回指定节点的父节点句柄。此函数假设节点一定有父节点，
/// 如果没有父节点会导致panic。
///
/// # 参数
/// - `child`: 子节点句柄
///
/// # 返回值
/// 返回父节点的句柄
///
/// # Panics
/// 如果节点没有父节点或父节点的弱引用无法升级时会panic
///
/// # 示例
/// ```rust
/// let parent = get_parent_node(&child_node);
/// // 现在可以对父节点进行操作
/// ```
///
/// # 注意
/// 使用此函数前请确保节点确实有父节点，否则会导致程序panic
pub fn get_parent_node(child: &Handle) -> Handle {
    let parent = child.parent.take().clone();
    parent.and_then(|node| node.upgrade()).unwrap()
}

/// 设置或删除节点属性
///
/// 为元素节点设置指定属性的值，或删除指定属性。
/// 如果属性已存在，则更新其值；如果不存在且提供了值，则添加新属性。
/// 如果 `attr_value` 为 `None`，则删除该属性。
///
/// # 参数
/// - `node`: 要操作的节点句柄，必须是元素节点
/// - `attr_name`: 属性名称
/// - `attr_value`: 属性值，`Some(value)` 表示设置值，`None` 表示删除属性
///
/// # 示例
/// ```rust
/// // 设置链接的href属性
/// set_node_attr(&link_node, "href", Some("https://example.com".to_string()));
///
/// // 设置图片的alt属性
/// set_node_attr(&img_node, "alt", Some("描述文本".to_string()));
///
/// // 删除某个属性
/// set_node_attr(&node, "old_attr", None);
/// ```
///
/// # 注意
/// 此函数只对元素节点有效，对其他类型的节点调用此函数不会产生任何效果
pub fn set_node_attr(node: &Handle, attr_name: &str, attr_value: Option<String>) {
    use html5ever::interface::{Attribute, QualName};
    use html5ever::tendril::format_tendril;
    use html5ever::{ns, LocalName};

    if let NodeData::Element { attrs, .. } = &node.data {
        let attrs_mut = &mut attrs.borrow_mut();
        let mut i = 0;
        let mut found_existing_attr: bool = false;

        // 遍历现有属性，寻找匹配的属性名
        while i < attrs_mut.len() {
            if &attrs_mut[i].name.local == attr_name {
                found_existing_attr = true;

                if let Some(attr_value) = attr_value.clone() {
                    // 更新现有属性的值
                    let _ = &attrs_mut[i].value.clear();
                    let _ = &attrs_mut[i].value.push_slice(attr_value.as_str());
                } else {
                    // 如果attr_value为None，则完全删除该属性
                    attrs_mut.remove(i);
                    continue;
                }
            }

            i += 1;
        }

        if !found_existing_attr {
            // 添加新属性（因为原节点没有该属性）
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
