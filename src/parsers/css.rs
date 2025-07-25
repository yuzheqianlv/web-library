use cssparser::{serialize_identifier, serialize_string, ParseError, Parser, ParserInput, Token};

use crate::network::session::Session;
use crate::utils::url::{create_data_url, resolve_url, Url, EMPTY_IMAGE_DATA_URL};

// CSS处理模块化功能

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

pub fn embed_css(session: &mut Session, document_url: &Url, css: &str) -> String {
    let mut input = ParserInput::new(css);
    let mut parser = Parser::new(&mut input);

    process_css(session, document_url, &mut parser, "", "", "").unwrap()
}

pub fn format_ident(ident: &str) -> String {
    let mut res: String = "".to_string();
    let _ = serialize_identifier(ident, &mut res);
    res = res.trim_end().to_string();
    res
}

pub fn format_quoted_string(string: &str) -> String {
    let mut res: String = "".to_string();
    let _ = serialize_string(string, &mut res);
    res
}

pub fn is_image_url_prop(prop_name: &str) -> bool {
    CSS_PROPS_WITH_IMAGE_URLS
        .iter()
        .any(|p| prop_name.eq_ignore_ascii_case(p))
}

/// CSS处理上下文，跟踪当前解析状态
#[derive(Debug, Clone)]
pub struct CssProcessingContext {
    pub current_rule: String,
    pub current_prop: String,
}

impl CssProcessingContext {
    pub fn new(rule_name: &str, prop_name: &str) -> Self {
        Self {
            current_rule: rule_name.to_string(),
            current_prop: prop_name.to_string(),
        }
    }
}

/// 主要的CSS处理函数，经过模块化重构
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

    loop {
        let token_offset = parser.position();
        let token = match parser.next_including_whitespace_and_comments() {
            Ok(token) => token,
            Err(_) => break,
        };

        let token_result = match &token {
            Token::Comment(_) => parser.slice_from(token_offset).to_string(),
            Token::Semicolon => ";".to_string(),
            Token::Colon => ":".to_string(),
            Token::Comma => ",".to_string(),
            Token::CloseParenthesis => ")".to_string(),
            Token::CloseSquareBracket => "]".to_string(),
            Token::CloseCurlyBracket => "}".to_string(),
            Token::IncludeMatch => "~=".to_string(),
            Token::DashMatch => "|=".to_string(),
            Token::PrefixMatch => "^=".to_string(),
            Token::SuffixMatch => "$=".to_string(),
            Token::SubstringMatch => "*=".to_string(),
            Token::CDO => "<!--".to_string(),
            Token::CDC => "-->".to_string(),
            Token::WhiteSpace(value) => value.to_string(),
            Token::Ident(value) => {
                context.current_rule.clear();
                context.current_prop = value.to_string();
                format_ident(value)
            }
            Token::AtKeyword(value) => {
                context.current_rule = value.to_string();
                if session.options.no_fonts && context.current_rule == "font-face" {
                    String::new()
                } else {
                    format!("@{}", value)
                }
            }
            Token::Hash(value) => format!("#{}", value),
            Token::IDHash(value) => {
                context.current_rule.clear();
                format!("#{}", format_ident(value))
            }
            Token::QuotedString(value) => {
                process_quoted_string_token(value, session, document_url, &mut context, func_name)?
            }
            Token::Number {
                has_sign, value, ..
            } => {
                let mut result = String::new();
                if *has_sign && *value >= 0.0 {
                    result.push('+');
                }
                result.push_str(&value.to_string());
                result
            }
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
            Token::UnquotedUrl(value) => {
                process_unquoted_url_token(value, session, document_url, &mut context)?
            }
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

    // Ensure empty CSS is really empty
    if !result.is_empty() && result.trim().is_empty() {
        result = result.trim().to_string();
    }

    Ok(result)
}

/// 处理引用字符串token
fn process_quoted_string_token(
    value: &str,
    session: &mut Session,
    document_url: &Url,
    context: &mut CssProcessingContext,
    func_name: &str,
) -> Result<String, ParseError<'static, String>> {
    if context.current_rule == "import" {
        process_import_quoted_string(value, session, document_url, context)
    } else if func_name == "url" {
        process_url_quoted_string(value, session, document_url, context)
    } else {
        Ok(format_quoted_string(value))
    }
}

/// 处理@import规则中的引用字符串
fn process_import_quoted_string(
    value: &str,
    session: &mut Session,
    document_url: &Url,
    context: &mut CssProcessingContext,
) -> Result<String, ParseError<'static, String>> {
    // Reset current at-rule value
    context.current_rule.clear();

    // Skip empty import values
    if value.is_empty() {
        return Ok("''".to_string());
    }

    let import_full_url = resolve_url(document_url, value);
    match session.retrieve_asset(document_url, &import_full_url) {
        Ok((import_contents, import_final_url, import_media_type, import_charset)) => {
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
            import_data_url.set_fragment(import_full_url.fragment());
            Ok(format_quoted_string(import_data_url.as_ref()))
        }
        Err(_) => {
            // Keep remote reference if unable to retrieve the asset
            if import_full_url.scheme() == "http" || import_full_url.scheme() == "https" {
                Ok(format_quoted_string(import_full_url.as_ref()))
            } else {
                Ok(String::new())
            }
        }
    }
}

/// 处理url()函数中的引用字符串
fn process_url_quoted_string(
    value: &str,
    session: &mut Session,
    document_url: &Url,
    context: &CssProcessingContext,
) -> Result<String, ParseError<'static, String>> {
    // Skip empty url()'s
    if value.is_empty() {
        return Ok(String::new());
    }

    if session.options.no_images && is_image_url_prop(&context.current_prop) {
        return Ok(format_quoted_string(EMPTY_IMAGE_DATA_URL));
    }

    let resolved_url = resolve_url(document_url, value);
    match session.retrieve_asset(document_url, &resolved_url) {
        Ok((data, final_url, media_type, charset)) => {
            let mut data_url = create_data_url(&media_type, &charset, &data, &final_url);
            data_url.set_fragment(resolved_url.fragment());
            Ok(format_quoted_string(data_url.as_ref()))
        }
        Err(_) => {
            // Keep remote reference if unable to retrieve the asset
            if resolved_url.scheme() == "http" || resolved_url.scheme() == "https" {
                Ok(format_quoted_string(resolved_url.as_ref()))
            } else {
                Ok(String::new())
            }
        }
    }
}

/// 处理未引用URL token
fn process_unquoted_url_token(
    value: &str,
    session: &mut Session,
    document_url: &Url,
    context: &mut CssProcessingContext,
) -> Result<String, ParseError<'static, String>> {
    let is_import = context.current_rule == "import";

    if is_import {
        context.current_rule.clear();
    }

    // Skip empty url()'s
    if value.is_empty() {
        return Ok("url()".to_string());
    }

    // Skip hash-only URLs
    if value.starts_with('#') {
        return Ok(format!("url({})", value));
    }

    let mut result = "url(".to_string();

    if is_import {
        process_import_unquoted_url(value, session, document_url, &mut result)?;
    } else if is_image_url_prop(&context.current_prop) && session.options.no_images {
        result.push_str(&format_quoted_string(EMPTY_IMAGE_DATA_URL));
    } else {
        process_regular_unquoted_url(value, session, document_url, &mut result)?;
    }

    result.push(')');
    Ok(result)
}

/// 处理@import中的未引用URL
fn process_import_unquoted_url(
    value: &str,
    session: &mut Session,
    document_url: &Url,
    result: &mut String,
) -> Result<(), ParseError<'static, String>> {
    let full_url = resolve_url(document_url, value);
    match session.retrieve_asset(document_url, &full_url) {
        Ok((css, final_url, media_type, charset)) => {
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
            // Keep remote reference if unable to retrieve the asset
            if full_url.scheme() == "http" || full_url.scheme() == "https" {
                result.push_str(&format_quoted_string(full_url.as_ref()));
            }
        }
    }
    Ok(())
}

/// 处理常规未引用URL
fn process_regular_unquoted_url(
    value: &str,
    session: &mut Session,
    document_url: &Url,
    result: &mut String,
) -> Result<(), ParseError<'static, String>> {
    let full_url = resolve_url(document_url, value);
    match session.retrieve_asset(document_url, &full_url) {
        Ok((data, final_url, media_type, charset)) => {
            let mut data_url = create_data_url(&media_type, &charset, &data, &final_url);
            data_url.set_fragment(full_url.fragment());
            result.push_str(&format_quoted_string(data_url.as_ref()));
        }
        Err(_) => {
            // Keep remote reference if unable to retrieve the asset
            if full_url.scheme() == "http" || full_url.scheme() == "https" {
                result.push_str(&format_quoted_string(full_url.as_ref()));
            }
        }
    }
    Ok(())
}
