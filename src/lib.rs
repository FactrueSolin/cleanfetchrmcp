pub mod converter;
pub mod entities;
pub mod fetcher;
pub mod limit;
pub mod parser;
pub mod server;
pub mod text;
pub mod urls;
pub mod word_count;

pub fn html_to_markdown(html: &str) -> String {
    let dom = parser::parse_html(html);
    converter::convert_to_markdown(&dom)
}

pub fn html_to_text(html: &str) -> String {
    let dom = parser::parse_html(html);
    text::convert_to_text(&dom)
}

pub fn html_to_urls_markdown(html: &str, base_url: &str) -> String {
    let dom = parser::parse_html(html);
    urls::extract_urls(&dom, base_url)
}
