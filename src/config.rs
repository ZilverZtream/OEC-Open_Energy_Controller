use anyhow::Result;
use figment::{providers::{Env, Format, Toml}, Figment};
use serde::Deserialize;
use std::net::SocketAddr;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub auth: AuthConfig,
    pub controller: ControllerConfig,
    pub battery: BatteryConfig,
    pub prices: PricesConfig,
    pub db: DbConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig { pub host: String, pub port: u16 }
impl ServerConfig {
    pub fn socket_addr(&self) -> Result<SocketAddr> {
        Ok(format!("{}:{}", self.host, self.port).parse()?)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthConfig { pub token: String }

#[derive(Debug, Clone, Deserialize)]
pub struct ControllerConfig {
    pub tick_seconds: u64,
    pub reoptimize_every_minutes: u64,
    pub default_area: String,
    pub default_horizon_hours: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BatteryConfig {
    pub capacity_kwh: f64,
    pub initial_soc_percent: f64,
    pub max_charge_kw: f64,
    pub max_discharge_kw: f64,
    pub efficiency: f64,
    pub degradation_per_cycle: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PricesConfig {
    pub provider: String,
    pub base_url: String,
    pub http_timeout_seconds: u64,
    pub cache_ttl_seconds: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DbConfig { pub url: String }

impl Config {
    pub fn load() -> Result<Self> {
        let figment = Figment::new()
            .merge(Toml::file("config/default.toml"))
            .merge(Env::prefixed("OEC__").split("__"));
        Ok(figment.extract()?)
    }
}
