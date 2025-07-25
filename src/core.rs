use chrono::{SecondsFormat, Utc};
use encoding_rs::Encoding;
use markup5ever_rcdom::RcDom;
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::Path;
use url::Url;

use crate::network::session::Session;
use crate::parsers::html::{
    add_favicon, create_metadata_tag, get_base_url, get_charset, get_robots, get_title,
    has_favicon, html_to_dom, serialize_document, set_base_url, set_charset, set_robots, walk,
};
use crate::utils::url::{create_data_url, resolve_url};

// 文档处理器模块（内联整合到core.rs中）

/// Represents errors that can occur during monolith processing
///
/// This error type encapsulates all possible errors that can occur
/// when processing a document with the monolith library.
#[derive(Debug)]
pub struct MonolithError {
    details: String,
}

impl MonolithError {
    /// Creates a new MonolithError with the given message
    ///
    /// # Arguments
    ///
    /// * `msg` - The error message describing what went wrong
    ///
    /// # Returns
    ///
    /// A new MonolithError instance
    pub fn new(msg: &str) -> MonolithError {
        MonolithError {
            details: msg.to_string(),
        }
    }
}

impl fmt::Display for MonolithError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.details)
    }
}

impl Error for MonolithError {
    fn description(&self) -> &str {
        &self.details
    }
}

/// Supported output formats for monolithic documents
///
/// This enum defines the available output formats for processed documents.
/// HTML is the default format.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum MonolithOutputFormat {
    /// Standard HTML format (default)
    #[default]
    HTML,
    /// MHTML format for email compatibility
    MHTML,
    // WARC,
    // ZIM,
    // HAR,
}

/// Configuration options for monolith processing
///
/// This struct contains all the configuration options that control
/// how a document is processed and what assets are included or excluded.
#[derive(Default, Clone)]
pub struct MonolithOptions {
    pub base_url: Option<String>,
    pub blacklist_domains: bool,
    pub domains: Option<Vec<String>>,
    pub encoding: Option<String>,
    pub ignore_errors: bool,
    pub insecure: bool,
    pub isolate: bool,
    pub no_audio: bool,
    pub no_css: bool,
    pub no_fonts: bool,
    pub no_frames: bool,
    pub no_images: bool,
    pub no_js: bool,
    pub no_metadata: bool,
    pub no_video: bool,
    pub output_format: MonolithOutputFormat,
    pub silent: bool,
    pub timeout: u64,
    pub unwrap_noscript: bool,
    pub user_agent: Option<String>,
    // Translation options
    #[cfg(feature = "translation")]
    pub enable_translation: bool,
    #[cfg(feature = "translation")]
    pub target_language: Option<String>,
    #[cfg(feature = "translation")]
    pub translation_api_url: Option<String>,
    #[cfg(feature = "translation")]
    pub rewrite_links: bool,
    #[cfg(feature = "translation")]
    pub translation_base_path: Option<String>,
}

const ANSI_COLOR_RED: &str = "\x1b[31m";
const ANSI_COLOR_RESET: &str = "\x1b[0m";
const FILE_SIGNATURES: [[&[u8]; 2]; 18] = [
    // Image
    [b"GIF87a", b"image/gif"],
    [b"GIF89a", b"image/gif"],
    [b"\xFF\xD8\xFF", b"image/jpeg"],
    [b"\x89PNG\x0D\x0A\x1A\x0A", b"image/png"],
    [b"<svg ", b"image/svg+xml"],
    [b"RIFF....WEBPVP8 ", b"image/webp"],
    [b"\x00\x00\x01\x00", b"image/x-icon"],
    // Audio
    [b"ID3", b"audio/mpeg"],
    [b"\xFF\x0E", b"audio/mpeg"],
    [b"\xFF\x0F", b"audio/mpeg"],
    [b"OggS", b"audio/ogg"],
    [b"RIFF....WAVEfmt ", b"audio/wav"],
    [b"fLaC", b"audio/x-flac"],
    // Video
    [b"RIFF....AVI LIST", b"video/avi"],
    [b"....ftyp", b"video/mp4"],
    [b"\x00\x00\x01\x0B", b"video/mpeg"],
    [b"....moov", b"video/quicktime"],
    [b"\x1A\x45\xDF\xA3", b"video/webm"],
];
// All known non-"text/..." plaintext media types
const PLAINTEXT_MEDIA_TYPES: &[&str] = &[
    "application/javascript",          // .js
    "application/json",                // .json
    "application/ld+json",             // .jsonld
    "application/x-sh",                // .sh
    "application/xhtml+xml",           // .xhtml
    "application/xml",                 // .xml
    "application/vnd.mozilla.xul+xml", // .xul
    "image/svg+xml",                   // .svg
];

/// Creates a monolithic document from raw data
///
/// 这个函数现在被重构为使用模块化的文档处理器系统，
/// 大大简化了原来的198行复杂函数
///
/// # Arguments
///
/// * `session` - The session containing configuration and cache
/// * `input_data` - Raw HTML data as bytes
/// * `input_encoding` - Optional character encoding (defaults to UTF-8)
/// * `input_target` - Optional target URL for resolving relative links
///
/// # Returns
///
/// Returns a tuple containing the processed document bytes and optional title,
/// or an error if processing fails.
///
/// # Examples
///
/// ```
/// use monolith::core::{create_monolithic_document_from_data, MonolithOptions};
/// use monolith::session::Session;
///
/// let options = MonolithOptions::default();
/// let session = Session::new(None, None, options);
/// let html = b"<html><body>Hello World</body></html>";
///
/// let result = create_monolithic_document_from_data(
///     session,
///     html.to_vec(),
///     None,
///     None
/// );
/// ```
pub fn create_monolithic_document_from_data(
    session: Session,
    input_data: Vec<u8>,
    input_encoding: Option<String>,
    input_target: Option<String>,
) -> Result<(Vec<u8>, Option<String>), MonolithError> {
    let processor = DocumentProcessor::new(session);
    processor.process_document(input_data, input_encoding, input_target)
}

/// Creates a monolithic document from a URL or file path
///
/// Fetches content from the given target and processes it into a self-contained document.
/// This function handles both local file paths and remote URLs.
///
/// # Arguments
///
/// * `session` - The session containing configuration and cache
/// * `target` - URL or file path to process
///
/// # Returns
///
/// Returns a tuple containing the processed document bytes and optional title,
/// or an error if processing fails.
///
/// # Examples
///
/// ```
/// use monolith::core::{create_monolithic_document, MonolithOptions};
/// use monolith::session::Session;
///
/// let options = MonolithOptions::default();
/// let session = Session::new(None, None, options);
///
/// let result = create_monolithic_document(session, "https://example.com");
/// ```
pub fn create_monolithic_document(
    mut session: Session,
    target: &str,
) -> Result<(Vec<u8>, Option<String>), MonolithError> {
    // 验证和解析目标URL以及获取数据
    let (input_data, target_url) = if target.starts_with("data:") {
        // Data URL
        if let Ok(parsed_url) = Url::parse(target) {
            let (media_type, _, data) = crate::utils::url::parse_data_url(&parsed_url);
            if media_type == "text/html" {
                (data, Some(parsed_url))
            } else {
                return Err(MonolithError::new(
                    "Can only process HTML documents from data URLs",
                ));
            }
        } else {
            return Err(MonolithError::new("Invalid data URL"));
        }
    } else if target.starts_with("http://") || target.starts_with("https://") {
        // HTTP/HTTPS URL
        if let Ok(parsed_url) = Url::parse(target) {
            match session.retrieve_asset(&parsed_url, &parsed_url) {
                Ok((data, _, media_type, _)) => {
                    if is_plaintext_media_type(&media_type) {
                        (data, Some(parsed_url))
                    } else {
                        return Err(MonolithError::new(&format!(
                            "Unsupported media type: {media_type}"
                        )));
                    }
                }
                Err(e) => {
                    return Err(MonolithError::new(&format!("Failed to fetch URL: {e}")));
                }
            }
        } else {
            return Err(MonolithError::new("Invalid URL"));
        }
    } else {
        // 本地文件路径
        let path = Path::new(target);
        if !path.exists() {
            return Err(MonolithError::new(&format!("File not found: {target}")));
        }

        match fs::read(path) {
            Ok(data) => {
                // 为本地文件创建file:// URL
                let file_url = if let Ok(canonical_path) = fs::canonicalize(path) {
                    Url::from_file_path(canonical_path).ok()
                } else {
                    None
                };
                (data, file_url)
            }
            Err(e) => {
                return Err(MonolithError::new(&format!("Failed to read file: {e}")));
            }
        }
    };

    // 使用重构后的处理器处理文档
    create_monolithic_document_from_data(
        session,
        input_data,
        None,
        target_url.map(|u| u.to_string()),
    )
}

/// Determines the media type of data based on its content signature
pub fn detect_media_type(data: &[u8], url: &Url) -> String {
    // First try to detect by magic bytes
    for signature in &FILE_SIGNATURES {
        let sig_bytes = signature[0];
        if data.len() >= sig_bytes.len() && data.starts_with(sig_bytes) {
            return String::from_utf8_lossy(signature[1]).to_string();
        }
    }

    // Fall back to detecting by file extension
    detect_media_type_by_file_name(url.path())
}

/// Determines the media type based on file extension
pub fn detect_media_type_by_file_name(filename: &str) -> String {
    let filename_lowercased = filename.to_lowercase();

    if filename_lowercased.ends_with(".css") {
        "text/css".to_string()
    } else if filename_lowercased.ends_with(".js") {
        "application/javascript".to_string()
    } else if filename_lowercased.ends_with(".json") {
        "application/json".to_string()
    } else if filename_lowercased.ends_with(".svg") {
        "image/svg+xml".to_string()
    } else if filename_lowercased.ends_with(".png") {
        "image/png".to_string()
    } else if filename_lowercased.ends_with(".jpg") || filename_lowercased.ends_with(".jpeg") {
        "image/jpeg".to_string()
    } else if filename_lowercased.ends_with(".gif") {
        "image/gif".to_string()
    } else if filename_lowercased.ends_with(".webp") {
        "image/webp".to_string()
    } else if filename_lowercased.ends_with(".ico") {
        "image/x-icon".to_string()
    } else if filename_lowercased.ends_with(".mp3") {
        "audio/mpeg".to_string()
    } else if filename_lowercased.ends_with(".ogg") {
        "audio/ogg".to_string()
    } else if filename_lowercased.ends_with(".wav") {
        "audio/wav".to_string()
    } else if filename_lowercased.ends_with(".flac") {
        "audio/x-flac".to_string()
    } else if filename_lowercased.ends_with(".mp4") {
        "video/mp4".to_string()
    } else if filename_lowercased.ends_with(".avi") {
        "video/avi".to_string()
    } else if filename_lowercased.ends_with(".mov") {
        "video/quicktime".to_string()
    } else if filename_lowercased.ends_with(".webm") {
        "video/webm".to_string()
    } else if filename_lowercased.ends_with(".mpeg") || filename_lowercased.ends_with(".mpg") {
        "video/mpeg".to_string()
    } else {
        "application/octet-stream".to_string()
    }
}

/// Parses Content-Type header value
pub fn parse_content_type(content_type: &str) -> (String, String, bool) {
    let mut media_type = String::new();
    let mut charset = String::new();
    let mut is_base64 = false;

    let parts: Vec<&str> = content_type.split(';').collect();

    if !parts.is_empty() {
        media_type = parts[0].trim().to_lowercase();
    }

    for part in parts.iter().skip(1) {
        let part = part.trim();
        if part.starts_with("charset=") {
            charset = part[8..].trim_matches('"').to_string();
        } else if part == "base64" {
            is_base64 = true;
        }
    }

    (media_type, charset, is_base64)
}

/// Checks if the given media type represents plaintext content
pub fn is_plaintext_media_type(media_type: &str) -> bool {
    media_type.starts_with("text/") || PLAINTEXT_MEDIA_TYPES.contains(&media_type)
}

/// Formats output path with title substitution and sanitization
pub fn format_output_path(path: &str, document_title: Option<&str>, is_mhtml: bool) -> String {
    let datetime: &str = &Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
    let title = document_title.unwrap_or("");

    path.replace("%timestamp%", &datetime.replace(':', "_"))
        .replace(
            "%title%",
            &title
                .to_string()
                .replace(['/', '\\'], "_")
                .replace('<', "[")
                .replace('>', "]")
                .replace(':', " - ")
                .replace('\"', "")
                .replace('|', "-")
                .replace('?', "")
                .trim_start_matches('.'),
        )
        .replace("%extension%", if is_mhtml { "mhtml" } else { "html" })
        .replace("%ext%", if is_mhtml { "mht" } else { "htm" })
}

/// Prints an error message to stderr
pub fn print_error_message(msg: &str) {
    eprintln!("{ANSI_COLOR_RED}{msg}{ANSI_COLOR_RESET}");
}

/// Prints an info message to stdout
pub fn print_info_message(msg: &str) {
    println!("{msg}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monolith_error_new() {
        let error = MonolithError::new("test error");
        assert_eq!(error.details, "test error");
    }

    #[test]
    fn test_monolith_error_display() {
        let error = MonolithError::new("test error");
        assert_eq!(format!("{}", error), "test error");
    }

    #[test]
    fn test_detect_media_type_by_file_name_common_types() {
        assert_eq!(detect_media_type_by_file_name("style.css"), "text/css");
        assert_eq!(
            detect_media_type_by_file_name("script.js"),
            "application/javascript"
        );
        assert_eq!(detect_media_type_by_file_name("image.png"), "image/png");
        assert_eq!(detect_media_type_by_file_name("photo.jpg"), "image/jpeg");
    }

    #[test]
    fn test_detect_media_type_by_file_name_case_insensitive() {
        assert_eq!(detect_media_type_by_file_name("STYLE.CSS"), "text/css");
        assert_eq!(
            detect_media_type_by_file_name("Script.JS"),
            "application/javascript"
        );
    }

    #[test]
    fn test_detect_media_type_by_file_name_unknown_extension() {
        assert_eq!(
            detect_media_type_by_file_name("file.unknown"),
            "application/octet-stream"
        );
    }

    #[test]
    fn test_detect_media_type_by_file_name_multiple_dots() {
        assert_eq!(
            detect_media_type_by_file_name("file.min.js"),
            "application/javascript"
        );
    }

    #[test]
    fn test_detect_media_type_with_url() {
        let url = Url::parse("https://example.com/style.css").unwrap();
        let data = b"body { color: red; }";
        assert_eq!(detect_media_type(data, &url), "text/css");
    }

    #[test]
    fn test_parse_content_type_basic() {
        let (media_type, charset, is_base64) = parse_content_type("text/html");
        assert_eq!(media_type, "text/html");
        assert_eq!(charset, "");
        assert!(!is_base64);
    }

    #[test]
    fn test_parse_content_type_with_charset() {
        let (media_type, charset, is_base64) = parse_content_type("text/html; charset=utf-8");
        assert_eq!(media_type, "text/html");
        assert_eq!(charset, "utf-8");
        assert!(!is_base64);
    }

    #[test]
    fn test_parse_content_type_complex() {
        let (media_type, charset, is_base64) =
            parse_content_type("text/html; charset=\"utf-8\"; boundary=something");
        assert_eq!(media_type, "text/html");
        assert_eq!(charset, "utf-8");
        assert!(!is_base64);
    }

    #[test]
    fn test_parse_content_type_empty() {
        let (media_type, charset, is_base64) = parse_content_type("");
        assert_eq!(media_type, "");
        assert_eq!(charset, "");
        assert!(!is_base64);
    }

    #[test]
    fn test_parse_content_type_with_base64() {
        let (media_type, charset, is_base64) =
            parse_content_type("text/plain; charset=utf-8; base64");
        assert_eq!(media_type, "text/plain");
        assert_eq!(charset, "utf-8");
        assert!(is_base64);
    }

    #[test]
    fn test_is_plaintext_media_type() {
        assert!(is_plaintext_media_type("text/html"));
        assert!(is_plaintext_media_type("text/css"));
        assert!(is_plaintext_media_type("application/javascript"));
        assert!(is_plaintext_media_type("application/json"));
        assert!(!is_plaintext_media_type("image/png"));
        assert!(!is_plaintext_media_type("video/mp4"));
    }

    #[test]
    fn test_format_output_path_basic() {
        let result = format_output_path("output", None, false);
        assert_eq!(result, "output");
    }

    #[test]
    fn test_format_output_path_with_title() {
        let result = format_output_path("%title%", Some("Test Page"), false);
        assert_eq!(result, "Test Page");
    }

    #[test]
    fn test_format_output_path_title_sanitization() {
        let result = format_output_path("%title%", Some("Test/Page<>"), false);
        assert_eq!(result, "Test_Page[]");
    }

    #[test]
    fn test_format_output_path_mhtml() {
        let result = format_output_path("output", None, true);
        assert_eq!(result, "output");
    }
}

/// 文档处理器，负责协调整个文档处理流程
pub struct DocumentProcessor {
    session: Session,
}

impl DocumentProcessor {
    pub fn new(session: Session) -> Self {
        Self { session }
    }

    /// 处理文档数据并返回最终结果
    pub fn process_document(
        mut self,
        input_data: Vec<u8>,
        input_encoding: Option<String>,
        input_target: Option<String>,
    ) -> Result<(Vec<u8>, Option<String>), MonolithError> {
        // 1. 验证配置
        let encoding_validator = EncodingValidator::new();
        encoding_validator.validate_options(&self.session.options)?;

        // 2. 确定基础URL和编码
        let url_resolver = UrlResolver::new();
        let mut base_url = url_resolver.determine_base_url(&input_target)?;

        let encoding_processor = EncodingProcessor::new();
        let (dom, document_encoding) =
            encoding_processor.process_encoding(&input_data, input_encoding)?;

        // 3. 解析自定义基础URL
        base_url = url_resolver.resolve_custom_base_url(base_url, &dom, &self.session.options)?;

        // 4. 遍历DOM并嵌入资源
        let dom_processor = DomProcessor::new();
        let dom = dom_processor.process_dom(dom, &mut self.session, &base_url)?;

        // 5. 翻译处理（如果启用）
        #[cfg(feature = "translation")]
        let dom = self.process_translation(dom)?;

        #[cfg(not(feature = "translation"))]
        let dom = dom;

        // 6. 更新BASE元素
        let dom = self.update_base_element(dom)?;

        // 7. 处理favicon
        let dom = self.process_favicon(dom, &base_url, &input_target)?;

        // 8. 设置robots meta标签
        let dom = self.set_robots_meta(dom);

        // 9. 处理自定义编码
        let (dom, final_encoding) = self.process_custom_encoding(dom, document_encoding)?;

        // 10. 序列化并格式化输出
        let document_title = get_title(&dom.document);
        let output_formatter = OutputFormatter::new(&self.session.options, &input_target);
        let result = output_formatter.format_output(dom, final_encoding)?;

        Ok((result, document_title))
    }

    #[cfg(feature = "translation")]
    fn process_translation(&self, dom: RcDom) -> Result<RcDom, MonolithError> {
        if self.session.options.enable_translation {
            use crate::translation::translate_dom_content_sync;

            let translated_dom = translate_dom_content_sync(
                dom,
                self.session
                    .options
                    .target_language
                    .as_deref()
                    .unwrap_or("zh"),
                self.session.options.translation_api_url.as_deref(),
            )
            .map_err(|e| MonolithError::new(&format!("Translation error: {e}")))?;

            if !self.session.options.silent {
                println!("Translation completed");
            }

            Ok(translated_dom)
        } else {
            Ok(dom)
        }
    }

    fn update_base_element(&self, dom: RcDom) -> Result<RcDom, MonolithError> {
        if let Some(new_base_url) = self.session.options.base_url.clone() {
            Ok(set_base_url(&dom.document, new_base_url))
        } else {
            Ok(dom)
        }
    }

    fn process_favicon(
        &mut self,
        dom: RcDom,
        base_url: &Url,
        input_target: &Option<String>,
    ) -> Result<RcDom, MonolithError> {
        let favicon_processor = FaviconProcessor::new();
        favicon_processor.process_favicon(dom, &mut self.session, base_url, input_target)
    }

    fn set_robots_meta(&self, dom: RcDom) -> RcDom {
        let meta_robots_content_value = get_robots(&dom.document).unwrap_or_default();
        if meta_robots_content_value.trim().is_empty() || meta_robots_content_value != "none" {
            set_robots(dom, "none")
        } else {
            dom
        }
    }

    fn process_custom_encoding(
        &self,
        dom: RcDom,
        document_encoding: String,
    ) -> Result<(RcDom, String), MonolithError> {
        if let Some(custom_encoding) = self.session.options.encoding.clone() {
            let new_dom = set_charset(dom, custom_encoding.clone());
            Ok((new_dom, custom_encoding))
        } else {
            Ok((dom, document_encoding))
        }
    }
}

/// 编码验证器
pub struct EncodingValidator;

impl EncodingValidator {
    pub fn new() -> Self {
        Self
    }

    pub fn validate_options(&self, options: &MonolithOptions) -> Result<(), MonolithError> {
        if let Some(custom_output_encoding) = &options.encoding {
            if Encoding::for_label_no_replacement(custom_output_encoding.as_bytes()).is_none() {
                return Err(MonolithError::new(&format!(
                    "unknown encoding \"{}\"",
                    custom_output_encoding
                )));
            }
        }
        Ok(())
    }
}

/// URL解析器
pub struct UrlResolver;

impl UrlResolver {
    pub fn new() -> Self {
        Self
    }

    pub fn determine_base_url(&self, input_target: &Option<String>) -> Result<Url, MonolithError> {
        if let Some(target) = input_target {
            Url::parse(target)
                .map_err(|e| MonolithError::new(&format!("Failed to parse target URL: {}", e)))
        } else {
            Ok(Url::parse("data:text/html,").unwrap())
        }
    }

    pub fn resolve_custom_base_url(
        &self,
        mut base_url: Url,
        dom: &RcDom,
        options: &MonolithOptions,
    ) -> Result<Url, MonolithError> {
        let custom_base_url = options.base_url.clone().unwrap_or_default();

        if custom_base_url.is_empty() {
            // 没有自定义基础URL；尝试从DOM获取BASE元素
            if let Some(existing_base_url) = get_base_url(&dom.document) {
                base_url = resolve_url(&base_url, &existing_base_url);
            }
        } else {
            // 提供了自定义基础URL
            base_url = self.parse_custom_base_url(base_url, &custom_base_url)?;
        }

        Ok(base_url)
    }

    fn parse_custom_base_url(
        &self,
        base_url: Url,
        custom_base_url: &str,
    ) -> Result<Url, MonolithError> {
        match Url::parse(custom_base_url) {
            Ok(parsed_url) => {
                if parsed_url.scheme() == "file" {
                    // File基础URL只能与文件系统保存的文档一起工作
                    if base_url.scheme() == "file" {
                        Ok(parsed_url)
                    } else {
                        Ok(base_url)
                    }
                } else {
                    Ok(parsed_url)
                }
            }
            Err(_) => {
                // 解析给定的基础URL失败，也许它是文件系统路径？
                if base_url.scheme() == "file" {
                    self.handle_filesystem_path(&custom_base_url)
                } else {
                    Ok(base_url)
                }
            }
        }
    }

    fn handle_filesystem_path(&self, custom_base_url: &str) -> Result<Url, MonolithError> {
        let path: &Path = Path::new(custom_base_url);
        if path.exists() {
            match Url::from_file_path(fs::canonicalize(path).unwrap()) {
                Ok(file_url) => Ok(file_url),
                Err(_) => Err(MonolithError::new(&format!(
                    "could not map given path to base URL \"{}\"",
                    custom_base_url
                ))),
            }
        } else {
            Err(MonolithError::new(&format!(
                "path does not exist: \"{}\"",
                custom_base_url
            )))
        }
    }
}

/// 编码处理器
pub struct EncodingProcessor;

impl EncodingProcessor {
    pub fn new() -> Self {
        Self
    }

    pub fn process_encoding(
        &self,
        input_data: &[u8],
        input_encoding: Option<String>,
    ) -> Result<(RcDom, String), MonolithError> {
        let mut document_encoding = input_encoding.unwrap_or_else(|| "utf-8".to_string());

        // 初始解析
        let mut dom = html_to_dom(input_data, document_encoding.clone());

        // 尝试确定文档的编码
        if let Some(html_charset) = get_charset(&dom.document) {
            if !html_charset.is_empty() {
                // 检查HTML内部指定的字符集是否有效
                if let Some(document_charset) =
                    Encoding::for_label_no_replacement(html_charset.as_bytes())
                {
                    document_encoding = html_charset;
                    dom = html_to_dom(input_data, document_charset.name().to_string());
                }
            }
        }

        Ok((dom, document_encoding))
    }
}

/// DOM处理器
pub struct DomProcessor;

impl DomProcessor {
    pub fn new() -> Self {
        Self
    }

    pub fn process_dom(
        &self,
        dom: RcDom,
        session: &mut Session,
        base_url: &Url,
    ) -> Result<RcDom, MonolithError> {
        // 遍历文档并嵌入远程资源
        walk(session, base_url, &dom.document);
        Ok(dom)
    }
}

/// Favicon处理器
pub struct FaviconProcessor;

impl FaviconProcessor {
    pub fn new() -> Self {
        Self
    }

    pub fn process_favicon(
        &self,
        dom: RcDom,
        session: &mut Session,
        base_url: &Url,
        input_target: &Option<String>,
    ) -> Result<RcDom, MonolithError> {
        if self.should_process_favicon(session, base_url, input_target, &dom) {
            self.fetch_and_embed_favicon(dom, session, base_url)
        } else {
            Ok(dom)
        }
    }

    fn should_process_favicon(
        &self,
        session: &Session,
        base_url: &Url,
        input_target: &Option<String>,
        dom: &RcDom,
    ) -> bool {
        !session.options.no_images
            && (base_url.scheme() == "http" || base_url.scheme() == "https")
            && self.is_http_target(input_target)
            && !has_favicon(&dom.document)
    }

    fn is_http_target(&self, input_target: &Option<String>) -> bool {
        input_target.as_ref().map_or(false, |target| {
            target.starts_with("http:") || target.starts_with("https:")
        })
    }

    fn fetch_and_embed_favicon(
        &self,
        dom: RcDom,
        session: &mut Session,
        base_url: &Url,
    ) -> Result<RcDom, MonolithError> {
        let favicon_ico_url = resolve_url(base_url, "/favicon.ico");

        match session.retrieve_asset(base_url, &favicon_ico_url) {
            Ok((data, final_url, media_type, charset)) => {
                let favicon_data_url = create_data_url(&media_type, &charset, &data, &final_url);
                Ok(add_favicon(&dom.document, favicon_data_url.to_string()))
            }
            Err(_) => {
                // 检索/favicon.ico失败
                Ok(dom)
            }
        }
    }
}

/// 输出格式化器
pub struct OutputFormatter<'a> {
    options: &'a MonolithOptions,
    input_target: &'a Option<String>,
}

impl<'a> OutputFormatter<'a> {
    pub fn new(options: &'a MonolithOptions, input_target: &'a Option<String>) -> Self {
        Self {
            options,
            input_target,
        }
    }

    pub fn format_output(
        &self,
        dom: RcDom,
        document_encoding: String,
    ) -> Result<Vec<u8>, MonolithError> {
        match self.options.output_format {
            MonolithOutputFormat::HTML => self.format_html_output(dom, document_encoding),
            MonolithOutputFormat::MHTML => self.format_mhtml_output(dom, document_encoding),
        }
    }

    fn format_html_output(
        &self,
        dom: RcDom,
        document_encoding: String,
    ) -> Result<Vec<u8>, MonolithError> {
        let mut result = serialize_document(dom, document_encoding, self.options);

        self.prepend_metadata_if_needed(&mut result)?;
        self.ensure_trailing_newline(&mut result);

        Ok(result)
    }

    fn format_mhtml_output(
        &self,
        dom: RcDom,
        document_encoding: String,
    ) -> Result<Vec<u8>, MonolithError> {
        let mut result = serialize_document(dom, document_encoding, self.options);

        self.prepend_metadata_if_needed(&mut result)?;
        self.add_mime_headers(&mut result);

        Ok(result)
    }

    fn prepend_metadata_if_needed(&self, result: &mut Vec<u8>) -> Result<(), MonolithError> {
        if !self.options.no_metadata {
            if let Some(target) = self.input_target.as_ref().filter(|t| !t.is_empty()) {
                let url = Url::parse(target).map_err(|e| {
                    MonolithError::new(&format!("Failed to parse target URL: {}", e))
                })?;
                let mut metadata_comment = create_metadata_tag(&url);
                metadata_comment.push('\n');
                result.splice(0..0, metadata_comment.as_bytes().to_vec());
            }
        }
        Ok(())
    }

    fn ensure_trailing_newline(&self, result: &mut Vec<u8>) {
        if result.last() != Some(&b'\n') {
            result.extend_from_slice(b"\n");
        }
    }

    fn add_mime_headers(&self, result: &mut Vec<u8>) {
        let mime_header = "MIME-Version: 1.0\r\n\
Content-Type: multipart/related; boundary=\"----=_NextPart_000_0000\"\r\n\
\r\n\
------=_NextPart_000_0000\r\n\
Content-Type: text/html; charset=\"utf-8\"\r\n\
Content-Location: http://example.com/\r\n\
\r\n";

        result.splice(0..0, mime_header.as_bytes().to_vec());

        let mime_footer = "\r\n------=_NextPart_000_0000--\r\n";
        result.extend_from_slice(mime_footer.as_bytes());
    }
}
