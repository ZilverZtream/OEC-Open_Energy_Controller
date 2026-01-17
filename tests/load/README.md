# Load Testing Suite

This directory contains load and stress tests for the Open Energy Controller.

## Purpose

These tests verify that the system maintains acceptable performance under high load:

- **Control Loop Latency**: Ensures the 10s control loop stays <1s even under heavy API load
- **Concurrent Operations**: Tests for deadlocks and race conditions
- **Memory Stability**: Verifies no memory leaks during sustained operation
- **Device Polling**: Simulates real-world Modbus/API polling patterns
- **Throughput**: Benchmarks operations per second

## Running Load Tests

Load tests are marked as `#[ignore]` by default to avoid slowing down CI builds.

### Run all load tests:
```bash
cargo test --test load_tests -- --ignored --test-threads=1
```

### Run a specific load test:
```bash
cargo test --test load_tests test_control_loop_latency_under_api_load -- --ignored --nocapture
```

### Run with verbose output:
```bash
RUST_LOG=debug cargo test --test load_tests -- --ignored --nocapture
```

## Test Scenarios

### 1. Control Loop Latency Under API Load
**File**: `stress_test.rs::test_control_loop_latency_under_api_load`

Simulates 50 concurrent API clients making requests while measuring control loop latency.

**Success Criteria**: Control loop latency must stay < 1 second

### 2. Concurrent Read/Write Operations
**File**: `stress_test.rs::test_concurrent_read_write_operations`

Spawns 20 readers and 10 writers to test for deadlocks and data corruption.

**Success Criteria**: All operations complete without errors or panics

### 3. Memory Stability Under Load
**File**: `stress_test.rs::test_memory_stability_under_load`

Runs continuous operations for 10 seconds to check for memory leaks.

**Success Criteria**: History buffer remains bounded

### 4. Simulated Device Polling
**File**: `stress_test.rs::test_simulated_device_polling`

Simulates 50 devices polling the controller every 10 seconds.

**Success Criteria**: Each poll completes in <500ms

### 5. Throughput Benchmark
**File**: `stress_test.rs::test_throughput_benchmark`

Measures how many operations per second the system can handle.

**Success Criteria**: At least 100 operations/second

## Performance Targets

| Metric | Target | Critical Threshold |
|--------|--------|-------------------|
| Control Loop Latency | < 500ms | < 1s |
| API Response Time | < 100ms | < 500ms |
| Concurrent Clients | 50+ | 20+ |
| Throughput | 200+ ops/s | 100+ ops/s |
| Memory Growth | Stable | < 10MB/hour |

## Adding New Load Tests

1. Create a new test function in `stress_test.rs` or a new file
2. Mark it with `#[ignore]` to exclude from normal test runs
3. Document the test scenario and success criteria
4. Update this README with the new test

Example:
```rust
#[tokio::test]
#[ignore]
async fn test_my_load_scenario() {
    // Test implementation
}
```

## Troubleshooting

### Tests timeout
Increase the timeout or reduce the load:
```bash
RUST_TEST_THREADS=1 cargo test --test load_tests -- --ignored --test-threads=1
```

### Out of memory
Reduce the number of concurrent clients or test duration in the test code.

### Flaky tests
Load tests can be sensitive to system load. Run on a dedicated test machine or reduce concurrency.
