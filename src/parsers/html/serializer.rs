//! HTML 文档序列化器
//!
//! 本模块提供了将 HTML DOM 树序列化为字节数组的功能。它能够：
//! - 将解析后的 DOM 结构序列化为完整的 HTML 文档
//! - 根据配置选项添加内容安全策略 (CSP) 元标签
//! - 处理 NOSCRIPT 元素的展开
//! - 支持不同字符编码的输出
//!
//! 序列化过程会考虑各种安全和隔离选项，确保生成的 HTML 文档符合用户的配置要求。

use encoding_rs::Encoding;
use html5ever::interface::{Attribute, QualName};
use html5ever::serialize::{serialize, SerializeOpts};
use html5ever::tendril::format_tendril;
use html5ever::tree_builder::create_element;
use html5ever::{namespace_url, ns, LocalName};
use markup5ever_rcdom::{RcDom, SerializableHandle};
use regex::Regex;

use crate::core::MonolithOptions;

use super::dom::get_child_node_by_name;
use super::utils::compose_csp;

/// 将 HTML DOM 树序列化为字节数组
///
/// 此函数将解析后的 HTML DOM 结构转换为完整的 HTML 文档字节数组。根据提供的选项，
/// 它会在序列化过程中应用各种安全和处理策略。
///
/// # 参数
///
/// * `dom` - 要序列化的 HTML DOM 树
/// * `document_encoding` - 文档的字符编码（如 "UTF-8"、"GBK" 等）
/// * `options` - 包含各种处理选项的配置结构体
///
/// # 返回值
///
/// 返回序列化后的 HTML 文档的字节数组
///
/// # 功能特性
///
/// ## 内容安全策略 (CSP)
/// 当启用以下任一选项时，会自动添加 CSP 元标签到文档头部：
/// - `isolate`: 隔离模式
/// - `no_css`: 禁用 CSS
/// - `no_fonts`: 禁用字体
/// - `no_frames`: 禁用框架
/// - `no_js`: 禁用 JavaScript
/// - `no_images`: 禁用图片
///
/// ## NOSCRIPT 处理
/// 当 `unwrap_noscript` 选项启用时，会将 `<noscript>` 标签转换为 HTML 注释
///
/// ## 字符编码
/// 支持将输出转换为指定的字符编码格式
///
/// # 示例
///
/// ```rust
/// use monolith::parsers::html::serializer::serialize_document;
/// use monolith::core::MonolithOptions;
/// use markup5ever_rcdom::RcDom;
///
/// let dom = RcDom::default(); // 假设已解析的 DOM
/// let encoding = "UTF-8".to_string();
/// let options = MonolithOptions::default();
///
/// let html_bytes = serialize_document(dom, encoding, &options);
/// ```
pub fn serialize_document(
    dom: RcDom,
    document_encoding: String,
    options: &MonolithOptions,
) -> Vec<u8> {
    // 初始化用于存储序列化结果的字节缓冲区
    let mut buf: Vec<u8> = Vec::new();

    // 检查是否需要添加内容安全策略 (CSP)
    // 当启用任何资源限制选项时，都需要添加相应的 CSP 规则
    if options.isolate
        || options.no_css
        || options.no_fonts
        || options.no_frames
        || options.no_js
        || options.no_images
    {
        // 处理内容安全策略 (CSP) 的添加
        if let Some(html) = get_child_node_by_name(&dom.document, "html") {
            if let Some(head) = get_child_node_by_name(&html, "head") {
                // 创建 CSP meta 元素
                let meta = create_element(
                    &dom,
                    QualName::new(None, ns!(), LocalName::from("meta")),
                    vec![
                        Attribute {
                            name: QualName::new(None, ns!(), LocalName::from("http-equiv")),
                            value: format_tendril!("Content-Security-Policy"),
                        },
                        Attribute {
                            name: QualName::new(None, ns!(), LocalName::from("content")),
                            value: format_tendril!("{}", compose_csp(options)),
                        },
                    ],
                );
                // CSP meta 标签必须被前置添加，绝不能追加到末尾
                // 因为原始文档中可能已经定义了一个 CSP 标签，
                // 而浏览器不允许重新定义它们（出于明显的安全原因）
                head.children.borrow_mut().reverse();
                head.children.borrow_mut().push(meta.clone());
                head.children.borrow_mut().reverse();
            }
        }
    }

    // 将 DOM 转换为可序列化的句柄并执行序列化
    let serializable: SerializableHandle = dom.document.into();
    serialize(&mut buf, &serializable, SerializeOpts::default())
        .expect("无法将 DOM 序列化到缓冲区");

    // 处理 NOSCRIPT 元素的展开
    // 将 <noscript> 标签转换为 HTML 注释，以便在禁用 JavaScript 的环境中显示内容
    if options.unwrap_noscript {
        let s: &str = &String::from_utf8_lossy(&buf);
        // 使用正则表达式匹配所有 noscript 标签（开始和结束标签）
        let noscript_re = Regex::new(r"<(?P<c>/?noscript[^>]*)>").unwrap();
        // 将匹配的标签替换为注释形式
        buf = noscript_re.replace_all(s, "<!--$c-->").as_bytes().to_vec();
    }

    // 处理文档字符编码转换
    // 如果指定了特定的字符编码，将输出转换为该编码格式
    if !document_encoding.is_empty() {
        if let Some(encoding) = Encoding::for_label(document_encoding.as_bytes()) {
            let s: &str = &String::from_utf8_lossy(&buf);
            // 使用指定编码重新编码文档内容
            let (data, _, _) = encoding.encode(s);
            buf = data.to_vec();
        }
    }

    // 返回最终的序列化结果
    buf
}
