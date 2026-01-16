#[cfg(feature = "swagger")]
pub mod openapi;
pub mod v1;
pub mod battery;
pub mod ev_charger;
pub mod grid;
pub mod inverter;
pub mod weather;
pub mod error;
pub mod response;
pub mod health;
pub mod status;
pub mod devices;
pub mod schedule;
pub mod forecast;
pub mod optimize;

use axum::Router;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::{config::Config, controller::AppState};

pub fn router(state: AppState, cfg: &Config) -> Router {
    Router::new()
        .nest("/api/v1", v1::router(state, cfg))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
}

#[cfg(feature = "swagger")]
pub fn with_swagger(app: Router) -> Router {
    use crate::api::openapi::ApiDoc;
    use utoipa_swagger_ui::SwaggerUi;
    app.merge(SwaggerUi::new("/swagger-ui").url("/api-doc/openapi.json", ApiDoc::openapi()))
}

#[cfg(feature = "metrics")]
pub fn with_metrics(app: Router) -> Router {
    use axum_prometheus::PrometheusMetricLayer;
    let (layer, handle) = PrometheusMetricLayer::pair();
    app.layer(layer).route(
        "/metrics",
        axum::routing::get(move || async move { handle.render() }),
    )
}
