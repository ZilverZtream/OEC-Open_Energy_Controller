#![allow(dead_code)]
use anyhow::{Context, Result};
use figment::{
    providers::{Env, Format, Toml},
    Figment,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::PathBuf;
use validator::Validate;

/// Top-level application configuration
#[derive(Debug, Clone, Deserialize, Serialize, Validate)]
pub struct AppConfig {
    #[validate(nested)]
    pub server: ServerConfig,

    #[validate(nested)]
    pub auth: AuthConfig,

    #[validate(nested)]
    pub controller: ControllerConfig,

    #[validate(nested)]
    pub battery: BatteryConfig,

    #[validate(nested)]
    pub hardware: HardwareConfig,

    #[validate(nested)]
    pub database: DatabaseConfig,

    #[validate(nested)]
    pub optimization: OptimizationConfig,

    #[validate(nested)]
    pub forecast: ForecastConfig,

    #[validate(nested)]
    pub telemetry: TelemetryConfig,

    #[validate(nested)]
    pub prices: PricesConfig,
}

/// HTTP server configuration
#[derive(Debug, Clone, Deserialize, Serialize, Validate)]
pub struct ServerConfig {
    #[validate(length(min = 1))]
    pub host: String,

    #[validate(range(min = 1, max = 65535))]
    pub port: u16,

    #[serde(default = "default_max_connections")]
    pub max_connections: usize,

    #[serde(default = "default_request_timeout_secs")]
    pub request_timeout_secs: u64,

    #[serde(default)]
    pub enable_cors: bool,

    #[serde(default)]
    pub enable_compression: bool,

    #[serde(default)]
    pub tls_cert_path: Option<PathBuf>,

    #[serde(default)]
    pub tls_key_path: Option<PathBuf>,
}

impl ServerConfig {
    pub fn socket_addr(&self) -> Result<SocketAddr> {
        format!("{}:{}", self.host, self.port)
            .parse()
            .context("Failed to parse socket address")
    }
}

/// Authentication configuration
#[derive(Debug, Clone, Deserialize, Serialize, Validate)]
pub struct AuthConfig {
    #[validate(length(min = 32))]
    pub token: String,

    #[serde(default = "default_token_expiry_hours")]
    pub token_expiry_hours: u64,

    #[serde(default)]
    pub enable_jwt: bool,

    #[serde(default)]
    pub jwt_secret: Option<String>,
}

/// Controller loop configuration
#[derive(Debug, Clone, Deserialize, Serialize, Validate)]
pub struct ControllerConfig {
    #[validate(range(min = 1, max = 3600))]
    pub tick_seconds: u64,

    #[validate(range(min = 1, max = 1440))]
    pub reoptimize_every_minutes: u64,

    #[validate(length(min = 1))]
    pub default_area: String,

    #[validate(range(min = 1, max = 168))]
    pub default_horizon_hours: u32,

    #[serde(default = "default_max_retries")]
    pub max_retries: u32,

    #[serde(default = "default_retry_delay_ms")]
    pub retry_delay_ms: u64,
}

/// Battery configuration
#[derive(Debug, Clone, Deserialize, Serialize, Validate)]
#[validate(schema(function = "validate_battery_config"))]
pub struct BatteryConfig {
    #[validate(range(min = 0.1, max = 1000.0))]
    pub capacity_kwh: f64,

    #[validate(range(min = 0.0, max = 100.0))]
    pub initial_soc_percent: f64,

    #[validate(range(min = 0.1, max = 100.0))]
    pub max_charge_kw: f64,

    #[validate(range(min = 0.1, max = 100.0))]
    pub max_discharge_kw: f64,

    #[validate(range(min = 0.5, max = 1.0))]
    pub efficiency: f64,

    #[validate(range(min = 0.0, max = 1.0))]
    pub degradation_per_cycle: f64,

    #[serde(default = "default_min_soc")]
    #[validate(range(min = 0.0, max = 100.0))]
    pub min_soc_percent: f64,

    #[serde(default = "default_max_soc")]
    #[validate(range(min = 0.0, max = 100.0))]
    pub max_soc_percent: f64,

    /// Battery replacement cost (SEK) - used for cycle penalty calculation
    #[serde(default = "default_battery_replacement_cost")]
    #[validate(range(min = 0.0, max = 1000000.0))]
    pub replacement_cost_sek: f64,
}

/// Hardware sensor fallback configuration
#[derive(Debug, Clone, Deserialize, Serialize, Validate)]
pub struct SensorFallbackConfig {
    /// Default PV production when sensor unavailable (kW)
    #[serde(default = "default_pv_production_kw")]
    #[validate(range(min = 0.0, max = 100.0))]
    pub default_pv_production_kw: f64,

    /// Default house load when sensor unavailable (kW)
    #[serde(default = "default_house_load_kw")]
    #[validate(range(min = 0.0, max = 100.0))]
    pub default_house_load_kw: f64,
}

/// Custom validation for BatteryConfig
fn validate_battery_config(config: &BatteryConfig) -> Result<(), validator::ValidationError> {
    // Validate min_soc < max_soc
    if config.min_soc_percent >= config.max_soc_percent {
        return Err(validator::ValidationError::new("min_soc must be less than max_soc"));
    }

    // Validate initial_soc is within min/max range
    if config.initial_soc_percent < config.min_soc_percent
        || config.initial_soc_percent > config.max_soc_percent
    {
        return Err(validator::ValidationError::new(
            "initial_soc_percent must be between min_soc_percent and max_soc_percent",
        ));
    }

    Ok(())
}

/// Hardware abstraction configuration
#[derive(Debug, Clone, Deserialize, Serialize, Validate)]
pub struct HardwareConfig {
    #[serde(default = "default_hardware_mode")]
    pub mode: HardwareMode,

    #[serde(default)]
    pub modbus: Option<ModbusConfig>,

    #[serde(default)]
    pub ocpp: Option<OcppConfig>,

    #[serde(default = "default_scan_interval_secs")]
    pub scan_interval_secs: u64,

    #[serde(default)]
    pub enable_discovery: bool,

    #[serde(default)]
    #[validate(nested)]
    pub sensor_fallback: SensorFallbackConfig,
}

impl Default for SensorFallbackConfig {
    fn default() -> Self {
        Self {
            default_pv_production_kw: default_pv_production_kw(),
            default_house_load_kw: default_house_load_kw(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum HardwareMode {
    Simulated,
    Modbus,
    Mock,
}

/// Modbus configuration
#[derive(Debug, Clone, Deserialize, Serialize, Validate)]
pub struct ModbusConfig {
    #[validate(range(min = 1, max = 65535))]
    pub default_port: u16,

    #[validate(range(min = 1, max = 247))]
    pub default_unit_id: u8,

    #[validate(range(min = 100, max = 30000))]
    pub timeout_ms: u64,

    #[validate(range(min = 0, max = 10))]
    pub max_retries: u32,

    #[serde(default)]
    pub scan_enabled: bool,

    #[serde(default)]
    pub scan_ranges: Vec<String>,
}

/// OCPP configuration
#[derive(Debug, Clone, Deserialize, Serialize, Validate)]
pub struct OcppConfig {
    #[validate(url)]
    pub server_url: String,

    #[validate(length(min = 1))]
    pub charge_point_id: String,

    #[validate(range(min = 5, max = 300))]
    pub heartbeat_interval_secs: u64,

    #[serde(default)]
    pub enable_tls: bool,
}

/// Database configuration
#[derive(Debug, Clone, Deserialize, Serialize, Validate)]
pub struct DatabaseConfig {
    #[validate(length(min = 1))]
    pub url: String,

    #[serde(default = "default_db_max_connections")]
    #[validate(range(min = 1, max = 100))]
    pub max_connections: u32,

    #[serde(default = "default_db_min_connections")]
    #[validate(range(min = 1, max = 100))]
    pub min_connections: u32,

    #[serde(default = "default_db_timeout_secs")]
    pub connect_timeout_secs: u64,

    #[serde(default = "default_db_idle_timeout_secs")]
    pub idle_timeout_secs: u64,

    #[serde(default)]
    pub enable_statement_logging: bool,
}

/// Optimization engine configuration
#[derive(Debug, Clone, Deserialize, Serialize, Validate)]
pub struct OptimizationConfig {
    #[serde(default = "default_optimization_strategy")]
    pub strategy: OptimizationStrategy,

    #[validate(range(min = 1, max = 168))]
    pub horizon_hours: u32,

    #[validate(range(min = 1, max = 60))]
    pub time_step_minutes: u32,

    #[serde(default = "default_max_iterations")]
    pub max_iterations: u32,

    #[serde(default = "default_convergence_threshold")]
    pub convergence_threshold: f64,

    #[serde(default)]
    pub enable_parallel: bool,

    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OptimizationStrategy {
    Greedy,
    DynamicProgramming,
    Milp,
    Mpc,
    ReinforcementLearning,
}

/// Forecasting configuration
#[derive(Debug, Clone, Deserialize, Serialize, Validate)]
pub struct ForecastConfig {
    #[serde(default)]
    pub use_ml_models: bool,

    #[validate(range(min = 1, max = 168))]
    pub horizon_hours: u32,

    #[serde(default = "default_update_interval_hours")]
    pub update_interval_hours: u32,

    #[serde(default)]
    pub weather: Option<WeatherConfig>,

    #[serde(default)]
    pub price: Option<PriceForecastConfig>,

    #[serde(default = "default_cache_ttl_secs")]
    pub cache_ttl_secs: u64,
}

/// Weather API configuration
#[derive(Debug, Clone, Deserialize, Serialize, Validate)]
pub struct WeatherConfig {
    #[validate(url)]
    pub api_url: String,

    #[serde(default)]
    pub api_key: Option<String>,

    #[validate(range(min = -90.0, max = 90.0))]
    pub latitude: f64,

    #[validate(range(min = -180.0, max = 180.0))]
    pub longitude: f64,

    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,
}

/// Price forecast configuration
#[derive(Debug, Clone, Deserialize, Serialize, Validate)]
pub struct PriceForecastConfig {
    #[validate(length(min = 1))]
    pub provider: String,

    #[validate(url)]
    pub base_url: String,

    #[serde(default)]
    pub api_key: Option<String>,

    #[validate(range(min = 1, max = 300))]
    pub http_timeout_secs: u64,
}

/// Telemetry and observability configuration
#[derive(Debug, Clone, Deserialize, Serialize, Validate)]
pub struct TelemetryConfig {
    #[serde(default = "default_log_level")]
    pub log_level: String,

    #[serde(default)]
    pub log_json: bool,

    #[serde(default)]
    pub log_file: Option<PathBuf>,

    #[serde(default)]
    pub enable_metrics: bool,

    #[serde(default = "default_metrics_port")]
    pub metrics_port: u16,

    #[serde(default)]
    pub enable_tracing: bool,

    #[serde(default)]
    pub tracing_endpoint: Option<String>,
}

/// Price API configuration (legacy, kept for backward compatibility)
#[derive(Debug, Clone, Deserialize, Serialize, Validate)]
pub struct PricesConfig {
    #[validate(length(min = 1))]
    pub provider: String,

    #[validate(url)]
    pub base_url: String,

    #[validate(range(min = 1, max = 300))]
    pub http_timeout_seconds: u64,

    #[serde(default = "default_cache_ttl_seconds")]
    pub cache_ttl_seconds: u64,
}

// Default value functions
fn default_max_connections() -> usize { 1000 }
fn default_request_timeout_secs() -> u64 { 30 }
fn default_token_expiry_hours() -> u64 { 24 }
fn default_max_retries() -> u32 { 3 }
fn default_retry_delay_ms() -> u64 { 1000 }
fn default_min_soc() -> f64 { 10.0 }
fn default_max_soc() -> f64 { 95.0 }
fn default_battery_replacement_cost() -> f64 { 50000.0 } // 50k SEK typical for home battery
fn default_pv_production_kw() -> f64 { 0.0 } // Conservative: assume no PV if sensor unavailable
fn default_house_load_kw() -> f64 { 2.0 } // Typical household base load
fn default_hardware_mode() -> HardwareMode { HardwareMode::Simulated }
fn default_scan_interval_secs() -> u64 { 300 }
fn default_db_max_connections() -> u32 { 10 }
fn default_db_min_connections() -> u32 { 2 }
fn default_db_timeout_secs() -> u64 { 30 }
fn default_db_idle_timeout_secs() -> u64 { 600 }
fn default_optimization_strategy() -> OptimizationStrategy { OptimizationStrategy::DynamicProgramming }
fn default_max_iterations() -> u32 { 1000 }
fn default_convergence_threshold() -> f64 { 0.001 }
fn default_timeout_secs() -> u64 { 300 }
fn default_update_interval_hours() -> u32 { 1 }
fn default_cache_ttl_secs() -> u64 { 3600 }
fn default_cache_ttl_seconds() -> u64 { 3600 }
fn default_log_level() -> String { "info".to_string() }
fn default_metrics_port() -> u16 { 9090 }

impl AppConfig {
    /// Load configuration from TOML files and environment variables
    ///
    /// Configuration is loaded in this order (later overrides earlier):
    /// 1. config/default.toml (base configuration)
    /// 2. config/development.toml or config/production.toml (environment-specific)
    /// 3. Environment variables with OEC__ prefix
    ///
    /// # Example
    ///
    /// ```no_run
    /// use open_energy_controller::config::AppConfig;
    ///
    /// let config = AppConfig::load().expect("Failed to load config");
    /// println!("Server will listen on {}:{}", config.server.host, config.server.port);
    /// ```
    pub fn load() -> Result<Self> {
        Self::load_with_env(None)
    }

    /// Load configuration with a specific environment override
    pub fn load_with_env(environment: Option<&str>) -> Result<Self> {
        let mut figment = Figment::new()
            .merge(Toml::file("config/default.toml"));

        // Load environment-specific config if provided
        if let Some(env) = environment {
            let env_file = format!("config/{}.toml", env);
            figment = figment.merge(Toml::file(env_file));
        } else {
            // Try to load development.toml by default
            figment = figment.merge(Toml::file("config/development.toml").nested());
        }

        // Override with environment variables (OEC__SERVER__PORT -> server.port)
        figment = figment.merge(Env::prefixed("OEC__").split("__"));

        let config: AppConfig = figment
            .extract()
            .context("Failed to parse configuration")?;

        // Validate configuration
        config.validate()
            .context("Configuration validation failed")?;

        Ok(config)
    }

    /// Validate configuration without loading from files
    pub fn validate_config(self) -> Result<Self> {
        self.validate()
            .context("Configuration validation failed")?;
        Ok(self)
    }
}

// Keep backward compatibility with old Config name
pub type Config = AppConfig;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_config_socket_addr() {
        let config = ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 8080,
            max_connections: 1000,
            request_timeout_secs: 30,
            enable_cors: false,
            enable_compression: true,
            tls_cert_path: None,
            tls_key_path: None,
        };

        let addr = config.socket_addr().unwrap();
        assert_eq!(addr.port(), 8080);
    }

    #[test]
    fn test_battery_config_validation() {
        let config = BatteryConfig {
            capacity_kwh: 10.0,
            initial_soc_percent: 50.0,
            max_charge_kw: 5.0,
            max_discharge_kw: 5.0,
            efficiency: 0.95,
            degradation_per_cycle: 0.0001,
            min_soc_percent: 10.0,
            max_soc_percent: 95.0,
        };

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_battery_config_invalid_soc() {
        let config = BatteryConfig {
            capacity_kwh: 10.0,
            initial_soc_percent: 150.0, // Invalid: > 100
            max_charge_kw: 5.0,
            max_discharge_kw: 5.0,
            efficiency: 0.95,
            degradation_per_cycle: 0.0001,
            min_soc_percent: 10.0,
            max_soc_percent: 95.0,
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_hardware_mode_deserialization() {
        let json = r#"{"mode": "simulated"}"#;
        let parsed: serde_json::Value = serde_json::from_str(json).unwrap();
        let mode: HardwareMode = serde_json::from_value(parsed["mode"].clone()).unwrap();

        matches!(mode, HardwareMode::Simulated);
    }

    #[test]
    fn test_optimization_strategy_deserialization() {
        let strategies = vec![
            ("greedy", OptimizationStrategy::Greedy),
            ("dynamic_programming", OptimizationStrategy::DynamicProgramming),
            ("milp", OptimizationStrategy::Milp),
            ("mpc", OptimizationStrategy::Mpc),
        ];

        for (name, expected) in strategies {
            let json = format!(r#"{{"strategy": "{}"}}"#, name);
            let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
            let strategy: OptimizationStrategy = serde_json::from_value(parsed["strategy"].clone()).unwrap();

            // Compare as strings since we can't derive PartialEq
            assert_eq!(
                format!("{:?}", strategy),
                format!("{:?}", expected)
            );
        }
    }
}
