use chrono::{DateTime, FixedOffset, Datelike, Timelike};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{Add, Sub, Mul, Div};

// ============================================================================
// Time Helper Types
// ============================================================================

/// Duration helper type for time intervals
/// Wraps chrono::Duration with convenience methods
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Duration(pub chrono::Duration);

impl Duration {
    /// Create a duration from seconds
    pub fn seconds(secs: i64) -> Self {
        Self(chrono::Duration::seconds(secs))
    }

    /// Create a duration from minutes
    pub fn minutes(mins: i64) -> Self {
        Self(chrono::Duration::minutes(mins))
    }

    /// Create a duration from hours
    pub fn hours(hours: i64) -> Self {
        Self(chrono::Duration::hours(hours))
    }

    /// Create a duration from days
    pub fn days(days: i64) -> Self {
        Self(chrono::Duration::days(days))
    }

    /// Get the duration in seconds
    pub fn as_seconds(&self) -> i64 {
        self.0.num_seconds()
    }

    /// Get the duration in minutes
    pub fn as_minutes(&self) -> i64 {
        self.0.num_minutes()
    }

    /// Get the duration in hours
    pub fn as_hours(&self) -> i64 {
        self.0.num_hours()
    }

    /// Get the duration in hours as f64
    pub fn as_hours_f64(&self) -> f64 {
        self.0.num_seconds() as f64 / 3600.0
    }

    /// Get the duration in days
    pub fn as_days(&self) -> i64 {
        self.0.num_days()
    }

    /// Get the inner chrono::Duration
    pub fn inner(&self) -> chrono::Duration {
        self.0
    }
}

impl From<chrono::Duration> for Duration {
    fn from(d: chrono::Duration) -> Self {
        Self(d)
    }
}

impl From<Duration> for chrono::Duration {
    fn from(d: Duration) -> Self {
        d.0
    }
}

impl fmt::Display for Duration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let hours = self.as_hours();
        let minutes = (self.as_seconds() % 3600) / 60;
        write!(f, "{}h{}m", hours, minutes)
    }
}

/// Timestamp helper type for specific points in time
/// Wraps DateTime<FixedOffset> with convenience methods
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Timestamp(pub DateTime<FixedOffset>);

impl Timestamp {
    /// Create a timestamp from the current time
    pub fn now() -> Self {
        Self(chrono::Utc::now().fixed_offset())
    }

    /// Create a timestamp from a Unix timestamp (seconds since epoch)
    pub fn from_unix(secs: i64) -> Option<Self> {
        chrono::DateTime::from_timestamp(secs, 0)
            .map(|dt| Self(dt.fixed_offset()))
    }

    /// Get the Unix timestamp (seconds since epoch)
    pub fn as_unix(&self) -> i64 {
        self.0.timestamp()
    }

    /// Add a duration to this timestamp
    pub fn add(&self, duration: Duration) -> Self {
        Self(self.0 + duration.0)
    }

    /// Subtract a duration from this timestamp
    pub fn sub(&self, duration: Duration) -> Self {
        Self(self.0 - duration.0)
    }

    /// Calculate the duration between two timestamps
    pub fn duration_since(&self, other: &Timestamp) -> Duration {
        Duration(self.0 - other.0)
    }

    /// Check if this timestamp is before another
    pub fn is_before(&self, other: &Timestamp) -> bool {
        self.0 < other.0
    }

    /// Check if this timestamp is after another
    pub fn is_after(&self, other: &Timestamp) -> bool {
        self.0 > other.0
    }

    /// Get the hour of the day (0-23)
    pub fn hour(&self) -> u32 {
        self.0.hour()
    }

    /// Get the day of the month (1-31)
    pub fn day(&self) -> u32 {
        self.0.day()
    }

    /// Get the month (1-12)
    pub fn month(&self) -> u32 {
        self.0.month()
    }

    /// Get the year
    pub fn year(&self) -> i32 {
        self.0.year()
    }

    /// Format as ISO 8601 string
    pub fn to_rfc3339(&self) -> String {
        self.0.to_rfc3339()
    }

    /// Get the inner DateTime
    pub fn inner(&self) -> DateTime<FixedOffset> {
        self.0
    }
}

impl From<DateTime<FixedOffset>> for Timestamp {
    fn from(dt: DateTime<FixedOffset>) -> Self {
        Self(dt)
    }
}

impl From<Timestamp> for DateTime<FixedOffset> {
    fn from(ts: Timestamp) -> Self {
        ts.0
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.format("%Y-%m-%d %H:%M:%S %Z"))
    }
}

// ============================================================================
// Physical Unit Newtypes
// ============================================================================

/// Power in Watts (W)
#[cfg_attr(feature = "swagger", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, PartialOrd)]
pub struct Power(pub f64);

impl Power {
    pub fn watts(w: f64) -> Self {
        Self(w)
    }

    pub fn kilowatts(kw: f64) -> Self {
        Self(kw * 1000.0)
    }

    pub fn as_watts(&self) -> f64 {
        self.0
    }

    pub fn as_kilowatts(&self) -> f64 {
        self.0 / 1000.0
    }
}

impl std::fmt::Display for Power {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.abs() >= 1000.0 {
            write!(f, "{:.2} kW", self.as_kilowatts())
        } else {
            write!(f, "{:.1} W", self.0)
        }
    }
}

impl Add for Power {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl Sub for Power {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

/// Energy in Watt-hours (Wh)
#[cfg_attr(feature = "swagger", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, PartialOrd)]
pub struct Energy(pub f64);

impl Energy {
    pub fn watt_hours(wh: f64) -> Self {
        Self(wh)
    }

    pub fn kilowatt_hours(kwh: f64) -> Self {
        Self(kwh * 1000.0)
    }

    pub fn as_watt_hours(&self) -> f64 {
        self.0
    }

    pub fn as_kilowatt_hours(&self) -> f64 {
        self.0 / 1000.0
    }
}

impl std::fmt::Display for Energy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.abs() >= 1000.0 {
            write!(f, "{:.2} kWh", self.as_kilowatt_hours())
        } else {
            write!(f, "{:.1} Wh", self.0)
        }
    }
}

impl Add for Energy {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl Sub for Energy {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

/// Voltage in Volts (V)
#[cfg_attr(feature = "swagger", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, PartialOrd)]
pub struct Voltage(pub f64);

impl Voltage {
    pub fn volts(v: f64) -> Self {
        Self(v)
    }

    pub fn as_volts(&self) -> f64 {
        self.0
    }
}

impl std::fmt::Display for Voltage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.1} V", self.0)
    }
}

/// Current in Amperes (A)
#[cfg_attr(feature = "swagger", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, PartialOrd)]
pub struct Current(pub f64);

impl Current {
    pub fn amperes(a: f64) -> Self {
        Self(a)
    }

    pub fn as_amperes(&self) -> f64 {
        self.0
    }
}

impl std::fmt::Display for Current {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.1} A", self.0)
    }
}

/// Temperature in Celsius (°C)
#[cfg_attr(feature = "swagger", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, PartialOrd)]
pub struct Temperature(pub f64);

impl Temperature {
    pub fn celsius(c: f64) -> Self {
        Self(c)
    }

    pub fn fahrenheit(f: f64) -> Self {
        Self((f - 32.0) * 5.0 / 9.0)
    }

    pub fn kelvin(k: f64) -> Self {
        Self(k - 273.15)
    }

    pub fn as_celsius(&self) -> f64 {
        self.0
    }

    pub fn as_fahrenheit(&self) -> f64 {
        self.0 * 9.0 / 5.0 + 32.0
    }

    pub fn as_kelvin(&self) -> f64 {
        self.0 + 273.15
    }
}

impl std::fmt::Display for Temperature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.1}°C", self.0)
    }
}

/// Percentage (0-100%)
#[cfg_attr(feature = "swagger", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, PartialOrd)]
pub struct Percentage(pub f64);

impl Percentage {
    pub fn new(value: f64) -> Self {
        Self(value.clamp(0.0, 100.0))
    }

    pub fn from_ratio(ratio: f64) -> Self {
        Self((ratio * 100.0).clamp(0.0, 100.0))
    }

    pub fn as_percent(&self) -> f64 {
        self.0
    }

    pub fn as_ratio(&self) -> f64 {
        self.0 / 100.0
    }
}

impl std::fmt::Display for Percentage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.1}%", self.0)
    }
}

/// Price in SEK per kilowatt-hour (SEK/kWh)
#[cfg_attr(feature = "swagger", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, PartialOrd)]
pub struct Price(pub f64);

impl Price {
    pub fn sek_per_kwh(price: f64) -> Self {
        Self(price)
    }

    pub fn ore_per_kwh(price: f64) -> Self {
        Self(price / 100.0)
    }

    pub fn as_sek_per_kwh(&self) -> f64 {
        self.0
    }

    pub fn as_ore_per_kwh(&self) -> f64 {
        self.0 * 100.0
    }
}

impl std::fmt::Display for Price {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.2} SEK/kWh", self.0)
    }
}

impl Mul<Energy> for Price {
    type Output = f64; // Cost in SEK
    fn mul(self, energy: Energy) -> Self::Output {
        self.0 * energy.as_kilowatt_hours()
    }
}

// ============================================================================
// Geographic and Market Types
// ============================================================================

#[cfg_attr(feature = "swagger", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum PriceArea {
    SE1,
    SE2,
    SE3,
    SE4,
}

impl std::fmt::Display for PriceArea {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::SE1 => "SE1",
            Self::SE2 => "SE2",
            Self::SE3 => "SE3",
            Self::SE4 => "SE4",
        };
        write!(f, "{s}")
    }
}
impl std::str::FromStr for PriceArea {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "SE1" => Ok(Self::SE1),
            "SE2" => Ok(Self::SE2),
            "SE3" => Ok(Self::SE3),
            "SE4" => Ok(Self::SE4),
            _ => Err("invalid area; expected SE1..SE4"),
        }
    }
}

#[cfg_attr(feature = "swagger", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricePoint {
    pub time_start: DateTime<FixedOffset>,
    pub time_end: DateTime<FixedOffset>,
    pub price_sek_per_kwh: f64,
}

#[cfg_attr(feature = "swagger", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsumptionPoint {
    pub time_start: DateTime<FixedOffset>,
    pub time_end: DateTime<FixedOffset>,
    pub load_kw: f64,
}

#[cfg_attr(feature = "swagger", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductionPoint {
    pub time_start: DateTime<FixedOffset>,
    pub time_end: DateTime<FixedOffset>,
    pub pv_kw: f64,
}

#[cfg_attr(feature = "swagger", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Forecast24h {
    pub area: PriceArea,
    pub generated_at: DateTime<FixedOffset>,
    pub prices: Vec<PricePoint>,
    pub consumption: Vec<ConsumptionPoint>,
    pub production: Vec<ProductionPoint>,
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_power_conversions() {
        let power = Power::kilowatts(5.0);
        assert_eq!(power.as_watts(), 5000.0);
        assert_eq!(power.as_kilowatts(), 5.0);

        let power2 = Power::watts(2500.0);
        assert_eq!(power2.as_kilowatts(), 2.5);
    }

    #[test]
    fn test_power_arithmetic() {
        let p1 = Power::kilowatts(3.0);
        let p2 = Power::kilowatts(2.0);

        let sum = p1 + p2;
        assert_eq!(sum.as_kilowatts(), 5.0);

        let diff = p1 - p2;
        assert_eq!(diff.as_kilowatts(), 1.0);
    }

    #[test]
    fn test_power_display() {
        let p1 = Power::watts(500.0);
        assert_eq!(format!("{}", p1), "500.0 W");

        let p2 = Power::kilowatts(5.5);
        assert_eq!(format!("{}", p2), "5.50 kW");
    }

    #[test]
    fn test_energy_conversions() {
        let energy = Energy::kilowatt_hours(10.0);
        assert_eq!(energy.as_watt_hours(), 10000.0);
        assert_eq!(energy.as_kilowatt_hours(), 10.0);

        let energy2 = Energy::watt_hours(5000.0);
        assert_eq!(energy2.as_kilowatt_hours(), 5.0);
    }

    #[test]
    fn test_energy_arithmetic() {
        let e1 = Energy::kilowatt_hours(10.0);
        let e2 = Energy::kilowatt_hours(3.0);

        let sum = e1 + e2;
        assert_eq!(sum.as_kilowatt_hours(), 13.0);

        let diff = e1 - e2;
        assert_eq!(diff.as_kilowatt_hours(), 7.0);
    }

    #[test]
    fn test_voltage() {
        let voltage = Voltage::volts(400.0);
        assert_eq!(voltage.as_volts(), 400.0);
        assert_eq!(format!("{}", voltage), "400.0 V");
    }

    #[test]
    fn test_current() {
        let current = Current::amperes(16.0);
        assert_eq!(current.as_amperes(), 16.0);
        assert_eq!(format!("{}", current), "16.0 A");
    }

    #[test]
    fn test_temperature_conversions() {
        let temp = Temperature::celsius(25.0);
        assert_eq!(temp.as_celsius(), 25.0);
        assert!((temp.as_fahrenheit() - 77.0).abs() < 0.1);
        assert!((temp.as_kelvin() - 298.15).abs() < 0.1);

        let temp_f = Temperature::fahrenheit(77.0);
        assert!((temp_f.as_celsius() - 25.0).abs() < 0.1);

        let temp_k = Temperature::kelvin(298.15);
        assert!((temp_k.as_celsius() - 25.0).abs() < 0.1);
    }

    #[test]
    fn test_percentage() {
        let pct = Percentage::new(75.0);
        assert_eq!(pct.as_percent(), 75.0);
        assert_eq!(pct.as_ratio(), 0.75);

        let pct2 = Percentage::from_ratio(0.5);
        assert_eq!(pct2.as_percent(), 50.0);

        // Test clamping
        let pct3 = Percentage::new(150.0);
        assert_eq!(pct3.as_percent(), 100.0);

        let pct4 = Percentage::new(-10.0);
        assert_eq!(pct4.as_percent(), 0.0);
    }

    #[test]
    fn test_price() {
        let price = Price::sek_per_kwh(1.5);
        assert_eq!(price.as_sek_per_kwh(), 1.5);
        assert_eq!(price.as_ore_per_kwh(), 150.0);

        let price2 = Price::ore_per_kwh(200.0);
        assert_eq!(price2.as_sek_per_kwh(), 2.0);
    }

    #[test]
    fn test_price_energy_multiplication() {
        let price = Price::sek_per_kwh(2.0);
        let energy = Energy::kilowatt_hours(10.0);
        let cost = price * energy;
        assert_eq!(cost, 20.0); // 20 SEK
    }

    #[test]
    fn test_price_area_parsing() {
        use std::str::FromStr;

        assert_eq!(PriceArea::from_str("SE1").unwrap(), PriceArea::SE1);
        assert_eq!(PriceArea::from_str("se2").unwrap(), PriceArea::SE2);
        assert_eq!(PriceArea::from_str("SE3").unwrap(), PriceArea::SE3);
        assert!(PriceArea::from_str("SE5").is_err());
    }

    #[test]
    fn test_serialization() {
        let power = Power::kilowatts(5.0);
        let json = serde_json::to_string(&power).unwrap();
        let deserialized: Power = serde_json::from_str(&json).unwrap();
        assert_eq!(power, deserialized);

        let energy = Energy::kilowatt_hours(10.0);
        let json = serde_json::to_string(&energy).unwrap();
        let deserialized: Energy = serde_json::from_str(&json).unwrap();
        assert_eq!(energy, deserialized);
    }
}
