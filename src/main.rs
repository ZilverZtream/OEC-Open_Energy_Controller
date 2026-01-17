use anyhow::Result;
use axum::Router;
use open_energy_controller::{api, config, controller, telemetry};
use config::Config;
use telemetry::init_tracing;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let cfg = Config::load()?;

    if cfg.auth.token.is_empty() || cfg.auth.token.starts_with("__SET_VIA_ENV") {
        anyhow::bail!(
            "SECURITY ERROR: OEC_AUTH_TOKEN environment variable must be set to a secure random token (min 32 chars). \
            Generate one with: openssl rand -base64 32"
        );
    }

    if cfg.auth.token == "devtoken" {
        warn!("Using 'devtoken' auth token - this is only safe for local development!");
    }

    let app_state = controller::AppState::new(cfg.clone()).await?;

    let mut app: Router = api::router(app_state.clone(), &cfg);

    #[cfg(feature = "swagger")]
    {
        app = api::with_swagger(app);
    }

    #[cfg(feature = "metrics")]
    {
        app = api::with_metrics(app, &cfg);
    }

    let addr = cfg.server.socket_addr()?;

    if cfg.server.host == "0.0.0.0" {
        warn!(
            "WARNING: Server binding to 0.0.0.0 - service will be accessible from network! \
            For production, bind to 127.0.0.1 unless behind a firewall/reverse proxy."
        );
    }

    info!(%addr, "starting Open Energy Controller");

    controller::spawn_controller_tasks(app_state.clone(), cfg.clone());

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(telemetry::shutdown_signal())
        .await?;

    warn!("shutdown complete");
    Ok(())
}
