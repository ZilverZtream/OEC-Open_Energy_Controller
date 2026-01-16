#![allow(dead_code)]
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::Constraints;
use crate::domain::{BatteryState, Forecast24h, Schedule};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemState {
    pub battery: BatteryState,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Action {
    Charge,
    Discharge,
    Idle,
}

#[async_trait]
pub trait OptimizationStrategy: Send + Sync {
    async fn optimize(
        &self,
        state: &SystemState,
        forecast: &Forecast24h,
        constraints: &Constraints,
    ) -> Result<Schedule>;
}

pub struct BatteryOptimizer {
    pub strategy: Box<dyn OptimizationStrategy>,
}

impl BatteryOptimizer {
    pub async fn optimize(
        &self,
        state: &SystemState,
        forecast: &Forecast24h,
        constraints: &Constraints,
    ) -> Result<Schedule> {
        self.strategy.optimize(state, forecast, constraints).await
    }
}
