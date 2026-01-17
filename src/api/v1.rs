#![allow(dead_code)]
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
    domain::{BatteryState, PriceArea, Schedule, ScheduleEntry},
};

pub fn router(state: AppState, cfg: &Config) -> Router {
    #[allow(unused_imports)]
    use crate::api::{battery, ev_charger, grid, inverter, weather};

    Router::new()
        .route("/status", get(get_status))
        .route("/forecast", get(get_forecast))
        .route("/schedule", get(get_schedule).post(set_schedule))
        .route("/optimize", post(trigger_optimization))
        .route("/devices", get(list_devices))
        .route("/simulation/step", post(simulation_step))
        .route("/healthz", get(healthz))
        // Battery routes
        .route("/battery/state", get(battery::get_battery_state))
        .route(
            "/battery/capabilities",
            get(battery::get_battery_capabilities),
        )
        .route("/battery/health", get(battery::get_battery_health))
        .route("/battery/power", post(battery::set_battery_power))
        .route("/battery/history", get(battery::get_battery_history))
        .route("/battery/statistics", get(battery::get_battery_statistics))
        // EV Charger routes
        .route("/ev-charger/state", get(ev_charger::get_charger_state))
        .route(
            "/ev-charger/current",
            post(ev_charger::set_charging_current),
        )
        .route("/ev-charger/start", post(ev_charger::start_charging))
        .route("/ev-charger/stop", post(ev_charger::stop_charging))
        .route(
            "/ev-charger/sessions",
            get(ev_charger::get_charging_sessions),
        )
        // Inverter routes
        .route("/inverter/state", get(inverter::get_inverter_state))
        .route("/inverter/mode", post(inverter::set_inverter_mode))
        .route("/inverter/export-limit", post(inverter::set_export_limit))
        .route(
            "/inverter/production",
            get(inverter::get_production_history),
        )
        .route("/inverter/efficiency", get(inverter::get_efficiency_stats))
        // Grid routes
        .route("/grid/status", get(grid::get_grid_status))
        .route("/grid/limits", get(grid::get_grid_limits))
        .route("/grid/statistics", get(grid::get_grid_statistics))
        // Weather routes
        .route("/weather/forecast", get(weather::get_weather_forecast))
        .with_state(state)
        // SECURITY: Authentication layer using Bearer token validation with constant-time comparison
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
    // CRITICAL FIX: Real-time power flow data for dashboard visualization
    pub grid_flow_w: f64,
    pub pv_production_w: f64,
    pub house_consumption_w: f64,
}

pub async fn get_status(
    State(st): State<AppState>,
    AuthBearer: AuthBearer,
) -> impl IntoResponse {
    match st.controller.get_current_state().await {
        Ok(battery_state) => {
            let schedule_next_4h = st
                .controller
                .get_schedule()
                .await
                .map(|s| s.next_hours(4))
                .unwrap_or_default();

            // CRITICAL FIX: Include real-time power flow data
            // Use sensor fallback values from config (TODO: read from actual sensors)
            let pv_production_kw = st.cfg.hardware.sensor_fallback.default_pv_production_kw;
            let house_load_kw = st.cfg.hardware.sensor_fallback.default_house_load_kw;
            let battery_power_kw = battery_state.power_w / 1000.0;

            // Calculate grid flow: positive = importing, negative = exporting
            // Grid = House Load - PV Production - Battery Discharge
            let grid_flow_kw = house_load_kw - pv_production_kw - battery_power_kw;

            (
                StatusCode::OK,
                Json(SystemStatus {
                    battery: battery_state,
                    schedule_next_4h,
                    forecast_updated_at: None,
                    grid_flow_w: grid_flow_kw * 1000.0,
                    pv_production_w: pv_production_kw * 1000.0,
                    house_consumption_w: house_load_kw * 1000.0,
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
    AuthBearer: AuthBearer,
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
    AuthBearer: AuthBearer,
) -> impl IntoResponse {
    let s: Option<Schedule> = st.controller.get_schedule().await;
    (StatusCode::OK, Json(s)).into_response()
}

pub async fn set_schedule(
    State(st): State<AppState>,
    AuthBearer: AuthBearer,
    Json(schedule): Json<Schedule>,
) -> impl IntoResponse {
    match st.controller.set_schedule(schedule).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(err(e))).into_response(),
    }
}

#[cfg_attr(feature = "swagger", derive(utoipa::ToSchema))]
#[derive(Debug, Deserialize)]
pub struct OptimizeRequest {
    pub horizon_hours: u32,
    pub area: PriceArea,
}

pub async fn trigger_optimization(
    State(st): State<AppState>,
    AuthBearer: AuthBearer,
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
    AuthBearer: AuthBearer,
) -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({ "devices": [] }))).into_response()
}

#[derive(Debug, Deserialize)]
pub struct SimulationStepRequest {
    pub steps: u32,
}

pub async fn simulation_step(
    State(st): State<AppState>,
    AuthBearer: AuthBearer,
    Json(_req): Json<SimulationStepRequest>,
) -> impl IntoResponse {
    if let Err(e) = st.controller.reoptimize_schedule().await {
        return (StatusCode::BAD_REQUEST, Json(err(e))).into_response();
    }
    get_status(State(st), AuthBearer)
        .await
        .into_response()
}

fn err(e: anyhow::Error) -> serde_json::Value {
    serde_json::json!({ "error": e.to_string() })
}
