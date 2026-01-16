mod api;
mod auth;
mod config;
mod controller;
mod discovery;
mod domain;
mod forecast;
mod hardware;
mod modbus;
mod optimizer;
mod repo;
mod telemetry;

use anyhow::Result;
use axum::Router;
use config::Config;
use telemetry::init_tracing;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let cfg = Config::load()?;
    let app_state = controller::AppState::new(cfg.clone()).await?;

    let mut app: Router = api::router(app_state.clone(), &cfg);

    #[cfg(feature = "swagger")]
    {
        app = api::with_swagger(app);
    }

    #[cfg(feature = "metrics")]
    {
        app = api::with_metrics(app);
    }

    let addr = cfg.server.socket_addr()?;
    info!(%addr, "starting Open Energy Controller");

    controller::spawn_controller_tasks(app_state.clone(), cfg.clone());

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(telemetry::shutdown_signal())
        .await?;

    warn!("shutdown complete");
    Ok(())
}
