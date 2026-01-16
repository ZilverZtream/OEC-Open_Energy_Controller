//! # Solar Production Simulation
//!
//! Models solar PV production using clear-sky radiation model with cloud cover,
//! seasonal variations, and geographic location.

use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, Timelike};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

/// Cloud cover level affecting solar production
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CloudCover {
    /// Clear sky (0-10% clouds)
    Clear,
    /// Partly cloudy (10-50% clouds)
    PartlyCloudy,
    /// Mostly cloudy (50-90% clouds)
    MostlyCloudy,
    /// Overcast (90-100% clouds)
    Overcast,
}

impl CloudCover {
    /// Get the radiation transmission factor (0.0 = blocked, 1.0 = full)
    pub fn transmission_factor(&self) -> f64 {
        match self {
            CloudCover::Clear => 1.0,
            CloudCover::PartlyCloudy => 0.7,
            CloudCover::MostlyCloudy => 0.4,
            CloudCover::Overcast => 0.15,
        }
    }

    /// Get a random cloud cover (weighted towards clear/partly cloudy)
    pub fn random<R: Rng>(rng: &mut R) -> Self {
        let roll = rng.gen_range(0..100);
        match roll {
            0..=40 => CloudCover::Clear,
            41..=70 => CloudCover::PartlyCloudy,
            71..=85 => CloudCover::MostlyCloudy,
            _ => CloudCover::Overcast,
        }
    }
}

/// Current state of the solar simulation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolarState {
    /// Current production in kW
    pub production_kw: f64,
    /// Clear-sky theoretical production in kW
    pub clear_sky_kw: f64,
    /// Cloud transmission factor (0.0-1.0)
    pub cloud_factor: f64,
    /// Solar elevation angle in degrees
    pub solar_elevation_deg: f64,
    /// Solar azimuth angle in degrees
    pub solar_azimuth_deg: f64,
    /// Day of year (1-365)
    pub day_of_year: u32,
    /// Current cloud cover
    pub cloud_cover: CloudCover,
    /// Timestamp of this state
    pub timestamp: NaiveDateTime,
}

/// Clear-sky solar radiation model
///
/// Implements simplified clear-sky model based on:
/// - Solar position calculation (elevation and azimuth)
/// - Atmospheric attenuation
/// - Seasonal variation
/// - Geographic location
pub struct ClearSkyModel {
    latitude_deg: f64,
    longitude_deg: f64,
    timezone_offset: i32, // Hours from UTC
}

impl ClearSkyModel {
    /// Create a new clear-sky model for a location
    pub fn new(latitude_deg: f64, longitude_deg: f64, timezone_offset: i32) -> Self {
        Self {
            latitude_deg,
            longitude_deg,
            timezone_offset,
        }
    }

    /// Calculate solar position (elevation and azimuth) for given time
    ///
    /// Returns: (elevation_deg, azimuth_deg)
    /// Elevation: angle above horizon (0 = horizon, 90 = directly overhead)
    /// Azimuth: angle from north (0 = north, 90 = east, 180 = south, 270 = west)
    pub fn solar_position(&self, time: NaiveDateTime) -> (f64, f64) {
        let day_of_year = time.ordinal() as f64;
        let hour = time.hour() as f64 + time.minute() as f64 / 60.0;

        // Solar declination (angle between sun and equatorial plane)
        // Varies from -23.45° (winter solstice) to +23.45° (summer solstice)
        let declination_deg = 23.45 * (360.0 / 365.0 * (day_of_year + 284.0) * PI / 180.0).sin();
        let declination_rad = declination_deg * PI / 180.0;
        let latitude_rad = self.latitude_deg * PI / 180.0;

        // Hour angle (angle of sun from solar noon)
        // Solar noon is when sun is highest in sky
        let solar_time = hour + self.longitude_deg / 15.0 - self.timezone_offset as f64;
        let hour_angle_deg = 15.0 * (solar_time - 12.0);
        let hour_angle_rad = hour_angle_deg * PI / 180.0;

        // Solar elevation angle (altitude)
        let elevation_sin = latitude_rad.sin() * declination_rad.sin()
            + latitude_rad.cos() * declination_rad.cos() * hour_angle_rad.cos();
        let elevation_rad = elevation_sin.asin();
        let elevation_deg = elevation_rad * 180.0 / PI;

        // Solar azimuth angle
        let azimuth_cos = (declination_rad.sin() - latitude_rad.sin() * elevation_rad.sin())
            / (latitude_rad.cos() * elevation_rad.cos());
        let mut azimuth_deg = azimuth_cos.acos() * 180.0 / PI;

        // Adjust azimuth for afternoon (sun in western sky)
        if hour_angle_deg > 0.0 {
            azimuth_deg = 360.0 - azimuth_deg;
        }

        (elevation_deg, azimuth_deg)
    }

    /// Calculate clear-sky irradiance in W/m²
    ///
    /// Simplified model that accounts for:
    /// - Solar elevation angle
    /// - Atmospheric attenuation
    /// - Typical clear-sky conditions
    pub fn clear_sky_irradiance(&self, time: NaiveDateTime) -> f64 {
        let (elevation_deg, _) = self.solar_position(time);

        // Sun below horizon = no radiation
        if elevation_deg <= 0.0 {
            return 0.0;
        }

        let elevation_rad = elevation_deg * PI / 180.0;

        // Solar constant (energy at top of atmosphere)
        let solar_constant = 1367.0; // W/m²

        // Air mass (path length through atmosphere relative to zenith)
        // AM = 1/cos(zenith_angle) ≈ 1/sin(elevation_angle) for low elevations
        let air_mass = if elevation_deg > 5.0 {
            1.0 / elevation_rad.sin()
        } else {
            // Approximate for very low angles to avoid singularity
            12.0 - elevation_deg / 5.0
        };

        // Atmospheric attenuation (simplified Kasten-Young formula)
        // Clear-sky transmittance decreases with air mass
        let transmittance = 0.7_f64.powf(air_mass.powf(0.678));

        // Irradiance = solar_constant × transmittance × sin(elevation)
        solar_constant * transmittance * elevation_rad.sin()
    }
}

/// Solar simulator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolarSimulatorConfig {
    /// Installed PV capacity in kW
    pub capacity_kw: f64,
    /// PV panel efficiency (0.0-1.0, typical 0.15-0.22)
    pub panel_efficiency: f64,
    /// System losses (inverter, wiring, etc., typical 0.85)
    pub system_efficiency: f64,
    /// Latitude in degrees (positive = north)
    pub latitude_deg: f64,
    /// Longitude in degrees (positive = east)
    pub longitude_deg: f64,
    /// Timezone offset from UTC in hours
    pub timezone_offset: i32,
    /// Enable dynamic cloud cover simulation
    pub enable_clouds: bool,
    /// Random seed for reproducibility (None = random)
    pub random_seed: Option<u64>,
}

impl Default for SolarSimulatorConfig {
    fn default() -> Self {
        Self {
            capacity_kw: 5.0,
            panel_efficiency: 0.18,
            system_efficiency: 0.85,
            latitude_deg: 59.3293,  // Stockholm
            longitude_deg: 18.0686,
            timezone_offset: 1, // CET (UTC+1)
            enable_clouds: true,
            random_seed: None,
        }
    }
}

/// Simulates solar PV production
pub struct SolarSimulator {
    config: SolarSimulatorConfig,
    clear_sky_model: ClearSkyModel,
    current_state: SolarState,
    rng: rand::rngs::StdRng,
    cloud_duration_minutes: i64, // How long current cloud condition persists
}

impl SolarSimulator {
    /// Create a new solar simulator
    pub fn new(config: SolarSimulatorConfig, start_time: NaiveDateTime) -> Self {
        use rand::SeedableRng;

        let clear_sky_model = ClearSkyModel::new(
            config.latitude_deg,
            config.longitude_deg,
            config.timezone_offset,
        );

        let rng = match config.random_seed {
            Some(seed) => rand::rngs::StdRng::seed_from_u64(seed),
            None => rand::rngs::StdRng::from_entropy(),
        };

        let mut simulator = Self {
            config,
            clear_sky_model,
            current_state: SolarState {
                production_kw: 0.0,
                clear_sky_kw: 0.0,
                cloud_factor: 1.0,
                solar_elevation_deg: 0.0,
                solar_azimuth_deg: 0.0,
                day_of_year: start_time.ordinal(),
                cloud_cover: CloudCover::Clear,
                timestamp: start_time,
            },
            rng,
            cloud_duration_minutes: 0,
        };

        simulator.update_state(start_time);
        simulator
    }

    /// Get current production in kW
    pub fn production_kw(&self) -> f64 {
        self.current_state.production_kw
    }

    /// Get current state snapshot
    pub fn state(&self) -> &SolarState {
        &self.current_state
    }

    /// Update simulation to a new timestamp
    pub fn tick(&mut self, new_time: NaiveDateTime) {
        self.update_state(new_time);
    }

    /// Update cloud cover if needed
    fn update_cloud_cover(&mut self, time: NaiveDateTime) {
        if !self.config.enable_clouds {
            return;
        }

        let minutes_elapsed = (time - self.current_state.timestamp).num_minutes();
        self.cloud_duration_minutes -= minutes_elapsed;

        // Change cloud cover when duration expires
        if self.cloud_duration_minutes <= 0 {
            self.current_state.cloud_cover = CloudCover::random(&mut self.rng);

            // Cloud conditions persist for 30-180 minutes
            self.cloud_duration_minutes = self.rng.gen_range(30..=180);
        }
    }

    /// Update the solar state for the given time
    fn update_state(&mut self, time: NaiveDateTime) {
        // Update cloud cover
        self.update_cloud_cover(time);

        // Calculate solar position
        let (elevation_deg, azimuth_deg) = self.clear_sky_model.solar_position(time);

        // Calculate clear-sky irradiance (W/m²)
        let irradiance_wm2 = self.clear_sky_model.clear_sky_irradiance(time);

        // Convert to power output (kW)
        // Power = Irradiance × Area × Panel_Eff × System_Eff
        // But we specify capacity directly, so:
        // Power = (Irradiance / 1000 W/m²) × Capacity × System_Eff
        let clear_sky_kw = (irradiance_wm2 / 1000.0) * self.config.capacity_kw
            * self.config.system_efficiency;

        // Apply cloud factor
        let cloud_factor = self.current_state.cloud_cover.transmission_factor();
        let production_kw = (clear_sky_kw * cloud_factor).max(0.0);

        self.current_state = SolarState {
            production_kw,
            clear_sky_kw,
            cloud_factor,
            solar_elevation_deg: elevation_deg,
            solar_azimuth_deg: azimuth_deg,
            day_of_year: time.ordinal(),
            cloud_cover: self.current_state.cloud_cover,
            timestamp: time,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_cloud_cover_transmission() {
        assert_eq!(CloudCover::Clear.transmission_factor(), 1.0);
        assert_eq!(CloudCover::PartlyCloudy.transmission_factor(), 0.7);
        assert_eq!(CloudCover::MostlyCloudy.transmission_factor(), 0.4);
        assert_eq!(CloudCover::Overcast.transmission_factor(), 0.15);
    }

    #[test]
    fn test_solar_position_noon() {
        // Stockholm on summer solstice at solar noon
        let model = ClearSkyModel::new(59.3293, 18.0686, 1);
        let time = NaiveDate::from_ymd_opt(2024, 6, 21)
            .unwrap()
            .and_hms_opt(13, 12, 0)
            .unwrap(); // ~13:12 is solar noon in Stockholm

        let (elevation, azimuth) = model.solar_position(time);

        // Sun should be high (near 54°) and roughly south (180°)
        assert!(elevation > 50.0 && elevation < 60.0);
        assert!(azimuth > 160.0 && azimuth < 200.0);
    }

    #[test]
    fn test_solar_position_midnight() {
        // Stockholm at midnight (sun below horizon)
        let model = ClearSkyModel::new(59.3293, 18.0686, 1);
        let time = NaiveDate::from_ymd_opt(2024, 6, 21)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();

        let (elevation, _) = model.solar_position(time);

        // Sun should be below horizon (but not far, as Stockholm has midnight sun)
        assert!(elevation < 10.0);
    }

    #[test]
    fn test_clear_sky_irradiance() {
        let model = ClearSkyModel::new(59.3293, 18.0686, 1);

        // Summer noon (high irradiance)
        let summer_noon = NaiveDate::from_ymd_opt(2024, 6, 21)
            .unwrap()
            .and_hms_opt(13, 0, 0)
            .unwrap();
        let summer_irradiance = model.clear_sky_irradiance(summer_noon);
        assert!(summer_irradiance > 700.0); // Should be high

        // Winter noon (lower irradiance)
        let winter_noon = NaiveDate::from_ymd_opt(2024, 12, 21)
            .unwrap()
            .and_hms_opt(13, 0, 0)
            .unwrap();
        let winter_irradiance = model.clear_sky_irradiance(winter_noon);
        assert!(winter_irradiance < 300.0); // Should be much lower
        assert!(winter_irradiance > 0.0);

        // Night (no irradiance)
        let night = NaiveDate::from_ymd_opt(2024, 6, 21)
            .unwrap()
            .and_hms_opt(2, 0, 0)
            .unwrap();
        let night_irradiance = model.clear_sky_irradiance(night);
        assert_eq!(night_irradiance, 0.0);
    }

    #[test]
    fn test_solar_simulator_initialization() {
        let config = SolarSimulatorConfig {
            capacity_kw: 5.0,
            enable_clouds: false,
            random_seed: Some(42),
            ..Default::default()
        };

        let start_time = NaiveDate::from_ymd_opt(2024, 6, 21)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();

        let simulator = SolarSimulator::new(config, start_time);
        assert!(simulator.production_kw() >= 0.0);
    }

    #[test]
    fn test_daily_production_cycle() {
        let config = SolarSimulatorConfig {
            capacity_kw: 5.0,
            enable_clouds: false,
            random_seed: Some(42),
            ..Default::default()
        };

        let base_date = NaiveDate::from_ymd_opt(2024, 6, 21).unwrap();
        let mut simulator = SolarSimulator::new(config, base_date.and_hms_opt(0, 0, 0).unwrap());

        let mut productions = Vec::new();
        for hour in 0..24 {
            simulator.tick(base_date.and_hms_opt(hour, 0, 0).unwrap());
            productions.push(simulator.production_kw());
        }

        // Check that production follows expected pattern:
        // - Zero or very low at night (hours 0-5, 21-23)
        // - Peak around noon (hour 12-14)
        let night_production = productions[2]; // 02:00
        let noon_production = productions[12]; // 12:00

        assert!(night_production < 0.1);
        assert!(noon_production > 3.0); // Should be substantial
    }

    #[test]
    fn test_cloud_impact() {
        let config_clear = SolarSimulatorConfig {
            capacity_kw: 5.0,
            enable_clouds: false,
            random_seed: Some(42),
            ..Default::default()
        };

        let time = NaiveDate::from_ymd_opt(2024, 6, 21)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();

        let mut sim_clear = SolarSimulator::new(config_clear, time);
        sim_clear.current_state.cloud_cover = CloudCover::Clear;
        sim_clear.update_state(time);
        let clear_production = sim_clear.production_kw();

        // Force overcast
        sim_clear.current_state.cloud_cover = CloudCover::Overcast;
        sim_clear.update_state(time);
        let overcast_production = sim_clear.production_kw();

        // Overcast should significantly reduce production
        assert!(overcast_production < clear_production * 0.2);
    }

    #[test]
    fn test_seasonal_variation() {
        let config = SolarSimulatorConfig {
            capacity_kw: 5.0,
            enable_clouds: false,
            ..Default::default()
        };

        // Summer production (June 21)
        let summer = NaiveDate::from_ymd_opt(2024, 6, 21)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        let summer_sim = SolarSimulator::new(config.clone(), summer);
        let summer_production = summer_sim.production_kw();

        // Winter production (December 21)
        let winter = NaiveDate::from_ymd_opt(2024, 12, 21)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        let winter_sim = SolarSimulator::new(config, winter);
        let winter_production = winter_sim.production_kw();

        // Summer should produce significantly more than winter
        assert!(summer_production > winter_production * 3.0);
    }
}
