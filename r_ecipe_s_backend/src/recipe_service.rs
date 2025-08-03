use crate::error::Error;
use crate::model::{
    Recipe, RecipeId, RecipeWithId, RecipesResponse, SearchQuery, SearchResponse, VectorSearch,
};
use crate::repository::RecipeAccess;
use axum::Extension;
use axum::{
    extract::{Path, Query},
    http,
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use axum_extra::{
    headers::authorization::{Authorization, Bearer},
    TypedHeader,
};
use base64::Engine;
use futures_util::{StreamExt, TryStreamExt};
use minilm::MiniLM;
use serde::{Deserialize, Serialize};
use sha2::Digest;
use sqlx::{types::time::OffsetDateTime, PgExecutor};
use sqlx::{FromRow, Postgres, Transaction};
use tracing::log::error;

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use thiserror::Error as ThisError;
const MAX_PAGE_SIZE: i64 = 100;

use crate::error::Result;

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let error_code = match self {
            Error::Fail => http::StatusCode::INTERNAL_SERVER_ERROR,
            Error::Serde(_) => http::StatusCode::INTERNAL_SERVER_ERROR,
            Error::DB(_) => http::StatusCode::INTERNAL_SERVER_ERROR,
            Error::ParseInt(_) => http::StatusCode::INTERNAL_SERVER_ERROR,
            Error::Missing { .. } => http::StatusCode::NOT_FOUND,
            Error::IncorrectPageSize(_) => http::StatusCode::BAD_REQUEST,
            Error::Auth => http::StatusCode::UNAUTHORIZED,
            Error::NotFoundId(_) => http::StatusCode::NOT_FOUND,
            Error::Vector(_) => http::StatusCode::INTERNAL_SERVER_ERROR,
            Error::Model(_) => http::StatusCode::INTERNAL_SERVER_ERROR,
        };
        (error_code, format!("{self}")).into_response()
    }
}

pub struct RecipeService;

#[derive(Clone)]
pub struct BearerValidation {
    hashed_secret: Box<[u8]>,
}
impl BearerValidation {
    pub fn new(hashed_secret: &str) -> Self {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(hashed_secret)
            .expect("Failed to decode secret");
        Self {
            hashed_secret: bytes.into(),
        }
    }

    fn authorise(&self, auth: Bearer) -> Result<()> {
        let mut hasher = sha2::Sha512::new();
        hasher.update(auth.token());
        let digest = hasher.finalize();
        if digest.as_slice() == self.hashed_secret.as_ref() {
            Ok(())
        } else {
            Err(Error::Auth)
        }
    }
}

impl RecipeService {
    pub fn recipe_routes<S: Clone + Send + Sync + 'static>(self) -> Router<S> {
        Router::new()
            .route("/recipes", get(get_all).put(put_recipe))
            .route(
                "/recipes/{id}",
                delete(delete_recipe).post(post_recipe).get(get_recipe),
            )
            .route("/recipes/search", post(search_recipe))
    }
}

#[derive(Deserialize, Debug)]
pub struct Paging {
    offset: Option<i64>,
}

const PAGE_SIZE: i64 = 9;

pub(crate) async fn get_all(
    recipe_access: Extension<RecipeAccess>,
    page: Query<Paging>,
) -> Result<Json<RecipesResponse>> {
    let page = page.offset;
    let data = recipe_access.get_all(page.unwrap_or(0), PAGE_SIZE).await?;
    Ok(data.into())
}

pub(crate) async fn get_recipe(
    path: Path<RecipeId>,
    recipe_access: Extension<RecipeAccess>,
) -> Result<Json<RecipeWithId>> {
    let id = *path;
    let data_option = recipe_access.get_by_id(id).await?;
    let data = data_option.ok_or_else(|| Error::Missing {
        item_type: "recipe".to_string(),
        id,
    })?;

    Ok(data.into())
}

pub(crate) async fn delete_recipe(
    path: Path<RecipeId>,
    bearer_auth: TypedHeader<Authorization<Bearer>>,
    recipe_access: Extension<RecipeAccess>,
    bearer_validation: Extension<BearerValidation>,
) -> Result<Json<()>> {
    bearer_validation.authorise(bearer_auth.0 .0)?;
    let id = recipe_access.delete(*path).await?;

    Ok(id.into())
}

pub(crate) async fn put_recipe(
    bearer_auth: TypedHeader<Authorization<Bearer>>,
    bearer_validation: Extension<BearerValidation>,
    recipe_access: Extension<RecipeAccess>,
    form: Json<Recipe>,
) -> Result<Json<RecipeId>> {
    bearer_validation.authorise(bearer_auth.0 .0)?;
    let id = recipe_access.insert(&form).await?;

    Ok(id.into())
}

#[axum::debug_handler]
pub(crate) async fn post_recipe(
    path: Path<RecipeId>,
    bearer_auth: TypedHeader<Authorization<Bearer>>,
    recipe_access: Extension<RecipeAccess>,
    bearer_validation: Extension<BearerValidation>,
    model: Extension<MiniLM>,
    form: Json<Recipe>,
) -> Result<Json<RecipeId>> {
    bearer_validation.authorise(bearer_auth.0 .0)?;
    let id = *path;

    let embedding = form.0.encode(&model)?;

    let recipe = recipe_access
        .update(id, &form, &embedding)
        .await?
        .ok_or_else(|| Error::Missing {
            item_type: "recipe".to_string(),
            id,
        })?;

    Ok(recipe.into())
}

pub(crate) async fn search_recipe(
    recipe_access: Extension<RecipeAccess>,
    model: Extension<MiniLM>,
    search_query: Query<SearchQuery>,
    vector_query: Query<Option<VectorSearch>>,
) -> Result<Json<SearchResponse>> {
    let results = match vector_query.0 {
        Some(_) => {
            let data = model.embed_str(&search_query.query)?;
            recipe_access.hybrid_search(data).await?
        }
        None => recipe_access.search(&search_query.query).await?,
    };
    Ok(SearchResponse { results }.into())

    // let vector_results = match data {
    //     Some(query) => {
    //         let request = SearchPoints {
    //             collection_name: RECIPES_VEC_COLLECTION_NAME.into(),
    //             vector: query,
    //             limit: 10,
    //             score_threshold: Some(0.25),
    //             with_payload: Some(WithPayloadSelector {
    //                 selector_options: Some(SelectorOptions::Enable(true)),
    //             }),
    //             // score_threshold: Some(0.3),
    //             ..Default::default()
    //         };
    //         let res = vector_client
    //             .search_points(&request)
    //             .await
    //             .map_err(Error::qdrant)?;
    //         res.result
    //     }
    //     None => Vec::new(),
    // };
    //
    // let index = search_client.index(R_ECIPE_S_INDEX_NAME);
    // let mut query = index.search();
    // let (search_ids, mut results) = query
    //     .with_query(&search_query.query)
    //     .execute::<RecipeWithId>()
    //     .await?
    //     .hits
    //     .into_iter()
    //     .map(|hit| {
    //         let score = hit.ranking_score.unwrap_or(0.0) as f32;
    //         let id = hit.result.id;
    //
    //         (id, (hit.result.id, (hit.result, score)))
    //     })
    //     .unzip::<_, _, HashSet<_>, HashMap<_, _>>();
    //
    // let (vector_ids, mut vector_results) = vector_results
    //     .into_iter()
    //     .filter_map(|point| match point.id.clone() {
    //         Some(PointId {
    //             point_id_options: Some(PointIdOptions::Num(num)),
    //         }) => {
    //             let name = point
    //                 .payload
    //                 .get("name")
    //                 .as_ref()
    //                 .and_then(|val| val.as_str())
    //                 .map(String::from)
    //                 .unwrap_or_default();
    //             let description = point
    //                 .payload
    //                 .get("description")
    //                 .as_ref()
    //                 .and_then(|val| val.as_str())
    //                 .map(String::from)
    //                 .unwrap_or_default();
    //             let data = Recipe {
    //                 name,
    //                 ingredients: Vec::new(),
    //                 description,
    //                 liked: None,
    //                 embedding: None,
    //             };
    //             let id = num as i64;
    //             let score = point.score;
    //             Some((
    //                 id,
    //                 (
    //                     id,
    //                     (
    //                         RecipeWithId {
    //                             id: num as i64,
    //                             data,
    //                         },
    //                         score,
    //                     ),
    //                 ),
    //             ))
    //         }
    //         _ => None,
    //     })
    //     .unzip::<_, _, HashSet<_>, HashMap<_, _>>();
    //
    // //.partition::<Vec<_>, _>(|(id, data)| search_ids.contains(&(*id as i64)));
    // let mut all = search_ids
    //     .union(&vector_ids)
    //     .filter_map(|id| {
    //         let vector_res = vector_results.remove(id);
    //         let vector_score = vector_res.as_ref().map(|(_, score)| *score).unwrap_or(0.);
    //         let search_res = results.remove(id);
    //         let search_score = search_res.as_ref().map(|(_, score)| *score).unwrap_or(0.);
    //         let (val, _) = search_res.or(vector_res)?;
    //         Some((val, vector_score + search_score.tanh()))
    //     })
    //     .collect::<Vec<_>>();
    // all.sort_by(|(_, score), (_, score_2)| score_2.partial_cmp(score).unwrap());
    // let results = all
    //     .into_iter()
    //     .map(|(recipe, _)| SearchResult { recipe })
    //     .collect();
    // Ok(SearchResponse { results }.into())
}
