/// Repository pattern implementations for database access
///
/// This module provides repository interfaces for accessing different types of data:
/// - Price: Electricity price data with CRUD operations
/// - Consumption: Household energy consumption with aggregations
/// - Production: Solar/renewable energy production with statistics

pub mod consumption;
pub mod price;
pub mod production;

pub use consumption::{
    ConsumptionRepository, ConsumptionRow, ConsumptionStatistics, DailyAverage as ConsumptionDailyAverage,
    HourlyAverage as ConsumptionHourlyAverage, HourlyPattern as ConsumptionHourlyPattern,
};
pub use price::{PriceRepository, PriceStatistics};
pub use production::{
    DailyAverage as ProductionDailyAverage, DailyTotal, HourlyAverage as ProductionHourlyAverage,
    HourlyPattern as ProductionHourlyPattern, MonthlySummary, ProductionRepository, ProductionRow,
    ProductionStatistics,
};
