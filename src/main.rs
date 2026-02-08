use std::{env, net::SocketAddr};

use axum::Router;
use cleanfetchrmcp::server::FetchServer;
use dotenvy::dotenv;
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();

    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,cleanfetchrmcp=debug")),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let selenium_url =
        env::var("SELENIUM_URL").unwrap_or_else(|_| "http://127.0.0.1:4444".to_string());
    let port: u16 = env::var("PORT")
        .ok()
        .and_then(|v| v.parse::<u16>().ok())
        .unwrap_or(3000);

    let mcp_service: StreamableHttpService<FetchServer, LocalSessionManager> =
        StreamableHttpService::new(
            move || Ok(FetchServer::new(selenium_url.clone())),
            LocalSessionManager::default().into(),
            StreamableHttpServerConfig::default(),
        );

    let app = Router::new().nest_service("/mcp", mcp_service);
    let addr: SocketAddr = format!("0.0.0.0:{port}").parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;

    tracing::info!("MCP httpstream server listening on http://{}/mcp", addr);

    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;

    Ok(())
}
