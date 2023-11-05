use futures::stream::TryStreamExt;
use futures_util::StreamExt;
// use futures_util::TryStreamExt;
use meilisearch_sdk::errors::Error as MeiliError;
use meilisearch_sdk::indexes::Index;
use meilisearch_sdk::{client::Client, MeilisearchError};
use qdrant_client::{
    prelude::{QdrantClient, QdrantClientConfig},
    qdrant::{
        vectors_config::Config, CreateCollection, Distance, PointId, PointStruct, VectorParams,
        VectorsConfig,
    },
};
use sqlx::postgres::{PgListener, PgNotification};
use sqlx::{Connection, PgPool};
use std::fmt::Display;
use std::{collections::HashMap, num::ParseIntError, sync::Arc, time::Duration};
use thiserror::Error as ThisError;
use tracing::log::{debug, error, info};
use tracing::warn;

use crate::db::DbAccess;
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

#[derive(Debug)]
pub struct ContextError {
    err: Error,
    context: Vec<String>,
}
impl Display for ContextError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}, context:\n", self.err)?;
        self.context
            .iter()
            .map(|context| writeln!(f, "{context}"))
            .collect::<std::result::Result<Vec<()>, _>>()?;
        Ok(())
    }
}
impl std::error::Error for ContextError {}
impl ContextError {
    fn more_context<S: ToString>(mut self, message: S) -> Self {
        self.context.push(message.to_string());
        self
    }
}

impl<T> ContextualError<T> for Result<T> {
    fn context<S: ToString>(self, message: S) -> Result<T> {
        self.map_err(|err| err.more_context(message))
    }
}

trait ContextualError<T> {
    fn context<S: ToString>(self, message: S) -> Result<T>;
}

impl<E> From<E> for ContextError
where
    Error: From<E>,
{
    fn from(value: E) -> Self {
        ContextError {
            err: Error::from(value),
            context: vec![],
        }
    }
}

impl<T, E> ContextualError<T> for std::result::Result<T, E>
where
    Error: From<E>,
{
    fn context<S: ToString>(self, message: S) -> Result<T> {
        self.map_err(|err| ContextError {
            err: Error::from(err),
            context: vec![message.to_string()],
        })
    }
}

type Result<T> = std::result::Result<T, ContextError>;

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
        info!("INDEX");
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
            info!("Successful indexing");
        } else {
            info!("No embedding");
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

async fn process_notification(can_index: &CanIndex, not: PgNotification) -> Result<()> {
    let payload = not.payload();
    info!("Payload: {payload}");
    let id: i64 = payload
        .parse::<i64>()
        .context("Failed to parse i64 form notification")?;
    can_index.index(id).await?;
    Ok(())
}

pub async fn index_loop(
    search_config: SearchConfig,
    db_access: Arc<DbAccess>,
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
    let can_index = Arc::new(can_index);
    match can_index
        .indexer
        .search_client
        .get_index(can_index.index.clone())
        .await
    {
        Err(meilisearch_sdk::Error::Meilisearch(MeilisearchError {
            error_code: meilisearch_sdk::ErrorCode::IndexNotFound,
            ..
        })) => {
            warn!(
                "Failed to find search index {}. Recreating",
                can_index.index.uid
            );
            let can_index = Arc::clone(&can_index);
            can_index
                .indexer
                .search_client
                .create_index(
                    can_index.index.uid.clone(),
                    can_index
                        .index
                        .primary_key
                        .clone()
                        .as_ref()
                        .map(AsRef::<str>::as_ref),
                )
                .await?;
        }
        Err(err) => return Err(err.into()),
        Ok(_) => (),
    };
    info!("Creating listener");
    let mut listener = PgListener::connect_with(db_access.get_pool())
        .await
        .context("Failed to create listener")?;
    info!("Subscribing to search_index notification stream");
    listener
        .listen("search_index")
        .await
        .context("Failed to start listening to 'search_index' topic")?;
    info!("Starting listen loop");
    listener
        .into_stream()
        .map(|res| res.map_err(ContextError::from))
        .try_fold(can_index, |can_index, not| async move {
            process_notification(&can_index, not)
                .await
                .context("Failed to process a notification")?;
            Ok(can_index)
        })
        .await?;
    Ok(())
}
