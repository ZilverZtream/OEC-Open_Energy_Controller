use crate::config::Config;
use anyhow::Result;

pub struct Repositories {
    #[cfg(feature = "db")]
    pub db: crate::repo::pg::PgRepo,
}

impl Repositories {
    pub async fn new(cfg: &Config) -> Result<Self> {
        #[cfg(feature = "db")]
        {
            return Ok(Self {
                db: crate::repo::pg::PgRepo::connect(&cfg.db.url).await?,
            });
        }

        #[cfg(not(feature = "db"))]
        {
            let _ = cfg;
            return Ok(Self {});
        }
    }
}

#[cfg(feature = "db")]
pub mod pg;
