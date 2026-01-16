pub mod pid;

use anyhow::Result;
use chrono::{DateTime, FixedOffset, Local};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};
use uuid::Uuid;

use crate::config::Config;

pub use pid::{PidController, PowerPidController};
use crate::domain::{Battery, BatteryCapabilities, BatteryState, Forecast24h, PriceArea, Schedule};
use crate::forecast::{
    ElprisetJustNuPriceForecaster, ForecastEngine, SimpleConsumptionForecaster,
    SimpleProductionForecaster,
};
use crate::optimizer::{BatteryOptimizer, Constraints, DynamicProgrammingOptimizer, SystemState};
use crate::repo::Repositories;

#[derive(Clone)]
pub struct AppState {
    pub cfg: Config,
    pub controller: Arc<BatteryController>,
    pub repos: Arc<Repositories>,
}

impl AppState {
    pub async fn new(cfg: Config) -> Result<Self> {
        let repos = Arc::new(Repositories::new(&cfg).await?);

        let caps = BatteryCapabilities {
            capacity_kwh: cfg.battery.capacity_kwh,
            max_charge_kw: cfg.battery.max_charge_kw,
            max_discharge_kw: cfg.battery.max_discharge_kw,
            efficiency: cfg.battery.efficiency,
            degradation_per_cycle: cfg.battery.degradation_per_cycle,
        };
        let initial = BatteryState {
            soc_percent: cfg.battery.initial_soc_percent,
            power_w: 0.0,
            voltage_v: 48.0,
            temperature_c: 25.0,
            health_percent: 100.0,
        };

        #[cfg(feature = "sim")]
        let battery: Arc<dyn Battery> =
            Arc::new(crate::domain::SimulatedBattery::new(initial, caps.clone()));
        #[cfg(not(feature = "sim"))]
        let battery: Arc<dyn Battery> = Arc::new(crate::domain::MockBattery::new(
            Default::default(),
            caps.clone(),
        ));

        let price = Box::new(ElprisetJustNuPriceForecaster::new(
            cfg.prices.base_url.clone(),
            std::time::Duration::from_secs(cfg.prices.cache_ttl_seconds),
        )?);

        let forecast_engine = Arc::new(ForecastEngine::new(
            price,
            Box::new(SimpleConsumptionForecaster),
            Box::new(SimpleProductionForecaster::default()),
        ));

        let optimizer = Arc::new(BatteryOptimizer {
            strategy: Box::new(DynamicProgrammingOptimizer),
        });
        let schedule = Arc::new(RwLock::new(None::<Schedule>));

        let controller = Arc::new(BatteryController {
            battery,
            optimizer,
            forecast_engine,
            schedule,
            constraints: Arc::new(RwLock::new(Constraints::default())),
            household_id: Uuid::new_v4(),
        });

        Ok(Self {
            cfg,
            controller,
            repos,
        })
    }
}

pub fn spawn_controller_tasks(state: AppState, cfg: Config) {
    let controller = state.controller.clone();
    tokio::spawn(async move {
        if let Err(e) = controller.run(cfg.controller.tick_seconds).await {
            warn!(error=%e, "controller loop stopped");
        }
    });

    let controller2 = state.controller.clone();
    tokio::spawn(async move {
        if let Err(e) = controller2
            .reoptimize_loop(cfg.controller.reoptimize_every_minutes)
            .await
        {
            warn!(error=%e, "reoptimize loop stopped");
        }
    });
}

pub struct BatteryController {
    pub battery: Arc<dyn Battery>,
    pub optimizer: Arc<BatteryOptimizer>,
    pub forecast_engine: Arc<ForecastEngine>,
    pub schedule: Arc<RwLock<Option<Schedule>>>,
    pub constraints: Arc<RwLock<Constraints>>,
    pub household_id: Uuid,
}

impl BatteryController {
    pub async fn run(&self, tick_seconds: u64) -> Result<()> {
        let mut interval =
            tokio::time::interval(std::time::Duration::from_secs(tick_seconds.max(1)));
        loop {
            interval.tick().await;
            let state = self.battery.read_state().await?;
            let now: DateTime<FixedOffset> = Local::now().fixed_offset();
            let target_power = self
                .schedule
                .read()
                .await
                .as_ref()
                .and_then(|s| s.power_at(now))
                .unwrap_or(0.0);
            let control = simple_p_control(state.power_w, target_power);
            self.battery.set_power(control).await?;
            info!(
                soc_percent = state.soc_percent,
                power_w = state.power_w,
                target_power_w = target_power,
                control_w = control,
                "control tick"
            );
            // TODO(db): persist battery state
        }
    }

    pub async fn reoptimize_loop(&self, every_minutes: u64) -> Result<()> {
        let mut interval =
            tokio::time::interval(std::time::Duration::from_secs(every_minutes.max(1) * 60));
        loop {
            interval.tick().await;
            if let Err(e) = self.reoptimize_schedule().await {
                warn!(error=%e, "reoptimize failed");
            }
        }
    }

    pub async fn reoptimize_schedule(&self) -> Result<()> {
        let area: PriceArea = "SE3".parse().unwrap_or(PriceArea::SE3);
        let forecast: Forecast24h = self
            .forecast_engine
            .get_forecast_24h(area, self.household_id)
            .await?;
        let battery_state = self.battery.read_state().await?;
        let constraints = self.constraints.read().await.clone();
        let state = SystemState {
            battery: battery_state,
        };
        let schedule = self
            .optimizer
            .optimize(&state, &forecast, &constraints)
            .await?;
        *self.schedule.write().await = Some(schedule);
        Ok(())
    }

    pub async fn get_schedule(&self) -> Option<Schedule> {
        self.schedule.read().await.clone()
    }
    pub async fn set_schedule(&self, schedule: Schedule) {
        *self.schedule.write().await = Some(schedule);
    }
    pub async fn get_current_state(&self) -> Result<BatteryState> {
        self.battery.read_state().await
    }
    pub async fn get_forecast(&self, area: PriceArea) -> Result<Forecast24h> {
        self.forecast_engine
            .get_forecast_24h(area, self.household_id)
            .await
    }
    pub async fn get_constraints(&self) -> Constraints {
        self.constraints.read().await.clone()
    }
}

fn simple_p_control(actual_w: f64, target_w: f64) -> f64 {
    let k = 0.5;
    target_w + (target_w - actual_w) * k
}
