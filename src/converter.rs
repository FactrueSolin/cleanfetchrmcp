use crate::parser::HtmlNode;

#[derive(Debug, Clone)]
enum ListType {
    Ordered(usize),
    Unordered,
}

#[derive(Debug, Clone)]
struct ConvertContext {
    indent_level: usize,
    list_stack: Vec<ListType>,
    in_code_block: bool,
    in_table: bool,
}

impl ConvertContext {
    fn new() -> Self {
        Self {
            indent_level: 0,
            list_stack: Vec::new(),
            in_code_block: false,
            in_table: false,
        }
    }
}

pub fn convert_to_markdown(nodes: &[HtmlNode]) -> String {
    let mut ctx = ConvertContext::new();
    let mut output = String::new();
    for node in nodes {
        output.push_str(&convert_node(node, &mut ctx));
    }
    output.trim().to_string()
}

fn convert_node(node: &HtmlNode, ctx: &mut ConvertContext) -> String {
    match node {
        HtmlNode::Text(text) => process_text(text, ctx),
        HtmlNode::Element {
            tag,
            attrs: _,
            children,
        } => match tag.as_str() {
            "h1" => format!("# {}\n\n", convert_children(children, ctx).trim()),
            "h2" => format!("## {}\n\n", convert_children(children, ctx).trim()),
            "h3" => format!("### {}\n\n", convert_children(children, ctx).trim()),
            "h4" => format!("#### {}\n\n", convert_children(children, ctx).trim()),
            "h5" => format!("##### {}\n\n", convert_children(children, ctx).trim()),
            "h6" => format!("###### {}\n\n", convert_children(children, ctx).trim()),
            "p" => {
                let content = convert_children(children, ctx).trim().to_string();
                if content.is_empty() {
                    String::new()
                } else {
                    format!("{}\n\n", content)
                }
            }
            "br" => "  \n".to_string(),
            "hr" => "---\n\n".to_string(),
            "strong" | "b" => format!("**{}**", convert_children(children, ctx)),
            "em" | "i" => format!("*{}*", convert_children(children, ctx)),
            "del" | "s" | "strike" => format!("~~{}~~", convert_children(children, ctx)),
            "code" => {
                if ctx.in_code_block {
                    convert_children(children, ctx)
                } else {
                    format!("`{}`", convert_children(children, ctx))
                }
            }
            "pre" => {
                ctx.in_code_block = true;
                let content = convert_children(children, ctx);
                ctx.in_code_block = false;
                format!("```\n{}\n```\n\n", content.trim_end())
            }
            "a" => convert_children(children, ctx),
            "img" => String::new(),
            "ul" => convert_list(children, ctx, false),
            "ol" => convert_list(children, ctx, true),
            "li" => convert_list_item(children, ctx),
            "blockquote" => {
                let content = convert_children(children, ctx);
                let lines: Vec<String> = content.lines().map(|line| format!("> {}", line)).collect();
                format!("{}\n\n", lines.join("\n"))
            }
            "table" => convert_table(children, ctx),
            "tr" | "td" | "th" => convert_children(children, ctx),
            "div" | "section" | "article" | "span" => convert_children(children, ctx),
            "script" | "style" | "head" | "noscript" => String::new(),
            _ => convert_children(children, ctx),
        },
    }
}

fn convert_children(children: &[HtmlNode], ctx: &mut ConvertContext) -> String {
    let mut output = String::new();
    for child in children {
        output.push_str(&convert_node(child, ctx));
    }
    output
}

fn convert_list(children: &[HtmlNode], ctx: &mut ConvertContext, ordered: bool) -> String {
    let list_type = if ordered {
        ListType::Ordered(1)
    } else {
        ListType::Unordered
    };

    ctx.list_stack.push(list_type);
    ctx.indent_level += 1;
    let content = convert_children(children, ctx);
    ctx.indent_level -= 1;
    ctx.list_stack.pop();
    format!("{}\n", content)
}

fn convert_list_item(children: &[HtmlNode], ctx: &mut ConvertContext) -> String {
    let indent = "  ".repeat(ctx.indent_level.saturating_sub(1));
    let marker = match ctx.list_stack.last_mut() {
        Some(ListType::Ordered(num)) => {
            let current = *num;
            *num += 1;
            format!("{}. ", current)
        }
        Some(ListType::Unordered) | None => "- ".to_string(),
    };
    let content = convert_children(children, ctx).trim().to_string();
    format!("{}{}{}\n", indent, marker, content)
}

fn convert_table(children: &[HtmlNode], ctx: &mut ConvertContext) -> String {
    ctx.in_table = true;

    let mut rows: Vec<Vec<String>> = Vec::new();
    for child in children {
        if let HtmlNode::Element { tag, children, .. } = child
            && tag == "tr"
        {
            let mut cells = Vec::new();
            for cell in children {
                if let HtmlNode::Element { tag, children, .. } = cell
                    && (tag == "td" || tag == "th")
                {
                    cells.push(convert_children(children, ctx).trim().to_string());
                }
            }
            if !cells.is_empty() {
                rows.push(cells);
            }
        }
    }

    ctx.in_table = false;

    if rows.is_empty() {
        return String::new();
    }

    let max_cols = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    for row in &mut rows {
        while row.len() < max_cols {
            row.push(String::new());
        }
    }

    let mut output = String::new();
    for (idx, row) in rows.iter().enumerate() {
        output.push('|');
        for cell in row {
            output.push(' ');
            output.push_str(cell);
            output.push(' ');
            output.push('|');
        }
        output.push('\n');
        if idx == 0 {
            output.push('|');
            for _ in 0..row.len() {
                output.push_str(" --- |");
            }
            output.push('\n');
        }
    }
    output.push('\n');
    output
}

fn process_text(text: &str, ctx: &ConvertContext) -> String {
    if ctx.in_code_block {
        text.to_string()
    } else if ctx.in_table {
        text.replace('|', "\\|")
    } else {
        text.replace('\n', " ").replace("  ", " ")
    }
}
