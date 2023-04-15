use std::process;
use tokio_postgres::{Client, Error, NoTls};

pub struct PostgresClient {
    client: Client,
}

impl PostgresClient {
    pub async fn new(host: &str, port: u16, user: &str, db: &str) -> Result<Self, Error> {
        let conn_str = &format!("host={host} port={port} user={user} dbname={db}");
        let (client, connection) = tokio_postgres::connect(conn_str, NoTls).await?;

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                log::error!("Failed to connect to the PostgreSQL database: {}", e);
                process::exit(1);
            }
        });

        Ok(PostgresClient { client })
    }

    pub fn client(&self) -> &Client {
        &self.client
    }
}
