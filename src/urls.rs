use std::collections::HashSet;

use crate::parser::{Attr, HtmlNode};

const LINK_REL_ALLOWED: [&str; 5] = ["canonical", "alternate", "prev", "next", "amphtml"];

pub fn extract_urls(nodes: &[HtmlNode], base_url: &str) -> String {
    let base = parse_base_url(base_url);
    let mut seen: HashSet<String> = HashSet::new();
    let mut output: Vec<String> = Vec::new();

    for node in nodes {
        collect_urls(node, &base, &mut seen, &mut output);
    }
    output.join("\n")
}

fn collect_urls(
    node: &HtmlNode,
    base: &Option<BaseUrl>,
    seen: &mut HashSet<String>,
    output: &mut Vec<String>,
) {
    match node {
        HtmlNode::Text(_) => {}
        HtmlNode::Element {
            tag,
            attrs,
            children,
        } => {
            match tag.as_str() {
                "a" => {
                    if let Some(href_raw) = get_attr_value(attrs, "href") {
                        add_url(
                            build_anchor_desc(children, href_raw),
                            href_raw,
                            base,
                            seen,
                            output,
                        );
                    }
                }
                "area" => {
                    if let Some(href_raw) = get_attr_value(attrs, "href") {
                        add_url("area href".to_string(), href_raw, base, seen, output);
                    }
                }
                "form" => {
                    if let Some(action_raw) = get_attr_value(attrs, "action") {
                        add_url("form action".to_string(), action_raw, base, seen, output);
                    }
                }
                "iframe" => {
                    if let Some(src_raw) = get_attr_value(attrs, "src") {
                        add_url("iframe src".to_string(), src_raw, base, seen, output);
                    }
                }
                "link" => {
                    if let Some(rel_raw) = get_attr_value(attrs, "rel")
                        && rel_matches_allowed(rel_raw)
                        && let Some(href_raw) = get_attr_value(attrs, "href")
                    {
                        add_url(
                            format!("link rel={}", rel_raw.trim()),
                            href_raw,
                            base,
                            seen,
                            output,
                        );
                    }
                }
                _ => {}
            }

            for child in children {
                collect_urls(child, base, seen, output);
            }
        }
    }
}

fn add_url(
    desc: String,
    href_raw: &str,
    base: &Option<BaseUrl>,
    seen: &mut HashSet<String>,
    output: &mut Vec<String>,
) {
    if let Some(absolute) = resolve_url(href_raw, base)
        && seen.insert(absolute.clone())
    {
        output.push(format!("![{}]({})", desc, absolute));
    }
}

fn build_anchor_desc(children: &[HtmlNode], href_raw: &str) -> String {
    let text = children
        .iter()
        .map(extract_text)
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if text.is_empty() {
        href_raw.trim().to_string()
    } else {
        text
    }
}

fn extract_text(node: &HtmlNode) -> String {
    match node {
        HtmlNode::Text(s) => s.clone(),
        HtmlNode::Element { children, .. } => children
            .iter()
            .map(extract_text)
            .collect::<Vec<_>>()
            .join(" "),
    }
}

fn get_attr_value<'a>(attrs: &'a [Attr], name: &str) -> Option<&'a str> {
    attrs
        .iter()
        .find(|attr| attr.name == name)
        .map(|attr| attr.value.as_str())
}

fn rel_matches_allowed(rel: &str) -> bool {
    let lower = rel.to_lowercase();
    lower
        .split_whitespace()
        .any(|token| LINK_REL_ALLOWED.iter().any(|allowed| allowed == &token))
}

fn resolve_url(href_raw: &str, base: &Option<BaseUrl>) -> Option<String> {
    let href = href_raw.trim();
    if href.is_empty() || href.starts_with('#') {
        return None;
    }

    let lower = href.to_lowercase();
    if lower.starts_with("javascript:")
        || lower.starts_with("data:")
        || lower.starts_with("mailto:")
        || lower.starts_with("tel:")
    {
        return None;
    }

    if lower.starts_with("http://") || lower.starts_with("https://") {
        return Some(href.to_string());
    }

    if href.starts_with("//") {
        return base
            .as_ref()
            .map(|parsed| format!("{}:{}", parsed.scheme, href))
            .or_else(|| Some(href.to_string()));
    }

    let base = match base {
        Some(parsed) => parsed,
        None => return Some(href.to_string()),
    };

    if href.starts_with('/') {
        return Some(format!(
            "{}://{}{}",
            base.scheme,
            base.host_with_port(),
            href
        ));
    }

    let (path_part, suffix) = split_suffix(href);
    let combined = format!("{}{}", base.base_dir, path_part);
    let normalized = normalize_path(&combined);
    Some(format!(
        "{}://{}{}{}",
        base.scheme,
        base.host_with_port(),
        normalized,
        suffix
    ))
}

fn split_suffix(input: &str) -> (&str, &str) {
    let mut split_index = input.len();
    for (idx, ch) in input.char_indices() {
        if ch == '?' || ch == '#' {
            split_index = idx;
            break;
        }
    }
    (&input[..split_index], &input[split_index..])
}

fn normalize_path(path: &str) -> String {
    let mut stack: Vec<&str> = Vec::new();
    let has_leading_slash = path.starts_with('/');
    let has_trailing_slash = path.ends_with('/') && path.len() > 1;

    for segment in path.split('/') {
        if segment.is_empty() || segment == "." {
            continue;
        }
        if segment == ".." {
            if !stack.is_empty() {
                stack.pop();
            }
            continue;
        }
        stack.push(segment);
    }

    let mut normalized = String::new();
    if has_leading_slash {
        normalized.push('/');
    }
    if !stack.is_empty() {
        normalized.push_str(&stack.join("/"));
    }
    if normalized.is_empty() {
        normalized.push('/');
    }
    if has_trailing_slash && !normalized.ends_with('/') {
        normalized.push('/');
    }
    normalized
}

#[derive(Clone)]
struct BaseUrl {
    scheme: String,
    host: String,
    port: Option<String>,
    base_dir: String,
}

impl BaseUrl {
    fn host_with_port(&self) -> String {
        match &self.port {
            Some(port) => format!("{}:{}", self.host, port),
            None => self.host.clone(),
        }
    }
}

fn parse_base_url(input: &str) -> Option<BaseUrl> {
    let trimmed = input.trim();
    let scheme_pos = trimmed.find("://")?;
    let scheme = trimmed[..scheme_pos].trim();
    if scheme.is_empty() {
        return None;
    }

    let rest = &trimmed[scheme_pos + 3..];
    let mut authority_end = rest.len();
    for (idx, ch) in rest.char_indices() {
        if ch == '/' || ch == '?' || ch == '#' {
            authority_end = idx;
            break;
        }
    }

    let authority_full = &rest[..authority_end];
    if authority_full.is_empty() {
        return None;
    }

    let authority = if let Some(at_pos) = authority_full.rfind('@') {
        &authority_full[at_pos + 1..]
    } else {
        authority_full
    };

    let (host, port) = split_host_port(authority);
    if host.is_empty() {
        return None;
    }

    let remainder = &rest[authority_end..];
    let mut path_end = remainder.len();
    for (idx, ch) in remainder.char_indices() {
        if ch == '?' || ch == '#' {
            path_end = idx;
            break;
        }
    }
    let path = if path_end == 0 { "/" } else { &remainder[..path_end] };
    let path = if path.is_empty() { "/" } else { path };

    let base_dir = if path.ends_with('/') {
        path.to_string()
    } else {
        match path.rfind('/') {
            Some(idx) => path[..=idx].to_string(),
            None => "/".to_string(),
        }
    };

    Some(BaseUrl {
        scheme: scheme.to_string(),
        host: host.to_string(),
        port,
        base_dir,
    })
}

fn split_host_port(authority: &str) -> (&str, Option<String>) {
    if let Some(colon_pos) = authority.rfind(':') {
        let host = &authority[..colon_pos];
        let port_part = &authority[colon_pos + 1..];
        if !host.is_empty() && !port_part.is_empty() && port_part.chars().all(|c| c.is_ascii_digit())
        {
            return (host, Some(port_part.to_string()));
        }
    }
    (authority, None)
}
