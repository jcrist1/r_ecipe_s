use crate::{
    app_config::{SearchConfig, VectorSearchConfig},
    auth::{AuthError, BearerToken, BearerValidation},
    db::DBAccess,
    search_indexer::{RECIPES_VEC_COLLECTION_NAME, R_ECIPE_S_INDEX_NAME},
};
use axum::{
    body::HttpBody,
    extract::{Path, Query},
    http,
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json as HttpJson, Router,
};
use futures_util::{StreamExt, TryStreamExt};
use qdrant_client::{
    prelude::{QdrantClient, QdrantClientConfig},
    qdrant::{
        point_id::PointIdOptions, with_payload_selector::SelectorOptions, PointId, SearchPoints,
        Value, WithPayloadSelector,
    },
};
use r_ecipe_s_model::{
    serde_json, Ingredient, Recipe, RecipeWithId, RecipesResponse, SearchQuery, SearchResponse,
    SearchResult,
};
use serde::{Deserialize, Serialize};
use sqlx::types::Json;
use sqlx::{types::time::OffsetDateTime, PgExecutor};
use sqlx::{FromRow, Postgres, Transaction};
use tracing::log::{error, info};

use meilisearch_sdk::client::Client;

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use thiserror::Error as ThisError;
const MAX_PAGE_SIZE: i64 = 100;

#[derive(Debug, ThisError)]
pub enum Error {
    #[error("RecipeService failed")]
    Fail,
    #[error("Failed to serialise recipe: Serde Error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("Database Error: {0}")]
    DB(#[from] sqlx::Error),
    #[error("Parse in error: {0}")]
    ParseInt(#[from] std::num::ParseIntError),
    #[error("Missing item of type: {item_type} with id: {id}")]
    Missing { item_type: String, id: i64 },
    #[error("Incorrect page size: {0}. Must be between 1 and 100")]
    IncorrectPageSize(i64),
    #[error("Error with authentication: {0}")]
    Auth(#[from] AuthError),
    #[error("Search error: {0}")]
    Search(#[from] meilisearch_sdk::errors::Error),
    #[error("Resource with Id {0} not found")]
    NotFoundId(i64),
    #[error("Error with vector DB {0}.")]
    Vector(String),
}

impl Error {
    pub(crate) fn qdrant(err: anyhow::Error) -> Error {
        Error::Vector(format!("{err}"))
    }
}

type Result<T> = std::result::Result<T, Error>;
struct RecipeId {
    id: i64,
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let error_code = match self {
            Error::Fail => http::StatusCode::INTERNAL_SERVER_ERROR,
            Error::Serde(_) => http::StatusCode::INTERNAL_SERVER_ERROR,
            Error::DB(_) => http::StatusCode::INTERNAL_SERVER_ERROR,
            Error::ParseInt(_) => http::StatusCode::INTERNAL_SERVER_ERROR,
            Error::Missing { .. } => http::StatusCode::NOT_FOUND,
            Error::IncorrectPageSize(_) => http::StatusCode::BAD_REQUEST,
            Error::Auth(_) => http::StatusCode::UNAUTHORIZED,
            Error::Search(_) => http::StatusCode::INTERNAL_SERVER_ERROR,
            Error::NotFoundId(_) => http::StatusCode::NOT_FOUND,
            Error::Vector(_) => http::StatusCode::INTERNAL_SERVER_ERROR,
        };
        (error_code, format!("{self:?}")).into_response()
    }
}

pub trait RecipeService {
    type ServiceType;
    fn bind_recipe_routes(
        self,
        recipe_access: &Arc<RecipeAccess>,
        search_config: &SearchConfig,
        vector_client: &Arc<QdrantClient>,
        bearer_validation: &Arc<BearerValidation>,
    ) -> Self::ServiceType;
}

impl<T, HttpError, Data> RecipeService for Router<T>
where
    T: HttpBody<Error = HttpError, Data = Data> + Send + 'static,
    HttpError: Sync + Send + std::error::Error + 'static,
    Data: Send + 'static,
{
    type ServiceType = Self;
    fn bind_recipe_routes(
        self,
        recipe_access: &Arc<RecipeAccess>,
        search_config: &SearchConfig,
        vector_client: &Arc<QdrantClient>,
        bearer_validation: &Arc<BearerValidation>,
    ) -> Self::ServiceType {
        let url = search_config.http_url();
        let search_client = Arc::new(Client::new(url, Some(search_config.api_key.clone())));

        self.route(
            "/recipes",
            get({
                let recipe_access = recipe_access.clone();
                |query| get_all(recipe_access, query)
            })
            .put({
                let recipe_access = recipe_access.clone();
                let bearer_validation = bearer_validation.clone();
                |form, bearer_auth| put_recipe(form, bearer_auth, recipe_access, bearer_validation)
            }),
        )
        .route(
            "/recipes/:id",
            delete({
                let recipe_access = recipe_access.clone();
                let bearer_validation = bearer_validation.clone();
                |path, bearer_auth| {
                    delete_recipe(path, bearer_auth, recipe_access, bearer_validation)
                }
            })
            .post({
                let recipe_access = recipe_access.clone();
                let bearer_validation = bearer_validation.clone();
                |path, bearer_auth, form_data| {
                    post_recipe(
                        path,
                        bearer_auth,
                        form_data,
                        recipe_access,
                        bearer_validation,
                    )
                }
            })
            .get({
                let recipe_access = recipe_access.clone();
                |path| get_recipe(path, recipe_access)
            }),
        )
        .route(
            "/recipes/search",
            post({
                let search_client = search_client.clone();
                let vector_client = Arc::clone(vector_client);
                move |search_query, vector| {
                    search_recipe(search_client, vector_client, search_query, vector)
                }
            }),
        )
    }
}

pub struct RecipeAccess {
    db_access: Arc<DBAccess>,
}

#[derive(Serialize, Deserialize, Debug, FromRow)]
pub struct RecipeRep {
    id: i64,
    name: String,
    ingredients: sqlx::types::Json<Vec<Ingredient>>,
    description: String,
    liked: Option<bool>,
    searchable: bool,
    embedding: Option<Vec<f32>>,
}

impl RecipeRep {
    pub fn model(self) -> Recipe {
        Recipe {
            name: self.name,
            ingredients: self.ingredients.0,
            description: self.description,
            liked: self.liked,
            embedding: self.embedding,
        }
    }

    pub fn model_with_id(self) -> RecipeWithId {
        let id = self.id;
        let data = self.model();
        RecipeWithId { id, data }
    }
}

const EMPTY_RECIPE_LIST: &[RecipeWithId] = &[];

impl RecipeAccess {
    pub(crate) async fn get_batch_for_insert(
        &self,
        batch_size: usize,
    ) -> Result<(Vec<RecipeWithId>, sqlx::Transaction<'_, Postgres>)> {
        let mut transaction = self.db_access.get_pool().begin().await?;
        let data_iter = sqlx::query_as!(
            RecipeRep,
            r#"
                SELECT
                    id, 
                    name, 
                    ingredients as "ingredients: Json<Vec<Ingredient>>", 
                    description, 
                    liked,
                    searchable,
                    embedding
                FROM recipes
                WHERE searchable = false
                ORDER BY id 
                LIMIT $1
            "#,
            batch_size as i64
        )
        .fetch(&mut transaction)
        .map(
            |rep_res: std::result::Result<RecipeRep, _>| -> Result<RecipeWithId> {
                let recipe = rep_res?;
                Ok(recipe.model_with_id())
            },
        )
        .try_collect::<Vec<_>>()
        .await?;
        Ok((data_iter, transaction))
    }

    pub(crate) async fn set_batch_searchable<'a>(
        &self,
        executor: Transaction<'a, Postgres>,
        recipe_ids: impl Iterator<Item = &i64>, // ensure the recipe_ids are part of the transaction for update
    ) -> Result<Vec<i64>> {
        // let pool = executor.borrow_mut();
        let (ret_size, _) = recipe_ids.size_hint();
        let updated = Vec::with_capacity(ret_size);
        let new_iter = recipe_ids;
        let (executor, res) = futures::stream::iter(new_iter)
            .fold(
                Ok((executor, updated)),
                |last_res: Result<(Transaction<Postgres>, Vec<_>)>, id| async {
                    let (mut executor, mut updated) = last_res?;
                    let new_res: i64 = sqlx::query_as!(
                        RecipeId,
                        r#"
                            UPDATE recipes
                                SET searchable = true
                            WHERE
                                id = ($1)
                            RETURNING id;
                        "#,
                        *id
                    )
                    .fetch_one(&mut executor) // We are guaranteed to
                    .await
                    .map(|row| row.id)
                    .map_err(Error::from)?;
                    updated.push(new_res);
                    Ok((executor, updated))
                },
            )
            .await?;
        executor.commit().await?;
        Ok(res)
    }

    async fn get_all(&self, page: i64, page_size: i64) -> Result<RecipesResponse> {
        if (page_size <= 0) || (page_size > MAX_PAGE_SIZE) {
            return Err(Error::IncorrectPageSize(page_size));
        }
        let offset = page * page_size;
        let data = sqlx::query_as!(
            RecipeRep,
            r#"
                SELECT
                    id, 
                    name, 
                    ingredients as "ingredients: Json<Vec<Ingredient>>", 
                    description, 
                    liked,
                    searchable,
                    embedding
                FROM recipes
                ORDER BY updated DESC
                OFFSET $1
                LIMIT $2
            "#,
            offset,
            page_size
        )
        .fetch(self.db_access.get_pool())
        .map(
            |rep_res: std::result::Result<RecipeRep, _>| -> Result<RecipeWithId> {
                let recipe = rep_res?;
                Ok(recipe.model_with_id())
            },
        )
        .try_collect::<Vec<_>>()
        .await;
        let count = sqlx::query!(
            r#"
                SELECT GREATEST (((COUNT(id) - 1)::int8 / $1),  0::int8)::int8 as count  FROM recipes;
            "#,
            page_size
        )
        .fetch_one(self.db_access.get_pool())
        .await?
        .count
        .expect("SQL COUNT returned NULL, when counting recipes, which shouldn't be possible");

        let data = data?;
        Ok(RecipesResponse {
            recipes: data,
            total_pages: count,
        })
    }

    async fn update(&self, id: i64, recipe: &Recipe) -> Result<Option<i64>> {
        sqlx::query_as!(
            RecipeId,
            r#"
                UPDATE recipes SET 
                    name = $1,
                    ingredients = $2, 
                    description = $3,
                    liked = $4,
                    updated = $5,
                    searchable = false,
                    embedding = $6
                where id = $7 RETURNING id as "id!: i64"
            "#,
            recipe.name,
            sqlx::types::Json(recipe.ingredients.clone()) as _,
            recipe.description,
            recipe.liked,
            OffsetDateTime::now_utc(),
            (&recipe.embedding)
                .as_ref()
                .map(|arr| <Vec<f32> as AsRef<[f32]>>::as_ref(arr)),
            id,
        )
        .fetch_optional(self.db_access.get_pool())
        .await
        .map(|opt| opt.map(|recipe_id| recipe_id.id))
        .map_err(|err| err.into())
    }

    async fn delete(&self, id: i64) -> Result<()> {
        let now = OffsetDateTime::now_utc();
        sqlx::query!(
            r#"
                DELETE FROM recipes WHERE id = $1
                RETURNING id;
            "#,
            id,
        )
        .fetch_optional(self.db_access.get_pool())
        .await?
        .map(|opt| (opt.id == id).then(|| ()))
        .flatten()
        .ok_or(Error::NotFoundId(id))
    }

    async fn insert(&self, recipe: &Recipe) -> Result<i64> {
        let now = OffsetDateTime::now_utc();
        let rec = sqlx::query!(
            r#"
                INSERT INTO recipes (
                    name,
                    ingredients,
                    description,
                    liked,
                    created,
                    updated,
                    searchable
                ) VALUES (
                    $1,
                    $2,
                    $3,
                    $4,
                    $5,
                    $5,
                    false
                ) RETURNING id
            "#,
            recipe.name,
            sqlx::types::Json(recipe.ingredients.clone()) as _,
            recipe.description,
            recipe.liked,
            now
        )
        .fetch_one(self.db_access.get_pool())
        .await?;
        Ok(rec.id)
    }

    async fn get_by_id_pool<'a, P: PgExecutor<'a>>(
        pool: P,
        id: i64,
    ) -> Result<Option<RecipeWithId>> {
        let ret: Option<RecipeWithId> = sqlx::query_as!(
            RecipeRep,
            r#"
                SELECT
                    id, 
                    name, 
                    ingredients as "ingredients: Json<Vec<Ingredient>>", 
                    description, 
                    liked,
                    searchable,
                    embedding
                FROM recipes
                WHERE id = $1
            "#,
            id,
        )
        .fetch_optional(pool)
        .await?
        .map(|rep| rep.model_with_id());
        Ok(ret)
    }

    pub(crate) async fn get_by_id(&self, id: i64) -> Result<Option<RecipeWithId>> {
        Self::get_by_id_pool(self.db_access.get_pool(), id).await
    }

    pub(crate) async fn get_by_id_for_update(
        &self,
        id: i64,
    ) -> Result<(Option<RecipeWithId>, Transaction<Postgres>)> {
        let mut transaction = self.db_access.get_pool().begin().await?;

        let ret: Option<RecipeWithId> = sqlx::query_as!(
            RecipeRep,
            r#"
                SELECT
                    id, 
                    name, 
                    ingredients as "ingredients: Json<Vec<Ingredient>>", 
                    description, 
                    liked,
                    searchable,
                    embedding
                FROM recipes
                WHERE id = $1
                FOR UPDATE
            "#,
            id
        )
        .fetch_optional(&mut transaction)
        .await?
        .map(|rep| rep.model_with_id());
        Ok((ret, transaction))
    }
}

impl RecipeAccess {
    pub fn new(db_access: &Arc<DBAccess>) -> Self {
        RecipeAccess {
            db_access: Arc::clone(db_access),
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct Paging {
    offset: Option<i64>,
}

const PAGE_SIZE: i64 = 9;

pub(crate) async fn get_all(
    recipe_access: Arc<RecipeAccess>,
    page: Query<Paging>,
) -> Result<HttpJson<RecipesResponse>> {
    let page = page.offset;
    let data = recipe_access.get_all(page.unwrap_or(0), PAGE_SIZE).await?;

    //let body = serde_json::to_string(&data)?;

    Ok(data.into())
}

pub(crate) async fn get_recipe(
    path: Path<i64>,
    recipe_access: Arc<RecipeAccess>,
) -> Result<HttpJson<RecipeWithId>> {
    let id = *path;
    let data_option = recipe_access.get_by_id(id).await?;
    let data = data_option.ok_or_else(|| Error::Missing {
        item_type: "recipe".to_string(),
        id,
    })?;

    Ok(data.into())
}

pub(crate) async fn delete_recipe(
    path: Path<i64>,
    bearer_auth: BearerToken,
    recipe_access: Arc<RecipeAccess>,
    bearer_validation: Arc<BearerValidation>,
) -> Result<HttpJson<()>> {
    bearer_validation.authorise(bearer_auth)?;
    let id = recipe_access.delete(*path).await?;

    Ok(id.into())
}

pub(crate) async fn put_recipe(
    form: HttpJson<Recipe>,
    bearer_auth: BearerToken,
    recipe_access: Arc<RecipeAccess>,
    bearer_validation: Arc<BearerValidation>,
) -> Result<HttpJson<i64>> {
    bearer_validation.authorise(bearer_auth)?;
    let id = recipe_access.insert(&form).await?;

    Ok(id.into())
}

pub(crate) async fn post_recipe(
    path: Path<i64>,
    bearer_auth: BearerToken,
    form: HttpJson<Recipe>,
    recipe_access: Arc<RecipeAccess>,
    bearer_validation: Arc<BearerValidation>,
) -> Result<HttpJson<i64>> {
    bearer_validation.authorise(bearer_auth)?;
    let id = *path;

    let recipe = recipe_access
        .update(id, &form)
        .await?
        .ok_or_else(|| Error::Missing {
            item_type: "recipe".to_string(),
            id,
        })?;

    Ok(recipe.into())
}

pub(crate) async fn search_recipe(
    search_client: Arc<Client>,
    vector_client: Arc<QdrantClient>,
    search_query: Query<SearchQuery>,
    form: HttpJson<Option<Vec<f32>>>,
) -> Result<HttpJson<SearchResponse>> {
    let data = form.0;
    let vector_results = match data {
        Some(query) => {
            let request = SearchPoints {
                collection_name: RECIPES_VEC_COLLECTION_NAME.into(),
                vector: query,
                limit: 10,
                score_threshold: Some(0.25),
                with_payload: Some(WithPayloadSelector {
                    selector_options: Some(SelectorOptions::Enable(true)),
                }),
                // score_threshold: Some(0.3),
                ..Default::default()
            };
            let res = vector_client
                .search_points(&request)
                .await
                .map_err(Error::qdrant)?;
            res.result
        }
        None => Vec::new(),
    };

    let index = search_client.index(R_ECIPE_S_INDEX_NAME);
    let mut query = index.search();
    let (search_ids, mut results) = query
        .with_query(&search_query.query)
        .execute::<RecipeWithId>()
        .await?
        .hits
        .into_iter()
        .map(|hit| {
            let score = hit.ranking_score.unwrap_or(0.0) as f32;
            let id = hit.result.id;

            (id, (hit.result.id, (hit.result, score)))
        })
        .unzip::<_, _, HashSet<_>, HashMap<_, _>>();
    //.collect();

    let (vector_ids, mut vector_results) = vector_results
        .into_iter()
        .filter_map(|point| match point.id.clone() {
            Some(PointId {
                point_id_options: Some(PointIdOptions::Num(num)),
            }) => {
                let name = point
                    .payload
                    .get("name")
                    .as_ref()
                    .and_then(|val| val.as_str())
                    .map(String::from)
                    .unwrap_or_default();
                let description = point
                    .payload
                    .get("description")
                    .as_ref()
                    .and_then(|val| val.as_str())
                    .map(String::from)
                    .unwrap_or_default();
                let data = Recipe {
                    name,
                    ingredients: Vec::new(),
                    description,
                    liked: None,
                    embedding: None,
                };
                let id = num as i64;
                let score = point.score;
                Some((
                    id,
                    (
                        id,
                        (
                            RecipeWithId {
                                id: num as i64,
                                data,
                            },
                            score,
                        ),
                    ),
                ))
            }
            _ => None,
        })
        .unzip::<_, _, HashSet<_>, HashMap<_, _>>();

    //.partition::<Vec<_>, _>(|(id, data)| search_ids.contains(&(*id as i64)));
    let mut all = search_ids
        .union(&vector_ids)
        .filter_map(|id| {
            let vector_res = vector_results.remove(id);
            let vector_score = vector_res.as_ref().map(|(_, score)| *score).unwrap_or(0.);
            let search_res = results.remove(id);
            let search_score = search_res.as_ref().map(|(_, score)| *score).unwrap_or(0.);
            let (val, _) = search_res.or(vector_res)?;
            Some((val, vector_score + search_score.tanh()))
        })
        .collect::<Vec<_>>();
    all.sort_by(|(_, score), (_, score_2)| score_2.partial_cmp(score).unwrap());
    let results = all
        .into_iter()
        .map(|(recipe, _)| SearchResult { recipe })
        .collect();
    Ok(SearchResponse { results }.into())
}
