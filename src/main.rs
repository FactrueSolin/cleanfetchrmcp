use std::{env, net::SocketAddr, sync::Arc};

use axum::{
    Router,
    extract::State,
    http::{HeaderMap, Request, StatusCode},
    middleware::{self, Next},
    response::Response,
};
use cleanfetchrmcp::server::FetchServer;
use dotenvy::dotenv;
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Clone)]
struct TokenAuthState {
    token: String,
}

fn extract_bearer_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get("Authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|auth_header| {
            auth_header
                .strip_prefix("Bearer ")
                .map(|stripped| stripped.to_string())
        })
}

async fn auth_middleware(
    State(state): State<Arc<TokenAuthState>>,
    headers: HeaderMap,
    request: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    match extract_bearer_token(&headers) {
        Some(token) if token == state.token => Ok(next.run(request).await),
        _ => Err(StatusCode::UNAUTHORIZED),
    }
}

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
    let auth_token = env::var("MCP_AUTH_TOKEN")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());

    let mcp_service: StreamableHttpService<FetchServer, LocalSessionManager> =
        StreamableHttpService::new(
            move || Ok(FetchServer::new(selenium_url.clone())),
            LocalSessionManager::default().into(),
            StreamableHttpServerConfig::default(),
        );

    let mcp_router = if let Some(token) = auth_token {
        tracing::info!("MCP token auth enabled for /mcp endpoint");
        Router::new()
            .nest_service("/mcp", mcp_service)
            .layer(middleware::from_fn_with_state(
                Arc::new(TokenAuthState { token }),
                auth_middleware,
            ))
    } else {
        tracing::info!("MCP token auth disabled (MCP_AUTH_TOKEN is empty or not set)");
        Router::new().nest_service("/mcp", mcp_service)
    };

    let app = Router::new().merge(mcp_router);
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
