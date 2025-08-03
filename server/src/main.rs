//use actix_web::{dev::*, http::header, middleware::Logger, web::Data, *};
use axum::{Extension, Router};
use futures::future::abortable;
use minilm::MiniLM;
use std::net::{AddrParseError, SocketAddr};
use tokio::net::TcpListener;
use tracing::info;

use r_ecipe_s_backend::app_config::{self, AppConfig};
use r_ecipe_s_backend::db;
use r_ecipe_s_backend::recipe_service::{BearerValidation, RecipeService};
use r_ecipe_s_backend::repository::RecipeAccess;
use std::env;
use thiserror::Error as ThisError;
use tower::ServiceExt;
use tower_http::{
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

const DEFAULT_CONFIG_PATH: &str = "config/config.toml";
#[derive(Debug, ThisError)]
enum Error {
    #[error("r_ecipe_s failed to bind server with io error: {0}")]
    IO(#[from] std::io::Error),
    #[error("r_ecipe_s failed to load config from {DEFAULT_CONFIG_PATH}, Config Error {0}")]
    Confg(#[from] r_ecipe_s_backend::app_config::Error),
    #[error("r_ecipe_s database error {0}")]
    DB(#[from] db::Error),
    #[error("Failed to parse address from connection config: {0}")]
    AddrParse(#[from] AddrParseError),
    #[error("{0}")]
    Message(String),
}

type Result<T> = std::result::Result<T, Error>;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "example_static_file_server=debug,tower_http=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();
    let app_config::AppConfig {
        http_config,
        db_config,
        model_config,
        serving_directory,
    }: app_config::AppConfig = AppConfig::load()?;
    info!("Loading model");
    let data = std::fs::read(&model_config.model_path).expect(&format!(
        "failed to load model data from: {}",
        model_config.model_path
    ));
    let model = MiniLM::build(data).expect(&format!(
        "Failed to load model at {}",
        model_config.model_path
    ));
    info!("Model loaded");
    info!("Running migrations: {db_config:?}");
    let db_access = db::DbMigrator::new(&db_config).await?.migrate().await?;
    info!("Migrations successfully run!");
    env::set_current_dir(serving_directory)?;
    info!("set directory");

    std::env::set_var("RUST_LOG", "axum=info,sqlx=warn");

    let bearer_validation = BearerValidation::new(&http_config.api_key);
    let host_port = http_config.connection_string();
    let recipe_access = RecipeAccess::new(db_access);

    let sock_addr = SocketAddr::new(http_config.host.parse()?, http_config.port); //&host_port.parse()?;
    let app = Router::new()
        .nest_service("/api/v1", RecipeService.recipe_routes())
        .nest_service(
            "/static",
            ServeDir::new("static"), //
                                     // .handle_error(|error: std::io::Error| async move {
                                     // (
                                     //     StatusCode::INTERNAL_SERVER_ERROR,
                                     //     format!("Unhandled internal error: {}", error),
                                     // )
                                     //          }),
        )
        .nest_service(
            "/index.html",
            ServeFile::new("index.html"), //     .handle_error(
                                          //     |error: std::io::Error| async move {
                                          //         (
                                          //             StatusCode::INTERNAL_SERVER_ERROR,
                                          //             format!("Unhandled internal error: {}", error),
                                          //         )
                                          //     },
                                          // ),
        )
        .fallback_service(
            ServeDir::new("dist"), //     .handle_error(|error: std::io::Error| async move {
                                   //     (
                                   //         StatusCode::INTERNAL_SERVER_ERROR,
                                   //         format!("Unhandled internal error: {}", error),
                                   //     )
                                   // }),
        )
        .layer(Extension(recipe_access))
        .layer(Extension(bearer_validation))
        .layer(Extension(model))
        .layer(TraceLayer::new_for_http());
    info!("Successfully bound server to {}", host_port);

    let listener = TcpListener::bind(sock_addr)
        .await
        .expect("failed to bind listener");

    let http_server = axum::serve(listener, app);

    let tasks = tokio::spawn(async move { http_server.await });
    let (fut, handle) = abortable(tasks);
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        handle.abort();
        hook(info);
    }));
    let res = fut
        .await
        .map_err(|err| Error::Message(format!("Failed to run server tasks: {err}")))?;
    match res {
        Ok(_) => {
            return Err(Error::Message(
                "Http server finished early without error.".into(),
            ))
        }
        Err(err) => {
            return Err(Error::Message(format!(
                "Http server finished early because of {err}"
            )));
        }
    }
}
