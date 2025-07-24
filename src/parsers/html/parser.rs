use super::utils::{is_favicon, WHITESPACES};

/// 链接类型枚举
#[derive(PartialEq, Eq)]
pub enum LinkType {
    Alternate,
    AppleTouchIcon,
    DnsPrefetch,
    Favicon,
    Preload,
    Stylesheet,
}

/// SrcSet 项目结构
pub struct SrcSetItem<'a> {
    pub path: &'a str,
    pub descriptor: &'a str, // Width or pixel density descriptor
}

/// 解析链接类型
pub fn parse_link_type(link_attr_rel_value: &str) -> Vec<LinkType> {
    let mut types: Vec<LinkType> = vec![];

    for link_attr_rel_type in link_attr_rel_value.split_whitespace() {
        if link_attr_rel_type.eq_ignore_ascii_case("alternate") {
            types.push(LinkType::Alternate);
        } else if link_attr_rel_type.eq_ignore_ascii_case("dns-prefetch") {
            types.push(LinkType::DnsPrefetch);
        } else if link_attr_rel_type.eq_ignore_ascii_case("preload") {
            types.push(LinkType::Preload);
        } else if link_attr_rel_type.eq_ignore_ascii_case("stylesheet") {
            types.push(LinkType::Stylesheet);
        } else if is_favicon(link_attr_rel_type) {
            types.push(LinkType::Favicon);
        } else if link_attr_rel_type.eq_ignore_ascii_case("apple-touch-icon") {
            types.push(LinkType::AppleTouchIcon);
        }
    }

    types
}

/// 解析 srcset 属性
pub fn parse_srcset(srcset: &str) -> Vec<SrcSetItem> {
    let mut srcset_items: Vec<SrcSetItem> = vec![];

    // Parse srcset
    let mut partials: Vec<&str> = srcset.split(WHITESPACES).collect();
    let mut path: Option<&str> = None;
    let mut descriptor: Option<&str> = None;
    let mut i = 0;
    while i < partials.len() {
        let partial = partials[i];

        i += 1;

        // Skip empty strings
        if partial.is_empty() {
            continue;
        }

        if partial.ends_with(',') {
            if path.is_none() {
                path = Some(partial.strip_suffix(',').unwrap());
                descriptor = Some("")
            } else {
                descriptor = Some(partial.strip_suffix(',').unwrap());
            }
        } else if path.is_none() {
            path = Some(partial);
        } else {
            let mut chunks: Vec<&str> = partial.split(',').collect();

            if !chunks.is_empty() && chunks.first().unwrap().ends_with(['x', 'w']) {
                descriptor = Some(chunks.first().unwrap());

                chunks.remove(0);
            }

            if !chunks.is_empty() {
                if let Some(desc) = descriptor {
                    partials.insert(0, &partial[desc.len()..]);
                } else {
                    partials.insert(0, partial);
                }
            }
        }

        if let (Some(p), Some(d)) = (path, descriptor) {
            srcset_items.push(SrcSetItem {
                path: p,
                descriptor: d,
            });

            path = None;
            descriptor = None;
        }
    }

    // Final attempt to process what was found
    if let Some(p) = path {
        srcset_items.push(SrcSetItem {
            path: p,
            descriptor: descriptor.unwrap_or_default(),
        });
    }

    srcset_items
}
