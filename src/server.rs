use std::env;

use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Content, ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router,
};
use serde::{Deserialize, Serialize};

use crate::{fetcher, html_to_markdown, html_to_text, html_to_urls_markdown, limit, markdown_to_image};

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
pub struct HtmlToImageParams {
    #[schemars(description = "原始 HTML 内容")]
    pub html: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct MarkdownToImageParams {
    #[schemars(description = "Markdown 内容")]
    pub markdown: String,
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

    #[tool(
        description = "可以同时抓取多个 url，并且返回抓取结果。kind 参数控制返回内容类型：markdown（转换后的 Markdown 文本），text（纯文本），urls（提取的页面链接列表），html（原始 HTML）。"
    )]
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

    #[tool(description = "将原始 HTML 转换为图片，返回 base64 编码的 PNG 图片")]
    async fn html_to_image(
        &self,
        Parameters(HtmlToImageParams { html }): Parameters<HtmlToImageParams>,
    ) -> Result<CallToolResult, McpError> {
        let base64_data = crate::html_to_image::html_to_image(&self.selenium_url, &html)
            .await
            .map_err(|e| McpError::internal_error(e, None))?;

        Ok(CallToolResult::success(vec![Content::image(
            base64_data,
            "image/png",
        )]))
    }

    #[tool(description = "将 Markdown 渲染为图片，返回 base64 编码的 PNG 图片")]
    async fn markdown_to_image(
        &self,
        Parameters(MarkdownToImageParams { markdown }): Parameters<MarkdownToImageParams>,
    ) -> Result<CallToolResult, McpError> {
        let base64_data = markdown_to_image::markdown_to_image(&self.selenium_url, &markdown)
            .await
            .map_err(|e| McpError::internal_error(e, None))?;

        Ok(CallToolResult::success(vec![Content::image(
            base64_data,
            "image/png",
        )]))
    }
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
