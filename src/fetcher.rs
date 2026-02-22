use std::time::Duration;

use fantoccini::ClientBuilder;
use futures::future::join_all;
use reqwest::{Client, Proxy};
use serde_json::{Map, json};
use url::Url;

pub fn is_html_complete(html: &str) -> bool {
    const MIN_HTML_LEN: usize = 500;
    const SPA_PLACEHOLDER_MAX_BODY_LEN: usize = 200;

    if html.len() < MIN_HTML_LEN {
        return false;
    }

    let lower = html.to_ascii_lowercase();

    if !lower.contains("<html") || !lower.contains("</html>") {
        return false;
    }

    if !lower.contains("<body") || !lower.contains("</body>") {
        return false;
    }

    let body_start = match lower.find("<body") {
        Some(idx) => idx,
        None => return false,
    };

    let body_open_end_rel = match lower[body_start..].find('>') {
        Some(idx) => idx,
        None => return false,
    };
    let body_content_start = body_start + body_open_end_rel + 1;

    let body_end = match lower[body_content_start..].find("</body>") {
        Some(idx) => body_content_start + idx,
        None => return false,
    };

    let body_content = lower[body_content_start..body_end].trim();

    let is_root_placeholder = body_content.contains("<div id=\"root\"></div>")
        || body_content.contains("<div id='root'></div>")
        || body_content.contains("<div id=\"app\"></div>")
        || body_content.contains("<div id='app'></div>");

    if is_root_placeholder && body_content.len() <= SPA_PLACEHOLDER_MAX_BODY_LEN {
        return false;
    }

    true
}

pub async fn simple_fetch_html(url: &str, proxy_url: Option<&str>) -> Result<String, String> {
    const SIMPLE_FETCH_TIMEOUT: Duration = Duration::from_secs(10);

    let mut builder = Client::builder().timeout(SIMPLE_FETCH_TIMEOUT);

    if let Some(proxy) = proxy_url {
        let proxy = proxy.trim();
        if !proxy.is_empty() {
            let proxy_with_scheme = if proxy.contains("://") {
                proxy.to_string()
            } else {
                format!("http://{proxy}")
            };

            let reqwest_proxy = Proxy::all(&proxy_with_scheme)
                .map_err(|e| format!("build proxy failed: {e}"))?;
            builder = builder.proxy(reqwest_proxy);
        }
    }

    let client = builder
        .build()
        .map_err(|e| format!("build http client failed: {e}"))?;

    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("simple fetch failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("simple fetch non-success status: {}", resp.status()));
    }

    resp.text()
        .await
        .map_err(|e| format!("read simple fetch body failed: {e}"))
}

pub async fn fetch_html(
    selenium_url: &str,
    url: &str,
    proxy_url: Option<&str>,
) -> Result<String, String> {
    if let Ok(html) = simple_fetch_html(url, proxy_url).await {
        if is_html_complete(&html) {
            return Ok(html);
        }
    }

    const FETCH_TIMEOUT: Duration = Duration::from_secs(30);
    const RENDER_WAIT_MS: u64 = 2000;
    const DOM_STABLE_CHECK_MS: u64 = 500;

    let mut caps = Map::new();
    if let Some(proxy) = proxy_url {
        caps.insert(
            "proxy".to_string(),
            json!({
                "proxyType": "manual",
                "httpProxy": proxy,
                "sslProxy": proxy
            }),
        );
    }

    let mut builder = ClientBuilder::native();
    if !caps.is_empty() {
        builder.capabilities(caps);
    }

    let client = builder
        .connect(selenium_url)
        .await
        .map_err(|e| format!("connect selenium failed: {e}"))?;

    let result = match tokio::time::timeout(FETCH_TIMEOUT, async {
        client
            .goto(url)
            .await
            .map_err(|e| format!("navigate failed: {e}"))?;

        wait_for_ready_state_complete(&client).await?;

        tokio::time::sleep(Duration::from_millis(RENDER_WAIT_MS)).await;

        let initial_dom_size = get_dom_node_count(&client).await?;
        tokio::time::sleep(Duration::from_millis(DOM_STABLE_CHECK_MS)).await;
        let final_dom_size = get_dom_node_count(&client).await?;

        if final_dom_size > initial_dom_size + 50 {
            tokio::time::sleep(Duration::from_millis(1000)).await;
        }

        client
            .source()
            .await
            .map_err(|e| format!("read html failed: {e}"))
    })
    .await
    {
        Ok(inner) => inner,
        Err(_) => Err("fetch html timeout after 30s".to_string()),
    };

    let _ = client.close().await;
    result
}

async fn wait_for_ready_state_complete(client: &fantoccini::Client) -> Result<(), String> {
    const READY_STATE_TIMEOUT: Duration = Duration::from_secs(10);
    const CHECK_INTERVAL: Duration = Duration::from_millis(100);
    const MAX_ATTEMPTS: u32 = 100;

    for _ in 0..MAX_ATTEMPTS {
        let result = tokio::time::timeout(READY_STATE_TIMEOUT, async {
            for _ in 0..MAX_ATTEMPTS {
                match client.execute("return document.readyState", Vec::new()).await {
                    Ok(json_result) => {
                        if let Some(state) = json_result.as_str() {
                            if state == "complete" {
                                return Ok(());
                            }
                        }
                    }
                    Err(_) => {}
                }
                tokio::time::sleep(CHECK_INTERVAL).await;
            }
            Err("readyState did not become complete in time")
        })
        .await;

        match result {
            Ok(Ok(())) => return Ok(()),
            Ok(Err(e)) => return Err(format!("wait for readyState failed: {e}")),
            Err(_) => {}
        }
    }

    Err("readyState check timeout".to_string())
}

async fn get_dom_node_count(client: &fantoccini::Client) -> Result<usize, String> {
    let result = client
        .execute("return document.documentElement.outerHTML.length", Vec::new())
        .await
        .map_err(|e| format!("get dom size failed: {e}"))?;

    let count = result
        .as_u64()
        .ok_or_else(|| "get dom size returned non-integer".to_string())?;

    Ok(count as usize)
}

pub async fn fetch_html_batch(
    selenium_url: &str,
    urls: &[String],
    proxy_url: Option<&str>,
) -> Vec<Result<String, String>> {
    let futures: Vec<_> = urls
        .iter()
        .map(|url| async move {
            match Url::parse(url) {
                Ok(parsed) if matches!(parsed.scheme(), "http" | "https") => {
                    fetch_html(selenium_url, url, proxy_url).await
                }
                _ => Err("invalid url".to_string()),
            }
        })
        .collect();

    join_all(futures).await
}
