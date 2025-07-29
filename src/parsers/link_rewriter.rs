//! 链接重写器模块
//!
//! 这个模块提供了重写HTML文档中链接的功能，主要用于将原始网页中的链接重定向到翻译服务。
//!
//! ## 主要功能
//!
//! - 遍历DOM树，识别并重写所有相关的链接属性
//! - 支持处理 `<a>` 标签的 `href` 属性和 `<form>` 标签的 `action` 属性
//! - 智能跳过不需要重写的特殊链接（如锚点链接、JavaScript链接、邮件链接等）
//! - 将相对URL转换为绝对URL，然后重写为指向翻译服务的路径
//! - 提供URL编码功能，确保重写后的链接能够正确传递给翻译服务
//!
//! ## 使用场景
//!
//! 当网页需要通过翻译服务进行处理时，原始页面中的所有链接都需要重写，
//! 使其通过翻译服务来访问，而不是直接访问原始网站。这样可以确保用户
//! 浏览时始终通过翻译服务获得翻译后的内容。
//!
//! ## 示例
//!
//! ```rust
//! use crate::parsers::link_rewriter::rewrite_links_in_html;
//!
//! let html = r#"<a href="https://example.com/page">链接</a>"#;
//! let base_url = "https://current-site.com";
//! let result = rewrite_links_in_html(html, base_url, Some("/translate/")).unwrap();
//! // 结果: <a href="/translate/https%3A%2F%2Fexample.com%2Fpage">链接</a>
//! ```

use crate::parsers::html::{get_node_attr, set_node_attr};
use crate::utils::url::{resolve_url, Url};
use markup5ever_rcdom::{Handle, NodeData, RcDom};
use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};

/// URL编码字符集定义
///
/// 定义了在URL编码时需要编码的字符集。包含了控制字符以及在URL中具有特殊含义的字符，
/// 确保重写后的URL能够安全地作为查询参数传递。
///
/// 编码的字符包括：
/// - 控制字符（如换行符、制表符等）
/// - 空格和引号字符
/// - HTML特殊字符（<, >, `）
/// - URL结构字符（:, /, ?, #）
/// - 其他特殊符号（@, !, $, &, ', (, ), *, +, ,, ;, =）
const URL_ENCODE_SET: &AsciiSet = &CONTROLS
    .add(b' ') // 空格
    .add(b'"') // 双引号
    .add(b'<') // 小于号
    .add(b'>') // 大于号
    .add(b'`') // 反引号
    .add(b':') // 冒号
    .add(b'/') // 斜杠
    .add(b'?') // 问号
    .add(b'#') // 井号
    .add(b'[') // 左方括号
    .add(b']') // 右方括号
    .add(b'@') // at符号
    .add(b'!') // 感叹号
    .add(b'$') // 美元符号
    .add(b'&') // 和号
    .add(b'\'') // 单引号
    .add(b'(') // 左圆括号
    .add(b')') // 右圆括号
    .add(b'*') // 星号
    .add(b'+') // 加号
    .add(b',') // 逗号
    .add(b';') // 分号
    .add(b'='); // 等号

/// 重写DOM树中的所有链接
///
/// 这是主要的公共接口函数，用于重写整个DOM树中的链接。函数会遍历DOM树的每个节点，
/// 识别包含链接的元素（如 `<a>` 和 `<form>` 标签），并将其链接重写为指向翻译服务。
///
/// # 参数
///
/// * `dom` - 待处理的HTML DOM树，使用 `RcDom` 类型
/// * `base_url` - 当前页面的基础URL，用于将相对链接转换为绝对链接
/// * `translation_base_path` - 翻译服务的基础路径，通常为 "/website/" 或类似格式
///
/// # 返回值
///
/// 返回处理后的DOM树，所有相关链接都已重写为指向翻译服务的路径
///
/// # 示例
///
/// ```rust
/// use markup5ever_rcdom::RcDom;
/// use url::Url;
/// use crate::parsers::link_rewriter::rewrite_links_in_dom;
///
/// let dom: RcDom = /* 从HTML解析得到的DOM树 */;
/// let base_url = "https://example.com/page".parse().unwrap();
/// let rewritten_dom = rewrite_links_in_dom(dom, &base_url, "/translate/");
/// ```
pub fn rewrite_links_in_dom(dom: RcDom, base_url: &Url, translation_base_path: &str) -> RcDom {
    walk_and_rewrite_links(&dom.document, base_url, translation_base_path);
    dom
}

/// 重写HTML字符串中的链接
///
/// 这是另一个公共接口函数，提供了更便利的方式来处理HTML字符串。函数内部会将HTML字符串
/// 解析为DOM树，调用链接重写功能，然后再序列化回HTML字符串。
///
/// # 参数
///
/// * `html` - 待处理的原始HTML字符串
/// * `base_url` - 当前页面的基础URL字符串，用于解析相对链接
/// * `translation_base_path` - 可选的翻译服务基础路径，如果为 `None` 则使用默认值 "/website/"
///
/// # 返回值
///
/// 返回 `Result<String, Box<dyn std::error::Error>>`：
/// - `Ok(String)` - 成功时返回重写链接后的HTML字符串
/// - `Err(_)` - 失败时返回错误信息，可能的错误包括无效的基础URL或HTML解析失败
///
/// # 错误处理
///
/// 函数可能在以下情况下返回错误：
/// - 提供的基础URL格式无效
/// - HTML解析过程中出现问题
/// - DOM序列化过程中出现问题
///
/// # 示例
///
/// ```rust
/// use crate::parsers::link_rewriter::rewrite_links_in_html;
///
/// let html = r#"<a href="/relative/path">链接</a>"#;
/// let base_url = "https://example.com";
/// let result = rewrite_links_in_html(html, base_url, Some("/api/translate/"))?;
/// // 结果包含重写后的链接
/// ```
pub fn rewrite_links_in_html(
    html: &str,
    base_url: &str,
    translation_base_path: Option<&str>,
) -> Result<String, Box<dyn std::error::Error>> {
    use crate::core::MonolithOptions;
    use crate::parsers::html::{html_to_dom, serialize_document};

    // 使用默认翻译路径如果未提供
    let base_path = translation_base_path.unwrap_or("/website/");

    // 解析基础URL字符串为URL对象
    let parsed_base_url = match base_url.parse::<Url>() {
        Ok(url) => url,
        Err(e) => return Err(format!("Invalid base URL: {e}").into()),
    };

    // 将HTML字符串解析为DOM树
    let dom = html_to_dom(html.as_bytes(), "utf-8".to_string());

    // 调用DOM链接重写函数
    let rewritten_dom = rewrite_links_in_dom(dom, &parsed_base_url, base_path);

    // 将处理后的DOM树序列化回HTML字符串
    let options = MonolithOptions::default();
    let serialized = serialize_document(rewritten_dom, "utf-8".to_string(), &options);

    Ok(String::from_utf8_lossy(&serialized).to_string())
}

/// 递归遍历DOM树并重写链接
///
/// 这是一个内部辅助函数，用于递归遍历DOM树的每个节点，识别包含链接的元素并进行重写。
/// 函数采用深度优先搜索的方式遍历整个DOM树。
///
/// # 参数
///
/// * `node` - 当前正在处理的DOM节点
/// * `base_url` - 基础URL，用于解析相对链接
/// * `translation_base_path` - 翻译服务的基础路径
///
/// # 处理的元素类型
///
/// - `Document` 节点：继续遍历子节点
/// - `Element` 节点：检查是否为链接相关元素（`<a>` 或 `<form>`），进行相应处理，然后继续遍历子节点
/// - 其他节点类型：跳过处理
fn walk_and_rewrite_links(node: &Handle, base_url: &Url, translation_base_path: &str) {
    match node.data {
        NodeData::Document => {
            // 对于文档节点，继续遍历所有子节点
            for child_node in node.children.borrow().iter() {
                walk_and_rewrite_links(child_node, base_url, translation_base_path);
            }
        }
        NodeData::Element { ref name, .. } => {
            // 对于元素节点，检查是否为需要处理的链接元素
            if name.local.as_ref() == "a" {
                // 处理锚点标签的href属性
                rewrite_anchor_link(node, base_url, translation_base_path);
            } else if name.local.as_ref() == "form" {
                // 处理表单标签的action属性
                rewrite_form_action(node, base_url, translation_base_path);
            }

            // 继续递归处理所有子节点
            for child_node in node.children.borrow().iter() {
                walk_and_rewrite_links(child_node, base_url, translation_base_path);
            }
        }
        _ => {
            // 其他类型的节点（如文本节点、注释节点等）不需要处理
        }
    }
}

/// 重写锚点链接的href属性
///
/// 处理 `<a>` 标签的 `href` 属性，将其重写为指向翻译服务的链接。
/// 函数会检查链接是否需要重写，跳过特殊类型的链接（如锚点链接、JavaScript链接等）。
///
/// # 参数
///
/// * `node` - 指向 `<a>` 标签的DOM节点句柄
/// * `base_url` - 基础URL，用于将相对链接转换为绝对链接
/// * `translation_base_path` - 翻译服务的基础路径
///
/// # 处理逻辑
///
/// 1. 获取节点的 `href` 属性值
/// 2. 去除首尾空白字符
/// 3. 检查是否为需要跳过的特殊链接
/// 4. 如果需要重写，则调用URL重写函数并更新属性值
fn rewrite_anchor_link(node: &Handle, base_url: &Url, translation_base_path: &str) {
    if let Some(href_value) = get_node_attr(node, "href") {
        let trimmed_href = href_value.trim();

        // 检查是否为需要跳过的特殊链接
        if should_skip_link(trimmed_href) {
            return;
        }

        // 重写链接并更新节点属性
        if let Some(rewritten_href) = rewrite_url(trimmed_href, base_url, translation_base_path) {
            set_node_attr(node, "href", Some(rewritten_href));
        }
    }
}

/// 重写表单的action属性
///
/// 处理 `<form>` 标签的 `action` 属性，将其重写为指向翻译服务的链接。
/// 表单提交时需要确保通过翻译服务进行处理，以保持用户在翻译环境中的连续性。
///
/// # 参数
///
/// * `node` - 指向 `<form>` 标签的DOM节点句柄
/// * `base_url` - 基础URL，用于将相对action转换为绝对URL
/// * `translation_base_path` - 翻译服务的基础路径
///
/// # 处理逻辑
///
/// 1. 获取节点的 `action` 属性值
/// 2. 去除首尾空白字符
/// 3. 跳过空的action或JavaScript action
/// 4. 重写action URL并更新节点属性
///
/// # 注意事项
///
/// - 空的action属性通常表示提交到当前页面，这种情况下不进行重写
/// - JavaScript action（如 `javascript:submitForm()`）不需要重写
fn rewrite_form_action(node: &Handle, base_url: &Url, translation_base_path: &str) {
    if let Some(action_value) = get_node_attr(node, "action") {
        let trimmed_action = action_value.trim();

        // 跳过空action或JavaScript action
        if trimmed_action.is_empty() || trimmed_action.starts_with("javascript:") {
            return;
        }

        // 重写action URL并更新节点属性
        if let Some(rewritten_action) = rewrite_url(trimmed_action, base_url, translation_base_path)
        {
            set_node_attr(node, "action", Some(rewritten_action));
        }
    }
}

/// 判断是否应该跳过重写的链接
///
/// 检查给定的链接是否属于不需要重写的特殊类型。这些特殊链接通常具有特定的功能，
/// 不应该通过翻译服务来处理。
///
/// # 参数
///
/// * `href` - 待检查的链接字符串
///
/// # 返回值
///
/// 返回 `true` 如果链接应该跳过重写，`false` 如果需要重写
///
/// # 跳过的链接类型
///
/// 1. **空链接** - 空字符串或只包含空白字符
/// 2. **锚点链接** - 以 `#` 开头的页面内导航链接
/// 3. **JavaScript链接** - 以 `javascript:` 开头的JavaScript代码
/// 4. **邮件链接** - 以 `mailto:` 开头的邮件地址链接
/// 5. **电话链接** - 以 `tel:` 开头的电话号码链接
/// 6. **数据URL** - 以 `data:` 开头的内嵌数据链接
/// 7. **Blob URL** - 以 `blob:` 开头的二进制数据链接
///
/// # 示例
///
/// ```rust
/// assert!(should_skip_link("#section"));           // 锚点链接
/// assert!(should_skip_link("javascript:void(0)")); // JavaScript链接
/// assert!(should_skip_link("mailto:test@example.com")); // 邮件链接
/// assert!(!should_skip_link("https://example.com")); // 普通HTTP链接
/// ```
fn should_skip_link(href: &str) -> bool {
    href.is_empty()                        // 空链接
        || href.starts_with('#')           // 锚点链接 (#section)
        || href.starts_with("javascript:") // JavaScript链接 (javascript:...)
        || href.starts_with("mailto:")     // 邮件链接 (mailto:...)
        || href.starts_with("tel:")        // 电话链接 (tel:...)
        || href.starts_with("data:")       // 数据URL (data:...)
        || href.starts_with("blob:") // Blob URL (blob:...)
}

/// 重写单个URL
///
/// 将给定的URL（可能是相对URL或绝对URL）重写为指向翻译服务的格式。
/// 这是链接重写的核心逻辑函数。
///
/// # 参数
///
/// * `url` - 待重写的原始URL字符串（可以是相对或绝对URL）
/// * `base_url` - 基础URL，用于将相对URL转换为绝对URL
/// * `translation_base_path` - 翻译服务的基础路径前缀
///
/// # 返回值
///
/// 返回 `Option<String>`：
/// - `Some(String)` - 成功重写的URL字符串
/// - `None` - 如果URL不是HTTP/HTTPS协议，则返回None（不进行重写）
///
/// # 处理流程
///
/// 1. **URL解析** - 使用基础URL将输入URL转换为绝对URL
/// 2. **协议检查** - 只处理HTTP和HTTPS协议的URL，其他协议（如FTP）会被跳过
/// 3. **URL编码** - 对绝对URL进行percent编码，确保能安全地作为路径参数传递
/// 4. **路径构建** - 将编码后的URL拼接到翻译服务基础路径后面
///
/// # 示例
///
/// ```rust
/// let base_url = "https://example.com/page".parse().unwrap();
/// let result = rewrite_url("/relative/path", &base_url, "/translate/");
/// // 结果: Some("/translate/https%3A%2F%2Fexample.com%2Frelative%2Fpath")
/// ```
fn rewrite_url(url: &str, base_url: &Url, translation_base_path: &str) -> Option<String> {
    // 使用基础URL解析为绝对URL
    let absolute_url = match resolve_url(base_url, url) {
        url if url.scheme() == "http" || url.scheme() == "https" => url,
        _ => return None, // 跳过非HTTP(S)协议的链接
    };

    // 对绝对URL进行percent编码并构建翻译服务链接
    let encoded_url = utf8_percent_encode(absolute_url.as_str(), URL_ENCODE_SET);
    Some(format!("{translation_base_path}{encoded_url}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试链接跳过逻辑
    ///
    /// 验证各种类型的特殊链接是否被正确识别并跳过重写
    #[test]
    fn test_should_skip_link() {
        // 应该跳过的链接类型
        assert!(should_skip_link("")); // 空链接
        assert!(should_skip_link("#section")); // 锚点链接
        assert!(should_skip_link("javascript:void(0)")); // JavaScript链接
        assert!(should_skip_link("mailto:test@example.com")); // 邮件链接
        assert!(should_skip_link("tel:+1234567890")); // 电话链接
        assert!(should_skip_link("data:text/plain;base64,SGVsbG8=")); // 数据URL

        // 不应该跳过的普通链接
        assert!(!should_skip_link("https://example.com")); // HTTPS链接
        assert!(!should_skip_link("/relative/path")); // 相对路径
        assert!(!should_skip_link("relative.html")); // 相对文件
    }

    /// 测试URL重写功能
    ///
    /// 验证各种类型的URL是否被正确重写为翻译服务格式
    #[test]
    fn test_rewrite_url() {
        let base_url = "https://example.com/page".parse().unwrap();
        let translation_path = "/website/";

        // 测试绝对URL重写
        let result = rewrite_url("https://other.com/page", &base_url, translation_path);
        assert_eq!(
            result,
            Some("/website/https%3A%2F%2Fother.com%2Fpage".to_string())
        );

        // 测试相对URL重写
        let result = rewrite_url("/relative", &base_url, translation_path);
        assert_eq!(
            result,
            Some("/website/https%3A%2F%2Fexample.com%2Frelative".to_string())
        );

        // 测试非HTTP协议URL应该返回None
        let result = rewrite_url("ftp://example.com", &base_url, translation_path);
        assert_eq!(result, None);
    }

    /// 测试HTML字符串链接重写功能
    ///
    /// 验证完整的HTML处理流程，包括解析、重写和序列化
    #[test]
    fn test_rewrite_links_in_html() {
        let html = "<html><body><a href=\"https://example.com/page\">External Link</a><a href=\"/relative\">Relative Link</a><a href=\"#section\">Anchor Link</a><a href=\"javascript:void(0)\">JS Link</a></body></html>";

        let result = rewrite_links_in_html(html, "https://test.com", None).unwrap();

        // 验证外部链接被正确重写
        assert!(result.contains("/website/https%3A%2F%2Fexample.com%2Fpage"));
        // 验证相对链接被正确重写
        assert!(result.contains("/website/https%3A%2F%2Ftest.com%2Frelative"));
        // 验证锚点链接保持不变
        assert!(result.contains("href=\"#section\""));
        // 验证JavaScript链接保持不变
        assert!(result.contains("href=\"javascript:void(0)\""));
    }
}
