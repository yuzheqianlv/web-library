//! HTML 解析器工具函数模块
//!
//! 本模块提供了 HTML 解析过程中使用的各种工具函数和常量定义，主要包括：
//! 
//! - Favicon 检测功能
//! - 空白字符的定义和处理
//! - 内容安全策略（CSP）的组合和生成
//! 
//! 这些工具函数被 HTML 解析器的其他模块广泛使用，用于处理网页资源的识别、
//! 安全策略的制定以及文本格式的标准化。

use crate::core::MonolithOptions;

/// Favicon 相关的属性值定义
/// 
/// 包含了所有可能表示 favicon 的 rel 属性值。这些值用于识别网页中的
/// 图标链接，以便在处理过程中进行特殊处理。
/// 
/// # 包含的值
/// - `"icon"`: 标准的图标类型
/// - `"shortcut icon"`: 传统的快捷图标类型（向后兼容）
pub const FAVICON_VALUES: &[&str] = &["icon", "shortcut icon"];

/// ASCII 空白字符集合
/// 
/// 定义了在 HTML 处理中需要识别和处理的各种空白字符。这些字符在
/// 文本处理、属性值清理和格式化过程中经常用到。
/// 
/// # 包含的字符
/// - `' '`: 普通空格
/// - `'\t'`: 制表符
/// - `'\n'`: 换行符
/// - `'\x0c'`: 换页符
/// - `'\r'`: 回车符
pub const WHITESPACES: &[char] = &[' ', '\t', '\n', '\x0c', '\r'];

/// 检查给定的属性值是否表示 favicon
/// 
/// 此函数用于识别 HTML `<link>` 标签的 `rel` 属性值是否指向网站图标。
/// 检查时会将输入转换为小写以进行不区分大小写的比较。
/// 
/// # 参数
/// - `attr_value`: 需要检查的属性值字符串
/// 
/// # 返回值
/// 如果属性值表示 favicon 则返回 `true`，否则返回 `false`
/// 
/// # 示例
/// ```rust
/// use monolith::parsers::html::utils::is_favicon;
/// 
/// assert_eq!(is_favicon("icon"), true);
/// assert_eq!(is_favicon("SHORTCUT ICON"), true);
/// assert_eq!(is_favicon("stylesheet"), false);
/// ```
pub fn is_favicon(attr_value: &str) -> bool {
    FAVICON_VALUES.contains(&attr_value.to_lowercase().as_str())
}

/// 根据选项组合生成内容安全策略（CSP）字符串
/// 
/// 此函数基于 `MonolithOptions` 配置生成相应的 CSP 规则，用于控制
/// 生成的 HTML 文档中各种资源的加载和执行权限。CSP 是一种重要的
/// 安全机制，可以防止 XSS 攻击和其他安全威胁。
/// 
/// # 参数
/// - `options`: Monolith 处理选项，包含各种资源处理配置
/// 
/// # 返回值
/// 组合后的 CSP 策略字符串，可直接用于 HTML meta 标签或 HTTP 头
/// 
/// # 生成的 CSP 规则
/// - `isolate`: 设置默认源为 `'unsafe-eval' 'unsafe-inline' data:`
/// - `no_css`: 禁用样式表加载 (`style-src 'none'`)
/// - `no_fonts`: 禁用字体加载 (`font-src 'none'`)
/// - `no_frames`: 禁用框架加载 (`frame-src 'none'; child-src 'none'`)
/// - `no_js`: 禁用 JavaScript 执行 (`script-src 'none'`)
/// - `no_images`: 限制图片仅加载 data URI (`img-src data:`)
/// 
/// # 注意事项
/// - 当禁用图片时，仍允许 `data:` URI 以支持透明像素图片的正常工作
/// - 各个策略之间用空格分隔，形成完整的 CSP 字符串
/// 
/// # 示例
/// ```rust
/// use monolith::core::MonolithOptions;
/// use monolith::parsers::html::utils::compose_csp;
/// 
/// let mut options = MonolithOptions::default();
/// options.no_js = true;
/// options.no_css = true;
/// 
/// let csp = compose_csp(&options);
/// assert!(csp.contains("script-src 'none'"));
/// assert!(csp.contains("style-src 'none'"));
/// ```
pub fn compose_csp(options: &MonolithOptions) -> String {
    let mut string_list = vec![];

    // 如果启用隔离模式，设置宽松的默认源策略
    if options.isolate {
        string_list.push("default-src 'unsafe-eval' 'unsafe-inline' data:;");
    }

    // 如果禁用 CSS，则完全阻止样式表加载
    if options.no_css {
        string_list.push("style-src 'none';");
    }

    // 如果禁用字体，则阻止字体资源加载
    if options.no_fonts {
        string_list.push("font-src 'none';");
    }

    // 如果禁用框架，则阻止所有框架和子框架内容
    if options.no_frames {
        string_list.push("frame-src 'none';");
        string_list.push("child-src 'none';");
    }

    // 如果禁用 JavaScript，则完全阻止脚本执行
    if options.no_js {
        string_list.push("script-src 'none';");
    }

    // 如果禁用图片，只允许 data URI（透明像素图片需要）
    if options.no_images {
        // 注意：data: 协议对于透明像素图片的正常工作是必需的
        string_list.push("img-src data:;");
    }

    // 将所有策略规则用空格连接成完整的 CSP 字符串
    string_list.join(" ")
}
