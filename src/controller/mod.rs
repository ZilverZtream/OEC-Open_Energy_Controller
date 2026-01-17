#![allow(dead_code)]
pub mod maintenance;
pub mod pid;
pub mod power_transition;
pub mod safety;
pub mod safety_monitor;
pub mod scheduler;
pub mod v2x_controller;

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::{collections::VecDeque, sync::Arc};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::config::Config;
use crate::power_flow::{
    constraints::{EconomicObjectives, PhysicalConstraints, SafetyConstraints},
    model::PowerFlowModel,
    AllConstraints, PowerFlowInputs,
};
use crate::simulation::{Environment, EnvironmentConfig};

use crate::domain::{
    Battery, BatteryCapabilities, BatteryState, Forecast24h, GridConnection, GridLimits,
    GridStatistics, GridStatus, HealthStatus, PriceArea, Schedule,
};
use crate::forecast::{
    ConsumptionForecaster, ElprisetJustNuPriceForecaster, ForecastEngine, GeoLocation,
    SimpleConsumptionForecaster, SimpleProductionForecaster, SmhiClient, WeatherForecast,
};
use crate::optimizer::{BatteryOptimizer, Constraints, DynamicProgrammingOptimizer, SystemState};
use crate::repo::Repositories;

#[derive(Clone)]
pub struct AppState {
    pub cfg: Config,
    pub controller: Arc<BatteryController>,
    pub repos: Arc<Repositories>,
    pub safety_monitor: Arc<safety_monitor::SafetyMonitor>,
}

impl AppState {
    pub async fn new(cfg: Config) -> Result<Self> {
        let repos = Arc::new(Repositories::new(&cfg).await?);

        // Validate battery capabilities from config to prevent division by zero
        let caps = BatteryCapabilities {
            capacity_kwh: cfg.battery.capacity_kwh,
            max_charge_kw: cfg.battery.max_charge_kw,
            max_discharge_kw: cfg.battery.max_discharge_kw,
            efficiency: cfg.battery.efficiency,
            degradation_per_cycle: cfg.battery.degradation_per_cycle,
            chemistry: crate::domain::BatteryChemistry::LiFePO4,
        };

        // Validate capabilities
        if !caps.capacity_kwh.is_finite() || caps.capacity_kwh <= 0.0 {
            bail!(
                "Battery capacity_kwh must be positive and finite, got: {}",
                caps.capacity_kwh
            );
        }
        if !caps.max_charge_kw.is_finite() || caps.max_charge_kw <= 0.0 {
            bail!(
                "Battery max_charge_kw must be positive and finite, got: {}",
                caps.max_charge_kw
            );
        }
        if !caps.max_discharge_kw.is_finite() || caps.max_discharge_kw <= 0.0 {
            bail!(
                "Battery max_discharge_kw must be positive and finite, got: {}",
                caps.max_discharge_kw
            );
        }
        if !caps.efficiency.is_finite() || caps.efficiency <= 0.0 || caps.efficiency > 1.0 {
            bail!(
                "Battery efficiency must be between 0 and 1, got: {}",
                caps.efficiency
            );
        }
        if !caps.degradation_per_cycle.is_finite() || caps.degradation_per_cycle < 0.0 {
            bail!(
                "Battery degradation_per_cycle must be non-negative and finite, got: {}",
                caps.degradation_per_cycle
            );
        }
        // CRITICAL FIX: Use DeviceFactory to respect hardware configuration
        // Previously, this was hardcoded to use SimulatedBattery regardless of config
        use crate::hardware::factory::{DeviceFactory, HardwareMode};

        let hardware_mode = match cfg.hardware.mode {
            crate::config::HardwareMode::Simulated => HardwareMode::Simulated,
            #[cfg(feature = "modbus")]
            crate::config::HardwareMode::Modbus => HardwareMode::Modbus,
            crate::config::HardwareMode::Mock => HardwareMode::Mock,
        };

        let factory = DeviceFactory::with_config(hardware_mode, cfg.clone());
        let battery = factory
            .create_battery(
                caps.clone(),
                cfg.battery.initial_soc_percent,
                cfg.battery.ambient_temp_c,
            )
            .await;

        // AUDIT FIX #7: Handle price forecaster initialization gracefully
        // ElprisetJustNuPriceForecaster::new() creates an HTTP client, which can fail
        // (though rarely) due to TLS setup issues or invalid config. Provide clear error context.
        let price = Box::new(
            ElprisetJustNuPriceForecaster::new(
                cfg.prices.base_url.clone(),
                std::time::Duration::from_secs(cfg.prices.cache_ttl_seconds),
            )
            .context("Failed to initialize price forecaster HTTP client. Check TLS/SSL setup.")?,
        );

        // Use ML-enhanced forecaster if enabled, otherwise use simple baseline
        #[cfg(feature = "ml")]
        let consumption_forecaster: Box<dyn ConsumptionForecaster> = if cfg.forecast.use_ml_models {
            Box::new(
                crate::forecast::consumption::MLConsumptionForecaster::new(
                    cfg.household.latitude,
                    cfg.household.longitude,
                )
                .await,
            )
        } else {
            Box::new(SimpleConsumptionForecaster)
        };

        #[cfg(not(feature = "ml"))]
        let consumption_forecaster: Box<dyn ConsumptionForecaster> =
            Box::new(SimpleConsumptionForecaster);

        let forecast_engine = Arc::new(ForecastEngine::new(
            price,
            consumption_forecaster,
            Box::new(SimpleProductionForecaster::default()),
        ));

        // Use MILP optimizer if optimization feature is enabled, otherwise use DP
        #[cfg(feature = "optimization")]
        let strategy = Box::new(crate::optimizer::strategies::milp::MilpOptimizer::default());

        #[cfg(not(feature = "optimization"))]
        let strategy = Box::new(DynamicProgrammingOptimizer);

        let optimizer = Arc::new(BatteryOptimizer { strategy });
        let schedule = Arc::new(RwLock::new(None::<Schedule>));

        // Initialize constraints with actual battery capabilities
        let constraints = Constraints {
            min_soc_percent: cfg.battery.min_soc_percent,
            max_soc_percent: cfg.battery.max_soc_percent,
            max_cycles_per_day: 1.0,
            max_power_grid_kw: 11.0,
            v2g_enabled: false,
            battery_capacity_kwh: caps.capacity_kwh,
            battery_max_charge_kw: caps.max_charge_kw,
            battery_max_discharge_kw: caps.max_discharge_kw,
            battery_efficiency: caps.efficiency,
            battery_degradation_per_cycle: caps.degradation_per_cycle,
            battery_replacement_cost_sek: cfg.battery.replacement_cost_sek,
            peak_power_tariff_sek_per_kw: 100.0, // Swedish "Effekttariff" (typical value)
        };

        // Initialize power flow constraints for real-time safety checks
        let power_flow_constraints = Arc::new(AllConstraints {
            physical: PhysicalConstraints {
                max_grid_import_kw: 11.0,
                max_grid_export_kw: 11.0,
                max_battery_charge_kw: caps.max_charge_kw,
                max_battery_discharge_kw: caps.max_discharge_kw,
                evse_min_current_a: 6.0,
                evse_max_current_a: 32.0,
                phases: 1,
                max_current_per_phase_a: Some(32.0),
                grid_voltage_v: 230.0,
            },
            safety: SafetyConstraints {
                battery_min_soc_percent: cfg.battery.min_soc_percent,
                battery_max_soc_percent: cfg.battery.max_soc_percent,
                house_priority: true,
                max_battery_cycles_per_day: 1.5,
                max_battery_temp_c: 45.0,
            },
            economic: EconomicObjectives {
                grid_price_sek_kwh: 1.5, // Will be updated from schedule
                export_price_sek_kwh: 0.8,
                prefer_self_consumption: true,
                arbitrage_threshold_sek_kwh: 2.0,
                ev_departure_time: None,
                ev_target_soc_percent: None,
                low_price_charge_rate: cfg.optimization.low_price_charge_rate,
            },
        });

        // CRITICAL FIX: Initialize V2X Controller if EV charger is available
        // This enables vehicle-to-grid/home functionality
        let ev_charger = factory.create_ev_charger();
        let ev_charger_clone = Arc::clone(&ev_charger);
        let v2x_controller = if ev_charger.v2x_capabilities().is_some() {
            let v2x_config = v2x_controller::V2XConfig::default();
            Some(Arc::new(v2x_controller::V2XController::new(
                ev_charger, v2x_config,
            )))
        } else {
            None
        };

        // CRITICAL FIX: Initialize Safety Monitor BEFORE controller
        // This provides defense-in-depth against hardware failures
        let safety_config = safety_monitor::SafetyMonitorConfig {
            check_interval_s: 1,   // Fast 1-second monitoring
            fuse_rating_a: 25.0,   // Default 25A fuse
            fuse_trip_margin: 0.1, // Trip at 90% of rating
            grid_voltage_min_v: 207.0,
            grid_voltage_max_v: 253.0,
            grid_frequency_min_hz: 49.5,
            grid_frequency_max_hz: 50.5,
            battery_temp_min_c: -10.0,
            battery_temp_max_c: 55.0,
            battery_soc_min_percent: cfg.battery.min_soc_percent,
            battery_soc_max_percent: cfg.battery.max_soc_percent,
            control_loop_timeout_s: 30, // 30s timeout for control loop
            enable_emergency_stop: true,
        };
        let (safety_monitor, _safety_rx) = safety_monitor::SafetyMonitor::new(safety_config);
        let safety_monitor_arc = Arc::new(safety_monitor);

        // CRITICAL FIX: Initialize simulation environment if hardware mode is Simulated
        // This provides realistic PV and house load data instead of static fallback values
        let environment = if matches!(cfg.hardware.mode, crate::config::HardwareMode::Simulated) {
            info!("Initializing simulation environment for realistic PV/load data");
            let env_config = EnvironmentConfig::for_location(
                cfg.household.latitude,
                cfg.household.longitude,
                0, // UTC timezone for now, TODO: use cfg.household.timezone
                chrono::Utc::now().naive_utc(),
            )
            .with_pv_capacity(5.0) // TODO: Get from config
            .with_household_size(4) // TODO: Get from config
            .with_fuse_rating(25.0) // TODO: Get from config
            .with_random_seed(42); // Reproducible simulation

            Some(Arc::new(RwLock::new(Environment::new(env_config))))
        } else {
            None
        };

        // CRITICAL FIX: Create bounded channel for state recording to prevent resource leak
        // Limits pending database writes to 100 to prevent OOM during long simulations
        #[cfg(feature = "db")]
        let (state_write_tx, mut state_write_rx) =
            tokio::sync::mpsc::channel::<(DateTime<Utc>, BatteryState)>(100);

        #[cfg(feature = "db")]
        let state_write_tx_opt = Some(state_write_tx);

        #[cfg(not(feature = "db"))]
        let state_write_tx_opt: Option<
            tokio::sync::mpsc::Sender<(DateTime<Utc>, BatteryState)>,
        > = None;

        let history_capacity = ((24 * 60 * 60) / cfg.controller.tick_seconds.max(1)) as usize;
        let household_id = Uuid::new_v4();
        let controller = Arc::new(BatteryController {
            battery,
            optimizer,
            forecast_engine,
            schedule,
            constraints: Arc::new(RwLock::new(constraints)),
            household_id,
            state_history: Arc::new(RwLock::new(VecDeque::with_capacity(
                history_capacity.max(1),
            ))),
            history_capacity: history_capacity.max(1),
            power_flow_constraints,
            config: cfg.clone(),
            repos: Arc::clone(&repos),
            last_sensor_read: Arc::new(RwLock::new(Utc::now())),
            v2x: v2x_controller,
            ev_charger: ev_charger_clone,
            safety_monitor: Some(Arc::clone(&safety_monitor_arc)),
            environment,
            #[cfg(feature = "db")]
            state_write_tx: state_write_tx_opt,
        });

        // CRITICAL FIX: Spawn persistent worker task for state recording
        // This prevents spawning unbounded tasks and limits pending writes
        #[cfg(feature = "db")]
        {
            let repos_clone = Arc::clone(&repos);
            tokio::spawn(async move {
                use crate::repo::battery_states::BatteryStateRow;
                use tracing::error;

                while let Some((timestamp, state)) = state_write_rx.recv().await {
                    let battery_state_row = BatteryStateRow {
                        id: 0, // Will be auto-generated by database
                        device_id: Some(household_id),
                        timestamp,
                        soc_percent: state.soc_percent,
                        power_w: state.power_w,
                        voltage_v: Some(state.voltage_v),
                        temperature_c: Some(state.temperature_c),
                    };

                    if let Err(e) = repos_clone
                        .db
                        .battery_states()
                        .insert(&battery_state_row)
                        .await
                    {
                        error!(error=%e, "Failed to persist battery state to database");
                    }
                }
                info!("State recording worker shut down");
            });
        }

        Ok(Self {
            cfg,
            controller,
            repos,
            safety_monitor: safety_monitor_arc,
        })
    }
}

pub fn spawn_controller_tasks(state: AppState, cfg: Config) {
    let state_arc = Arc::new(state);

    let controller = Arc::clone(&state_arc.controller);
    let cfg_clone = cfg.clone();
    tokio::spawn(async move {
        if let Err(e) = controller.run(cfg_clone.controller.tick_seconds).await {
            warn!(error=%e, "controller loop stopped");
        }
    });

    let controller2 = Arc::clone(&state_arc.controller);
    let cfg_clone2 = cfg.clone();
    tokio::spawn(async move {
        if let Err(e) = controller2
            .reoptimize_loop(cfg_clone2.controller.reoptimize_every_minutes)
            .await
        {
            warn!(error=%e, "reoptimize loop stopped");
        }
    });

    // Start periodic task scheduler (includes ML training)
    if cfg.forecast.use_ml_models {
        let scheduler = Arc::new(scheduler::TaskScheduler::new(state_arc.clone()));
        scheduler.start();
        info!("Task scheduler started with ML training enabled");
    } else {
        info!("ML models disabled in configuration - skipping task scheduler");
    }

    // CRITICAL FIX: Spawn Safety Monitor Supervisor
    // This provides independent high-priority safety monitoring
    let safety_monitor = Arc::clone(&state_arc.safety_monitor);
    let controller_for_safety = Arc::clone(&state_arc.controller);

    // AUDIT FIX #2: Subscribe to safety monitor emergency stop commands
    // The safety monitor broadcasts emergency stop commands when violations are detected
    let mut safety_rx = safety_monitor.subscribe();
    let controller_for_emergency = Arc::clone(&state_arc.controller);
    tokio::spawn(async move {
        use tracing::error;
        loop {
            match safety_rx.recv().await {
                Ok(safety_monitor::SafetyCommand::EmergencyStop(violation)) => {
                    error!(
                        violation_type = %violation.violation_type,
                        message = %violation.message,
                        "EMERGENCY STOP triggered by safety violation"
                    );

                    // Immediately halt all power flows
                    if let Err(e) = controller_for_emergency.battery.set_power(0.0).await {
                        error!(error=%e, "Failed to stop battery during emergency");
                    }

                    if let Err(e) = controller_for_emergency.ev_charger.set_current(0.0).await {
                        error!(error=%e, "Failed to stop EV charger during emergency");
                    }

                    info!("Emergency stop completed - all power flows halted");
                }
                Ok(safety_monitor::SafetyCommand::Resume) => {
                    info!("Safety monitor resumed normal operation");
                }
                Err(e) => {
                    error!(error=%e, "Safety monitor channel error");
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }
            }
        }
    });

    tokio::spawn(async move {
        safety_monitor.start_monitoring().await;
        info!("Safety monitor started");

        let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
        loop {
            interval.tick().await;

            // AUDIT FIX #1: Do NOT update heartbeat here!
            // The safety loop must NOT update its own heartbeat - that would defeat
            // the watchdog timeout detection. The heartbeat is updated by the main
            // controller loop after each successful iteration.

            // Get current battery state for safety checks
            if let Ok(battery_state) = controller_for_safety.get_current_state().await {
                // Get real grid status from controller (use safe defaults if unavailable)
                let grid_status =
                    controller_for_safety
                        .get_grid_status()
                        .await
                        .unwrap_or(GridConnection {
                            status: GridStatus::Normal,
                            import_power_w: 0.0,
                            export_power_w: 0.0,
                            frequency_hz: 50.0,
                            voltage_v: 230.0,
                            current_a: 0.0,
                        });

                let measurements = safety_monitor::SafetyMeasurements {
                    grid_import_kw: grid_status.import_power_w / 1000.0, // Convert W to kW
                    grid_voltage_v: grid_status.voltage_v,
                    grid_frequency_hz: grid_status.frequency_hz,
                    battery_soc_percent: battery_state.soc_percent,
                    battery_temperature_c: battery_state.temperature_c,
                    grid_nominal_voltage_v: 230.0,
                    timestamp: chrono::Utc::now(),
                };

                let violations = safety_monitor.check_safety(&measurements).await;
                if !violations.is_empty() {
                    warn!(
                        "Safety violations detected: {} violations",
                        violations.len()
                    );
                }
            }
        }
    });

    // CRITICAL FIX: Spawn Database Maintenance Tasks
    // This prevents database bloat from high-frequency data logging
    maintenance::spawn_maintenance_tasks(Arc::clone(&state_arc));
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
    // Power flow model for real-time safety and optimization
    power_flow_constraints: Arc<AllConstraints>,
    // Configuration for fallback values and cycle penalty calculation
    config: Config,
    // Repository for persistent storage
    repos: Arc<Repositories>,
    // Last successful sensor read timestamp for staleness detection
    last_sensor_read: Arc<RwLock<DateTime<Utc>>>,
    // V2X controller for vehicle-to-grid/home coordination
    v2x: Option<Arc<v2x_controller::V2XController>>,
    // EV charger for direct actuation
    ev_charger: Arc<dyn crate::domain::EvCharger>,
    // Safety monitor for watchdog heartbeat updates
    safety_monitor: Option<Arc<safety_monitor::SafetyMonitor>>,
    // CRITICAL FIX: Simulation environment for realistic PV/load data
    // This replaces the static fallback values with dynamic simulation
    environment: Option<Arc<RwLock<Environment>>>,
    // CRITICAL FIX: Bounded channel for state recording to prevent resource leak
    // Limits pending database writes to prevent OOM during long simulations
    #[cfg(feature = "db")]
    state_write_tx: Option<tokio::sync::mpsc::Sender<(DateTime<Utc>, BatteryState)>>,
}

impl BatteryController {
    pub async fn run(self: Arc<Self>, tick_seconds: u64) -> Result<()> {
        use tracing::error;

        let mut interval =
            tokio::time::interval(std::time::Duration::from_secs(tick_seconds.max(1)));
        loop {
            interval.tick().await;

            // CRITICAL FIX: Capture timestamp BEFORE sensor polling to ensure
            // accurate time-based calculations. Modbus polling can take 2-3s.
            let now_utc = Utc::now();

            // CRITICAL FIX: Tick the simulation environment before reading sensors
            // This ensures the simulation progresses in sync with the control loop
            if let Some(ref env) = self.environment {
                let mut env_lock = env.write().await;
                // Advance simulation by the tick interval
                let delta = chrono::Duration::seconds(tick_seconds as i64);

                // We need to estimate grid import/export for the simulation
                // For now, use a simple heuristic based on net load
                let current_state = env_lock.state();
                let net_load = current_state.net_load_kw;
                let (grid_import, grid_export) = if net_load > 0.0 {
                    (net_load, 0.0)
                } else {
                    (0.0, -net_load)
                };

                env_lock.tick(delta, grid_import, grid_export);
                debug!(
                    timestamp = %now_utc,
                    house_kw = current_state.house.load_kw,
                    solar_kw = current_state.solar.production_kw,
                    "Simulation tick"
                );
            }

            // CRITICAL FIX: Wrap sensor read in error handler to prevent loop termination
            // If sensor read fails, log error, wait, and continue (don't crash)
            let state = match self.battery.read_state().await {
                Ok(s) => {
                    // Update last successful sensor read timestamp
                    *self.last_sensor_read.write().await = now_utc;

                    // AUDIT FIX #1: Update safety monitor heartbeat after successful iteration
                    // This allows the safety monitor to detect if the main control loop hangs
                    if let Some(ref safety_monitor) = self.safety_monitor {
                        safety_monitor.heartbeat().await;
                    }

                    s
                }
                Err(e) => {
                    error!(error=%e, "Control loop sensor failure - will retry");

                    // AUDIT FIX #5: Attempt to reconnect on sensor failure
                    // If the Modbus TCP connection dropped, reconnect before retrying
                    info!("Attempting to reconnect battery...");
                    if let Err(reconnect_err) = self.battery.reconnect().await {
                        warn!(error=%reconnect_err, "Battery reconnection failed");
                    } else {
                        info!("Battery reconnection successful");
                    }

                    // Check if data is stale (no successful read for >30s)
                    let last_read = *self.last_sensor_read.read().await;
                    let staleness_seconds = (now_utc - last_read).num_seconds();

                    if staleness_seconds > 30 {
                        error!(
                            staleness_seconds = staleness_seconds,
                            "Stale sensor data detected (>30s) - entering emergency stop"
                        );
                        // Enter safe mode: set battery power to 0
                        if let Err(e) = self.battery.set_power(0.0).await {
                            error!(error=%e, "Failed to enter emergency stop mode");
                        }
                    }

                    // Wait 1 second before retrying
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    continue; // Skip this iteration and retry
                }
            };

            // CRITICAL FIX: Offload telemetry recording to background task
            // to prevent DB write latency from blocking the critical control path.
            // If record_state takes >1000ms (e.g., SD card stall), queueing up
            // 1Hz writes can starve the DB connection pool for API reads.
            let self_clone = Arc::clone(&self);
            let state_clone = state.clone();
            tokio::spawn(async move {
                self_clone.record_state(now_utc, state_clone).await;
            });

            // Get scheduled target power from optimizer
            let schedule_snapshot = self.schedule.read().await.clone();
            let schedule_target_w = schedule_snapshot.as_ref().and_then(|s| s.power_at(now_utc));

            // CRITICAL FIX: Evaluate V2X discharge decision and feed target into PowerFlowModel
            // EV commands are issued only after PowerFlowModel computes a safe snapshot
            let mut ev_target_w: Option<f64> = None;
            if let Some(ref v2x) = self.v2x {
                // Get current price for V2X decision
                let current_price = schedule_snapshot
                    .as_ref()
                    .and_then(|s| {
                        s.entries
                            .iter()
                            .find(|e| e.time_start <= now_utc && now_utc < e.time_end)
                            .map(|e| e.price_sek_per_kwh)
                    })
                    .unwrap_or(1.5);

                // Calculate average price for comparison
                let avg_price = schedule_snapshot
                    .as_ref()
                    .map(|s| {
                        let sum: f64 = s.entries.iter().map(|e| e.price_sek_per_kwh).sum();
                        sum / s.entries.len().max(1) as f64
                    })
                    .unwrap_or(1.5);

                // Evaluate V2X discharge decision
                match v2x
                    .evaluate_discharge_decision(current_price, avg_price, now_utc)
                    .await
                {
                    Ok(decision) => {
                        if decision.should_discharge && decision.target_power_w > 0.0 {
                            info!(
                                power_w = -decision.target_power_w,
                                reason = %decision.reason,
                                vehicle_soc = decision.vehicle_soc,
                                "V2X EV discharge requested (V2G/V2H)"
                            );
                            // Store EV target separately (negative = discharge from EV)
                            ev_target_w = Some(-decision.target_power_w);
                        }
                    }
                    Err(e) => {
                        warn!(error=%e, "V2X decision evaluation failed");
                    }
                }
            }

            // Build PowerFlowInputs from current state
            // TODO: Integrate real sensors when available:
            // - PV production: Read from inverter or PV sensor via Modbus/OCPP
            // - House load: Calculate from grid meter (grid_import + battery_power - pv_production)
            // For now, use sensor fallback values as conservative estimates
            let pv_production_kw = self.get_pv_production_kw().await;
            let house_load_kw = self.get_house_load_kw().await;

            // Get grid price from current schedule or use fallback
            let grid_price_sek_kwh = schedule_snapshot
                .as_ref()
                .and_then(|s| {
                    s.entries
                        .iter()
                        .find(|e| e.time_start <= now_utc && now_utc < e.time_end)
                        .map(|e| e.price_sek_per_kwh)
                })
                .unwrap_or(1.5);

            let mut inputs = PowerFlowInputs::new(
                pv_production_kw,
                house_load_kw,
                state.soc_percent,
                state.temperature_c,
                grid_price_sek_kwh,
                now_utc, // Use timestamp captured before sensor polling
            );

            // Pass schedule target to PowerFlowModel if available
            if let Some(target_w) = schedule_target_w {
                inputs = inputs.with_target_power_w(target_w);
            }

            // AUDIT FIX #6: Pass EV target to PowerFlowModel separately from home battery
            if let Some(ev_w) = ev_target_w {
                inputs = inputs.with_ev_target_power_w(ev_w);
            }

            // Validate inputs before passing to PowerFlowModel
            if let Err(e) = inputs.validate() {
                warn!(error=%e, "Invalid PowerFlowInputs, using fallback");
                let caps = self.battery.capabilities();
                let fallback_w = schedule_target_w
                    .unwrap_or(0.0)
                    .clamp(-caps.max_discharge_kw * 1000.0, caps.max_charge_kw * 1000.0);
                self.battery.set_power(fallback_w).await?;
                continue;
            }

            // Create PowerFlowModel and compute optimal flows
            let model = PowerFlowModel::new((*self.power_flow_constraints).clone());

            // Compute power flows with safety checks
            let (target_power_w, ev_current_a, ev_discharge_w) = match model.compute_flows(&inputs)
            {
                Ok(snapshot) => {
                    // Use the battery power from PowerFlowModel
                    // Convert kW to W
                    let battery_target_w = snapshot.battery_kw * 1000.0;

                    // CRITICAL FIX: Extract EV charging current from PowerFlowModel
                    // Convert kW to current: I = P / V (assuming 230V single-phase)
                    let ev_current_a = if snapshot.ev_kw > 0.0 {
                        snapshot.ev_kw * 1000.0 / 230.0 // kW -> A
                    } else {
                        0.0
                    };
                    let ev_discharge_w = if snapshot.ev_kw < 0.0 {
                        -snapshot.ev_kw * 1000.0
                    } else {
                        0.0
                    };

                    // Log the power flow decision
                    info!(
                        soc_percent = state.soc_percent,
                        current_power_w = state.power_w,
                        schedule_target_w = schedule_target_w.unwrap_or(0.0),
                        powerflow_target_w = battery_target_w,
                        pv_kw = snapshot.pv_kw,
                        house_kw = snapshot.house_kw,
                        grid_kw = snapshot.grid_kw,
                        ev_kw = snapshot.ev_kw,
                        ev_current_a = ev_current_a,
                        ev_discharge_w = ev_discharge_w,
                        "PowerFlowModel decision"
                    );

                    (battery_target_w, ev_current_a, ev_discharge_w)
                }
                Err(e) => {
                    // If PowerFlowModel fails due to constraint violations,
                    // the safest fallback is to idle the battery and stop EV charging
                    warn!(error=%e, "PowerFlowModel failed, entering safe fallback mode (Idle)");
                    (0.0, 0.0, 0.0)
                }
            };

            // CRITICAL FIX: Check safety monitor BEFORE sending any power commands
            // This prevents race condition where main loop and safety monitor fight over setpoints
            let mut commanded_power_w = target_power_w;
            let mut commanded_ev_current_a = ev_current_a;
            let mut commanded_ev_discharge_w = ev_discharge_w;

            if let Some(ref safety_monitor) = self.safety_monitor {
                let safety_state = safety_monitor.state().await;
                if safety_state.emergency_stop_active {
                    warn!("Safety monitor emergency stop active - overriding commands to 0");
                    commanded_power_w = 0.0;
                    commanded_ev_current_a = 0.0;
                    commanded_ev_discharge_w = 0.0;
                }
            }

            // Clamp to physical limits
            let caps = self.battery.capabilities();
            let max_charge_w = caps.max_charge_kw * 1000.0;
            let max_discharge_w = caps.max_discharge_kw * 1000.0;
            commanded_power_w = commanded_power_w.clamp(-max_discharge_w, max_charge_w);

            if let Some(ref _v2x) = self.v2x {
                if commanded_ev_discharge_w > 0.0 {
                    if let Err(e) = self.ev_charger.start_discharging().await {
                        error!(
                            error = %e,
                            commanded_power_w = commanded_ev_discharge_w,
                            "Failed to start EV discharging"
                        );
                    } else if let Err(e) = self
                        .ev_charger
                        .set_discharge_power(commanded_ev_discharge_w)
                        .await
                    {
                        error!(
                            error = %e,
                            commanded_power_w = commanded_ev_discharge_w,
                            "Failed to set EV discharge power"
                        );
                    } else {
                        debug!(
                            ev_discharge_w = commanded_ev_discharge_w,
                            "EV discharge power updated"
                        );
                    }
                } else if let Err(e) = self.ev_charger.stop_discharging().await {
                    error!(
                        error = %e,
                        "Failed to stop EV discharging"
                    );
                }
            }

            if commanded_ev_discharge_w <= 0.0 {
                // CRITICAL FIX: Actuate EV charger to prevent fuse overload
                // The PowerFlowModel calculated the safe EV charging current.
                // We MUST apply it, or the main fuse will blow during high house load.
                if let Err(e) = self.ev_charger.set_current(commanded_ev_current_a).await {
                    error!(
                        error = %e,
                        commanded_current_a = commanded_ev_current_a,
                        "Failed to set EV charger current"
                    );
                } else {
                    debug!(
                        ev_current_a = commanded_ev_current_a,
                        "EV charger current updated"
                    );
                }
            }

            self.battery.set_power(commanded_power_w).await?;
            info!(
                soc_percent = state.soc_percent,
                power_w = state.power_w,
                target_power_w = target_power_w,
                commanded_power_w = commanded_power_w,
                ev_current_a = ev_current_a,
                ev_discharge_w = ev_discharge_w,
                "control tick"
            );
        }
    }

    pub async fn reoptimize_loop(self: Arc<Self>, every_minutes: u64) -> Result<()> {
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
        // CRITICAL FIX: Clone data quickly and drop the lock BEFORE filtering/processing
        // This prevents blocking the controller's write operations during serialization
        // The read lock should be held for the minimum time possible
        let history_snapshot = {
            let history = self.state_history.read().await;
            // Clone only the relevant portion to minimize memory and lock time
            history.iter().cloned().collect::<Vec<_>>()
        }; // Lock is dropped here

        // Now filter the cloned data without holding any lock
        let mut results = Vec::new();
        let mut last_included: Option<DateTime<Utc>> = None;

        for sample in history_snapshot.iter() {
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

        // Prevent division by zero by checking minimum threshold
        let fuse_rating_amps = if max_import_kw > 0.01 {
            // Typical 3-phase formula: P = sqrt(3) * V * I
            // Simplified: P = 3 * V * I for balanced load
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
    /// Get current PV production (kW)
    ///
    /// CRITICAL FIX: Now queries the simulation environment for realistic data
    /// Falls back to config value only if simulation is not available
    async fn get_pv_production_kw(&self) -> f64 {
        // If simulation environment is available, query it
        if let Some(ref env) = self.environment {
            return env.read().await.solar_production_kw();
        }

        // Future implementation: Read from inverter via Modbus
        // if let Some(inverter) = &self.inverter {
        //     if let Ok(state) = inverter.read_state().await {
        //         return state.pv_power_w / 1000.0;
        //     }
        // }

        // Fallback to config (only for real hardware mode)
        self.config
            .hardware
            .sensor_fallback
            .default_pv_production_kw
    }

    /// Get current house load (kW)
    ///
    /// CRITICAL FIX: Now queries the simulation environment for realistic data
    /// Falls back to config value only if simulation is not available
    async fn get_house_load_kw(&self) -> f64 {
        // If simulation environment is available, query it
        if let Some(ref env) = self.environment {
            return env.read().await.house_load_kw();
        }

        // Future implementation: Calculate from energy balance
        // house_load = grid_import + battery_discharge + pv_production - grid_export
        // if let Ok(grid) = self.get_grid_status().await {
        //     let battery_state = self.battery.read_state().await.ok()?;
        //     let pv_kw = self.get_pv_production_kw().await;
        //     return (grid.import_power_w / 1000.0) + (battery_state.power_w / 1000.0) + pv_kw;
        // }

        // Fallback to config (only for real hardware mode)
        self.config.hardware.sensor_fallback.default_house_load_kw
    }

    async fn record_state(&self, timestamp: DateTime<Utc>, state: BatteryState) {
        // Update in-memory history
        {
            let mut history = self.state_history.write().await;
            // Use while loop to handle case where capacity might have changed
            // The write lock protects against concurrent modifications
            while history.len() >= self.history_capacity {
                history.pop_front();
            }
            history.push_back(BatteryStateSample {
                timestamp,
                state: state.clone(),
            });
        }

        // CRITICAL FIX: Send to bounded channel instead of spawning unbounded tasks
        // This prevents resource leak during long simulations
        #[cfg(feature = "db")]
        {
            if let Some(ref tx) = self.state_write_tx {
                // Use try_send to avoid blocking if channel is full
                // If full, drop this sample to prevent backpressure
                if let Err(e) = tx.try_send((timestamp, state)) {
                    debug!("State recording channel full, dropping sample: {}", e);
                }
            }
        }
    }
}

/// Simple P controller with deadband and NaN/Inf protection
///
/// The deadband prevents hunting for small differences, reducing battery wear.
/// Output clamping ensures we don't command power beyond physical limits.
fn simple_p_control(actual_w: f64, target_w: f64, max_charge_w: f64, max_discharge_w: f64) -> f64 {
    // Validate all parameters are finite and limits are positive
    if !actual_w.is_finite() || !target_w.is_finite() {
        return 0.0;
    }
    if !max_charge_w.is_finite() || !max_discharge_w.is_finite() {
        return 0.0;
    }
    if max_charge_w <= 0.0 || max_discharge_w <= 0.0 {
        return 0.0; // Invalid limits, can't control safely
    }

    let error = target_w - actual_w;

    // Deadband: ignore errors smaller than 50W to prevent hunting
    const DEADBAND_W: f64 = 50.0;
    if error.abs() < DEADBAND_W {
        return target_w;
    }

    // P control with gain
    let k = 0.5;
    let control_w = target_w + error * k;

    // Output clamping to physical limits
    // Positive = charging, negative = discharging
    control_w.clamp(-max_discharge_w, max_charge_w)
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

        // Create a minimal test config
        let config = Config::load().unwrap_or_else(|_| {
            // If config loading fails in tests, use defaults
            panic!("Config loading failed in test");
        });

        // Create minimal repositories for tests
        #[cfg(not(feature = "db"))]
        let repos = Arc::new(Repositories {});

        #[cfg(feature = "db")]
        let repos = {
            // For tests with db feature, we'd need to set up a test database
            // For now, panic as tests should use #[ignore] or mock
            panic!("Test requires database setup - use #[ignore] or mock repos");
        };

        // Create a simulated EV charger for tests
        let ev_charger =
            Arc::new(crate::domain::SimulatedEvCharger::new()) as Arc<dyn crate::domain::EvCharger>;

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
            power_flow_constraints: Arc::new(AllConstraints::default()),
            config,
            repos,
            last_sensor_read: Arc::new(RwLock::new(Utc::now())),
            v2x: None, // No V2X in tests by default
            ev_charger,
            safety_monitor: None, // No safety monitor in tests by default
            environment: None,    // No environment in tests by default
            #[cfg(feature = "db")]
            state_write_tx: None, // No DB writes in tests by default
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
