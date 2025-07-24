use crate::core::MonolithOptions;

/// Favicon 相关的值
pub const FAVICON_VALUES: &[&str] = &["icon", "shortcut icon"];

/// ASCII 空白字符
pub const WHITESPACES: &[char] = &[' ', '\t', '\n', '\x0c', '\r'];

/// 检查是否为 favicon
pub fn is_favicon(attr_value: &str) -> bool {
    FAVICON_VALUES.contains(&attr_value.to_lowercase().as_str())
}

/// 组合内容安全策略
pub fn compose_csp(options: &MonolithOptions) -> String {
    let mut string_list = vec![];

    if options.isolate {
        string_list.push("default-src 'unsafe-eval' 'unsafe-inline' data:;");
    }

    if options.no_css {
        string_list.push("style-src 'none';");
    }

    if options.no_fonts {
        string_list.push("font-src 'none';");
    }

    if options.no_frames {
        string_list.push("frame-src 'none';");
        string_list.push("child-src 'none';");
    }

    if options.no_js {
        string_list.push("script-src 'none';");
    }

    if options.no_images {
        // Note: "data:" is required for transparent pixel images to work
        string_list.push("img-src data:;");
    }

    string_list.join(" ")
}
