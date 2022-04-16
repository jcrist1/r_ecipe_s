use crate::{
    auth::{AuthError, BearerToken, BearerValidation},
    db::DBAccess,
};
use axum::{
    body::HttpBody,
    extract::{Path, Query},
    http,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json as HttpJson, Router,
};
use futures_util::{StreamExt, TryStreamExt};
use r_ecipe_s_model::{Ingredient, Recipe, RecipeWithId, RecipesResponse};
use serde::{Deserialize, Serialize};
use sqlx::types::time::OffsetDateTime;
use sqlx::types::Json;
use sqlx::{FromRow, Postgres, Transaction};
use tracing::log::error;

use std::sync::Arc;
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
            Error::Auth(_) => http::StatusCode::BAD_REQUEST,
        };
        (error_code, format!("{self:?}")).into_response()
    }
}

pub trait RecipeService {
    type ServiceType;
    fn bind_recipe_routes(
        self,
        recipe_access: &Arc<RecipeAccess>,
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
        bearer_validation: &Arc<BearerValidation>,
    ) -> Self::ServiceType {
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
            post({
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
    }
}
//    }
//}

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
}

impl RecipeRep {
    pub fn model(self) -> Recipe {
        Recipe {
            name: self.name,
            ingredients: self.ingredients.0,
            description: self.description,
            liked: self.liked,
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
                    searchable
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
        println!("New res: {res:?}");
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
                    searchable
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
                SELECT (COUNT(id) / $1)::int8 as count  FROM recipes;
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
                    searchable = false
                where id = $6 RETURNING id as "id!: i64"
            "#,
            recipe.name,
            sqlx::types::Json(recipe.ingredients.clone()) as _,
            recipe.description,
            recipe.liked,
            OffsetDateTime::now_utc(),
            id,
        )
        .fetch_optional(self.db_access.get_pool())
        .await
        .map(|opt| opt.map(|recipe_id| recipe_id.id))
        .map_err(|err| err.into())
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

    async fn get_by_id(&self, id: i64) -> Result<Option<RecipeWithId>> {
        let ret = sqlx::query_as!(
            RecipeRep,
            r#"
                SELECT
                    id, 
                    name, 
                    ingredients as "ingredients: Json<Vec<Ingredient>>", 
                    description, 
                    liked,
                    searchable
                FROM recipes
                WHERE id = $1
            "#,
            id,
        )
        .fetch_optional(self.db_access.get_pool())
        .await?
        .map(|rep| rep.model_with_id());
        Ok(ret)
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
