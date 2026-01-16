use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Electricity price database row
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ElectricityPriceRow {
    pub id: i64,
    pub timestamp: DateTime<Utc>,
    pub price_sek_per_kwh: f64,
    pub source: String,
    pub area: String,
}

impl ElectricityPriceRow {
    /// Create a new electricity price row
    pub fn new(timestamp: DateTime<Utc>, price: f64, source: &str, area: &str) -> Self {
        Self {
            id: 0, // Will be set by database
            timestamp,
            price_sek_per_kwh: price,
            source: source.to_string(),
            area: area.to_string(),
        }
    }

    /// Check if this price is from Nordpool
    pub fn is_nordpool(&self) -> bool {
        self.source.to_lowercase().contains("nordpool")
    }

    /// Get the price in öre per kWh
    pub fn price_ore_per_kwh(&self) -> f64 {
        self.price_sek_per_kwh * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_electricity_price_row_creation() {
        let now = Utc::now();
        let price = ElectricityPriceRow::new(now, 1.5, "nordpool", "SE3");

        assert_eq!(price.price_sek_per_kwh, 1.5);
        assert_eq!(price.source, "nordpool");
        assert_eq!(price.area, "SE3");
    }

    #[test]
    fn test_is_nordpool() {
        let price = ElectricityPriceRow {
            id: 1,
            timestamp: Utc::now(),
            price_sek_per_kwh: 1.0,
            source: "nordpool".to_string(),
            area: "SE3".to_string(),
        };

        assert!(price.is_nordpool());

        let price2 = ElectricityPriceRow {
            id: 2,
            timestamp: Utc::now(),
            price_sek_per_kwh: 1.0,
            source: "manual".to_string(),
            area: "SE3".to_string(),
        };

        assert!(!price2.is_nordpool());
    }

    #[test]
    fn test_price_ore_conversion() {
        let price = ElectricityPriceRow {
            id: 1,
            timestamp: Utc::now(),
            price_sek_per_kwh: 2.0,
            source: "nordpool".to_string(),
            area: "SE3".to_string(),
        };

        assert_eq!(price.price_ore_per_kwh(), 200.0);
    }
}
