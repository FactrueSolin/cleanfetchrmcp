use std::time::Duration;

use fantoccini::ClientBuilder;

pub async fn fetch_html(selenium_url: &str, url: &str) -> Result<String, String> {
    let client = ClientBuilder::native()
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

pub async fn fetch_html_batch(selenium_url: &str, urls: &[String]) -> Vec<Result<String, String>> {
    let mut out = Vec::with_capacity(urls.len());
    for url in urls {
        out.push(fetch_html(selenium_url, url).await);
    }
    out
}
