use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraints {
    pub min_soc_percent: f64,
    pub max_soc_percent: f64,
    pub max_cycles_per_day: f64,
    pub max_power_grid_kw: f64,
    pub v2g_enabled: bool,
}

impl Default for Constraints {
    fn default() -> Self {
        Self {
            min_soc_percent: 20.0,
            max_soc_percent: 90.0,
            max_cycles_per_day: 1.0,
            max_power_grid_kw: 11.0,
            v2g_enabled: false,
        }
    }
}
