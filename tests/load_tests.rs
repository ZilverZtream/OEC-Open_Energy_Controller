//! Load Test Runner
//!
//! This file makes the load tests discoverable by cargo test.
//!
//! To run load tests:
//! ```bash
//! cargo test --test load_tests -- --ignored --test-threads=1
//! ```
//!
//! Note: Load tests are marked as #[ignore] by default to avoid
//! running them during normal CI builds. Use --ignored to run them.

mod load;
