//! HTML 解析器模块
//!
//! 该模块提供HTML文档解析的核心功能，主要用于处理HTML中的链接和资源引用。
//! 支持解析各种类型的链接（如样式表、图标、预加载等）以及响应式图片的srcset属性。
//!
//! ## 主要功能
//!
//! - **链接类型解析**: 解析HTML `<link>` 元素的 `rel` 属性，识别不同类型的链接关系
//! - **SrcSet解析**: 解析HTML `<img>` 元素的 `srcset` 属性，支持响应式图片加载
//!
//! ## 使用示例
//!
//! ```rust
//! use monolith::parsers::html::parser::{parse_link_type, parse_srcset, LinkType};
//!
//! // 解析链接类型
//! let link_types = parse_link_type("stylesheet preload");
//! assert_eq!(link_types, vec![LinkType::Stylesheet, LinkType::Preload]);
//!
//! // 解析srcset属性
//! let srcset_items = parse_srcset("small.jpg 480w, large.jpg 800w");
//! assert_eq!(srcset_items.len(), 2);
//! ```

use super::utils::{is_favicon, WHITESPACES};

/// HTML链接类型枚举
///
/// 表示HTML `<link>` 元素中 `rel` 属性的不同类型值。
/// 用于标识链接资源的关系和用途。
///
/// ## 支持的链接类型
///
/// - `Alternate`: 备用版本链接
/// - `AppleTouchIcon`: Apple设备的触摸图标
/// - `DnsPrefetch`: DNS预取链接
/// - `Favicon`: 网站图标
/// - `Preload`: 预加载资源
/// - `Stylesheet`: CSS样式表
#[derive(PartialEq, Eq)]
pub enum LinkType {
    /// 备用版本链接 - 指向文档的替代版本（如不同语言版本）
    Alternate,
    /// Apple设备触摸图标 - 用于iOS设备主屏幕图标
    AppleTouchIcon,
    /// DNS预取 - 提示浏览器预先解析域名
    DnsPrefetch,
    /// 网站图标 - 浏览器标签页和书签中显示的小图标
    Favicon,
    /// 预加载资源 - 提示浏览器预先加载重要资源
    Preload,
    /// CSS样式表 - 链接到外部CSS文件
    Stylesheet,
}

/// SrcSet 属性项目结构
///
/// 表示HTML `srcset` 属性中的单个图片项目，包含图片路径和对应的描述符。
/// 用于响应式图片加载，根据不同的屏幕尺寸或像素密度选择合适的图片。
///
/// ## 字段说明
///
/// - `path`: 图片文件的路径或URL
/// - `descriptor`: 图片的描述符，可以是宽度（如 "480w"）或像素密度（如 "2x"）
///
/// ## 使用示例
///
/// ```rust
/// # use monolith::parsers::html::parser::SrcSetItem;
/// let item = SrcSetItem {
///     path: "image-480.jpg",
///     descriptor: "480w",
/// };
/// ```
pub struct SrcSetItem<'a> {
    /// 图片文件的路径或URL
    pub path: &'a str,
    /// 图片描述符 - 宽度描述符（如 "480w"）或像素密度描述符（如 "2x"）
    pub descriptor: &'a str,
}

/// 解析HTML链接的rel属性值
///
/// 根据HTML `<link>` 元素的 `rel` 属性值，解析出对应的链接类型列表。
/// 支持多个空格分隔的rel值，并且不区分大小写。
///
/// ## 参数
///
/// - `link_attr_rel_value`: `<link>` 元素的 `rel` 属性值字符串
///
/// ## 返回值
///
/// 返回解析出的 `LinkType` 枚举向量。如果rel属性包含多个值，
/// 则返回包含多个类型的向量。不支持的rel值会被忽略。
///
/// ## 支持的rel值
///
/// - "alternate" → `LinkType::Alternate`
/// - "dns-prefetch" → `LinkType::DnsPrefetch`
/// - "preload" → `LinkType::Preload`
/// - "stylesheet" → `LinkType::Stylesheet`
/// - "icon", "shortcut icon" 等 → `LinkType::Favicon`
/// - "apple-touch-icon" → `LinkType::AppleTouchIcon`
///
/// ## 使用示例
///
/// ```rust
/// # use monolith::parsers::html::parser::{parse_link_type, LinkType};
/// // 单个rel值
/// let types = parse_link_type("stylesheet");
/// assert_eq!(types, vec![LinkType::Stylesheet]);
///
/// // 多个rel值
/// let types = parse_link_type("preload stylesheet");
/// assert_eq!(types, vec![LinkType::Preload, LinkType::Stylesheet]);
///
/// // 不区分大小写
/// let types = parse_link_type("STYLESHEET");
/// assert_eq!(types, vec![LinkType::Stylesheet]);
/// ```
pub fn parse_link_type(link_attr_rel_value: &str) -> Vec<LinkType> {
    let mut types: Vec<LinkType> = vec![];

    // 按空白字符分割rel属性值，处理每个独立的rel类型
    for link_attr_rel_type in link_attr_rel_value.split_whitespace() {
        // 不区分大小写地匹配各种链接类型
        if link_attr_rel_type.eq_ignore_ascii_case("alternate") {
            types.push(LinkType::Alternate);
        } else if link_attr_rel_type.eq_ignore_ascii_case("dns-prefetch") {
            types.push(LinkType::DnsPrefetch);
        } else if link_attr_rel_type.eq_ignore_ascii_case("preload") {
            types.push(LinkType::Preload);
        } else if link_attr_rel_type.eq_ignore_ascii_case("stylesheet") {
            types.push(LinkType::Stylesheet);
        } else if is_favicon(link_attr_rel_type) {
            // 使用工具函数检查是否为图标类型（支持多种图标rel值）
            types.push(LinkType::Favicon);
        } else if link_attr_rel_type.eq_ignore_ascii_case("apple-touch-icon") {
            types.push(LinkType::AppleTouchIcon);
        }
        // 不支持的rel值被忽略
    }

    types
}

/// 解析HTML图片的srcset属性
///
/// 解析HTML `<img>` 或 `<source>` 元素的 `srcset` 属性字符串，
/// 提取出图片路径和对应的描述符信息。支持响应式图片的完整语法。
///
/// ## 参数
///
/// - `srcset`: `srcset` 属性的字符串值
///
/// ## 返回值
///
/// 返回 `SrcSetItem` 结构体的向量，每个元素包含图片路径和描述符。
///
/// ## SrcSet语法支持
///
/// - **宽度描述符**: `image.jpg 480w` - 表示图片的原始宽度
/// - **像素密度描述符**: `image.jpg 2x` - 表示设备像素比
/// - **逗号分隔**: 多个图片用逗号分隔
/// - **空白字符**: 路径和描述符用空格、制表符或换行符分隔
///
/// ## 使用示例
///
/// ```rust
/// # use monolith::parsers::html::parser::parse_srcset;
/// // 基本用法
/// let items = parse_srcset("small.jpg 480w, large.jpg 800w");
/// assert_eq!(items.len(), 2);
/// assert_eq!(items[0].path, "small.jpg");
/// assert_eq!(items[0].descriptor, "480w");
///
/// // 像素密度描述符
/// let items = parse_srcset("normal.jpg 1x, retina.jpg 2x");
/// assert_eq!(items[1].descriptor, "2x");
///
/// // 复杂格式
/// let items = parse_srcset("image1.jpg 480w, image2.jpg, image3.jpg 2x");
/// ```
///
/// ## 注意事项
///
/// - 解析器会忽略空字符串和无效格式
/// - 如果没有描述符，将使用空字符串作为默认值
/// - 支持复杂的逗号和空白字符组合格式
pub fn parse_srcset(srcset: &str) -> Vec<SrcSetItem> {
    let mut srcset_items: Vec<SrcSetItem> = vec![];

    // 按空白字符分割srcset字符串，获取所有的片段
    let mut partials: Vec<&str> = srcset.split(WHITESPACES).collect();
    let mut path: Option<&str> = None; // 当前处理的图片路径
    let mut descriptor: Option<&str> = None; // 当前处理的描述符
    let mut i = 0;

    while i < partials.len() {
        let partial = partials[i];
        i += 1;

        // 跳过空字符串片段
        if partial.is_empty() {
            continue;
        }

        // 处理以逗号结尾的片段（表示一个srcset项的结束）
        if partial.ends_with(',') {
            if path.is_none() {
                // 如果还没有路径，说明这个片段是路径部分
                path = Some(partial.strip_suffix(',').unwrap());
                descriptor = Some(""); // 没有描述符时使用空字符串
            } else {
                // 如果已经有路径，说明这个片段是描述符部分
                descriptor = Some(partial.strip_suffix(',').unwrap());
            }
        } else if path.is_none() {
            // 如果还没有路径，将当前片段作为路径
            path = Some(partial);
        } else {
            // 已经有路径的情况下，处理可能包含逗号的复杂片段
            let mut chunks: Vec<&str> = partial.split(',').collect();

            // 检查第一个chunk是否是有效的描述符（以'x'或'w'结尾）
            if !chunks.is_empty() && chunks.first().unwrap().ends_with(['x', 'w']) {
                descriptor = Some(chunks.first().unwrap());
                chunks.remove(0); // 移除已处理的描述符
            }

            // 如果还有剩余的chunks，将它们重新插入到待处理列表的开头
            if !chunks.is_empty() {
                if let Some(desc) = descriptor {
                    // 如果有描述符，跳过已处理的部分
                    partials.insert(0, &partial[desc.len()..]);
                } else {
                    // 如果没有描述符，将整个片段重新插入
                    partials.insert(0, partial);
                }
            }
        }

        // 如果路径和描述符都已确定，创建一个SrcSetItem
        if let (Some(p), Some(d)) = (path, descriptor) {
            srcset_items.push(SrcSetItem {
                path: p,
                descriptor: d,
            });

            // 重置状态，准备处理下一个项目
            path = None;
            descriptor = None;
        }
    }

    // 最后尝试处理剩余的路径（可能没有描述符）
    if let Some(p) = path {
        srcset_items.push(SrcSetItem {
            path: p,
            descriptor: descriptor.unwrap_or_default(), // 没有描述符时使用空字符串
        });
    }

    srcset_items
}
