use std::{
    io::Write,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};

use api::{
    AppState, cli::Command, config::Config, database, sessions::SessionStore, users::DisplayName,
};
use time::OffsetDateTime;

use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

const PUBLIC_ADDRESS: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 8080);
const INTERNAL_ADDRESS: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 8081);

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .json()
        // Logs go to stderr so `issue-session`'s stdout carries only the cookie value.
        .with_writer(std::io::stderr)
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let command = Command::parse(std::env::args().skip(1))?;
    let config = Config::from_environment()?;
    let state = AppState {
        database: database::connect(&config.database_url)?,
        sessions: SessionStore::connect(&config.valkey_url)?,
        config: Arc::new(config),
    };
    database::run_migrations(&state.config.database_url).await?;
    match command {
        Command::Serve => serve(state).await,
        Command::IssueSession(display_name) => issue_session(&state, &display_name).await,
    }
}

async fn serve(state: AppState) -> anyhow::Result<()> {
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

async fn issue_session(state: &AppState, display_name: &DisplayName) -> anyhow::Result<()> {
    let now = OffsetDateTime::now_utc();
    let user = api::users::create(&state.database, display_name, now).await?;
    let token = state.sessions.create(user.id, now).await?;
    // The cookie value is this command's only output; `print_stdout` guards the server against stray prints.
    writeln!(std::io::stdout().lock(), "{}", token.to_cookie_value())?;
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
