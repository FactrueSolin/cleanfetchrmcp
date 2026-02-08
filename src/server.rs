use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Content, ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router,
};
use serde::{Deserialize, Serialize};

use crate::{fetcher, html_to_markdown, html_to_text, html_to_urls_markdown, limit};

#[derive(Debug, Clone)]
pub struct FetchServer {
    tool_router: ToolRouter<Self>,
    selenium_url: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct FetchUrlsParams {
    #[schemars(description = "要抓取的 URL 列表，至少一个")]
    pub urls: Vec<String>,
}

#[derive(Debug, Serialize)]
struct MarkdownResult {
    url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    markdown: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct TextResult {
    url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct UrlsResult {
    url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    urls_markdown: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct HtmlResult {
    url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    html: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[tool_router]
impl FetchServer {
    pub fn new(selenium_url: String) -> Self {
        Self {
            tool_router: Self::tool_router(),
            selenium_url,
        }
    }

    #[tool(description = "抓取多个URL并返回Markdown（按128000总词数限制截断）")]
    async fn fetch_markdown(
        &self,
        Parameters(FetchUrlsParams { urls }): Parameters<FetchUrlsParams>,
    ) -> Result<CallToolResult, McpError> {
        if urls.is_empty() {
            return Ok(text_result_json("[]".to_string()));
        }

        let htmls = fetcher::fetch_html_batch(&self.selenium_url, &urls).await;

        let mut markdowns: Vec<Option<String>> = Vec::with_capacity(urls.len());
        let mut succ_texts = Vec::new();
        let mut succ_index = Vec::new();
        let mut errors: Vec<Option<String>> = vec![None; urls.len()];

        for (idx, item) in htmls.iter().enumerate() {
            match item {
                Ok(html) => {
                    let md = html_to_markdown(html);
                    succ_texts.push(md.clone());
                    succ_index.push(idx);
                    markdowns.push(Some(md));
                }
                Err(e) => {
                    markdowns.push(None);
                    errors[idx] = Some(e.clone());
                }
            }
        }

        let limits = limit::limit_items(&succ_texts);
        for (pos, lim) in limits.iter().enumerate() {
            if !lim.include {
                let idx = succ_index[pos];
                markdowns[idx] = None;
                errors[idx] = Some(
                    lim.error
                        .clone()
                        .unwrap_or_else(|| limit::ERROR_MESSAGE.to_string()),
                );
            }
        }

        let payload: Vec<MarkdownResult> = urls
            .iter()
            .enumerate()
            .map(|(idx, url)| MarkdownResult {
                url: url.clone(),
                markdown: markdowns[idx].clone(),
                error: errors[idx].clone(),
            })
            .collect();

        Ok(text_result_json(to_json(payload)))
    }

    #[tool(description = "抓取多个URL并返回纯文本（去URL，按128000总词数限制截断）")]
    async fn fetch_txt(
        &self,
        Parameters(FetchUrlsParams { urls }): Parameters<FetchUrlsParams>,
    ) -> Result<CallToolResult, McpError> {
        if urls.is_empty() {
            return Ok(text_result_json("[]".to_string()));
        }

        let htmls = fetcher::fetch_html_batch(&self.selenium_url, &urls).await;

        let mut texts: Vec<Option<String>> = Vec::with_capacity(urls.len());
        let mut succ_texts = Vec::new();
        let mut succ_index = Vec::new();
        let mut errors: Vec<Option<String>> = vec![None; urls.len()];

        for (idx, item) in htmls.iter().enumerate() {
            match item {
                Ok(html) => {
                    let txt = html_to_text(html);
                    succ_texts.push(txt.clone());
                    succ_index.push(idx);
                    texts.push(Some(txt));
                }
                Err(e) => {
                    texts.push(None);
                    errors[idx] = Some(e.clone());
                }
            }
        }

        let limits = limit::limit_items(&succ_texts);
        for (pos, lim) in limits.iter().enumerate() {
            if !lim.include {
                let idx = succ_index[pos];
                texts[idx] = None;
                errors[idx] = Some(
                    lim.error
                        .clone()
                        .unwrap_or_else(|| limit::ERROR_MESSAGE.to_string()),
                );
            }
        }

        let payload: Vec<TextResult> = urls
            .iter()
            .enumerate()
            .map(|(idx, url)| TextResult {
                url: url.clone(),
                text: texts[idx].clone(),
                error: errors[idx].clone(),
            })
            .collect();

        Ok(text_result_json(to_json(payload)))
    }

    #[tool(description = "抓取多个URL并提取页面链接，返回Markdown列表")]
    async fn fetch_urls(
        &self,
        Parameters(FetchUrlsParams { urls }): Parameters<FetchUrlsParams>,
    ) -> Result<CallToolResult, McpError> {
        let htmls = fetcher::fetch_html_batch(&self.selenium_url, &urls).await;
        let payload: Vec<UrlsResult> = urls
            .iter()
            .enumerate()
            .map(|(idx, url)| match &htmls[idx] {
                Ok(html) => UrlsResult {
                    url: url.clone(),
                    urls_markdown: Some(html_to_urls_markdown(html, url)),
                    error: None,
                },
                Err(e) => UrlsResult {
                    url: url.clone(),
                    urls_markdown: None,
                    error: Some(e.clone()),
                },
            })
            .collect();

        Ok(text_result_json(to_json(payload)))
    }

    #[tool(description = "抓取多个URL并返回原始HTML")]
    async fn fetch_html(
        &self,
        Parameters(FetchUrlsParams { urls }): Parameters<FetchUrlsParams>,
    ) -> Result<CallToolResult, McpError> {
        let htmls = fetcher::fetch_html_batch(&self.selenium_url, &urls).await;
        let payload: Vec<HtmlResult> = urls
            .iter()
            .enumerate()
            .map(|(idx, url)| match &htmls[idx] {
                Ok(html) => HtmlResult {
                    url: url.clone(),
                    html: Some(html.clone()),
                    error: None,
                },
                Err(e) => HtmlResult {
                    url: url.clone(),
                    html: None,
                    error: Some(e.clone()),
                },
            })
            .collect();

        Ok(text_result_json(to_json(payload)))
    }
}

#[tool_handler]
impl ServerHandler for FetchServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Fetch MCP server with tools: fetch_markdown, fetch_txt, fetch_urls, fetch_html"
                    .to_string(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

fn text_result_json(text: String) -> CallToolResult {
    CallToolResult::success(vec![Content::text(text)])
}

fn to_json<T: Serialize>(value: T) -> String {
    serde_json::to_string(&value).unwrap_or_else(|_| "[]".to_string())
}
