use crate::postgres::PgPool;
use anyhow::Error as AnyError;
use deadpool_postgres::PoolError;
use futures::future::BoxFuture;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Error as SerdeError;
use std::{fmt::Debug, sync::Arc};
use teloxide::{dispatching::dialogue::Storage, types::ChatId};
use thiserror::Error;
use tokio_postgres::Error as PgError;

#[derive(Debug, Error)]
pub enum PgStorageError {
    #[error("any error: {0}")]
    AnyError(#[from] AnyError),

    #[error("postgres error: {0}")]
    PgError(#[from] PgError),

    #[error("pool error: {0}")]
    PoolError(#[from] PoolError),

    #[error("serde error: {0}")]
    SerdeError(#[from] SerdeError),

    #[error("storage error: {0}")]
    StorageError(String),
}

#[allow(clippy::module_name_repetitions)]
pub struct PgStorage {
    pool: PgPool,
}

impl PgStorage {
    #[must_use]
    pub fn new(pool: PgPool) -> Arc<Self> {
        let storage = Self { pool };
        Arc::new(storage)
    }

    #[must_use]
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

impl<D> Storage<D> for PgStorage
where
    D: Serialize + DeserializeOwned + Debug + Send + 'static,
{
    type Error = PgStorageError;

    fn remove_dialogue(
        self: Arc<Self>,
        ChatId(chat_id): ChatId,
    ) -> BoxFuture<'static, Result<(), Self::Error>> {
        Box::pin(async move {
            let client = self.pool.get().await?;
            client
                .query("select api.dialogue_delete($1)", &[&chat_id])
                .await?;
            Ok(())
        })
    }

    fn update_dialogue(
        self: Arc<Self>,
        ChatId(chat_id): ChatId,
        dialogue: D,
    ) -> BoxFuture<'static, Result<(), Self::Error>> {
        Box::pin(async move {
            let client = self.pool.get().await?;
            let data = serde_json::to_string(&dialogue)?;
            log::debug!("Updating dialogue: {}", data);
            client
                .query("select api.dialogue_append($1, $2)", &[&chat_id, &data])
                .await?;
            Ok(())
        })
    }

    fn get_dialogue(
        self: Arc<Self>,
        ChatId(chat_id): ChatId,
    ) -> BoxFuture<'static, Result<Option<D>, Self::Error>> {
        Box::pin(async move {
            let client = self.pool.get().await?;
            let mut rows = client
                .query("select api.dialogue_latest($1)", &[&chat_id])
                .await?;
            log::debug!("Fetched results from the dialog: {:?}", rows);
            let row = match rows.pop() {
                Some(row) => row,
                None => return Ok(None),
            };
            log::debug!("Fetched row from the dialog: {:?}", row);
            let data: &str = match row.try_get::<usize, &str>(0) {
                Ok(data) => data,
                Err(_) => return Ok(None),
            };
            let dialogue: D = serde_json::from_str(data)?;
            log::debug!("Fetched dialogue from the dialog: {:?}", dialogue);
            Ok(Some(dialogue))
        })
    }
}
