mod config;
mod rpc;
mod routes;

use anyhow::Result;
use std::sync::Arc;
use tracing::info;

#[derive(Clone)]
pub struct AppState {
    pub rpc: Arc<rpc::RpcClient>,
    pub config: Arc<config::Config>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let config = Arc::new(config::Config::from_env()?);
    let rpc = Arc::new(rpc::RpcClient::new(&config.rpc_endpoint));

    info!(
        rpc = %config.rpc_endpoint,
        bind = %config.bind_addr,
        "Starting rill-explorer"
    );

    let state = AppState { rpc, config: config.clone() };
    let app = routes::router(state);

    let listener = tokio::net::TcpListener::bind(&config.bind_addr).await?;
    info!("Explorer listening on http://{}", config.bind_addr);
    axum::serve(listener, app).await?;
    Ok(())
}
