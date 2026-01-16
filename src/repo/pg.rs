#![cfg(feature="db")]
use anyhow::Result;
use sqlx::{postgres::PgPoolOptions, PgPool};

pub struct PgRepo { pub pool: PgPool }

impl PgRepo {
    pub async fn connect(url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new().max_connections(10).connect(url).await?;
        Ok(Self { pool })
    }
}
