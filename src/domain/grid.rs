use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Grid connection status and limits
#[cfg_attr(feature = "swagger", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridConnection {
    pub status: GridStatus,
    pub import_power_w: f64,
    pub export_power_w: f64,
    pub frequency_hz: f64,
    pub voltage_v: f64,
    pub current_a: f64,
}

#[cfg_attr(feature = "swagger", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum GridStatus {
    Normal,
    Blackout,
    Islanded,
    Unstable,
    Reconnecting,
}

#[cfg_attr(feature = "swagger", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridLimits {
    pub fuse_rating_amps: f64,
    pub max_import_kw: f64,
    pub max_export_kw: f64,
    pub voltage_min_v: f64,
    pub voltage_max_v: f64,
    pub frequency_min_hz: f64,
    pub frequency_max_hz: f64,
}

impl GridLimits {
    /// Default grid limits for Swedish 3-phase 25A connection
    pub fn default_se_25a() -> Self {
        Self {
            fuse_rating_amps: 25.0,
            max_import_kw: 17.25, // 25A * 230V * 3 phases / 1000
            max_export_kw: 17.25,
            voltage_min_v: 207.0, // -10% of 230V
            voltage_max_v: 253.0, // +10% of 230V
            frequency_min_hz: 49.5,
            frequency_max_hz: 50.5,
        }
    }

    /// Default grid limits for Swedish 3-phase 16A connection
    pub fn default_se_16a() -> Self {
        Self {
            fuse_rating_amps: 16.0,
            max_import_kw: 11.04, // 16A * 230V * 3 phases / 1000
            max_export_kw: 11.04,
            voltage_min_v: 207.0,
            voltage_max_v: 253.0,
            frequency_min_hz: 49.5,
            frequency_max_hz: 50.5,
        }
    }

    /// Check if current power draw is within limits
    pub fn is_within_limits(&self, import_w: f64, export_w: f64) -> bool {
        let import_kw = import_w / 1000.0;
        let export_kw = export_w / 1000.0;

        import_kw <= self.max_import_kw && export_kw <= self.max_export_kw
    }

    /// Calculate headroom for additional power draw (in watts)
    pub fn import_headroom_w(&self, current_import_w: f64) -> f64 {
        (self.max_import_kw * 1000.0 - current_import_w).max(0.0)
    }

    /// Calculate headroom for additional power export (in watts)
    pub fn export_headroom_w(&self, current_export_w: f64) -> f64 {
        (self.max_export_kw * 1000.0 - current_export_w).max(0.0)
    }
}

/// Time-of-use tariff structure
#[cfg_attr(feature = "swagger", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridTariff {
    pub fixed_cost_sek_per_month: f64,
    pub energy_tax_sek_per_kwh: f64,
    pub vat_percent: f64,
    pub peak_hour_start: u8,
    pub peak_hour_end: u8,
    pub peak_price_multiplier: f64,
}

impl GridTariff {
    /// Typical Swedish grid tariff
    pub fn default_se() -> Self {
        Self {
            fixed_cost_sek_per_month: 250.0,
            energy_tax_sek_per_kwh: 0.42,
            vat_percent: 25.0,
            peak_hour_start: 6,
            peak_hour_end: 22,
            peak_price_multiplier: 1.2,
        }
    }

    /// Calculate total cost including taxes and VAT
    pub fn calculate_cost(&self, kwh: f64, base_price_sek_per_kwh: f64, hour: u8) -> f64 {
        let is_peak = hour >= self.peak_hour_start && hour < self.peak_hour_end;
        let price = if is_peak {
            base_price_sek_per_kwh * self.peak_price_multiplier
        } else {
            base_price_sek_per_kwh
        };

        let subtotal = kwh * (price + self.energy_tax_sek_per_kwh);
        subtotal * (1.0 + self.vat_percent / 100.0)
    }
}

/// Aggregated grid import/export statistics over a time window.
#[cfg_attr(feature = "swagger", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridStatistics {
    /// Total imported energy in kWh.
    pub total_import_kwh: f64,
    /// Total exported energy in kWh.
    pub total_export_kwh: f64,
    /// Average import power in kW.
    pub average_import_kw: f64,
    /// Average export power in kW.
    pub average_export_kw: f64,
    /// Number of samples included.
    pub sample_count: u32,
    /// Start of the statistics window.
    pub window_start: DateTime<Utc>,
    /// End of the statistics window.
    pub window_end: DateTime<Utc>,
}

impl GridStatistics {
    /// Returns net imported energy (import - export) in kWh.
    pub fn net_import_kwh(&self) -> f64 {
        self.total_import_kwh - self.total_export_kwh
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_limits_within_bounds() {
        let limits = GridLimits::default_se_25a();

        // Within limits
        assert!(limits.is_within_limits(15000.0, 0.0));

        // Over import limit
        assert!(!limits.is_within_limits(20000.0, 0.0));

        // Over export limit
        assert!(!limits.is_within_limits(0.0, 20000.0));
    }

    #[test]
    fn test_headroom_calculation() {
        let limits = GridLimits::default_se_16a();

        // Max is 11.04 kW = 11,040W
        let headroom = limits.import_headroom_w(5000.0);
        assert_eq!(headroom, 6040.0);

        // No headroom when at limit
        let headroom = limits.import_headroom_w(11040.0);
        assert_eq!(headroom, 0.0);
    }

    #[test]
    fn test_tariff_cost_calculation() {
        let tariff = GridTariff::default_se();

        // Off-peak hour (no multiplier)
        let cost = tariff.calculate_cost(1.0, 1.0, 2);
        // (1.0 + 0.42) * 1.25 = 1.775 SEK
        assert!((cost - 1.775).abs() < 0.001);

        // Peak hour (with multiplier)
        let cost = tariff.calculate_cost(1.0, 1.0, 10);
        // (1.0 * 1.2 + 0.42) * 1.25 = 2.025 SEK
        assert!((cost - 2.025).abs() < 0.001);
    }

    #[test]
    fn test_grid_statistics_net_import() {
        let stats = GridStatistics {
            total_import_kwh: 12.5,
            total_export_kwh: 3.0,
            average_import_kw: 1.2,
            average_export_kw: 0.4,
            sample_count: 10,
            window_start: Utc::now(),
            window_end: Utc::now(),
        };

        assert!((stats.net_import_kwh() - 9.5).abs() < f64::EPSILON);
    }
}
