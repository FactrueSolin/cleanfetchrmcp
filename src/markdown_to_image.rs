use pulldown_cmark::{html, Options, Parser};

use crate::html_to_image::html_to_image;

const MARKDOWN_TEMPLATE: &str = include_str!("../html/markdown.html");

pub async fn markdown_to_image(selenium_url: &str, markdown: &str) -> Result<String, String> {
    let html = markdown_to_html(markdown);
    html_to_image(selenium_url, &html).await
}

fn markdown_to_html(markdown: &str) -> String {
    let options = Options::all();
    let parser = Parser::new_ext(markdown, options);
    
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    
    MARKDOWN_TEMPLATE.replace("{{markdown_html}}", &html_output)
}
