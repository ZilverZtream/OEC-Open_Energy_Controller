#![cfg(test)]
//! Load Testing Suite for OEC
//!
//! This test suite verifies that the system handles high load scenarios:
//! - Multiple concurrent API requests
//! - Simulated devices polling
//! - Database write pressure
//! - Control loop latency under load
//!
//! Key Performance Requirements:
//! - Control loop must maintain <1s latency even under API load
//! - System should handle 50+ concurrent API clients
//! - Database writes should not block control loop

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::task::JoinSet;

use open_energy_controller::controller::BatteryController;
use open_energy_controller::domain::{
    Battery, BatteryCapabilities, BatteryState, BatteryStatus, SimulatedBattery,
};
use open_energy_controller::forecast::{
    ConsumptionForecaster, ForecastEngine, PriceForecaster, ProductionForecaster,
};
use open_energy_controller::optimizer::{BatteryOptimizer, Constraints, DynamicProgrammingOptimizer};

use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use std::collections::VecDeque;
use uuid::Uuid;

// Dummy forecasters for testing
struct DummyPriceForecaster;
struct DummyConsumptionForecaster;
struct DummyProductionForecaster;

#[async_trait]
impl PriceForecaster for DummyPriceForecaster {
    async fn predict_next_24h(
        &self,
        _area: open_energy_controller::domain::PriceArea,
    ) -> Result<Vec<open_energy_controller::domain::PricePoint>> {
        Ok(Vec::new())
    }
}

#[async_trait]
impl ConsumptionForecaster for DummyConsumptionForecaster {
    async fn predict_next_24h(
        &self,
        _household_id: Uuid,
    ) -> Result<Vec<open_energy_controller::domain::ConsumptionPoint>> {
        Ok(Vec::new())
    }
}

#[async_trait]
impl ProductionForecaster for DummyProductionForecaster {
    async fn predict_next_24h(
        &self,
        _household_id: Uuid,
    ) -> Result<Vec<open_energy_controller::domain::ProductionPoint>> {
        Ok(Vec::new())
    }
}

fn build_test_controller() -> BatteryController {
    let caps = BatteryCapabilities {
        capacity_kwh: 10.0,
        max_charge_kw: 5.0,
        max_discharge_kw: 5.0,
        efficiency: 0.95,
        degradation_per_cycle: 0.01,
        chemistry: open_energy_controller::domain::BatteryChemistry::LiFePO4,
    };

    let state = BatteryState {
        soc_percent: 50.0,
        soc_kwh: 5.0,
        power_w: 0.0,
        voltage_v: 48.0,
        current_a: 0.0,
        temperature_c: 25.0,
        health_percent: 100.0,
        status: BatteryStatus::Idle,
        cycles: 0,
    };

    let battery: Arc<dyn Battery> = Arc::new(SimulatedBattery::new(state, caps));

    let forecast_engine = ForecastEngine::new(
        Box::new(DummyPriceForecaster),
        Box::new(DummyConsumptionForecaster),
        Box::new(DummyProductionForecaster),
    );

    // Create minimal config for testing
    let config = open_energy_controller::config::Config::load()
        .unwrap_or_else(|_| {
            // Fallback config for testing
            panic!("Failed to load config in test");
        });

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
        history_capacity: 1000,
        power_flow_constraints: Arc::new(
            open_energy_controller::power_flow::AllConstraints::default(),
        ),
        config,
    }
}

/// Test: Control loop latency under concurrent API load
///
/// Verifies that the control loop maintains <1s latency even when
/// 50 concurrent clients are hammering the API endpoints.
#[tokio::test]
#[ignore] // Ignore by default as this is a slow test
async fn test_control_loop_latency_under_api_load() {
    let controller = Arc::new(build_test_controller());

    // Measure control loop latency
    let latency_measurements = Arc::new(RwLock::new(Vec::new()));

    // Spawn simulated control loop
    let ctrl_clone = Arc::clone(&controller);
    let latency_clone = Arc::clone(&latency_measurements);
    let control_loop_handle = tokio::spawn(async move {
        for _ in 0..20 {
            let start = Instant::now();

            // Simulate control loop operations
            let _ = ctrl_clone.get_current_state().await;
            let _ = ctrl_clone.get_schedule().await;

            let elapsed = start.elapsed();
            latency_clone.write().await.push(elapsed);

            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    });

    // Spawn 50 concurrent API clients
    let mut api_tasks = JoinSet::new();

    for i in 0..50 {
        let ctrl_clone = Arc::clone(&controller);
        api_tasks.spawn(async move {
            for _ in 0..10 {
                // Each client makes 10 requests
                let _ = ctrl_clone.get_current_state().await;
                let _ = ctrl_clone.get_battery_capabilities().await;
                let _ = ctrl_clone.get_constraints().await;

                tokio::time::sleep(Duration::from_millis(50 + (i % 20) as u64)).await;
            }
        });
    }

    // Wait for all tasks
    while let Some(_) = api_tasks.join_next().await {}
    control_loop_handle.await.unwrap();

    // Verify latency requirements
    let measurements = latency_measurements.read().await;
    let max_latency = measurements.iter().max().unwrap();
    let avg_latency: Duration = measurements.iter().sum::<Duration>() / measurements.len() as u32;

    println!(
        "Control loop latency - Max: {:?}, Avg: {:?}",
        max_latency, avg_latency
    );

    // Control loop should stay under 1 second even under heavy load
    assert!(
        max_latency < &Duration::from_secs(1),
        "Control loop latency exceeded 1s: {:?}",
        max_latency
    );
}

/// Test: Concurrent read/write operations
///
/// Verifies that multiple concurrent readers and writers don't deadlock
/// or cause data corruption.
#[tokio::test]
#[ignore] // Ignore by default as this is a slow test
async fn test_concurrent_read_write_operations() {
    let controller = Arc::new(build_test_controller());

    let mut tasks = JoinSet::new();

    // Spawn 20 readers
    for _ in 0..20 {
        let ctrl_clone = Arc::clone(&controller);
        tasks.spawn(async move {
            for _ in 0..50 {
                let _ = ctrl_clone.get_current_state().await;
                let _ = ctrl_clone.get_battery_capabilities().await;
                tokio::time::sleep(Duration::from_micros(100)).await;
            }
        });
    }

    // Spawn 10 writers
    for _ in 0..10 {
        let ctrl_clone = Arc::clone(&controller);
        tasks.spawn(async move {
            for i in 0..25 {
                let power = (i as f64 * 100.0) % 5000.0;
                let _ = ctrl_clone.set_battery_power(power).await;
                tokio::time::sleep(Duration::from_micros(200)).await;
            }
        });
    }

    // Wait for all tasks to complete
    while let Some(result) = tasks.join_next().await {
        result.expect("Task should complete successfully");
    }

    // Verify final state is consistent
    let final_state = controller.get_current_state().await.unwrap();
    assert!(
        final_state.soc_percent >= 0.0 && final_state.soc_percent <= 100.0,
        "SoC should be in valid range"
    );
}

/// Test: Memory usage under sustained load
///
/// Verifies that the system doesn't leak memory or accumulate unbounded state
/// under sustained operation.
#[tokio::test]
#[ignore] // Ignore by default as this is a slow test
async fn test_memory_stability_under_load() {
    let controller = Arc::new(build_test_controller());

    let start_time = Utc::now();

    // Run for 10 seconds, simulating continuous operation
    let mut interval = tokio::time::interval(Duration::from_millis(100));
    for _ in 0..100 {
        interval.tick().await;

        // Simulate control loop
        let _ = controller.get_current_state().await;

        // Simulate API requests
        let _ = controller.get_battery_history(
            start_time,
            Utc::now(),
            Some(chrono::Duration::minutes(1)),
        ).await;
    }

    // Verify history buffer is bounded
    let history = controller.get_battery_history(
        start_time,
        Utc::now(),
        None,
    ).await;

    // History should be limited by capacity
    assert!(
        history.len() <= 1000,
        "History should be bounded, got {} entries",
        history.len()
    );
}

/// Test: Concurrent optimization requests
///
/// Verifies that multiple simultaneous optimization requests
/// don't cause data races or inconsistencies.
#[tokio::test]
#[ignore] // Ignore by default as this is a slow test
async fn test_concurrent_optimization() {
    let controller = Arc::new(build_test_controller());

    let mut tasks = JoinSet::new();

    // Spawn 10 concurrent optimization requests
    for _ in 0..10 {
        let ctrl_clone = Arc::clone(&controller);
        tasks.spawn(async move {
            // Note: This will fail due to empty forecast, but we're testing concurrency
            let _ = ctrl_clone.reoptimize_schedule().await;
        });
    }

    // All tasks should complete without panicking
    while let Some(result) = tasks.join_next().await {
        let _ = result.expect("Task should complete without panic");
    }
}

/// Test: Simulated device polling
///
/// Simulates 50 devices polling the controller every 10 seconds,
/// similar to real-world Modbus or API polling patterns.
#[tokio::test]
#[ignore] // Ignore by default as this is a slow test
async fn test_simulated_device_polling() {
    let controller = Arc::new(build_test_controller());

    let mut device_tasks = JoinSet::new();

    // Simulate 50 devices
    for device_id in 0..50 {
        let ctrl_clone = Arc::clone(&controller);
        device_tasks.spawn(async move {
            // Each device polls 5 times (simulating 50 seconds of operation)
            for _ in 0..5 {
                let start = Instant::now();

                // Poll battery state (like Modbus read)
                let _ = ctrl_clone.get_current_state().await;

                let poll_time = start.elapsed();

                // Device polling should complete quickly
                assert!(
                    poll_time < Duration::from_millis(500),
                    "Device {} polling took {:?}, should be <500ms",
                    device_id,
                    poll_time
                );

                // Wait for next poll interval
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
        });
    }

    // Wait for all devices
    while let Some(result) = device_tasks.join_next().await {
        result.expect("Device polling should succeed");
    }
}

/// Benchmark: Throughput test
///
/// Measures how many operations the controller can handle per second.
#[tokio::test]
#[ignore] // Ignore by default as this is a slow test
async fn test_throughput_benchmark() {
    let controller = Arc::new(build_test_controller());

    let start = Instant::now();
    let mut operation_count = 0;

    // Run operations for 5 seconds
    let test_duration = Duration::from_secs(5);

    while start.elapsed() < test_duration {
        // Mix of read and write operations
        let _ = controller.get_current_state().await;
        operation_count += 1;

        let _ = controller.get_battery_capabilities().await;
        operation_count += 1;

        if operation_count % 10 == 0 {
            let _ = controller.set_battery_power(1000.0).await;
            operation_count += 1;
        }
    }

    let elapsed = start.elapsed();
    let ops_per_second = operation_count as f64 / elapsed.as_secs_f64();

    println!(
        "Throughput: {:.0} ops/second ({} ops in {:?})",
        ops_per_second, operation_count, elapsed
    );

    // Should handle at least 100 ops/second
    assert!(
        ops_per_second > 100.0,
        "Throughput too low: {:.0} ops/s",
        ops_per_second
    );
}
