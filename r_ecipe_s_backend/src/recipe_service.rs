use crate::db::DBAccess;
use actix_web::middleware::Logger;
use actix_web::{dev::*, http::header, web::Data, *};
use futures_util::{StreamExt, TryStreamExt};
use log::{debug, error, info, log_enabled, Level};
use r_ecipe_s_model::{Ingredient, Recipe, RecipeId, RecipeWithId};
use serde::{Deserialize, Serialize};
use sqlx::types::time::OffsetDateTime;
use sqlx::types::Json;
use sqlx::FromRow;

use std::sync::Arc;
use thiserror::Error as ThisError;

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
}
type Result<T> = std::result::Result<T, Error>;

impl ResponseError for Error {
    fn status_code(&self) -> http::StatusCode {
        match self {
            Error::Fail => http::StatusCode::INTERNAL_SERVER_ERROR,
            Error::Serde(_) => http::StatusCode::INTERNAL_SERVER_ERROR,
            Error::DB(_) => http::StatusCode::INTERNAL_SERVER_ERROR,
            Error::ParseInt(_) => http::StatusCode::INTERNAL_SERVER_ERROR,
            Error::Missing { .. } => http::StatusCode::NOT_FOUND,
        }
    }
}

pub trait RecipeService {
    type ServiceType;
    fn bind_recipe_routes(self, recipe_access: Data<RecipeAccess>) -> Self::ServiceType;
}

impl<T> RecipeService for App<T> {
    type ServiceType = Self;
    fn bind_recipe_routes(self, recipe_access: Data<RecipeAccess>) -> Self::ServiceType {
        self.app_data(recipe_access)
            .service(get_all)
            .service(get)
            .service(put)
            .service(post)
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
        RecipeWithId {
            id: RecipeId { id },
            data,
        }
    }
}

impl RecipeAccess {
    async fn get_all(&self, page: i64, page_size: i64) -> Result<Vec<RecipeWithId>> {
        let offset = page * page_size;
        let data = sqlx::query_as!(
            RecipeRep,
            r#"
                SELECT
                    id, 
                    name, 
                    ingredients as "ingredients: Json<Vec<Ingredient>>", 
                    description, 
                    liked  
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
        let data = data?;
        Ok(data)
    }

    async fn update(&self, id: i64, recipe: &Recipe) -> Result<Option<RecipeId>> {
        let rec = sqlx::query_as!(
            RecipeId,
            r#"
                UPDATE recipes SET 
                    name = $1,
                    ingredients = $2, 
                    description = $3,
                    liked = $4,
                    updated = $5
                where id = $6 RETURNING id
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
        .map_err(|err| err.into());
        println!("{:?}", rec);
        rec
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
                    updated
                ) VALUES (
                    $1,
                    $2,
                    $3,
                    $4,
                    $5,
                    $5
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
                    liked  
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

#[get("api/v1/recipes")]
pub(crate) async fn get_all(recipe_access: Data<RecipeAccess>) -> Result<HttpResponse> {
    let data = recipe_access.get_all(0, 9).await?;

    let body = serde_json::to_string(&data)?;

    Ok(HttpResponse::Ok()
        .insert_header(header::ContentType::json())
        .body(body))
}

#[get("api/v1/recipes/{id}")]
pub(crate) async fn get(
    path: web::Path<i64>,
    recipe_access: Data<RecipeAccess>,
) -> Result<HttpResponse> {
    let id = path.into_inner();
    let data_option = recipe_access.get_by_id(id).await?;
    let data = data_option.ok_or_else(|| Error::Missing {
        item_type: "recipe".to_string(),
        id,
    })?;

    debug!("Request data {:?}", data);

    let body = serde_json::to_string(&data)?;

    Ok(HttpResponse::Ok()
        .insert_header(header::ContentType::json())
        .body(body))
}

#[put("api/v1/recipes")]
pub(crate) async fn put(
    recipe_access: Data<RecipeAccess>,
    form: web::Json<Recipe>,
) -> Result<HttpResponse> {
    let id = recipe_access.insert(&form).await?;

    Ok(HttpResponse::Ok()
        .insert_header(header::ContentType::json())
        .body(serde_json::to_string(&id)?))
}

#[post("api/v1/recipes/{id}")]
pub(crate) async fn post(
    path: web::Path<i64>,
    recipe_access: Data<RecipeAccess>,
    form: web::Json<Recipe>,
) -> Result<HttpResponse> {
    let id = path.into_inner();

    debug!("Request data {:?}", form);
    let recipe = recipe_access
        .update(id, &form)
        .await?
        .ok_or_else(|| Error::Missing {
            item_type: "recipe".to_string(),
            id,
        })?;

    Ok(HttpResponse::Ok()
        .insert_header(header::ContentType::json())
        .body(serde_json::to_string(&recipe)?))
}
