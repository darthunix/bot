use anyhow::Result;
use deadpool_postgres::{Client, Manager, ManagerConfig, Pool, RecyclingMethod};
use std::env;
use tokio_postgres::{Config, NoTls};

pub const DEFAULT_POOL_CAPACITY: usize = 10;

pub fn config_from_env() -> Result<Config> {
    let mut pg_config = tokio_postgres::Config::new();
    let host = env::var("PG_HOST").unwrap_or_else(|_| "localhost".to_string());
    let port = env::var("PG_PORT").unwrap_or_else(|_| "5432".to_string());
    let user = env::var("PG_USER").unwrap_or_else(|_| "postgres".to_string());
    let database = env::var("PG_DATABASE").unwrap_or_else(|_| "bot".to_string());

    pg_config.host(&host);
    pg_config.port(port.parse()?);
    pg_config.user(&user);
    pg_config.dbname(&database);

    Ok(pg_config)
}

pub fn capacity_from_env() -> usize {
    let capacity =
        env::var("PG_POOL_CAPACITY").unwrap_or_else(|_| DEFAULT_POOL_CAPACITY.to_string());
    capacity.parse().unwrap_or(DEFAULT_POOL_CAPACITY)
}

pub struct PgPool {
    pool: Pool,
}

impl PgPool {
    pub fn new(pg_config: Config, capacity: usize) -> Result<PgPool> {
        let mgr_config = ManagerConfig {
            recycling_method: RecyclingMethod::Fast,
        };
        let mgr = Manager::from_config(pg_config, NoTls, mgr_config);
        let pool = Pool::builder(mgr).max_size(capacity).build()?;
        Ok(PgPool { pool })
    }

    pub async fn get(&self) -> Result<Client> {
        let client = self.pool.get().await?;
        Ok(client)
    }
}
