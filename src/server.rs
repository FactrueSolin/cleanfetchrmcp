use std::env;

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
    proxy_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum FetchKind {
    Markdown,
    Text,
    Urls,
    Html,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CleanFetchParams {
    #[schemars(description = "要抓取的 URL 列表，至少一个")]
    pub urls: Vec<String>,
    #[schemars(description = "返回类型：markdown | text | urls | html")]
    pub kind: FetchKind,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct CleanFetchItem {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub markdown: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub urls_markdown: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub html: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/*
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
*/

#[tool_router]
impl FetchServer {
    pub fn new(selenium_url: String) -> Self {
        let proxy_url = env::var("PROXY_URL")
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());

        Self {
            tool_router: Self::tool_router(),
            selenium_url,
            proxy_url,
        }
    }

    #[tool(description = "抓取多个URL并按 kind 返回统一 cleanfetch 结果")]
    async fn cleanfetch(
        &self,
        Parameters(CleanFetchParams { urls, kind }): Parameters<CleanFetchParams>,
    ) -> Result<CallToolResult, McpError> {
        if urls.is_empty() {
            return Ok(text_result_json("[]".to_string()));
        }

        let htmls =
            fetcher::fetch_html_batch(&self.selenium_url, &urls, self.proxy_url.as_deref()).await;

        let mut datas: Vec<Option<String>> = vec![None; urls.len()];
        let mut errors: Vec<Option<String>> = vec![None; urls.len()];

        let mut succ_texts: Vec<String> = Vec::new();
        let mut succ_index: Vec<usize> = Vec::new();

        for (idx, item) in htmls.iter().enumerate() {
            match item {
                Ok(html) => {
                    let data = match kind {
                        FetchKind::Markdown => html_to_markdown(html),
                        FetchKind::Text => html_to_text(html),
                        FetchKind::Urls => html_to_urls_markdown(html, &urls[idx]),
                        FetchKind::Html => html.clone(),
                    };

                    if matches!(kind, FetchKind::Markdown | FetchKind::Text) {
                        succ_texts.push(data.clone());
                        succ_index.push(idx);
                    }

                    datas[idx] = Some(data);
                }
                Err(e) => {
                    errors[idx] = Some(e.clone());
                }
            }
        }

        if matches!(kind, FetchKind::Markdown | FetchKind::Text) {
            let limits = limit::limit_items(&succ_texts);
            for (pos, lim) in limits.iter().enumerate() {
                if !lim.include {
                    let idx = succ_index[pos];
                    datas[idx] = None;
                    errors[idx] = Some(
                        lim.error
                            .clone()
                            .unwrap_or_else(|| limit::ERROR_MESSAGE.to_string()),
                    );
                }
            }
        }

        let payload: Vec<CleanFetchItem> = urls
            .iter()
            .enumerate()
            .map(|(idx, url)| {
                let data = datas[idx].clone();
                let markdown = match kind {
                    FetchKind::Markdown => data.clone(),
                    _ => None,
                };
                let text = match kind {
                    FetchKind::Text => data.clone(),
                    _ => None,
                };
                let urls_markdown = match kind {
                    FetchKind::Urls => data.clone(),
                    _ => None,
                };
                let html = match kind {
                    FetchKind::Html => data.clone(),
                    _ => None,
                };

                CleanFetchItem {
                    url: url.clone(),
                    data,
                    markdown,
                    text,
                    urls_markdown,
                    html,
                    error: errors[idx].clone(),
                }
            })
            .collect();

        Ok(text_result_json(to_json(payload)))
    }

    /*
    #[tool(description = "抓取多个URL并返回Markdown（按128000总词数限制截断）")]
    async fn fetch_markdown(
        &self,
        Parameters(FetchUrlsParams { urls }): Parameters<FetchUrlsParams>,
    ) -> Result<CallToolResult, McpError> {
        // deprecated: replaced by cleanfetch(kind=markdown)
    }

    #[tool(description = "抓取多个URL并返回纯文本（去URL，按128000总词数限制截断）")]
    async fn fetch_txt(
        &self,
        Parameters(FetchUrlsParams { urls }): Parameters<FetchUrlsParams>,
    ) -> Result<CallToolResult, McpError> {
        // deprecated: replaced by cleanfetch(kind=text)
    }

    #[tool(description = "抓取多个URL并提取页面链接，返回Markdown列表")]
    async fn fetch_urls(
        &self,
        Parameters(FetchUrlsParams { urls }): Parameters<FetchUrlsParams>,
    ) -> Result<CallToolResult, McpError> {
        // deprecated: replaced by cleanfetch(kind=urls)
    }

    #[tool(description = "抓取多个URL并返回原始HTML")]
    async fn fetch_html(
        &self,
        Parameters(FetchUrlsParams { urls }): Parameters<FetchUrlsParams>,
    ) -> Result<CallToolResult, McpError> {
        // deprecated: replaced by cleanfetch(kind=html)
    }
    */
}

#[tool_handler]
impl ServerHandler for FetchServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Fetch MCP server with unified tool: cleanfetch (kind: markdown | text | urls | html)"
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
