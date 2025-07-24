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

/// 序列化文档
pub fn serialize_document(
    dom: RcDom,
    document_encoding: String,
    options: &MonolithOptions,
) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();

    if options.isolate
        || options.no_css
        || options.no_fonts
        || options.no_frames
        || options.no_js
        || options.no_images
    {
        // Take care of CSP
        if let Some(html) = get_child_node_by_name(&dom.document, "html") {
            if let Some(head) = get_child_node_by_name(&html, "head") {
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
                // The CSP meta-tag has to be prepended, never appended,
                //  since there already may be one defined in the original document,
                //   and browsers don't allow re-defining them (for obvious reasons)
                head.children.borrow_mut().reverse();
                head.children.borrow_mut().push(meta.clone());
                head.children.borrow_mut().reverse();
            }
        }
    }

    let serializable: SerializableHandle = dom.document.into();
    serialize(&mut buf, &serializable, SerializeOpts::default())
        .expect("Unable to serialize DOM into buffer");

    // Unwrap NOSCRIPT elements
    if options.unwrap_noscript {
        let s: &str = &String::from_utf8_lossy(&buf);
        let noscript_re = Regex::new(r"<(?P<c>/?noscript[^>]*)>").unwrap();
        buf = noscript_re.replace_all(s, "<!--$c-->").as_bytes().to_vec();
    }

    if !document_encoding.is_empty() {
        if let Some(encoding) = Encoding::for_label(document_encoding.as_bytes()) {
            let s: &str = &String::from_utf8_lossy(&buf);
            let (data, _, _) = encoding.encode(s);
            buf = data.to_vec();
        }
    }

    buf
}
