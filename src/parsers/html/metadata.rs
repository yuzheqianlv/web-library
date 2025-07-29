//! HTML 文档元数据处理模块
//!
//! 此模块提供对 HTML 文档元数据的完整处理功能，包括：
//! - 获取和设置 base URL
//! - 处理字符编码声明
//! - 管理 robots meta 标签
//! - 提取和操作文档标题
//! - 检测 favicon 存在性
//! - 生成保存元数据标签
//!
//! 这些功能主要用于 monolith 工具在打包 HTML 文档时保持正确的元数据信息，
//! 确保生成的单文件 HTML 能够正确显示和被搜索引擎正确解析。

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
///
/// 从 HTML 文档的 `<head>` 部分查找 `<base>` 标签并提取其 `href` 属性值。
/// 根据 HTML 规范，只有第一个 `<base>` 标签有效，其余的将被忽略。
///
/// # 参数
///
/// * `handle` - HTML 文档的根节点句柄
///
/// # 返回值
///
/// * `Some(String)` - 如果找到 base 标签且有 href 属性，返回其值
/// * `None` - 如果没有找到 base 标签或 href 属性为空
///
/// # 示例
///
/// ```ignore
/// // 对于包含 <base href="https://example.com/"> 的文档
/// let base_url = get_base_url(&document_handle);
/// assert_eq!(base_url, Some("https://example.com/".to_string()));
/// ```
pub fn get_base_url(handle: &Handle) -> Option<String> {
    if let Some(base_node) = find_nodes(handle, vec!["html", "head", "base"]).first() {
        // 只有第一个 base 标签起作用（忽略其余的，如果有的话）
        get_node_attr(base_node, "href")
    } else {
        None
    }
}

/// 获取文档字符编码
///
/// 从 HTML 文档的 meta 标签中提取字符编码信息。支持两种格式：
/// 1. HTML5 格式：`<meta charset="utf-8">`
/// 2. HTML4 格式：`<meta http-equiv="content-type" content="text/html; charset=utf-8">`
///
/// # 参数
///
/// * `node` - HTML 文档的根节点句柄
///
/// # 返回值
///
/// * `Some(String)` - 如果找到字符编码声明，返回编码名称（如 "utf-8"）
/// * `None` - 如果没有找到任何字符编码声明
///
/// # 示例
///
/// ```ignore
/// // 对于包含 <meta charset="utf-8"> 的文档
/// let charset = get_charset(&document_handle);
/// assert_eq!(charset, Some("utf-8".to_string()));
/// ```
pub fn get_charset(node: &Handle) -> Option<String> {
    for meta_node in find_nodes(node, vec!["html", "head", "meta"]).iter() {
        if let Some(meta_charset_node_attr_value) = get_node_attr(meta_node, "charset") {
            // 处理 <meta charset="..." /> 格式
            return Some(meta_charset_node_attr_value);
        }

        if get_node_attr(meta_node, "http-equiv")
            .unwrap_or_default()
            .eq_ignore_ascii_case("content-type")
        {
            if let Some(meta_content_type_node_attr_value) = get_node_attr(meta_node, "content") {
                // 处理 <meta http-equiv="content-type" content="text/html; charset=..." /> 格式
                let (_media_type, charset, _is_base64) =
                    parse_content_type(&meta_content_type_node_attr_value);
                return Some(charset);
            }
        }
    }

    None
}

/// 获取 robots meta 标签内容
///
/// 查找并提取 `<meta name="robots" content="...">` 标签的内容。
/// robots 标签用于指导搜索引擎爬虫如何处理页面内容。
///
/// # 参数
///
/// * `handle` - HTML 文档的根节点句柄
///
/// # 返回值
///
/// * `Some(String)` - 如果找到 robots meta 标签，返回其 content 属性值
/// * `None` - 如果没有找到 robots meta 标签
///
/// # 示例
///
/// ```ignore
/// // 对于包含 <meta name="robots" content="noindex, nofollow"> 的文档
/// let robots = get_robots(&document_handle);
/// assert_eq!(robots, Some("noindex, nofollow".to_string()));
/// ```
pub fn get_robots(handle: &Handle) -> Option<String> {
    for meta_node in find_nodes(handle, vec!["html", "head", "meta"]).iter() {
        // 只有第一个匹配的 robots 标签起作用（忽略其余的，如果有的话）
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
///
/// 从 HTML 文档的 `<title>` 标签中提取文本内容。
/// 只返回第一个 title 标签的文本内容。
///
/// # 参数
///
/// * `node` - HTML 文档的根节点句柄
///
/// # 返回值
///
/// * `Some(String)` - 如果找到 title 标签且包含文本，返回标题文本
/// * `None` - 如果没有找到 title 标签或标签为空
///
/// # 示例
///
/// ```ignore
/// // 对于包含 <title>我的网页标题</title> 的文档
/// let title = get_title(&document_handle);
/// assert_eq!(title, Some("我的网页标题".to_string()));
/// ```
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
///
/// 检查 HTML 文档是否包含 favicon 相关的 link 标签。
/// 支持多种 favicon 格式，包括 icon、shortcut icon、apple-touch-icon 等。
///
/// # 参数
///
/// * `handle` - HTML 文档的根节点句柄
///
/// # 返回值
///
/// * `true` - 如果文档包含任何 favicon 相关的 link 标签
/// * `false` - 如果文档不包含 favicon 标签
///
/// # 示例
///
/// ```ignore
/// // 对于包含 <link rel="icon" href="favicon.ico"> 的文档
/// let has_icon = has_favicon(&document_handle);
/// assert_eq!(has_icon, true);
/// ```
pub fn has_favicon(handle: &Handle) -> bool {
    let mut found_favicon: bool = false;

    for link_node in find_nodes(handle, vec!["html", "head", "link"]).iter() {
        if let Some(attr_value) = get_node_attr(link_node, "rel") {
            // 检查 rel 属性是否匹配任何已知的 favicon 类型
            if FAVICON_VALUES.contains(&attr_value.trim().to_lowercase().as_str()) {
                found_favicon = true;
                break;
            }
        }
    }

    found_favicon
}

/// 设置 base URL
///
/// 在 HTML 文档中设置或更新 base URL。如果文档中已经存在 `<base>` 标签，
/// 则更新其 `href` 属性；否则创建一个新的 `<base>` 标签并添加到 `<head>` 中。
///
/// # 参数
///
/// * `document` - HTML 文档的根节点句柄
/// * `base_href_value` - 要设置的 base URL 值
///
/// # 返回值
///
/// 返回更新后的 DOM 树
///
/// # 注意事项
///
/// 此函数会重新序列化和解析 DOM，可能会对性能产生影响。
///
/// # 示例
///
/// ```ignore
/// let updated_dom = set_base_url(&document_handle, "https://example.com/".to_string());
/// ```
pub fn set_base_url(document: &Handle, base_href_value: String) -> RcDom {
    use super::dom::get_child_node_by_name;

    // 将 DOM 序列化为字节数组然后重新解析
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
            // 检查 DOM 树中是否已经存在 BASE 节点
            if let Some(base_node) = get_child_node_by_name(&head_node, "base") {
                set_node_attr(&base_node, "href", Some(base_href_value));
            } else {
                // 创建新的 BASE 元素
                let base_node = create_element(
                    &dom,
                    QualName::new(None, ns!(), LocalName::from("base")),
                    vec![Attribute {
                        name: QualName::new(None, ns!(), LocalName::from("href")),
                        value: format_tendril!("{}", base_href_value),
                    }],
                );

                // 将新创建的 BASE 节点插入到 HEAD 中
                head_node.children.borrow_mut().push(base_node.clone());
            }
        }
    }

    dom
}

/// 设置字符编码
///
/// 在 HTML 文档中设置或更新字符编码声明。优先尝试更新现有的 meta 标签，
/// 如果不存在则创建新的 HTML5 格式的 charset meta 标签。
///
/// # 参数
///
/// * `dom` - 要修改的 DOM 树
/// * `charset` - 要设置的字符编码名称（如 "utf-8"）
///
/// # 返回值
///
/// 返回更新后的 DOM 树
///
/// # 处理逻辑
///
/// 1. 首先查找 HTML5 格式的 `<meta charset="...">` 标签
/// 2. 其次查找 HTML4 格式的 `<meta http-equiv="content-type">` 标签
/// 3. 如果都不存在，则创建新的 HTML5 格式的 meta 标签
///
/// # 示例
///
/// ```ignore
/// let updated_dom = set_charset(dom, "utf-8".to_string());
/// ```
pub fn set_charset(dom: RcDom, charset: String) -> RcDom {
    for meta_node in find_nodes(&dom.document, vec!["html", "head", "meta"]).iter() {
        // 检查是否有 HTML5 格式的 charset 属性
        if get_node_attr(meta_node, "charset").is_some() {
            set_node_attr(meta_node, "charset", Some(charset));
            return dom;
        }

        // 检查是否有 HTML4 格式的 http-equiv content-type 标签
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

    // 手动在 HEAD 中添加 charset META 节点
    {
        let meta_charset_node: Handle = create_element(
            &dom,
            QualName::new(None, ns!(), LocalName::from("meta")),
            vec![Attribute {
                name: QualName::new(None, ns!(), LocalName::from("charset")),
                value: format_tendril!("{}", charset),
            }],
        );

        // 将新创建的 META charset 节点插入到 HEAD 中
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
///
/// 在 HTML 文档中设置或更新 robots meta 标签。如果文档中已经存在 robots 标签，
/// 则更新其 content 属性；否则创建一个新的 robots meta 标签。
///
/// # 参数
///
/// * `dom` - 要修改的 DOM 树
/// * `content_value` - robots 指令内容（如 "noindex, nofollow"）
///
/// # 返回值
///
/// 返回更新后的 DOM 树
///
/// # robots 指令说明
///
/// 常用的 robots 指令包括：
/// - `index` / `noindex` - 允许/禁止索引页面
/// - `follow` / `nofollow` - 允许/禁止跟随链接
/// - `archive` / `noarchive` - 允许/禁止存档页面
///
/// # 示例
///
/// ```ignore
/// let updated_dom = set_robots(dom, "noindex, nofollow");
/// ```
pub fn set_robots(dom: RcDom, content_value: &str) -> RcDom {
    for meta_node in find_nodes(&dom.document, vec!["html", "head", "meta"]).iter() {
        // 查找现有的 robots meta 标签
        if get_node_attr(meta_node, "name")
            .unwrap_or_default()
            .eq_ignore_ascii_case("robots")
        {
            set_node_attr(meta_node, "content", Some(content_value.to_string()));
            return dom;
        }
    }

    // 手动在 HEAD 中添加 robots META 节点
    {
        let meta_robots_node: Handle = create_element(
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

        // 将新创建的 META robots 节点插入到 HEAD 中
        if let Some(head_node) = find_nodes(&dom.document, vec!["html", "head"]).first() {
            head_node
                .children
                .borrow_mut()
                .push(meta_robots_node.clone());
        }
    }

    dom
}

/// 创建元数据标签
///
/// 生成一个 HTML 注释标签，记录文档的来源 URL、保存时间和工具信息。
/// 此标签将被插入到生成的 monolith HTML 文件中，作为源信息的记录。
///
/// # 参数
///
/// * `url` - 原始文档的 URL
///
/// # 返回值
///
/// 返回一个 HTML 注释字符串，包含来源、时间和工具信息
///
/// # 安全性
///
/// 为了保护隐私，此函数会自动移除 URL 中的用户名和密码信息。
///
/// # 示例
///
/// ```ignore
/// use url::Url;
/// let url = Url::parse("https://example.com/page.html").unwrap();
/// let metadata = create_metadata_tag(&url);
/// // 输出类似：<!-- Saved from https://example.com/page.html at 2023-12-01T10:30:45Z using monolith v2.7.0 -->
/// ```
pub fn create_metadata_tag(url: &Url) -> String {
    let datetime: &str = &Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
    let mut clean_url: Url = clean_url(url.clone());

    // 防止凭据信息泄露到元数据中
    if clean_url.scheme() == "http" || clean_url.scheme() == "https" {
        // 只有 HTTP(S) URL 才可能包含凭据信息
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
