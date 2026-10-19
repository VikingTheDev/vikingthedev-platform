use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};

use api::{AppState, config::Config, sessions::SessionStore};

use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

const PUBLIC_ADDRESS: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 8080);
const INTERNAL_ADDRESS: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 8081);

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let config = Config::from_environment()?;
    let state = AppState {
        sessions: SessionStore::connect(&config.valkey_url)?,
        config: Arc::new(config),
    };

    let public = TcpListener::bind(PUBLIC_ADDRESS).await?;
    let internal = TcpListener::bind(INTERNAL_ADDRESS).await?;
    tracing::info!(%PUBLIC_ADDRESS, %INTERNAL_ADDRESS, "listening");

    tokio::try_join!(
        axum::serve(public, api::public_router(state.clone()))
            .with_graceful_shutdown(shutdown_signal()),
        axum::serve(internal, api::internal_router(state))
            .with_graceful_shutdown(shutdown_signal()),
    )?;
    Ok(())
}

async fn shutdown_signal() {
    let interrupt = tokio::signal::ctrl_c();
    let mut terminate =
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(stream) => stream,
            Err(error) => {
                tracing::error!(%error, "cannot listen for SIGTERM");
                let _ = interrupt.await;
                return;
            }
        };
    tokio::select! {
        _ = interrupt => {},
        _ = terminate.recv() => {},
    }
}
