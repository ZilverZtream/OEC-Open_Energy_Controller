use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "swagger", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum PriceArea { SE1, SE2, SE3, SE4 }

impl std::fmt::Display for PriceArea {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self { Self::SE1 => "SE1", Self::SE2 => "SE2", Self::SE3 => "SE3", Self::SE4 => "SE4" };
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
