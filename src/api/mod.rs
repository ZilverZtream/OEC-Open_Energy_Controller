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

use axum::{Router, body::Body, extract::Request};
use tower_http::{cors::CorsLayer, trace::TraceLayer, timeout::TimeoutLayer};
use std::time::Duration;
use tower::ServiceBuilder;

use crate::{config::Config, controller::AppState};

pub fn router(state: AppState, cfg: &Config) -> Router {
    let mut router = Router::new()
        .nest("/api/v1", v1::router(state, cfg));

    if cfg.server.enable_cors {
        use tower_http::cors::{AllowOrigin, Any};
        let cors = CorsLayer::new()
            .allow_origin(AllowOrigin::exact("http://localhost:3000".parse().unwrap()))
            .allow_methods([
                axum::http::Method::GET,
                axum::http::Method::POST,
                axum::http::Method::PUT,
                axum::http::Method::DELETE,
            ])
            .allow_headers([
                axum::http::header::AUTHORIZATION,
                axum::http::header::CONTENT_TYPE,
            ]);
        router = router.layer(cors);
    }

    router
        .layer(
            ServiceBuilder::new()
                .layer(axum::extract::DefaultBodyLimit::max(1024 * 1024))
                .layer(TimeoutLayer::new(Duration::from_secs(cfg.server.request_timeout_secs)))
        )
        .layer(TraceLayer::new_for_http())
}

#[cfg(feature = "swagger")]
pub fn with_swagger(app: Router) -> Router {
    use crate::api::openapi::ApiDoc;
    use utoipa::OpenApi;
    use utoipa_swagger_ui::SwaggerUi;
    app.merge(SwaggerUi::new("/swagger-ui").url("/api-doc/openapi.json", ApiDoc::openapi()))
}

#[cfg(feature = "metrics")]
pub fn with_metrics(app: Router, cfg: &Config) -> Router {
    use axum_prometheus::PrometheusMetricLayer;
    let (layer, handle) = PrometheusMetricLayer::pair();

    let metrics_router = Router::new()
        .route("/metrics", axum::routing::get(move || async move { handle.render() }))
        .layer(crate::auth::auth_layer(cfg.auth.token.clone()));

    app.layer(layer).merge(metrics_router)
}
