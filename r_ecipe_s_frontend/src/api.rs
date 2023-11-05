use gloo_net::http::{self, QueryParams};
use leptos::logging::warn;
use r_ecipe_s_model::{Recipe, RecipeWithId, RecipesResponse, SearchResponse};
use serde::de::DeserializeOwned;
use std::future::Future;
use std::pin::Pin;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Failed to do some HTTP: {0}")]
    Request(#[from] gloo_net::Error),
    #[error("Failed to do some JSON: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("Bad response: {0}")]
    Http(String),
    #[error("Action not allowed with configuring API token")]
    Forbidden,
}
trait HttpErr {
    fn http_ok_json<T: DeserializeOwned + Unpin + 'static>(
        self,
    ) -> Pin<Box<dyn Future<Output = Result<T, Error>>>>;
}

async fn response_http_err<T: DeserializeOwned + Unpin + 'static>(
    resp: http::Response,
) -> Result<T, Error> {
    if !resp.ok() {
        let status = resp.status_text();
        let code = resp.status();
        let text = resp.text().await?;
        Err(Error::Http(format!("{status} {code} – {text}")))
    } else {
        Ok(resp.json::<T>().await?)
    }
}

impl HttpErr for http::Response {
    fn http_ok_json<T: DeserializeOwned + 'static + Unpin>(
        self,
    ) -> Pin<Box<dyn Future<Output = Result<T, Error>>>> {
        Box::pin(response_http_err(self))
    }
}

pub async fn put_recipe(recipe: &Recipe, token: Option<&str>) -> Result<i64, Error> {
    let token = token.ok_or(Error::Forbidden)?;
    http::Request::put("/api/v1/recipes")
        .header("Content-Type", "application/json")
        .header("Authorization", &format!("Bearer {token}"))
        .body(&serde_json::to_string(recipe)?)?
        .send()
        .await?
        .http_ok_json::<i64>()
        .await
}

pub async fn update_recipe(id: i64, recipe: &Recipe, token: Option<&str>) -> Result<i64, Error> {
    let token = token.ok_or(Error::Forbidden)?;
    http::Request::post(&format!("/api/v1/recipes/{id}"))
        .header("Content-Type", "application/json")
        .header("Authorization", &format!("Bearer {token}"))
        .body(&serde_json::to_string(recipe)?)?
        .send()
        // .expect("failed to get response from POST recipes/:id")
        .await?
        .http_ok_json::<i64>()
        .await
}

pub async fn download(origin: &str, host: &str, static_file: &str) -> Result<Vec<u8>, Error> {
    // todo, fix
    let resp = http::Request::get(&format!("https://{host}/{static_file}"))
        .header("Origin", origin)
        // .header(
        //     "Access-Control-Request-Headers",
        //     &format!("access-control-allow-origin: {origin}"),
        // )
        .header("Access-Control-Request-Method", "GET")
        .send()
        .await?;
    warn!("Response was : {resp:#?}");
    if !resp.ok() {
        let status = resp.status_text();
        let code = resp.status();
        let text = resp.text().await?;
        Err(Error::Http(format!("{status} {code} - {text}")))
    } else {
        let body = resp.binary().await?;
        Ok(body)
    }
}

pub async fn get_recipes_at_offset(offset: i64) -> Result<RecipesResponse, Error> {
    http::Request::get(&format!("/api/v1/recipes?offset={offset}"))
        .send()
        .await?
        .http_ok_json::<RecipesResponse>()
        .await
}

pub async fn delete_recipe(id: i64, token: Option<&str>) -> Result<(), Error> {
    let token = token.ok_or(Error::Forbidden)?;
    http::Request::delete(&format!("/api/v1/recipes/{id}"))
        .header("Authorization", &format!("Bearer {token}"))
        .send()
        .await?
        .http_ok_json::<()>()
        .await
}
pub async fn search(query: &str, vector: Option<&[f32]>) -> Result<SearchResponse, Error> {
    http::Request::post(&format!("/api/v1/recipes/search"))
        .header("Content-Type", "application/json")
        .query([("query", query)])
        .body(serde_json::to_string(&vector)?)?
        .send()
        .await?
        .http_ok_json()
        .await
}
