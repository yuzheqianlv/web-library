//! CSS 解析器模块
//!
//! 此模块提供CSS样式表的解析和处理功能，主要用于将CSS中的外部资源（如图片、字体、
//! @import导入的样式表等）嵌入到单一的HTML文档中。该模块使用cssparser库进行
//! CSS语法解析，并支持各种CSS特性的处理。
//!
//! # 主要功能
//!
//! - **资源嵌入**: 将CSS中引用的外部资源转换为data URI嵌入
//! - **@import处理**: 递归处理@import规则，将导入的样式表内联
//! - **URL重写**: 处理url()函数中的资源引用
//! - **选择性过滤**: 支持根据配置选择性忽略某些资源（如字体、图片）
//! - **语法保持**: 确保处理后的CSS语法正确性
//!
//! # 使用示例
//!
//! ```rust
//! use monolith::parsers::css::embed_css;
//! use monolith::network::session::Session;
//! use url::Url;
//!
//! let mut session = Session::new();
//! let document_url = Url::parse("https://example.com").unwrap();
//! let css_content = "body { background-image: url('bg.jpg'); }";
//!
//! let embedded_css = embed_css(&mut session, &document_url, css_content);
//! ```

use cssparser::{serialize_identifier, serialize_string, ParseError, Parser, ParserInput, Token};

use crate::network::session::Session;
use crate::utils::url::{create_data_url, resolve_url, Url, EMPTY_IMAGE_DATA_URL};

/// 包含图片URL的CSS属性列表
///
/// 这个常量数组定义了所有可能包含图片URL引用的CSS属性名称。
/// 当处理这些属性时，解析器会特别处理其中的url()函数，
/// 将引用的图片资源嵌入为data URI。
///
/// # 包含的属性类型
///
/// - **通用属性**: background、border-image、content、cursor等
/// - **列表样式**: list-style、list-style-image
/// - **遮罩属性**: mask、mask-image
/// - **@counter-style特定属性**: additive-symbols、negative、pad等
const CSS_PROPS_WITH_IMAGE_URLS: &[&str] = &[
    // Universal
    "background",
    "background-image",
    "border-image",
    "border-image-source",
    "content",
    "cursor",
    "list-style",
    "list-style-image",
    "mask",
    "mask-image",
    // Specific to @counter-style
    "additive-symbols",
    "negative",
    "pad",
    "prefix",
    "suffix",
    "symbols",
];

/// 将CSS样式表中的外部资源嵌入为data URI
///
/// 这是CSS处理的主入口函数，它解析提供的CSS字符串，
/// 并将其中引用的所有外部资源（图片、字体、@import的样式表等）
/// 转换为嵌入式的data URI，从而创建一个自包含的CSS样式表。
///
/// # 参数
///
/// * `session` - 可变的会话对象，用于获取外部资源和访问配置选项
/// * `document_url` - 当前文档的URL，用作解析相对URL的基准
/// * `css` - 要处理的CSS字符串内容
///
/// # 返回值
///
/// 返回处理后的CSS字符串，其中所有外部资源已被嵌入为data URI
///
/// # 示例
///
/// ```rust
/// let css = "body { background: url('image.jpg'); }";
/// let embedded = embed_css(&mut session, &document_url, css);
/// // 结果: "body { background: url('data:image/jpeg;base64,...'); }"
/// ```
pub fn embed_css(session: &mut Session, document_url: &Url, css: &str) -> String {
    let mut input = ParserInput::new(css);
    let mut parser = Parser::new(&mut input);

    process_css(session, document_url, &mut parser, "", "", "").unwrap()
}

/// 格式化CSS标识符
///
/// 使用cssparser库的标准化函数来正确格式化CSS标识符，
/// 确保特殊字符被正确转义，符合CSS语法规范。
///
/// # 参数
///
/// * `ident` - 要格式化的标识符字符串
///
/// # 返回值
///
/// 返回格式化后的标识符字符串
///
/// # 示例
///
/// ```rust
/// let formatted = format_ident("my-class");
/// // 对于普通标识符，通常保持不变: "my-class"
/// ```
pub fn format_ident(ident: &str) -> String {
    let mut res: String = "".to_string();
    let _ = serialize_identifier(ident, &mut res);
    res = res.trim_end().to_string();
    res
}

/// 格式化带引号的CSS字符串
///
/// 使用cssparser库的标准化函数来正确格式化CSS字符串值，
/// 自动添加适当的引号并转义特殊字符。
///
/// # 参数
///
/// * `string` - 要格式化的字符串内容
///
/// # 返回值
///
/// 返回带引号且正确转义的CSS字符串
///
/// # 示例
///
/// ```rust
/// let formatted = format_quoted_string("hello world");
/// // 结果: "\"hello world\""
/// ```
pub fn format_quoted_string(string: &str) -> String {
    let mut res: String = "".to_string();
    let _ = serialize_string(string, &mut res);
    res
}

/// 检查CSS属性是否可能包含图片URL
///
/// 根据预定义的CSS属性列表，判断给定的属性名称是否
/// 可能包含图片URL引用。这用于决定是否需要特殊处理
/// 该属性中的url()函数。
///
/// # 参数
///
/// * `prop_name` - 要检查的CSS属性名称
///
/// # 返回值
///
/// 如果属性可能包含图片URL则返回`true`，否则返回`false`
///
/// # 示例
///
/// ```rust
/// assert_eq!(is_image_url_prop("background-image"), true);
/// assert_eq!(is_image_url_prop("color"), false);
/// ```
pub fn is_image_url_prop(prop_name: &str) -> bool {
    CSS_PROPS_WITH_IMAGE_URLS
        .iter()
        .any(|p| prop_name.eq_ignore_ascii_case(p))
}

/// CSS处理上下文结构体
///
/// 用于在CSS解析过程中跟踪当前的解析状态和上下文信息。
/// 这个结构体帮助解析器了解当前正在处理的CSS规则类型
/// 和属性名称，从而做出正确的处理决策。
///
/// # 字段说明
///
/// * `current_rule` - 当前正在处理的CSS规则名称（如"import"、"font-face"等）
/// * `current_prop` - 当前正在处理的CSS属性名称
#[derive(Debug, Clone)]
pub struct CssProcessingContext {
    /// 当前正在处理的CSS at-rule名称（如"import"、"font-face"等）
    pub current_rule: String,
    /// 当前正在处理的CSS属性名称
    pub current_prop: String,
}

impl CssProcessingContext {
    /// 创建新的CSS处理上下文
    ///
    /// # 参数
    ///
    /// * `rule_name` - 初始的CSS规则名称
    /// * `prop_name` - 初始的CSS属性名称
    ///
    /// # 返回值
    ///
    /// 返回新创建的上下文实例
    pub fn new(rule_name: &str, prop_name: &str) -> Self {
        Self {
            current_rule: rule_name.to_string(),
            current_prop: prop_name.to_string(),
        }
    }
}

/// CSS解析和处理的核心函数
///
/// 这是CSS处理的核心函数，负责递归解析CSS内容并处理其中的各种token。
/// 函数会识别并处理@import规则、url()函数、各种CSS语法元素，
/// 并将外部资源嵌入为data URI。
///
/// # 参数
///
/// * `session` - 可变的会话对象，用于获取外部资源和访问配置
/// * `document_url` - 文档的基准URL，用于解析相对路径
/// * `parser` - CSS解析器实例
/// * `rule_name` - 当前处理的CSS规则名称
/// * `prop_name` - 当前处理的CSS属性名称  
/// * `func_name` - 当前处理的CSS函数名称
///
/// # 返回值
///
/// 返回处理后的CSS字符串，包装在Result中
///
/// # 错误
///
/// 当CSS解析过程中遇到语法错误时返回ParseError
pub fn process_css<'a>(
    session: &mut Session,
    document_url: &Url,
    parser: &mut Parser,
    rule_name: &str,
    prop_name: &str,
    func_name: &str,
) -> Result<String, ParseError<'static, String>> {
    let mut result = String::new();
    let mut context = CssProcessingContext::new(rule_name, prop_name);

    // 逐个处理CSS token，构建最终的CSS字符串
    loop {
        let token_offset = parser.position();
        // 获取下一个token，包括空白字符和注释
        let token = match parser.next_including_whitespace_and_comments() {
            Ok(token) => token,
            Err(_) => break,
        };

        // 根据token类型进行相应的处理
        let token_result = match &token {
            // 保持注释原样
            Token::Comment(_) => parser.slice_from(token_offset).to_string(),
            // CSS语法符号直接转换为字符串
            Token::Semicolon => ";".to_string(),
            Token::Colon => ":".to_string(),
            Token::Comma => ",".to_string(),
            Token::CloseParenthesis => ")".to_string(),
            Token::CloseSquareBracket => "]".to_string(),
            Token::CloseCurlyBracket => "}".to_string(),
            // CSS属性选择器操作符
            Token::IncludeMatch => "~=".to_string(),
            Token::DashMatch => "|=".to_string(),
            Token::PrefixMatch => "^=".to_string(),
            Token::SuffixMatch => "$=".to_string(),
            Token::SubstringMatch => "*=".to_string(),
            // HTML注释标记（在CSS中可能出现）
            Token::CDO => "<!--".to_string(),
            Token::CDC => "-->".to_string(),
            // 保持空白字符
            Token::WhiteSpace(value) => value.to_string(),
            // CSS标识符（属性名、值等）
            Token::Ident(value) => {
                context.current_rule.clear();
                context.current_prop = value.to_string();
                format_ident(value)
            }
            // CSS at-rule（@import、@font-face等）
            Token::AtKeyword(value) => {
                context.current_rule = value.to_string();
                // 如果禁用字体且当前是@font-face规则，则跳过
                if session.options.no_fonts && context.current_rule == "font-face" {
                    String::new()
                } else {
                    format!("@{}", value)
                }
            }
            // CSS哈希值（颜色或ID选择器）
            Token::Hash(value) => format!("#{}", value),
            Token::IDHash(value) => {
                context.current_rule.clear();
                format!("#{}", format_ident(value))
            }
            // 带引号的字符串（可能包含URL）
            Token::QuotedString(value) => {
                process_quoted_string_token(value, session, document_url, &mut context, func_name)?
            }
            // CSS数值
            Token::Number {
                has_sign, value, ..
            } => {
                let mut result = String::new();
                // 处理正负号
                if *has_sign && *value >= 0.0 {
                    result.push('+');
                }
                result.push_str(&value.to_string());
                result
            }
            // CSS百分比值
            Token::Percentage {
                has_sign,
                unit_value,
                ..
            } => {
                let mut result = String::new();
                if *has_sign && *unit_value >= 0.0 {
                    result.push('+');
                }
                result.push_str(&(unit_value * 100.0).to_string());
                result.push('%');
                result
            }
            // CSS带单位的数值（px、em、rem等）
            Token::Dimension {
                has_sign,
                value,
                unit,
                ..
            } => {
                let mut result = String::new();
                if *has_sign && *value >= 0.0 {
                    result.push('+');
                }
                result.push_str(&value.to_string());
                result.push_str(unit);
                result
            }
            // 未带引号的URL（url(xxx)格式）
            Token::UnquotedUrl(value) => {
                process_unquoted_url_token(value, session, document_url, &mut context)?
            }
            // CSS分隔符
            Token::Delim(value) => value.to_string(),
            Token::ParenthesisBlock | Token::SquareBracketBlock | Token::CurlyBracketBlock => {
                // 处理块结构token
                if session.options.no_fonts && context.current_rule == "font-face" {
                    String::new()
                } else {
                    let (open_char, close_char) = match token {
                        Token::ParenthesisBlock => ('(', ')'),
                        Token::SquareBracketBlock => ('[', ']'),
                        Token::CurlyBracketBlock => ('{', '}'),
                        _ => ('(', ')'), // fallback
                    };

                    let mut result = String::new();
                    result.push(open_char);

                    let block_css = parser
                        .parse_nested_block(|parser| {
                            process_css(
                                session,
                                document_url,
                                parser,
                                rule_name,
                                &context.current_prop,
                                func_name,
                            )
                        })
                        .unwrap();
                    result.push_str(&block_css);

                    result.push(close_char);
                    result
                }
            }
            Token::Function(name) => {
                // 处理函数token
                let function_name = name.to_string();
                let mut result = String::new();
                result.push_str(&function_name);
                result.push('(');

                let block_css = parser
                    .parse_nested_block(|parser| {
                        process_css(
                            session,
                            document_url,
                            parser,
                            rule_name,
                            &context.current_prop,
                            &function_name,
                        )
                    })
                    .unwrap();
                result.push_str(&block_css);

                result.push(')');
                result
            }
            Token::BadUrl(_) | Token::BadString(_) => String::new(),
        };

        result.push_str(&token_result);
    }

    // 确保空CSS真正为空（清理仅包含空白字符的情况）
    if !result.is_empty() && result.trim().is_empty() {
        result = result.trim().to_string();
    }

    Ok(result)
}

/// 处理带引号的字符串token
///
/// 根据当前的CSS上下文（@import规则或url()函数），
/// 对带引号的字符串进行相应的处理。
///
/// # 参数
///
/// * `value` - 字符串内容（不包含引号）
/// * `session` - 会话对象，用于获取外部资源
/// * `document_url` - 文档基准URL
/// * `context` - CSS处理上下文
/// * `func_name` - 当前函数名称
///
/// # 返回值
///
/// 返回处理后的CSS字符串
fn process_quoted_string_token(
    value: &str,
    session: &mut Session,
    document_url: &Url,
    context: &mut CssProcessingContext,
    func_name: &str,
) -> Result<String, ParseError<'static, String>> {
    if context.current_rule == "import" {
        // 处理@import规则中的字符串
        process_import_quoted_string(value, session, document_url, context)
    } else if func_name == "url" {
        // 处理url()函数中的字符串
        process_url_quoted_string(value, session, document_url, context)
    } else {
        // 普通字符串，仅格式化
        Ok(format_quoted_string(value))
    }
}

/// 处理@import规则中的带引号字符串
///
/// 当遇到`@import "style.css"`这样的语句时，此函数会：
/// 1. 获取导入的CSS文件内容
/// 2. 递归处理导入的CSS（嵌入其中的资源）
/// 3. 将处理后的CSS转换为data URI
///
/// # 参数
///
/// * `value` - 导入的CSS文件路径
/// * `session` - 会话对象
/// * `document_url` - 当前文档URL
/// * `context` - CSS处理上下文
///
/// # 返回值
///
/// 返回包含嵌入CSS的data URI字符串
fn process_import_quoted_string(
    value: &str,
    session: &mut Session,
    document_url: &Url,
    context: &mut CssProcessingContext,
) -> Result<String, ParseError<'static, String>> {
    // 重置当前at-rule状态
    context.current_rule.clear();

    // 跳过空的导入值
    if value.is_empty() {
        return Ok("''".to_string());
    }

    let import_full_url = resolve_url(document_url, value);
    // 尝试获取导入的CSS文件
    match session.retrieve_asset(document_url, &import_full_url) {
        Ok((import_contents, import_final_url, import_media_type, import_charset)) => {
            // 成功获取CSS文件，递归处理并创建data URI
            let mut import_data_url = create_data_url(
                &import_media_type,
                &import_charset,
                embed_css(
                    session,
                    &import_final_url,
                    &String::from_utf8_lossy(&import_contents),
                )
                .as_bytes(),
                &import_final_url,
            );
            // 保持URL片段（#fragment）
            import_data_url.set_fragment(import_full_url.fragment());
            Ok(format_quoted_string(import_data_url.as_ref()))
        }
        Err(_) => {
            // 如果无法获取资源，保留远程引用
            if import_full_url.scheme() == "http" || import_full_url.scheme() == "https" {
                Ok(format_quoted_string(import_full_url.as_ref()))
            } else {
                // 本地文件无法访问时返回空字符串
                Ok(String::new())
            }
        }
    }
}

/// 处理url()函数中的带引号字符串
///
/// 当遇到`url("image.jpg")`这样的语句时，此函数会：
/// 1. 检查是否需要处理该类型的资源（根据配置）
/// 2. 获取资源内容并转换为data URI
/// 3. 如果获取失败，保留原始URL引用
///
/// # 参数
///
/// * `value` - URL路径字符串
/// * `session` - 会话对象
/// * `document_url` - 当前文档URL
/// * `context` - CSS处理上下文
///
/// # 返回值
///
/// 返回处理后的URL字符串（data URI或原始URL）
fn process_url_quoted_string(
    value: &str,
    session: &mut Session,
    document_url: &Url,
    context: &CssProcessingContext,
) -> Result<String, ParseError<'static, String>> {
    // 跳过空的URL值
    if value.is_empty() {
        return Ok(String::new());
    }

    // 如果禁用图片且当前属性包含图片URL，返回空图片
    if session.options.no_images && is_image_url_prop(&context.current_prop) {
        return Ok(format_quoted_string(EMPTY_IMAGE_DATA_URL));
    }

    let resolved_url = resolve_url(document_url, value);
    // 尝试获取URL指向的资源
    match session.retrieve_asset(document_url, &resolved_url) {
        Ok((data, final_url, media_type, charset)) => {
            // 成功获取资源，创建data URI
            let mut data_url = create_data_url(&media_type, &charset, &data, &final_url);
            data_url.set_fragment(resolved_url.fragment());
            Ok(format_quoted_string(data_url.as_ref()))
        }
        Err(_) => {
            // 如果无法获取资源，保留远程引用
            if resolved_url.scheme() == "http" || resolved_url.scheme() == "https" {
                Ok(format_quoted_string(resolved_url.as_ref()))
            } else {
                // 本地文件无法访问时返回空字符串
                Ok(String::new())
            }
        }
    }
}

/// 处理未带引号的URL token
///
/// 处理`url(image.jpg)`（不带引号）格式的URL引用。
/// 根据上下文判断是@import还是普通URL引用，并进行相应处理。
///
/// # 参数
///
/// * `value` - URL路径字符串
/// * `session` - 会话对象
/// * `document_url` - 当前文档URL
/// * `context` - CSS处理上下文（可变，因为可能需要更新状态）
///
/// # 返回值
///
/// 返回完整的url()表达式字符串
fn process_unquoted_url_token(
    value: &str,
    session: &mut Session,
    document_url: &Url,
    context: &mut CssProcessingContext,
) -> Result<String, ParseError<'static, String>> {
    let is_import = context.current_rule == "import";

    if is_import {
        // 处理完@import后清除规则状态
        context.current_rule.clear();
    }

    // 跳过空的URL值
    if value.is_empty() {
        return Ok("url()".to_string());
    }

    // 跳过仅包含片段标识符的URL（如#section）
    if value.starts_with('#') {
        return Ok(format!("url({})", value));
    }

    let mut result = "url(".to_string();

    if is_import {
        // 处理@import中的URL
        process_import_unquoted_url(value, session, document_url, &mut result)?;
    } else if is_image_url_prop(&context.current_prop) && session.options.no_images {
        // 如果禁用图片，使用空图片占位符
        result.push_str(&format_quoted_string(EMPTY_IMAGE_DATA_URL));
    } else {
        // 处理普通URL引用
        process_regular_unquoted_url(value, session, document_url, &mut result)?;
    }

    result.push(')');
    Ok(result)
}

/// 处理@import规则中的未带引号URL
///
/// 处理如`@import url(style.css)`这样的导入语句，
/// 获取CSS文件内容并将其转换为嵌入式data URI。
///
/// # 参数
///
/// * `value` - CSS文件的URL路径
/// * `session` - 会话对象
/// * `document_url` - 当前文档URL
/// * `result` - 用于构建最终URL字符串的可变引用
///
/// # 返回值
///
/// 返回处理结果，成功时result参数被修改
fn process_import_unquoted_url(
    value: &str,
    session: &mut Session,
    document_url: &Url,
    result: &mut String,
) -> Result<(), ParseError<'static, String>> {
    let full_url = resolve_url(document_url, value);
    // 尝试获取导入的CSS文件
    match session.retrieve_asset(document_url, &full_url) {
        Ok((css, final_url, media_type, charset)) => {
            // 成功获取CSS文件，递归处理并创建data URI
            let mut data_url = create_data_url(
                &media_type,
                &charset,
                embed_css(session, &final_url, &String::from_utf8_lossy(&css)).as_bytes(),
                &final_url,
            );
            data_url.set_fragment(full_url.fragment());
            result.push_str(&format_quoted_string(data_url.as_ref()));
        }
        Err(_) => {
            // 如果无法获取资源，保留远程引用
            if full_url.scheme() == "http" || full_url.scheme() == "https" {
                result.push_str(&format_quoted_string(full_url.as_ref()));
            }
            // 本地文件无法访问时不添加任何内容
        }
    }
    Ok(())
}

/// 处理常规的未带引号URL
///
/// 处理CSS中普通的URL引用，如背景图片、字体文件等。
/// 将资源内容转换为data URI以实现资源嵌入。
///
/// # 参数
///
/// * `value` - 资源的URL路径
/// * `session` - 会话对象
/// * `document_url` - 当前文档URL
/// * `result` - 用于构建最终URL字符串的可变引用
///
/// # 返回值
///
/// 返回处理结果，成功时result参数被修改
fn process_regular_unquoted_url(
    value: &str,
    session: &mut Session,
    document_url: &Url,
    result: &mut String,
) -> Result<(), ParseError<'static, String>> {
    let full_url = resolve_url(document_url, value);
    // 尝试获取URL指向的资源
    match session.retrieve_asset(document_url, &full_url) {
        Ok((data, final_url, media_type, charset)) => {
            // 成功获取资源，创建data URI
            let mut data_url = create_data_url(&media_type, &charset, &data, &final_url);
            data_url.set_fragment(full_url.fragment());
            result.push_str(&format_quoted_string(data_url.as_ref()));
        }
        Err(_) => {
            // 如果无法获取资源，保留远程引用
            if full_url.scheme() == "http" || full_url.scheme() == "https" {
                result.push_str(&format_quoted_string(full_url.as_ref()));
            }
            // 本地文件无法访问时不添加任何内容
        }
    }
    Ok(())
}
