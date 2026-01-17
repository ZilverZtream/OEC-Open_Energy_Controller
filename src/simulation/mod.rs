//! # Environment Simulation Module
//!
//! Provides realistic simulation of the physical environment that drives energy system behavior.
//!
//! ## Components
//!
//! - **House**: Household load profiles with time-of-day patterns, random noise, and appliance events
//! - **Solar**: Clear-sky solar radiation model with cloud cover, seasonal variations, and geographic location
//! - **Grid**: Grid frequency/voltage fluctuations, fuse limits, and fault conditions
//! - **Environment**: Master orchestrator that ticks the simulation clock and updates all components
//!
//! ## Usage
//!
//! ```rust
//! use energy_controller::simulation::{Environment, EnvironmentConfig};
//!
//! let config = EnvironmentConfig {
//!     latitude: 59.3293,  // Stockholm
//!     longitude: 18.0686,
//!     timezone_offset: 1,
//!     household_size: 4,
//!     solar_capacity_kw: 8.0,
//!     ..Default::default()
//! };
//!
//! let mut env = Environment::new(config);
//!
//! // Advance simulation by 1 hour
//! env.tick(std::time::Duration::from_secs(3600));
//!
//! // Get current environmental state
//! let house_load = env.house_load_kw();
//! let solar_production = env.solar_production_kw();
//! let grid_frequency = env.grid_frequency_hz();
//! ```

pub mod battery_thermal;
pub mod environment;
pub mod ev_driver;
pub mod grid;
pub mod house;
pub mod solar;
pub mod three_phase;

pub use battery_thermal::{BatteryThermalConfig, BatteryThermalSimulator, BatteryThermalState};
pub use environment::{Environment, EnvironmentConfig, EnvironmentState};
pub use ev_driver::{EVDriverConfig, EVDriverSimulator, EVDriverState, EVState};
pub use grid::{GridSimulator, GridSimulatorConfig, GridState};
pub use house::{HouseSimulator, HouseSimulatorConfig, HouseState, LoadProfile};
pub use solar::{ClearSkyModel, CloudCover, SolarSimulator, SolarSimulatorConfig, SolarState};
pub use three_phase::{
    LoadDistribution, ThreePhaseCurrent, ThreePhaseGridState, ThreePhasePower,
    ThreePhaseSimulator, ThreePhaseVoltage,
};
