use crate::entities::decode_entities;

#[derive(Debug, Clone, PartialEq)]
pub enum HtmlNode {
    Element {
        tag: String,
        attrs: Vec<Attr>,
        children: Vec<HtmlNode>,
    },
    Text(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Attr {
    pub name: String,
    pub value: String,
}

#[derive(Debug, PartialEq)]
enum ParserState {
    Text,
    TagOpen,
    TagName,
    CloseTagName,
    AttrName,
    AttrValueStart,
    AttrValueQuoted(char),
    AttrValueUnquoted,
    SelfClosing,
    Comment,
}

pub struct HtmlParser {
    input: Vec<char>,
    pos: usize,
    state: ParserState,
    stack: Vec<Element>,
    root: Vec<HtmlNode>,
}

struct Element {
    tag: String,
    attrs: Vec<Attr>,
    children: Vec<HtmlNode>,
}

impl HtmlParser {
    pub fn new(html: &str) -> Self {
        Self {
            input: html.chars().collect(),
            pos: 0,
            state: ParserState::Text,
            stack: Vec::new(),
            root: Vec::new(),
        }
    }

    pub fn parse(mut self) -> Vec<HtmlNode> {
        while self.pos < self.input.len() {
            self.parse_step();
        }

        while let Some(elem) = self.stack.pop() {
            self.add_node(self.element_to_node(elem));
        }
        self.root
    }

    fn parse_step(&mut self) {
        match self.state {
            ParserState::Text => self.parse_text(),
            ParserState::TagOpen => self.parse_tag_open(),
            ParserState::TagName => self.parse_tag_name(),
            ParserState::CloseTagName => self.parse_close_tag_name(),
            ParserState::AttrName => self.parse_attr_name(),
            ParserState::AttrValueStart => self.parse_attr_value_start(),
            ParserState::AttrValueQuoted(quote) => self.parse_attr_value_quoted(quote),
            ParserState::AttrValueUnquoted => self.parse_attr_value_unquoted(),
            ParserState::SelfClosing => self.parse_self_closing(),
            ParserState::Comment => self.parse_comment(),
        }
    }

    fn parse_text(&mut self) {
        let start = self.pos;
        while self.pos < self.input.len() {
            if self.input[self.pos] == '<' {
                break;
            }
            self.pos += 1;
        }

        if start < self.pos {
            let text: String = self.input[start..self.pos].iter().collect();
            let decoded = decode_entities(&text);
            if !decoded.trim().is_empty() {
                self.add_node(HtmlNode::Text(decoded));
            }
        }

        if self.pos < self.input.len() && self.input[self.pos] == '<' {
            self.state = ParserState::TagOpen;
            self.pos += 1;
        }
    }

    fn parse_tag_open(&mut self) {
        if self.pos >= self.input.len() {
            return;
        }

        if self.pos + 2 < self.input.len()
            && self.input[self.pos] == '!'
            && self.input[self.pos + 1] == '-'
            && self.input[self.pos + 2] == '-'
        {
            self.state = ParserState::Comment;
            self.pos += 3;
            return;
        }

        if self.input[self.pos] == '/' {
            self.state = ParserState::CloseTagName;
            self.pos += 1;
        } else if self.input[self.pos] == '!' || self.input[self.pos] == '?' {
            self.skip_until('>');
            self.state = ParserState::Text;
        } else {
            self.state = ParserState::TagName;
        }
    }

    fn parse_tag_name(&mut self) {
        let start = self.pos;
        while self.pos < self.input.len() {
            let ch = self.input[self.pos];
            if ch.is_whitespace() || ch == '>' || ch == '/' {
                break;
            }
            self.pos += 1;
        }

        let tag: String = self.input[start..self.pos].iter().collect();
        let tag = tag.to_lowercase();

        self.stack.push(Element {
            tag: tag.clone(),
            attrs: Vec::new(),
            children: Vec::new(),
        });

        self.skip_whitespace();
        if self.pos < self.input.len() {
            match self.input[self.pos] {
                '>' => {
                    self.pos += 1;
                    self.handle_tag_close(&tag);
                }
                '/' => {
                    self.state = ParserState::SelfClosing;
                    self.pos += 1;
                }
                _ => self.state = ParserState::AttrName,
            }
        }
    }

    fn parse_close_tag_name(&mut self) {
        let start = self.pos;
        while self.pos < self.input.len() && self.input[self.pos] != '>' {
            self.pos += 1;
        }
        let tag: String = self.input[start..self.pos].iter().collect();
        let tag = tag.trim().to_lowercase();
        if self.pos < self.input.len() {
            self.pos += 1;
        }
        self.close_tag(&tag);
        self.state = ParserState::Text;
    }

    fn parse_attr_name(&mut self) {
        self.skip_whitespace();
        if self.pos >= self.input.len() {
            return;
        }

        if self.input[self.pos] == '>' {
            self.pos += 1;
            if let Some(elem) = self.stack.last() {
                let tag = elem.tag.clone();
                self.handle_tag_close(&tag);
            }
            return;
        }

        if self.input[self.pos] == '/' {
            self.state = ParserState::SelfClosing;
            self.pos += 1;
            return;
        }

        let start = self.pos;
        while self.pos < self.input.len() {
            let ch = self.input[self.pos];
            if ch == '=' || ch.is_whitespace() || ch == '>' || ch == '/' {
                break;
            }
            self.pos += 1;
        }

        let name: String = self.input[start..self.pos].iter().collect();
        let name = name.to_lowercase();

        self.skip_whitespace();
        if self.pos < self.input.len() && self.input[self.pos] == '=' {
            self.pos += 1;
            self.state = ParserState::AttrValueStart;
            if let Some(elem) = self.stack.last_mut() {
                elem.attrs.push(Attr {
                    name,
                    value: String::new(),
                });
            }
        } else {
            if let Some(elem) = self.stack.last_mut() {
                elem.attrs.push(Attr {
                    name,
                    value: String::new(),
                });
            }
            self.state = ParserState::AttrName;
        }
    }

    fn parse_attr_value_start(&mut self) {
        self.skip_whitespace();
        if self.pos >= self.input.len() {
            return;
        }
        let ch = self.input[self.pos];
        if ch == '"' || ch == '\'' {
            self.pos += 1;
            self.state = ParserState::AttrValueQuoted(ch);
        } else {
            self.state = ParserState::AttrValueUnquoted;
        }
    }

    fn parse_attr_value_quoted(&mut self, quote: char) {
        let start = self.pos;
        while self.pos < self.input.len() && self.input[self.pos] != quote {
            self.pos += 1;
        }
        let value: String = self.input[start..self.pos].iter().collect();
        let value = decode_entities(&value);

        if let Some(elem) = self.stack.last_mut()
            && let Some(attr) = elem.attrs.last_mut()
        {
            attr.value = value;
        }

        if self.pos < self.input.len() {
            self.pos += 1;
        }
        self.state = ParserState::AttrName;
    }

    fn parse_attr_value_unquoted(&mut self) {
        let start = self.pos;
        while self.pos < self.input.len() {
            let ch = self.input[self.pos];
            if ch.is_whitespace() || ch == '>' || ch == '/' {
                break;
            }
            self.pos += 1;
        }

        let value: String = self.input[start..self.pos].iter().collect();
        let value = decode_entities(&value);

        if let Some(elem) = self.stack.last_mut()
            && let Some(attr) = elem.attrs.last_mut()
        {
            attr.value = value;
        }

        self.state = ParserState::AttrName;
    }

    fn parse_self_closing(&mut self) {
        self.skip_whitespace();
        if self.pos < self.input.len() && self.input[self.pos] == '>' {
            self.pos += 1;
        }
        if let Some(elem) = self.stack.pop() {
            self.add_node(self.element_to_node(elem));
        }
        self.state = ParserState::Text;
    }

    fn parse_comment(&mut self) {
        while self.pos + 2 < self.input.len() {
            if self.input[self.pos] == '-'
                && self.input[self.pos + 1] == '-'
                && self.input[self.pos + 2] == '>'
            {
                self.pos += 3;
                break;
            }
            self.pos += 1;
        }
        self.state = ParserState::Text;
    }

    fn handle_tag_close(&mut self, tag: &str) {
        if is_void_element(tag) {
            if let Some(elem) = self.stack.pop() {
                self.add_node(self.element_to_node(elem));
            }
            self.state = ParserState::Text;
        } else if should_skip_content(tag) {
            self.skip_until_close_tag(tag);
            let _ = self.stack.pop();
            self.state = ParserState::Text;
        } else {
            self.state = ParserState::Text;
        }
    }

    fn close_tag(&mut self, tag: &str) {
        let mut pos = self.stack.len();
        while pos > 0 {
            pos -= 1;
            if self.stack[pos].tag == tag {
                let mut elements = Vec::new();
                while self.stack.len() > pos {
                    elements.push(self.stack.pop().expect("stack pop should work"));
                }
                for elem in elements.into_iter().rev() {
                    self.add_node(self.element_to_node(elem));
                }
                return;
            }
        }
    }

    fn add_node(&mut self, node: HtmlNode) {
        if let Some(parent) = self.stack.last_mut() {
            parent.children.push(node);
        } else {
            self.root.push(node);
        }
    }

    fn element_to_node(&self, elem: Element) -> HtmlNode {
        HtmlNode::Element {
            tag: elem.tag,
            attrs: elem.attrs,
            children: elem.children,
        }
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.input.len() && self.input[self.pos].is_whitespace() {
            self.pos += 1;
        }
    }

    fn skip_until(&mut self, target: char) {
        while self.pos < self.input.len() && self.input[self.pos] != target {
            self.pos += 1;
        }
        if self.pos < self.input.len() {
            self.pos += 1;
        }
    }

    fn skip_until_close_tag(&mut self, tag: &str) {
        let close_tag = format!("</{}>", tag);
        let close_chars: Vec<char> = close_tag.chars().collect();

        while self.pos < self.input.len() {
            if self.pos + close_chars.len() <= self.input.len() {
                let slice: String = self.input[self.pos..self.pos + close_chars.len()]
                    .iter()
                    .collect();
                if slice.to_lowercase() == close_tag.to_lowercase() {
                    self.pos += close_chars.len();
                    return;
                }
            }
            self.pos += 1;
        }
    }
}

fn is_void_element(tag: &str) -> bool {
    matches!(
        tag,
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}

fn should_skip_content(tag: &str) -> bool {
    matches!(tag, "script" | "style" | "noscript")
}

pub fn parse_html(html: &str) -> Vec<HtmlNode> {
    HtmlParser::new(html).parse()
}
