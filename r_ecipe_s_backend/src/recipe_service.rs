use crate::db::DBAccess;
use actix_web::{dev::*, http::header, web::Data, *};
use futures_util::{StreamExt, TryStreamExt};
use r_ecipe_s_model::{Ingredient, Recipe};
use serde::{Deserialize, Serialize};
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
}
type Result<T> = std::result::Result<T, Error>;

impl ResponseError for Error {
    fn status_code(&self) -> http::StatusCode {
        match self {
            Error::Fail => http::StatusCode::INTERNAL_SERVER_ERROR,
            Error::Serde(_) => http::StatusCode::INTERNAL_SERVER_ERROR,
            Error::DB(_) => http::StatusCode::INTERNAL_SERVER_ERROR,
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
        self.app_data(recipe_access).service(get_all).service(put)
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
}

impl RecipeAccess {
    async fn get_all(&self) -> Result<Vec<RecipeRep>> {
        let data = sqlx::query_as(
            r#"
SELECT
    id, 
    name, 
    ingredients, 
    description, 
    liked  
FROM recipes"#,
        )
        .fetch(self.db_access.get_pool())
        .map(
            |rep_res: std::result::Result<RecipeRep, _>| -> Result<RecipeRep> { Ok(rep_res?) }, //.model()) },
        )
        .try_collect::<Vec<_>>()
        .await;
        let data = data?;
        Ok(data)
    }

    async fn insert(&self, recipe: &Recipe) -> Result<i64> {
        let rec = sqlx::query!(
            r#"
INSERT INTO recipes (
    name,
    ingredients,
    description,
    liked
) VALUES (
    $1,
    $2,
    $3,
    $4
) RETURNING id
            "#,
            recipe.name,
            sqlx::types::Json(recipe.ingredients.clone()) as _,
            recipe.description,
            recipe.liked
        )
        .fetch_one(self.db_access.get_pool())
        .await?;
        Ok(rec.id)
    }
}

impl RecipeAccess {
    pub fn new(db_access: &Arc<DBAccess>) -> Self {
        RecipeAccess {
            db_access: Arc::clone(db_access),
        }
    }
}

#[get("/recipes")]
pub(crate) async fn get_all(recipe_access: Data<RecipeAccess>) -> Result<HttpResponse> {
    let data = recipe_access.get_all().await?;

    let body = serde_json::to_string(&data)?;

    Ok(HttpResponse::Ok()
        .insert_header(header::ContentType::json())
        .body(body))
}

#[put("/recipes")]
pub(crate) async fn put(
    recipe_access: Data<RecipeAccess>,
    form: web::Json<Recipe>,
) -> Result<HttpResponse> {
    let id = recipe_access.insert(&form).await?;

    Ok(HttpResponse::Ok()
        .insert_header(header::ContentType::json())
        .body(serde_json::to_string(&id)?))
}

#[post("/recipes/{id}")]
pub(crate) async fn update(
    recipe_access: Data<RecipeAccess>,
    form: web::Json<Recipe>,
) -> Result<HttpResponse> {
    todo!()
}
