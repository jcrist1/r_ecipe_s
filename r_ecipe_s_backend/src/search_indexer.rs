use meilisearch_sdk::client::Client;
use meilisearch_sdk::errors::Error as MeiliError;
use meilisearch_sdk::indexes::Index;
use std::{sync::Arc, time::Duration};
use thiserror::Error as ThisError;
use tracing::log::{debug, error, info};

use crate::{
    app_config::SearchConfig,
    recipe_service::{self, RecipeAccess},
};

#[derive(Debug, ThisError)]
pub enum Error {
    #[error("Meilisearch Error {0}")]
    SearchError(#[from] MeiliError),
    #[error("Transaction Error {0}")]
    TransactionError(#[from] sqlx::Error),
    #[error("Recipe Access Error {0}")]
    RecipeAccess(#[from] recipe_service::Error),
    #[error("Failed insert into Meilisearch")]
    MeiliSearchInserteFailure,
}

type Result<T> = std::result::Result<T, Error>;

const R_ECIPE_S_INDEX_NAME: &str = "r_ecipe_s";

struct SearchIndexer {
    search_client: Client,
    recipe_access: Arc<RecipeAccess>,
}
impl SearchIndexer {
    fn new(search_config: SearchConfig, recipe_access: Arc<RecipeAccess>) -> Self {
        let url = search_config.http_url();
        info!("Search URL: {url}");
        let search_client = Client::new(url, search_config.api_key);
        SearchIndexer {
            search_client,
            recipe_access,
        }
    }
}

struct CanIndex {
    indexer: SearchIndexer,
    index: Index,
}

impl CanIndex {
    async fn transition(&self) -> Result<()> {
        let batch_size = 50;
        let (data_iter, transaction) = self
            .indexer
            .recipe_access
            .get_batch_for_insert(batch_size)
            .await?;
        // println!("Batch: {data_iter:?}");

        if !data_iter.is_empty() {
            let b = self
                .index
                .add_or_update(&data_iter, Some("id"))
                .await?
                .wait_for_completion(&self.indexer.search_client, None, None)
                .await?;
            debug!("inserted: {b:?}");
            if b.is_failure() {
                return Err(Error::MeiliSearchInserteFailure);
            } else {
                self.indexer
                    .recipe_access
                    .set_batch_searchable(transaction, data_iter.iter().map(|id| &id.id))
                    .await?;
            }
        } else {
            transaction.commit().await?;
        }
        Ok(())
    }
}

pub async fn index_loop(
    search_config: SearchConfig,
    recipe_access: Arc<RecipeAccess>,
) -> Result<()> {
    println!("Starting background job");
    let indexer = SearchIndexer::new(search_config, recipe_access);
    let index = indexer.search_client.index(R_ECIPE_S_INDEX_NAME);
    let can_index = CanIndex { index, indexer };
    loop {
        let _: Result<()> = match can_index.transition().await {
            Err(err) => {
                tokio::time::sleep(Duration::from_secs(5)).await;
                error!("ERROR {err:?}");
                Ok(())
            }
            Ok(_res) => {
                tokio::time::sleep(Duration::from_millis(100)).await;
                Ok(())
            }
        };
    }
}
