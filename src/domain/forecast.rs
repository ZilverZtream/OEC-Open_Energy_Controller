use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};

/// Forecast confidence level
#[cfg_attr(feature = "swagger", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ForecastConfidence {
    High,   // > 90% accuracy expected
    Medium, // 70-90% accuracy expected
    Low,    // < 70% accuracy expected
}

impl std::fmt::Display for ForecastConfidence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::High => write!(f, "high"),
            Self::Medium => write!(f, "medium"),
            Self::Low => write!(f, "low"),
        }
    }
}

impl ForecastConfidence {
    /// Get a numerical confidence score (0.0 - 1.0)
    pub fn as_score(&self) -> f64 {
        match self {
            Self::High => 0.95,
            Self::Medium => 0.80,
            Self::Low => 0.65,
        }
    }

    /// Create from a numerical accuracy value (0.0 - 1.0)
    pub fn from_accuracy(accuracy: f64) -> Self {
        if accuracy >= 0.9 {
            Self::High
        } else if accuracy >= 0.7 {
            Self::Medium
        } else {
            Self::Low
        }
    }
}

/// Optimization objective
#[cfg_attr(feature = "swagger", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OptimizationObjective {
    /// Minimize total energy cost
    MinimizeCost,
    /// Maximize battery arbitrage profits
    MaximizeArbitrage,
    /// Maximize self-consumption of solar energy
    MaximizeSelfConsumption,
    /// Maintain minimum battery state of charge
    MaintainReserve,
    /// Peak shaving - reduce maximum grid import
    PeakShaving,
    /// Load following - match consumption patterns
    LoadFollowing,
    /// Multi-objective with weighted priorities
    MultiObjective,
}

impl std::fmt::Display for OptimizationObjective {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::MinimizeCost => "minimize_cost",
            Self::MaximizeArbitrage => "maximize_arbitrage",
            Self::MaximizeSelfConsumption => "maximize_self_consumption",
            Self::MaintainReserve => "maintain_reserve",
            Self::PeakShaving => "peak_shaving",
            Self::LoadFollowing => "load_following",
            Self::MultiObjective => "multi_objective",
        };
        write!(f, "{}", s)
    }
}

/// Optimization constraints
#[cfg_attr(feature = "swagger", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraints {
    /// Minimum battery state of charge (%)
    pub min_soc_percent: f64,

    /// Maximum battery state of charge (%)
    pub max_soc_percent: f64,

    /// Maximum charge power (W)
    pub max_charge_power_w: f64,

    /// Maximum discharge power (W)
    pub max_discharge_power_w: f64,

    /// Maximum charge/discharge cycles per day
    pub max_cycles_per_day: f64,

    /// Grid import limit (W)
    pub max_grid_import_w: Option<f64>,

    /// Grid export limit (W)
    pub max_grid_export_w: Option<f64>,

    /// Minimum grid export power to be worthwhile (W)
    pub min_export_power_w: Option<f64>,

    /// EV charging deadline (if applicable)
    pub ev_deadline: Option<DateTime<FixedOffset>>,

    /// EV minimum charge level by deadline (%)
    pub ev_min_charge_percent: Option<f64>,

    /// Priority loads that must always be met
    pub priority_load_w: Option<f64>,

    /// Allow battery discharge to grid
    pub allow_battery_to_grid: bool,

    /// Force charge during specific hours
    pub force_charge_hours: Vec<u8>,

    /// Avoid discharge during specific hours
    pub avoid_discharge_hours: Vec<u8>,
}

impl Default for Constraints {
    fn default() -> Self {
        Self {
            min_soc_percent: 20.0,
            max_soc_percent: 100.0,
            max_charge_power_w: 5000.0,
            max_discharge_power_w: 5000.0,
            max_cycles_per_day: 2.0,
            max_grid_import_w: Some(16000.0), // 16A @ 230V ~= 3.68 kW
            max_grid_export_w: Some(16000.0),
            min_export_power_w: Some(100.0),
            ev_deadline: None,
            ev_min_charge_percent: None,
            priority_load_w: Some(500.0), // Always keep 500W for essential loads
            allow_battery_to_grid: false,
            force_charge_hours: vec![],
            avoid_discharge_hours: vec![],
        }
    }
}

impl Constraints {
    /// Validate that all constraints are physically possible
    pub fn validate(&self) -> Result<(), String> {
        if self.min_soc_percent < 0.0 || self.min_soc_percent > 100.0 {
            return Err("min_soc_percent must be between 0 and 100".to_string());
        }

        if self.max_soc_percent < 0.0 || self.max_soc_percent > 100.0 {
            return Err("max_soc_percent must be between 0 and 100".to_string());
        }

        if self.min_soc_percent > self.max_soc_percent {
            return Err("min_soc_percent must be <= max_soc_percent".to_string());
        }

        if self.max_charge_power_w <= 0.0 {
            return Err("max_charge_power_w must be positive".to_string());
        }

        if self.max_discharge_power_w <= 0.0 {
            return Err("max_discharge_power_w must be positive".to_string());
        }

        if self.max_cycles_per_day < 0.0 {
            return Err("max_cycles_per_day must be non-negative".to_string());
        }

        for hour in &self.force_charge_hours {
            if *hour >= 24 {
                return Err(format!("Invalid hour in force_charge_hours: {}", hour));
            }
        }

        for hour in &self.avoid_discharge_hours {
            if *hour >= 24 {
                return Err(format!("Invalid hour in avoid_discharge_hours: {}", hour));
            }
        }

        Ok(())
    }

    /// Check if a given SoC is within constraints
    pub fn is_soc_valid(&self, soc_percent: f64) -> bool {
        soc_percent >= self.min_soc_percent && soc_percent <= self.max_soc_percent
    }

    /// Check if a given charge power is within constraints
    pub fn is_charge_power_valid(&self, power_w: f64) -> bool {
        power_w >= 0.0 && power_w <= self.max_charge_power_w
    }

    /// Check if a given discharge power is within constraints
    pub fn is_discharge_power_valid(&self, power_w: f64) -> bool {
        power_w >= 0.0 && power_w <= self.max_discharge_power_w
    }

    /// Get the available SoC range (min to max)
    pub fn soc_range(&self) -> f64 {
        self.max_soc_percent - self.min_soc_percent
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_forecast_confidence_from_accuracy() {
        assert_eq!(ForecastConfidence::from_accuracy(0.95), ForecastConfidence::High);
        assert_eq!(ForecastConfidence::from_accuracy(0.85), ForecastConfidence::Medium);
        assert_eq!(ForecastConfidence::from_accuracy(0.60), ForecastConfidence::Low);
    }

    #[test]
    fn test_forecast_confidence_as_score() {
        assert_eq!(ForecastConfidence::High.as_score(), 0.95);
        assert_eq!(ForecastConfidence::Medium.as_score(), 0.80);
        assert_eq!(ForecastConfidence::Low.as_score(), 0.65);
    }

    #[test]
    fn test_optimization_objective_display() {
        assert_eq!(OptimizationObjective::MinimizeCost.to_string(), "minimize_cost");
        assert_eq!(OptimizationObjective::MaximizeArbitrage.to_string(), "maximize_arbitrage");
    }

    #[test]
    fn test_constraints_default() {
        let constraints = Constraints::default();
        assert_eq!(constraints.min_soc_percent, 20.0);
        assert_eq!(constraints.max_soc_percent, 100.0);
        assert!(constraints.validate().is_ok());
    }

    #[test]
    fn test_constraints_validate_invalid_soc() {
        let mut constraints = Constraints::default();
        constraints.min_soc_percent = 150.0;
        assert!(constraints.validate().is_err());

        constraints.min_soc_percent = 20.0;
        constraints.max_soc_percent = 10.0;
        assert!(constraints.validate().is_err());
    }

    #[test]
    fn test_constraints_validate_invalid_power() {
        let mut constraints = Constraints::default();
        constraints.max_charge_power_w = -1000.0;
        assert!(constraints.validate().is_err());

        constraints.max_charge_power_w = 5000.0;
        constraints.max_discharge_power_w = 0.0;
        assert!(constraints.validate().is_err());
    }

    #[test]
    fn test_constraints_validate_invalid_hours() {
        let mut constraints = Constraints::default();
        constraints.force_charge_hours = vec![0, 12, 25];
        assert!(constraints.validate().is_err());

        constraints.force_charge_hours = vec![0, 12, 23];
        assert!(constraints.validate().is_ok());
    }

    #[test]
    fn test_constraints_is_soc_valid() {
        let constraints = Constraints::default();
        assert!(constraints.is_soc_valid(50.0));
        assert!(constraints.is_soc_valid(20.0));
        assert!(constraints.is_soc_valid(100.0));
        assert!(!constraints.is_soc_valid(10.0));
        assert!(!constraints.is_soc_valid(110.0));
    }

    #[test]
    fn test_constraints_is_charge_power_valid() {
        let constraints = Constraints::default();
        assert!(constraints.is_charge_power_valid(2500.0));
        assert!(constraints.is_charge_power_valid(0.0));
        assert!(constraints.is_charge_power_valid(5000.0));
        assert!(!constraints.is_charge_power_valid(6000.0));
        assert!(!constraints.is_charge_power_valid(-100.0));
    }

    #[test]
    fn test_constraints_soc_range() {
        let constraints = Constraints::default();
        assert_eq!(constraints.soc_range(), 80.0); // 100 - 20
    }
}
