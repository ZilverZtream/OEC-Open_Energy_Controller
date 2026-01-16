#[cfg(feature="swagger")]
use utoipa::OpenApi;

#[cfg(feature="swagger")]
use crate::api::v1::{OptimizeRequest, SystemStatus};

#[cfg(feature="swagger")]
use crate::domain::{Forecast24h, PriceArea, Schedule};

#[cfg(feature="swagger")]
#[derive(OpenApi)]
#[openapi(
    paths(
        crate::api::v1::get_status,
        crate::api::v1::get_forecast,
        crate::api::v1::get_schedule,
        crate::api::v1::set_schedule,
        crate::api::v1::trigger_optimization,
        crate::api::v1::list_devices,
        crate::api::v1::simulation_step,
        crate::api::v1::healthz,
    ),
    components(
        schemas(SystemStatus, OptimizeRequest, Forecast24h, PriceArea, Schedule)
    ),
    tags((name="oec", description="Open Energy Controller API v1"))
)]
pub struct ApiDoc;
