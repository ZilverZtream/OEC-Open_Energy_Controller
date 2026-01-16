use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::{
    auth::AuthBearer,
    config::Config,
    controller::AppState,
    domain::{BatteryState, Forecast24h, PriceArea, Schedule, ScheduleEntry},
};

pub fn router(state: AppState, cfg: &Config) -> Router {
    Router::new()
        .route("/status", get(get_status))
        .route("/forecast", get(get_forecast))
        .route("/schedule", get(get_schedule).post(set_schedule))
        .route("/optimize", post(trigger_optimization))
        .route("/devices", get(list_devices))
        .route("/simulation/step", post(simulation_step))
        .route("/healthz", get(healthz))
        .with_state(state)
        .layer(crate::auth::auth_layer(cfg.auth.token.clone()))
}

pub async fn healthz() -> impl IntoResponse {
    StatusCode::OK
}

#[cfg_attr(feature = "swagger", derive(utoipa::ToSchema))]
#[derive(Debug, Serialize)]
pub struct SystemStatus {
    pub battery: BatteryState,
    pub schedule_next_4h: Vec<ScheduleEntry>,
    pub forecast_updated_at: Option<String>,
}

pub async fn get_status(
    State(st): State<AppState>,
    AuthBearer(_): AuthBearer,
) -> impl IntoResponse {
    match st.controller.get_current_state().await {
        Ok(battery_state) => {
            let schedule_next_4h = st
                .controller
                .get_schedule()
                .await
                .map(|s| s.next_hours(4))
                .unwrap_or_default();
            (
                StatusCode::OK,
                Json(SystemStatus {
                    battery: battery_state,
                    schedule_next_4h,
                    forecast_updated_at: None,
                }),
            )
                .into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(err(e))).into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct ForecastQuery {
    pub horizon_hours: Option<u32>,
    pub area: Option<PriceArea>,
}

pub async fn get_forecast(
    State(st): State<AppState>,
    AuthBearer(_): AuthBearer,
    Query(q): Query<ForecastQuery>,
) -> impl IntoResponse {
    let _ = q.horizon_hours;
    let area = q.area.unwrap_or(PriceArea::SE3);
    match st.controller.get_forecast(area).await {
        Ok(f) => (StatusCode::OK, Json(f)).into_response(),
        Err(e) => (StatusCode::BAD_GATEWAY, Json(err(e))).into_response(),
    }
}

pub async fn get_schedule(
    State(st): State<AppState>,
    AuthBearer(_): AuthBearer,
) -> impl IntoResponse {
    let s: Option<Schedule> = st.controller.get_schedule().await;
    (StatusCode::OK, Json(s)).into_response()
}

pub async fn set_schedule(
    State(st): State<AppState>,
    AuthBearer(_): AuthBearer,
    Json(schedule): Json<Schedule>,
) -> impl IntoResponse {
    st.controller.set_schedule(schedule).await;
    StatusCode::NO_CONTENT
}

#[cfg_attr(feature = "swagger", derive(utoipa::ToSchema))]
#[derive(Debug, Deserialize)]
pub struct OptimizeRequest {
    pub horizon_hours: u32,
    pub area: PriceArea,
}

pub async fn trigger_optimization(
    State(st): State<AppState>,
    AuthBearer(_): AuthBearer,
    Json(req): Json<OptimizeRequest>,
) -> impl IntoResponse {
    let _ = req;
    if let Err(e) = st.controller.reoptimize_schedule().await {
        return (StatusCode::BAD_GATEWAY, Json(err(e))).into_response();
    }
    let s = st.controller.get_schedule().await;
    (StatusCode::OK, Json(s)).into_response()
}

pub async fn list_devices(
    State(_st): State<AppState>,
    AuthBearer(_): AuthBearer,
) -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({ "devices": [] }))).into_response()
}

#[derive(Debug, Deserialize)]
pub struct SimulationStepRequest {
    pub steps: u32,
}

pub async fn simulation_step(
    State(st): State<AppState>,
    AuthBearer(_): AuthBearer,
    Json(_req): Json<SimulationStepRequest>,
) -> impl IntoResponse {
    if let Err(e) = st.controller.reoptimize_schedule().await {
        return (StatusCode::BAD_REQUEST, Json(err(e))).into_response();
    }
    get_status(State(st), AuthBearer(uuid::Uuid::nil()))
        .await
        .into_response()
}

fn err(e: anyhow::Error) -> serde_json::Value {
    serde_json::json!({ "error": e.to_string() })
}
