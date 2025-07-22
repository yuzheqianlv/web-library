//! 链接重写模块
//!
//! 负责重写HTML中的链接，使其指向翻译服务

use markup5ever_rcdom::{Handle, NodeData, RcDom};
use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
use crate::parsers::html::{get_node_attr, set_node_attr};
use crate::utils::url::{resolve_url, Url};

/// Define the percent-encoding set for URLs - encode everything except unreserved characters
const URL_ENCODE_SET: &AsciiSet = &CONTROLS
    .add(b' ').add(b'"').add(b'<').add(b'>').add(b'`')
    .add(b':').add(b'/').add(b'?').add(b'#').add(b'[').add(b']').add(b'@')
    .add(b'!').add(b'$').add(b'&').add(b'\'').add(b'(').add(b')')
    .add(b'*').add(b'+').add(b',').add(b';').add(b'=');

/// 重写HTML中的所有链接
///
/// # Arguments
///
/// * `dom` - DOM树
/// * `base_url` - 当前页面的基础URL
/// * `translation_base_path` - 翻译服务的基础路径，默认为"/website/"
///
/// # Returns
///
/// 处理过的DOM树，所有链接已重写为指向翻译服务
pub fn rewrite_links_in_dom(
    dom: RcDom,
    base_url: &Url,
    translation_base_path: &str,
) -> RcDom {
    walk_and_rewrite_links(&dom.document, base_url, translation_base_path);
    dom
}

/// 重写HTML字符串中的链接
///
/// # Arguments
///
/// * `html` - 原始HTML字符串
/// * `base_url` - 当前页面的基础URL
/// * `translation_base_path` - 翻译服务的基础路径，默认为"/website/"
///
/// # Returns
///
/// 重写链接后的HTML字符串
pub fn rewrite_links_in_html(
    html: &str,
    base_url: &str,
    translation_base_path: Option<&str>,
) -> Result<String, Box<dyn std::error::Error>> {
    use crate::parsers::html::{html_to_dom, serialize_document};
    use crate::core::MonolithOptions;

    let base_path = translation_base_path.unwrap_or("/website/");
    
    // 解析base URL
    let parsed_base_url = match base_url.parse::<Url>() {
        Ok(url) => url,
        Err(e) => return Err(format!("Invalid base URL: {}", e).into()),
    };

    // 将HTML转换为DOM
    let dom = html_to_dom(html.as_bytes(), "utf-8".to_string());
    
    // 重写链接
    let rewritten_dom = rewrite_links_in_dom(dom, &parsed_base_url, base_path);
    
    // 序列化回HTML
    let options = MonolithOptions::default();
    let serialized = serialize_document(rewritten_dom, "utf-8".to_string(), &options);
    
    Ok(String::from_utf8_lossy(&serialized).to_string())
}

/// 递归遍历DOM树并重写链接
fn walk_and_rewrite_links(node: &Handle, base_url: &Url, translation_base_path: &str) {
    match node.data {
        NodeData::Document => {
            // 继续遍历子节点
            for child_node in node.children.borrow().iter() {
                walk_and_rewrite_links(child_node, base_url, translation_base_path);
            }
        }
        NodeData::Element { ref name, .. } => {
            // 处理链接元素
            if name.local.as_ref() == "a" {
                rewrite_anchor_link(node, base_url, translation_base_path);
            } else if name.local.as_ref() == "form" {
                rewrite_form_action(node, base_url, translation_base_path);
            }

            // 继续遍历子节点
            for child_node in node.children.borrow().iter() {
                walk_and_rewrite_links(child_node, base_url, translation_base_path);
            }
        }
        _ => {}
    }
}

/// 重写锚点链接的href属性
fn rewrite_anchor_link(node: &Handle, base_url: &Url, translation_base_path: &str) {
    if let Some(href_value) = get_node_attr(node, "href") {
        let trimmed_href = href_value.trim();
        
        // 跳过特殊链接
        if should_skip_link(trimmed_href) {
            return;
        }

        // 重写链接
        if let Some(rewritten_href) = rewrite_url(trimmed_href, base_url, translation_base_path) {
            set_node_attr(node, "href", Some(rewritten_href));
        }
    }
}

/// 重写表单的action属性
fn rewrite_form_action(node: &Handle, base_url: &Url, translation_base_path: &str) {
    if let Some(action_value) = get_node_attr(node, "action") {
        let trimmed_action = action_value.trim();
        
        // 跳过空action或javascript
        if trimmed_action.is_empty() || trimmed_action.starts_with("javascript:") {
            return;
        }

        // 重写action URL
        if let Some(rewritten_action) = rewrite_url(trimmed_action, base_url, translation_base_path) {
            set_node_attr(node, "action", Some(rewritten_action));
        }
    }
}

/// 判断是否应该跳过重写的链接
fn should_skip_link(href: &str) -> bool {
    // 跳过以下类型的链接：
    // 1. 锚点链接 (#section)
    // 2. JavaScript链接 (javascript:...)
    // 3. 邮件链接 (mailto:...)
    // 4. 电话链接 (tel:...)
    // 5. 空链接
    href.is_empty()
        || href.starts_with('#')
        || href.starts_with("javascript:")
        || href.starts_with("mailto:")
        || href.starts_with("tel:")
        || href.starts_with("data:")
        || href.starts_with("blob:")
}

/// 重写单个URL
fn rewrite_url(url: &str, base_url: &Url, translation_base_path: &str) -> Option<String> {
    // 解析URL为绝对URL
    let absolute_url = match resolve_url(base_url, url) {
        url if url.scheme() == "http" || url.scheme() == "https" => url,
        _ => return None, // 跳过非HTTP(S)链接
    };

    // 编码URL并构建翻译链接
    let encoded_url = utf8_percent_encode(absolute_url.as_str(), URL_ENCODE_SET);
    Some(format!("{}{}", translation_base_path, encoded_url))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_skip_link() {
        assert!(should_skip_link(""));
        assert!(should_skip_link("#section"));
        assert!(should_skip_link("javascript:void(0)"));
        assert!(should_skip_link("mailto:test@example.com"));
        assert!(should_skip_link("tel:+1234567890"));
        assert!(should_skip_link("data:text/plain;base64,SGVsbG8="));
        
        assert!(!should_skip_link("https://example.com"));
        assert!(!should_skip_link("/relative/path"));
        assert!(!should_skip_link("relative.html"));
    }

    #[test]
    fn test_rewrite_url() {
        let base_url = "https://example.com/page".parse().unwrap();
        let translation_path = "/website/";

        // 测试绝对URL
        let result = rewrite_url("https://other.com/page", &base_url, translation_path);
        assert_eq!(result, Some("/website/https%3A%2F%2Fother.com%2Fpage".to_string()));

        // 测试相对URL
        let result = rewrite_url("/relative", &base_url, translation_path);
        assert_eq!(result, Some("/website/https%3A%2F%2Fexample.com%2Frelative".to_string()));

        // 测试非HTTP URL应该返回None
        let result = rewrite_url("ftp://example.com", &base_url, translation_path);
        assert_eq!(result, None);
    }

    #[test]
    fn test_rewrite_links_in_html() {
        let html = "<html><body><a href=\"https://example.com/page\">External Link</a><a href=\"/relative\">Relative Link</a><a href=\"#section\">Anchor Link</a><a href=\"javascript:void(0)\">JS Link</a></body></html>";

        let result = rewrite_links_in_html(html, "https://test.com", None).unwrap();
        
        // 检查外部链接被重写
        assert!(result.contains("/website/https%3A%2F%2Fexample.com%2Fpage"));
        // 检查相对链接被重写
        assert!(result.contains("/website/https%3A%2F%2Ftest.com%2Frelative"));
        // 检查锚点链接未被重写
        assert!(result.contains("href=\"#section\""));
        // 检查JavaScript链接未被重写
        assert!(result.contains("href=\"javascript:void(0)\""));
    }
}