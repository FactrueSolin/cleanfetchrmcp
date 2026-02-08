use crate::parser::HtmlNode;

const BLOCK_TAGS: [&str; 20] = [
    "p", "div", "section", "article", "li", "h1", "h2", "h3", "h4", "h5", "h6",
    "blockquote", "tr", "table", "ul", "ol", "header", "footer", "nav", "main",
];
const SKIP_TAGS: [&str; 4] = ["script", "style", "head", "noscript"];

pub fn convert_to_text(nodes: &[HtmlNode]) -> String {
    let mut output = String::new();
    for node in nodes {
        append_node(node, &mut output);
    }
    let stripped = strip_urls(&output);
    normalize_whitespace(&stripped)
}

fn append_node(node: &HtmlNode, output: &mut String) {
    match node {
        HtmlNode::Text(text) => push_text(output, text),
        HtmlNode::Element { tag, children, .. } => {
            if SKIP_TAGS.iter().any(|skip| skip == tag) {
                return;
            }
            if tag == "br" || tag == "hr" {
                output.push('\n');
                return;
            }

            let is_block = BLOCK_TAGS.iter().any(|block| block == tag);
            if is_block && !output.is_empty() {
                output.push('\n');
            }
            for child in children {
                append_node(child, output);
            }
            if is_block {
                output.push('\n');
            }
        }
    }
}

fn push_text(output: &mut String, text: &str) {
    if text.trim().is_empty() {
        return;
    }
    let needs_space = output.chars().last().map(|ch| !ch.is_whitespace()).unwrap_or(false)
        && !text
            .chars()
            .next()
            .map(|ch| ch.is_whitespace())
            .unwrap_or(false);
    if needs_space {
        output.push(' ');
    }
    output.push_str(text);
}

fn normalize_whitespace(text: &str) -> String {
    let mut output = String::new();
    let mut last_space = false;
    let mut last_newline = false;

    for ch in text.chars() {
        if ch == '\n' {
            if !output.ends_with('\n') && !output.is_empty() {
                output.push('\n');
            }
            last_space = false;
            last_newline = true;
            continue;
        }
        if ch.is_whitespace() {
            if !last_space && !last_newline {
                output.push(' ');
                last_space = true;
            }
            continue;
        }

        output.push(ch);
        last_space = false;
        last_newline = false;
    }

    output.trim().to_string()
}

pub fn strip_urls(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut output = String::new();
    let mut i = 0;
    while i < chars.len() {
        if is_url_start(&chars, i) {
            i = skip_url(&chars, i);
            continue;
        }
        output.push(chars[i]);
        i += 1;
    }
    output
}

fn is_url_start(chars: &[char], index: usize) -> bool {
    if index > 0 {
        let prev = chars[index - 1];
        if prev.is_ascii_alphanumeric() || prev == '.' {
            return false;
        }
    }
    starts_with(chars, index, "http://")
        || starts_with(chars, index, "https://")
        || starts_with(chars, index, "www.")
}

fn starts_with(chars: &[char], index: usize, pattern: &str) -> bool {
    let pat: Vec<char> = pattern.chars().collect();
    if index + pat.len() > chars.len() {
        return false;
    }
    chars[index..index + pat.len()] == pat[..]
}

fn skip_url(chars: &[char], mut index: usize) -> usize {
    while index < chars.len() {
        let ch = chars[index];
        if ch.is_whitespace() || is_url_terminator(ch) {
            break;
        }
        index += 1;
    }
    index
}

fn is_url_terminator(ch: char) -> bool {
    matches!(ch, ')' | ']' | '}' | '>' | '"' | '\'' | '<')
}
