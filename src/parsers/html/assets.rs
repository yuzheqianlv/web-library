//! HTML 静态资源处理模块
//!
//! 本模块负责处理 HTML 文档中的各种静态资源，包括：
//! - 图片资源（img 标签、srcset 属性）
//! - 样式表资源（link 标签的 CSS 文件）
//! - 脚本资源（script 标签的 JavaScript 文件）
//! - 嵌入式框架（iframe、frame 标签）
//! - 网站图标（favicon）
//!
//! 主要功能包括：
//! - 将外部资源转换为 data URL 并嵌入到 HTML 文档中
//! - 验证资源的完整性（通过 SHA 哈希）
//! - 处理响应式图片的 srcset 属性
//! - 递归处理嵌套的 HTML 文档（如 iframe）
//! - 处理不同编码格式的资源
//!
//! 这个模块是 Monolith 工具将网页转换为单文件 HTML 的核心组件之一。

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

/// 为 HTML 文档添加网站图标（favicon）
///
/// 此函数将指定的 favicon 以 data URL 的形式添加到 HTML 文档的 `<head>` 部分。
/// 如果文档中已经存在 favicon，此函数会添加一个新的 `<link>` 元素。
///
/// # 参数
///
/// * `document` - HTML 文档的根节点句柄
/// * `favicon_data_url` - favicon 的 data URL 字符串，包含完整的图标数据
///
/// # 返回值
///
/// 返回修改后的 DOM 树，其中包含新添加的 favicon 链接
///
/// # 示例
///
/// ```rust,ignore
/// let favicon_url = "data:image/x-icon;base64,AAABAAEAEBAQAAEABAAoAQAAFgAAACgAAAAQAAAAIAAAAAEABAAAAAAAgAAAAAAAAAAAAAAAEAAAAAAAAAAAAAAA";
/// let new_dom = add_favicon(&document, favicon_url.to_string());
/// ```
pub fn add_favicon(document: &Handle, favicon_data_url: String) -> RcDom {
    // 将当前文档序列化为字节缓冲区
    let mut buf: Vec<u8> = Vec::new();
    serialize(
        &mut buf,
        &SerializableHandle::from(document.clone()),
        SerializeOpts::default(),
    )
    .expect("unable to serialize DOM into buffer");

    // 重新解析为 DOM 树
    let dom = html_to_dom(&buf, "utf-8".to_string());
    
    // 查找文档中的 <head> 元素
    for head in find_nodes(&dom.document, vec!["html", "head"]).iter() {
        // 创建 favicon 的 <link> 元素
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

        // 将 favicon 链接标签插入到 HEAD 元素中
        head.children.borrow_mut().push(favicon_node.clone());
    }

    dom
}

/// 验证资源数据的完整性
///
/// 此函数根据提供的完整性哈希值验证资源数据是否未被篡改。
/// 支持 SHA-256、SHA-384 和 SHA-512 哈希算法，遵循 Web 标准的子资源完整性（SRI）规范。
///
/// # 参数
///
/// * `data` - 需要验证的资源数据字节数组
/// * `integrity` - 完整性哈希字符串，格式为 "sha256-base64hash"、"sha384-base64hash" 或 "sha512-base64hash"
///
/// # 返回值
///
/// * `true` - 如果数据的哈希值与提供的完整性哈希匹配
/// * `false` - 如果哈希不匹配或哈希格式不支持
///
/// # 示例
///
/// ```rust,ignore
/// let data = b"console.log('Hello World');";
/// let integrity = "sha256-WHfB8BRhZGFbrEKV+1GNVAV2c2KGOXqrNdF/jOjcpBU=";
/// let is_valid = check_integrity(data, integrity);
/// ```
pub fn check_integrity(data: &[u8], integrity: &str) -> bool {
    // 检查 SHA-256 哈希
    if let Some(hash_value) = integrity.strip_prefix("sha256-") {
        let mut hasher = Sha256::new();
        hasher.update(data);
        BASE64_STANDARD.encode(hasher.finalize()) == hash_value
    // 检查 SHA-384 哈希
    } else if let Some(hash_value) = integrity.strip_prefix("sha384-") {
        let mut hasher = Sha384::new();
        hasher.update(data);
        BASE64_STANDARD.encode(hasher.finalize()) == hash_value
    // 检查 SHA-512 哈希
    } else if let Some(hash_value) = integrity.strip_prefix("sha512-") {
        let mut hasher = Sha512::new();
        hasher.update(data);
        BASE64_STANDARD.encode(hasher.finalize()) == hash_value
    } else {
        // 不支持的哈希算法或格式错误
        false
    }
}

/// 处理并嵌入 srcset 属性中的图像资源
///
/// 此函数解析 HTML 图像标签的 `srcset` 属性，获取其中的所有图像资源，
/// 并将它们转换为 data URL 格式嵌入到文档中。支持响应式图像的不同尺寸和密度描述符。
///
/// # 参数
///
/// * `session` - 用于网络请求和配置的会话对象
/// * `document_url` - 当前文档的 URL，用于解析相对路径
/// * `srcset` - 原始的 srcset 属性值，包含多个图像源和描述符
///
/// # 返回值
///
/// 返回处理后的 srcset 字符串，其中所有图像 URL 都被替换为 data URL 或保持原状（如果获取失败）
///
/// # 注意事项
///
/// - 如果会话配置禁用图像（`no_images`），将使用空图像 data URL
/// - 如果无法获取图像资源，HTTP/HTTPS URL 会保持不变，其他协议的 URL 会被替换为空图像
/// - 保留原始的尺寸和密度描述符（如 "2x", "100w" 等）
///
/// # 示例
///
/// ```rust,ignore
/// let srcset = "small.jpg 480w, medium.jpg 800w, large.jpg 1200w";
/// let embedded_srcset = embed_srcset(&mut session, &document_url, srcset);
/// // 返回: "data:image/jpeg;base64,... 480w, data:image/jpeg;base64,... 800w, ..."
/// ```
pub fn embed_srcset(session: &mut Session, document_url: &Url, srcset: &str) -> String {
    // 解析 srcset 字符串为结构化项目列表
    let srcset_items: Vec<SrcSetItem> = parse_srcset(srcset);

    // 嵌入资源并构建新的 srcset 字符串
    let mut result: String = "".to_string();
    let mut i: usize = srcset_items.len();
    
    for srcset_item in srcset_items {
        // 如果配置禁用图像，使用空图像占位符
        if session.options.no_images {
            result.push_str(EMPTY_IMAGE_DATA_URL);
        } else {
            // 解析完整的图像 URL
            let image_full_url: Url = resolve_url(document_url, srcset_item.path);
            
            // 尝试获取图像资源
            match session.retrieve_asset(document_url, &image_full_url) {
                Ok((data, final_url, media_type, charset)) => {
                    // 成功获取资源，创建 data URL
                    let image_data_url: Url =
                        create_data_url(&media_type, &charset, &data, &final_url);

                    result.push_str(image_data_url.as_ref());
                }
                Err(_) => {
                    // 获取失败时的处理策略
                    if image_full_url.scheme() == "http" || image_full_url.scheme() == "https" {
                        // 保留 HTTP/HTTPS URL，让浏览器尝试加载
                        result.push_str(image_full_url.as_ref());
                    } else {
                        // 对于其他协议（如 file://），使用空图像占位符
                        result.push_str(EMPTY_IMAGE_DATA_URL);
                    }
                }
            }
        }

        // 添加描述符（如尺寸或密度信息）
        if !srcset_item.descriptor.is_empty() {
            result.push(' ');
            result.push_str(srcset_item.descriptor);
        }

        // 在项目之间添加逗号分隔符（最后一项除外）
        if i > 1 {
            result.push_str(", ");
        }

        i -= 1;
    }

    result
}

/// 检索外部资源并将其嵌入到 HTML 元素中
///
/// 这是资源处理的核心函数，负责获取各种类型的外部资源（CSS、JavaScript、图像、iframe等），
/// 并根据资源类型采用不同的嵌入策略。支持完整性验证、编码处理和递归文档处理。
///
/// # 参数
///
/// * `session` - 网络会话对象，包含配置选项和缓存
/// * `document_url` - 当前文档的基础 URL，用于解析相对路径
/// * `node` - 需要处理的 HTML 元素节点
/// * `attr_name` - 包含资源 URL 的属性名称（如 "src", "href"）
/// * `attr_value` - 资源的原始 URL 或路径
///
/// # 资源处理策略
///
/// - **CSS 文件**（link[rel=stylesheet]）：递归处理 CSS 中的 @import 和 url() 引用，转换为 data URL
/// - **JavaScript 文件**：
///   - `text/javascript` 或 `application/javascript`：直接嵌入代码到 `<script>` 标签内容中
///   - 其他类型：转换为 data URL
/// - **iframe/frame**：递归处理嵌套的 HTML 文档，完整地嵌入处理后的文档
/// - **其他资源**（图像、字体等）：转换为 data URL
///
/// # 完整性验证
///
/// 对于带有 `integrity` 属性的 link 和 script 元素，会验证资源的 SHA 哈希值：
/// - 验证通过：正常嵌入资源并移除 integrity 属性
/// - 验证失败：跳过资源嵌入
///
/// # 错误处理
///
/// - HTTP/HTTPS URL：获取失败时保留原始 URL
/// - 其他协议 URL：获取失败时移除属性
///
/// # 示例
///
/// ```rust,ignore
/// // 处理 CSS 链接
/// retrieve_and_embed_asset(&mut session, &doc_url, &link_node, "href", "styles.css");
/// 
/// // 处理 JavaScript 脚本
/// retrieve_and_embed_asset(&mut session, &doc_url, &script_node, "src", "script.js");
/// 
/// // 处理图像
/// retrieve_and_embed_asset(&mut session, &doc_url, &img_node, "src", "image.png");
/// ```
pub fn retrieve_and_embed_asset(
    session: &mut Session,
    document_url: &Url,
    node: &Handle,
    attr_name: &str,
    attr_value: &str,
) {
    // 解析相对 URL 为绝对 URL
    let resolved_url: Url = resolve_url(document_url, attr_value);

    // 尝试获取资源
    match session.retrieve_asset(&document_url.clone(), &resolved_url) {
        Ok((data, final_url, media_type, charset)) => {
            let node_name: &str = get_node_name(node).unwrap();

            // 对 LINK 和 SCRIPT 元素进行完整性验证
            let mut ok_to_include: bool = true;
            if node_name == "link" || node_name == "script" {
                // 检查是否有完整性属性
                if let Some(node_integrity_attr_value) = get_node_attr(node, "integrity") {
                    if !node_integrity_attr_value.is_empty() {
                        // 验证资源数据的完整性
                        ok_to_include = check_integrity(&data, &node_integrity_attr_value);
                    }

                    // 移除完整性属性，因为我们已经验证过了
                    set_node_attr(node, "integrity", None);
                }
            }

            // 只有通过完整性验证的资源才会被嵌入
            if ok_to_include {
                // 处理 CSS 样式表文件
                if node_name == "link"
                    && parse_link_type(&get_node_attr(node, "rel").unwrap_or(String::from("")))
                        .contains(&LinkType::Stylesheet)
                {
                    // 根据字符编码解码 CSS 内容
                    let stylesheet: String;
                    if let Some(encoding) = Encoding::for_label(charset.as_bytes()) {
                        let (string, _, _) = encoding.decode(&data);
                        stylesheet = string.to_string();
                    } else {
                        stylesheet = String::from_utf8_lossy(&data).to_string();
                    }

                    // CSS 样式表需要特殊处理，递归嵌入其中的资源
                    let css: String = embed_css(session, &final_url, &stylesheet);

                    // 创建并设置 CSS 的 data URL
                    let css_data_url =
                        create_data_url(&media_type, &charset, css.as_bytes(), &final_url);
                    set_node_attr(node, attr_name, Some(css_data_url.to_string()));
                
                // 处理 iframe 和 frame 元素
                } else if node_name == "frame" || node_name == "iframe" {
                    // iframe 和 frame 需要特殊处理，它们包含完整的 HTML 文档
                    let frame_dom = html_to_dom(&data, charset.clone());
                    
                    // 设置iframe处理标记，防止在递归处理中触发翻译
                    let original_iframe_flag = session.in_iframe_processing;
                    session.in_iframe_processing = true;
                    
                    // 递归处理嵌套文档中的所有资源
                    walk(session, &final_url, &frame_dom.document);
                    
                    // 恢复原始标记
                    session.in_iframe_processing = original_iframe_flag;

                    // 序列化处理后的文档
                    let mut frame_data: Vec<u8> = Vec::new();
                    let serializable: SerializableHandle = frame_dom.document.into();
                    serialize(&mut frame_data, &serializable, SerializeOpts::default()).unwrap();

                    // 创建并设置 iframe 的 data URL，保留原始的片段标识符
                    let mut frame_data_url =
                        create_data_url(&media_type, &charset, &frame_data, &final_url);
                    frame_data_url.set_fragment(resolved_url.fragment());
                    set_node_attr(node, attr_name, Some(frame_data_url.to_string()));
                
                // 处理其他类型的资源
                } else {
                    // 所有其他类型的元素都在这里处理

                    // 对 SCRIPT 元素进行特殊的媒体类型解析
                    if node_name == "script" {
                        let script_media_type =
                            get_node_attr(node, "type").unwrap_or(String::from("text/javascript"));

                        // 对于 JavaScript 代码，直接嵌入而不使用 data URL
                        if script_media_type == "text/javascript"
                            || script_media_type == "application/javascript"
                        {
                            // 直接嵌入 JavaScript 代码到 script 标签内容中
                            use html5ever::parse_document;
                            use html5ever::tendril::TendrilSink;

                            // 创建一个临时的 script DOM 来获取文本节点
                            let script_dom: RcDom =
                                parse_document(RcDom::default(), Default::default())
                                    .one("<script>;</script>");
                            for script_node in
                                find_nodes(&script_dom.document, vec!["html", "head", "script"])
                                    .iter()
                            {
                                let text_node = &script_node.children.borrow()[0];

                                // 将 JavaScript 代码添加到文本节点中
                                if let NodeData::Text { ref contents } = text_node.data {
                                    let mut tendril = contents.borrow_mut();
                                    tendril.clear();
                                    // 转义可能存在的 </script> 标签以防止 HTML 解析错误
                                    tendril.push_slice(
                                        &String::from_utf8_lossy(&data)
                                            .replace("</script>", "<\\/script>"),
                                    );
                                }

                                // 将文本节点添加到原始 script 元素中
                                node.children.borrow_mut().push(text_node.clone());
                                // 移除原始的 src 属性
                                set_node_attr(node, attr_name, None);
                            }
                        } else {
                            // 对于非 JavaScript 脚本类型，使用 data URL
                            let mut data_url =
                                create_data_url(&script_media_type, &charset, &data, &final_url);
                            data_url.set_fragment(resolved_url.fragment());
                            set_node_attr(node, attr_name, Some(data_url.to_string()));
                        }
                    } else {
                        // 对于所有其他类型的资源（图像、字体等），创建 data URL
                        let mut data_url =
                            create_data_url(&media_type, &charset, &data, &final_url);
                        data_url.set_fragment(resolved_url.fragment());
                        set_node_attr(node, attr_name, Some(data_url.to_string()));
                    }
                }
            }
        }
        // 资源获取失败的错误处理
        Err(_) => {
            if resolved_url.scheme() == "http" || resolved_url.scheme() == "https" {
                // 如果无法检索远程资源，保留原始的远程引用让浏览器尝试加载
                set_node_attr(node, attr_name, Some(resolved_url.to_string()));
            } else {
                // 如果本地引用无法成功嵌入为 data URL，则移除该引用
                set_node_attr(node, attr_name, None);
            }
        }
    }
}
