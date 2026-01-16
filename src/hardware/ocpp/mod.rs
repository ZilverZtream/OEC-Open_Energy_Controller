//! # OCPP Hardware Implementations
//!
//! Hardware implementations that use the OCPP 1.6 protocol.

pub mod ev_charger;

pub use ev_charger::{OcppEvCharger, OcppEvChargerConfig};
