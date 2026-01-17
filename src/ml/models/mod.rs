//! ML Model Implementations
//!
//! This module contains specialized ML models for various forecasting tasks:
//! - Base models (linear regression, moving average, exponential smoothing)
//! - Solar production forecasting with physics-based features
//! - Price forecasting using time-series models
//! - Consumption prediction

pub mod base;
pub mod solar_production;
pub mod price_lstm;

pub use base::*;
pub use solar_production::*;
pub use price_lstm::*;
