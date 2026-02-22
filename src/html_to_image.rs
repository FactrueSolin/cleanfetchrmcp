use std::time::Duration;

use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use fantoccini::ClientBuilder;
use serde_json::Map;

const IMAGE_WIDTH: i32 = 1080;
const CONVERT_TIMEOUT: Duration = Duration::from_secs(30);

pub async fn html_to_image(selenium_url: &str, html: &str) -> Result<String, String> {
    let mut caps = Map::new();
    caps.insert(
        "goog:chromeOptions".to_string(),
        serde_json::json!({
            "args": [
                "--headless=new",
                "--disable-gpu",
                "--no-sandbox",
                "--disable-dev-shm-usage",
            ]
        }),
    );

    let client = ClientBuilder::native()
        .capabilities(caps)
        .connect(selenium_url)
        .await
        .map_err(|e| format!("connect selenium failed: {e}"))?;

    let result = tokio::time::timeout(CONVERT_TIMEOUT, async {
        let encoded_html = urlencoding::encode(html);
        let data_uri = format!("data:text/html;charset=utf-8,{encoded_html}");

        client
            .goto(&data_uri)
            .await
            .map_err(|e| format!("navigate failed: {e}"))?;

        client
            .execute(
                "return new Promise(resolve => {
                    window.scrollTo(0, 0);
                    const height = Math.max(
                        document.body.scrollHeight,
                        document.body.offsetHeight,
                        document.documentElement.clientHeight,
                        document.documentElement.scrollHeight,
                        document.documentElement.offsetHeight
                    );
                    resolve(height);
                })",
                vec![],
            )
            .await
            .map_err(|e| format!("get page height failed: {e}"))?;

        client
            .execute(
                &format!(
                    "return new Promise(resolve => {{
                        window.scrollTo(0, 0);
                        const height = Math.max(
                            document.body.scrollHeight,
                            document.body.offsetHeight,
                            document.documentElement.clientHeight,
                            document.documentElement.scrollHeight,
                            document.documentElement.offsetHeight
                        );
                        window.resizeTo({IMAGE_WIDTH}, height);
                        resolve();
                    }})"
                ),
                vec![],
            )
            .await
            .map_err(|e| format!("resize window failed: {e}"))?;

        let screenshot = client
            .screenshot()
            .await
            .map_err(|e| format!("screenshot failed: {e}"))?;

        Ok::<String, String>(BASE64.encode(&screenshot))
    })
    .await
    .map_err(|_| "convert timeout after 30s".to_string())?;

    let _ = client.close().await;
    result
}
