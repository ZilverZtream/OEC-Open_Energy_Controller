pub mod pid;

use anyhow::{bail, Result};
use chrono::{DateTime, FixedOffset, Local, Utc};
use serde::Serialize;
use std::{collections::VecDeque, sync::Arc};
use tokio::sync::RwLock;
use tracing::{info, warn};
use uuid::Uuid;

use crate::config::Config;

use crate::domain::{
    Battery, BatteryCapabilities, BatteryState, Forecast24h, GridConnection, GridLimits,
    GridStatistics, GridStatus, HealthStatus, PriceArea, Schedule,
};
use crate::forecast::{
    ElprisetJustNuPriceForecaster, ForecastEngine, GeoLocation, SimpleConsumptionForecaster,
    SimpleProductionForecaster, SmhiClient, WeatherForecast,
};
use crate::optimizer::{BatteryOptimizer, Constraints, DynamicProgrammingOptimizer, SystemState};
use crate::repo::Repositories;
pub use pid::{PidController, PowerPidController};

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
            chemistry: crate::domain::BatteryChemistry::LiFePO4,
        };
        let initial = BatteryState {
            soc_percent: cfg.battery.initial_soc_percent,
            power_w: 0.0,
            voltage_v: 48.0,
            temperature_c: 25.0,
            health_percent: 100.0,
            status: crate::domain::BatteryStatus::Idle,
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

        let history_capacity = ((24 * 60 * 60) / cfg.controller.tick_seconds.max(1)) as usize;
        let controller = Arc::new(BatteryController {
            battery,
            optimizer,
            forecast_engine,
            schedule,
            constraints: Arc::new(RwLock::new(Constraints::default())),
            household_id: Uuid::new_v4(),
            state_history: Arc::new(RwLock::new(VecDeque::with_capacity(
                history_capacity.max(1),
            ))),
            history_capacity: history_capacity.max(1),
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
    state_history: Arc<RwLock<VecDeque<BatteryStateSample>>>,
    history_capacity: usize,
}

impl BatteryController {
    pub async fn run(&self, tick_seconds: u64) -> Result<()> {
        let mut interval =
            tokio::time::interval(std::time::Duration::from_secs(tick_seconds.max(1)));
        loop {
            interval.tick().await;
            let state = self.battery.read_state().await?;
            let now_utc = Utc::now();
            self.record_state(now_utc, state.clone()).await;
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
        schedule.validate().map_err(|err| anyhow::anyhow!(err))?;
        *self.schedule.write().await = Some(schedule);
        Ok(())
    }

    pub async fn get_schedule(&self) -> Option<Schedule> {
        self.schedule.read().await.clone()
    }
    pub async fn set_schedule(&self, schedule: Schedule) -> Result<()> {
        schedule.validate().map_err(|err| anyhow::anyhow!(err))?;
        *self.schedule.write().await = Some(schedule);
        Ok(())
    }
    pub async fn get_current_state(&self) -> Result<BatteryState> {
        self.battery.read_state().await
    }
    pub async fn get_battery_capabilities(&self) -> BatteryCapabilities {
        self.battery.capabilities()
    }
    pub async fn get_battery_health(&self) -> Result<HealthStatus> {
        self.battery.health_check().await
    }
    pub async fn set_battery_power(&self, power_w: f64) -> Result<()> {
        if !power_w.is_finite() {
            bail!("power must be finite");
        }
        let caps = self.battery.capabilities();
        let max_charge_w = caps.max_charge_kw * 1000.0;
        let max_discharge_w = caps.max_discharge_kw * 1000.0;
        if power_w >= 0.0 && power_w > max_charge_w {
            bail!("charge power {}W exceeds max {}W", power_w, max_charge_w);
        }
        if power_w < 0.0 && power_w.abs() > max_discharge_w {
            bail!(
                "discharge power {}W exceeds max {}W",
                power_w.abs(),
                max_discharge_w
            );
        }
        self.battery.set_power(power_w).await
    }
    pub async fn get_battery_history(
        &self,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        interval: Option<chrono::Duration>,
    ) -> Vec<BatteryStateSample> {
        let history = self.state_history.read().await;
        let mut results = Vec::new();
        let mut last_included: Option<DateTime<Utc>> = None;

        for sample in history.iter() {
            if sample.timestamp < start_time || sample.timestamp > end_time {
                continue;
            }
            if let (Some(interval), Some(last)) = (interval, last_included) {
                if sample.timestamp - last < interval {
                    continue;
                }
            }
            last_included = Some(sample.timestamp);
            results.push(sample.clone());
        }

        results
    }
    pub async fn get_battery_statistics(
        &self,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Option<BatteryStatistics> {
        let history = self.get_battery_history(start_time, end_time, None).await;
        if history.is_empty() {
            return None;
        }

        let mut min_soc = f64::MAX;
        let mut max_soc = f64::MIN;
        let mut sum_soc = 0.0;
        let mut sum_power = 0.0;
        for sample in &history {
            let soc = sample.state.soc_percent;
            min_soc = min_soc.min(soc);
            max_soc = max_soc.max(soc);
            sum_soc += soc;
            sum_power += sample.state.power_w;
        }

        let count = history.len() as f64;
        Some(BatteryStatistics {
            average_soc_percent: sum_soc / count,
            min_soc_percent: min_soc,
            max_soc_percent: max_soc,
            average_power_w: sum_power / count,
            sample_count: history.len() as u32,
            window_start: start_time,
            window_end: end_time,
        })
    }
    pub async fn get_forecast(&self, area: PriceArea) -> Result<Forecast24h> {
        self.forecast_engine
            .get_forecast_24h(area, self.household_id)
            .await
    }
    /// Fetch weather forecast data for the provided location.
    pub async fn get_weather_forecast(&self, location: GeoLocation) -> Result<WeatherForecast> {
        let client = SmhiClient::new();
        client.fetch_forecast(&location).await
    }
    /// Return the latest grid connection status.
    pub async fn get_grid_status(&self) -> Result<GridConnection> {
        let limits = self.get_grid_limits().await?;
        let voltage_v = (limits.voltage_min_v + limits.voltage_max_v) / 2.0;

        Ok(GridConnection {
            status: GridStatus::Normal,
            import_power_w: 0.0,
            export_power_w: 0.0,
            frequency_hz: 50.0,
            voltage_v,
            current_a: 0.0,
        })
    }
    /// Return grid limits derived from controller constraints.
    pub async fn get_grid_limits(&self) -> Result<GridLimits> {
        let constraints = self.constraints.read().await.clone();
        let max_import_kw = constraints.max_power_grid_kw;
        let max_export_kw = constraints.max_power_grid_kw;
        let fuse_rating_amps = if max_import_kw > 0.0 {
            max_import_kw * 1000.0 / (230.0 * 3.0)
        } else {
            0.0
        };

        Ok(GridLimits {
            fuse_rating_amps,
            max_import_kw,
            max_export_kw,
            voltage_min_v: 207.0,
            voltage_max_v: 253.0,
            frequency_min_hz: 49.5,
            frequency_max_hz: 50.5,
        })
    }
    /// Return aggregated grid import/export statistics.
    pub async fn get_grid_statistics(&self) -> Result<GridStatistics> {
        let now = Utc::now();
        Ok(GridStatistics {
            total_import_kwh: 0.0,
            total_export_kwh: 0.0,
            average_import_kw: 0.0,
            average_export_kw: 0.0,
            sample_count: 0,
            window_start: now - chrono::Duration::hours(24),
            window_end: now,
        })
    }
    pub async fn get_constraints(&self) -> Constraints {
        self.constraints.read().await.clone()
    }
    async fn record_state(&self, timestamp: DateTime<Utc>, state: BatteryState) {
        let mut history = self.state_history.write().await;
        if history.len() >= self.history_capacity {
            history.pop_front();
        }
        history.push_back(BatteryStateSample { timestamp, state });
    }
}

fn simple_p_control(actual_w: f64, target_w: f64) -> f64 {
    let k = 0.5;
    target_w + (target_w - actual_w) * k
}

#[derive(Debug, Clone, Serialize)]
pub struct BatteryStateSample {
    pub timestamp: DateTime<Utc>,
    pub state: BatteryState,
}

#[derive(Debug, Clone, Serialize)]
pub struct BatteryStatistics {
    pub average_soc_percent: f64,
    pub min_soc_percent: f64,
    pub max_soc_percent: f64,
    pub average_power_w: f64,
    pub sample_count: u32,
    pub window_start: DateTime<Utc>,
    pub window_end: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{BatteryCapabilities, BatteryState, BatteryStatus};
    use crate::forecast::{ConsumptionForecaster, PriceForecaster, ProductionForecaster};
    use async_trait::async_trait;
    use chrono::Duration;
    use std::collections::VecDeque;

    struct DummyPriceForecaster;
    struct DummyConsumptionForecaster;
    struct DummyProductionForecaster;

    #[async_trait]
    impl PriceForecaster for DummyPriceForecaster {
        async fn predict_next_24h(
            &self,
            _area: PriceArea,
        ) -> Result<Vec<crate::domain::PricePoint>> {
            Ok(Vec::new())
        }
    }

    #[async_trait]
    impl ConsumptionForecaster for DummyConsumptionForecaster {
        async fn predict_next_24h(
            &self,
            _household_id: Uuid,
        ) -> Result<Vec<crate::domain::ConsumptionPoint>> {
            Ok(Vec::new())
        }
    }

    #[async_trait]
    impl ProductionForecaster for DummyProductionForecaster {
        async fn predict_next_24h(
            &self,
            _household_id: Uuid,
        ) -> Result<Vec<crate::domain::ProductionPoint>> {
            Ok(Vec::new())
        }
    }

    fn build_controller() -> BatteryController {
        let caps = BatteryCapabilities {
            capacity_kwh: 10.0,
            max_charge_kw: 5.0,
            max_discharge_kw: 5.0,
            efficiency: 0.95,
            degradation_per_cycle: 0.01,
            chemistry: crate::domain::BatteryChemistry::LiFePO4,
        };
        let state = BatteryState {
            soc_percent: 50.0,
            power_w: 0.0,
            voltage_v: 48.0,
            temperature_c: 25.0,
            health_percent: 100.0,
            status: BatteryStatus::Idle,
        };
        let battery = Arc::new(crate::domain::SimulatedBattery::new(state, caps));
        let forecast_engine = ForecastEngine::new(
            Box::new(DummyPriceForecaster),
            Box::new(DummyConsumptionForecaster),
            Box::new(DummyProductionForecaster),
        );

        BatteryController {
            battery,
            optimizer: Arc::new(BatteryOptimizer {
                strategy: Box::new(DynamicProgrammingOptimizer),
            }),
            forecast_engine: Arc::new(forecast_engine),
            schedule: Arc::new(RwLock::new(None)),
            constraints: Arc::new(RwLock::new(Constraints::default())),
            household_id: Uuid::new_v4(),
            state_history: Arc::new(RwLock::new(VecDeque::new())),
            history_capacity: 10,
        }
    }

    #[tokio::test]
    async fn battery_history_downsamples_by_interval() {
        let controller = build_controller();
        let start = Utc::now() - Duration::minutes(10);
        for i in 0..5 {
            let timestamp = start + Duration::minutes(i);
            let state = BatteryState {
                soc_percent: 40.0 + i as f64,
                power_w: 100.0,
                voltage_v: 48.0,
                temperature_c: 25.0,
                health_percent: 100.0,
                status: BatteryStatus::Idle,
            };
            controller.record_state(timestamp, state).await;
        }

        let history = controller
            .get_battery_history(
                start,
                start + Duration::minutes(4),
                Some(Duration::minutes(2)),
            )
            .await;

        assert_eq!(history.len(), 3);
        assert_eq!(history[0].state.soc_percent, 40.0);
        assert_eq!(history[1].state.soc_percent, 42.0);
        assert_eq!(history[2].state.soc_percent, 44.0);
    }

    #[tokio::test]
    async fn battery_statistics_are_computed_from_history() {
        let controller = build_controller();
        let start = Utc::now() - Duration::minutes(5);
        for (i, soc) in [10.0, 20.0, 30.0].iter().enumerate() {
            let timestamp = start + Duration::minutes(i as i64);
            let state = BatteryState {
                soc_percent: *soc,
                power_w: 100.0 * (i as f64 + 1.0),
                voltage_v: 48.0,
                temperature_c: 25.0,
                health_percent: 100.0,
                status: BatteryStatus::Idle,
            };
            controller.record_state(timestamp, state).await;
        }

        let stats = controller
            .get_battery_statistics(start, start + Duration::minutes(2))
            .await
            .expect("expected stats");

        assert_eq!(stats.sample_count, 3);
        assert!((stats.average_soc_percent - 20.0).abs() < f64::EPSILON);
        assert!((stats.average_power_w - 200.0).abs() < f64::EPSILON);
        assert_eq!(stats.min_soc_percent, 10.0);
        assert_eq!(stats.max_soc_percent, 30.0);
    }

    #[tokio::test]
    async fn grid_limits_follow_constraints() {
        let controller = build_controller();
        {
            let mut constraints = controller.constraints.write().await;
            constraints.max_power_grid_kw = 9.0;
        }

        let limits = controller.get_grid_limits().await.expect("limits");
        assert!((limits.max_import_kw - 9.0).abs() < f64::EPSILON);
        assert!((limits.max_export_kw - 9.0).abs() < f64::EPSILON);
        assert!(limits.fuse_rating_amps > 0.0);
    }
}
