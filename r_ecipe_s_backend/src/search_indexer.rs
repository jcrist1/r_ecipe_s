use futures::stream::TryStreamExt;
use futures_util::StreamExt;
// use futures_util::TryStreamExt;
use meilisearch_sdk::client::Client;
use meilisearch_sdk::errors::Error as MeiliError;
use meilisearch_sdk::indexes::Index;
use qdrant_client::{
    prelude::{QdrantClient, QdrantClientConfig},
    qdrant::{
        vectors_config::Config, CreateCollection, Distance, PointId, PointStruct, VectorParams,
        VectorsConfig,
    },
};
use sqlx::postgres::{PgListener, PgNotification};
use std::{collections::HashMap, num::ParseIntError, sync::Arc, time::Duration};
use thiserror::Error as ThisError;
use tracing::log::{debug, error, info};

use crate::{
    app_config::{SearchConfig, VectorSearchConfig},
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
    #[error("Failed to parse i64 id. Check notification query in db. {0}")]
    NotificationError(#[from] ParseIntError),
    #[error("Qdrant error: {0}")]
    Qdrant(String),
}

impl Error {
    fn qdrant(err: anyhow::Error) -> Error {
        Error::Qdrant(format!("{err}"))
    }
}

type Result<T> = std::result::Result<T, Error>;

pub(crate) const R_ECIPE_S_INDEX_NAME: &str = "r_ecipe_s";
pub(crate) const RECIPES_VEC_COLLECTION_NAME: &str = "recipes";

#[derive(Clone)]
struct SearchIndexer {
    search_client: Client,
    vector_client: Arc<QdrantClient>,
    recipe_access: Arc<RecipeAccess>,
}
impl SearchIndexer {
    fn new(
        search_config: SearchConfig,
        vector_search_config: QdrantClientConfig,
        recipe_access: Arc<RecipeAccess>,
    ) -> Self {
        let vector_client =
            Arc::new(QdrantClient::new(Some(vector_search_config)).expect("Failed to do it"));
        let url = search_config.http_url();
        info!("Search URL: {url}");
        let search_client = Client::new(url, Some(search_config.api_key));
        SearchIndexer {
            search_client,
            vector_client,
            recipe_access,
        }
    }
}

#[derive(Clone)]
struct CanIndex {
    indexer: SearchIndexer,
    index: Index,
}
use qdrant_client::prelude::Value;

impl CanIndex {
    async fn index(&self, id: i64) -> Result<()> {
        println!("INDEX");
        let (recipe, transaction) = self.indexer.recipe_access.get_by_id_for_update(id).await?;
        let Some(mut recipe) = recipe else {
            return Ok(());
        };
        let embedding = recipe.data.embedding.take();

        // let recipe_payload = serde_json:: recipe.data.
        if let Some(embedding) = embedding {
            info!("Vector time");
            let point = PointStruct {
                id: Some(PointId::from(recipe.id as u64)),
                payload: [
                    ("name".to_owned(), Value::from(recipe.data.name.clone())),
                    (
                        "description".into(),
                        Value::from(recipe.data.description.clone()),
                    ),
                ]
                .into_iter()
                .collect::<HashMap<_, _>>(),
                vectors: Some(embedding.into()),
            };
            self.indexer
                .vector_client
                .upsert_points(RECIPES_VEC_COLLECTION_NAME, [point].to_vec(), None)
                .await
                // todo: delete
                .map_err(Error::qdrant)?;
            debug!("Successful indexing");
        } else {
            debug!("No embedding");
        }
        let recipe_arr = [recipe];
        self.index
            .add_or_update(&recipe_arr, Some("id"))
            .await?
            .wait_for_completion(&self.indexer.search_client, None, None)
            .await?;
        self.indexer
            .recipe_access
            .set_batch_searchable(transaction, recipe_arr.iter().map(|recipe| &recipe.id))
            .await?;
        Ok(())
    }
}

pub async fn process_notification(can_index: &CanIndex, not: PgNotification) -> Result<()> {
    let payload = not.payload();
    info!("Payload: {payload}");
    let id: i64 = payload.parse()?;
    can_index.index(id).await?;
    Ok(())
}

pub async fn index_loop(
    search_config: SearchConfig,
    vector_search_config: VectorSearchConfig,
    recipe_access: Arc<RecipeAccess>,
) -> Result<()> {
    let uri = format!(
        "http://{host}:{port}",
        host = vector_search_config.host,
        port = vector_search_config.port
    );
    info!("{uri}");
    let vector_search_config = QdrantClientConfig::from_url(&uri);
    println!("Starting background job");
    let indexer = SearchIndexer::new(search_config, vector_search_config, recipe_access);
    let index = indexer.search_client.index(R_ECIPE_S_INDEX_NAME);
    if !indexer
        .vector_client
        .has_collection(RECIPES_VEC_COLLECTION_NAME)
        .await
        .map_err(Error::qdrant)?
    {
        indexer
            .vector_client
            .create_collection(&CreateCollection {
                collection_name: RECIPES_VEC_COLLECTION_NAME.into(),
                vectors_config: Some(VectorsConfig {
                    config: Some(Config::Params(VectorParams {
                        size: 384,
                        distance: Distance::Cosine as i32,
                        ..Default::default()
                    })),
                }),
                ..Default::default()
            })
            .await
            .map_err(Error::qdrant)?;
        info!("Success");
    }

    let can_index = CanIndex { index, indexer };
    let mut listener =
        PgListener::connect("postgresql://r_ecipe_s_user:secret123@localhost/r-ecipe-s").await?;
    listener.listen("search_index").await?;
    listener
        .into_stream()
        .map(|res| res.map_err(Error::from))
        .try_fold(can_index, |can_index, not| async move {
            process_notification(&can_index, not).await?;
            Ok(can_index)
        })
        .await?;
    Ok(())
}
