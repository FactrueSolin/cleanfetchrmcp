use std::time::Duration;

use fantoccini::ClientBuilder;
use futures::future::join_all;
use serde_json::{Map, json};
use url::Url;

pub async fn fetch_html(
    selenium_url: &str,
    url: &str,
    proxy_url: Option<&str>,
) -> Result<String, String> {
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

    let result = async {
        client
            .goto(url)
            .await
            .map_err(|e| format!("navigate failed: {e}"))?;
        tokio::time::sleep(Duration::from_millis(800)).await;
        client
            .source()
            .await
            .map_err(|e| format!("read html failed: {e}"))
    }
    .await;

    let _ = client.close().await;
    result
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
