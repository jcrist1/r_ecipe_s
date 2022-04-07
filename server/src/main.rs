//use actix_web::{dev::*, http::header, middleware::Logger, web::Data, *};
use axum::http::StatusCode;
use axum::Router;
use futures::executor::block_on;
use log::info;
use std::net::{AddrParseError, SocketAddr};
use std::sync::Arc;

use axum::routing::get_service;
use axum::Extension;
use env_logger;
use r_ecipe_s_backend::app_config;
use r_ecipe_s_backend::recipe_service::{RecipeAccess, RecipeService};
use r_ecipe_s_backend::{db, search_indexer};
use std::env;
use std::fs;
use thiserror::Error as ThisError;
use tower::ServiceBuilder;
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
    Confg(#[from] config::ConfigError),
    #[error("r_ecipe_s database error {0}")]
    DB(#[from] db::Error),
    #[error("r_ecipe_s search indexing error {0}")]
    SearchIndexer(#[from] search_indexer::Error),
    #[error("Failed to parse address from connection config: {0}")]
    AddrParse(#[from] AddrParseError),
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
        search_config,
    }: app_config::AppConfig = app_config::AppConfig::load("config/config.toml")?;
    let db_access = Arc::new(db::DBMigrator::new(&db_config).await?.migrate().await?);
    env::set_current_dir("../frontend")?;

    std::env::set_var("RUST_LOG", "actix_web=info");
    let host_port = http_config.connection_string();
    let recipe_access = Arc::new(RecipeAccess::new(&db_access));

    let sock_addr = SocketAddr::new(http_config.host.parse()?, http_config.port); //&host_port.parse()?;
    let app = Router::new()
        .nest("/api/v1", Router::new().bind_recipe_routes(&recipe_access))
        .bind_recipe_routes(&recipe_access)
        .nest(
            "/static",
            get_service(ServeDir::new("static")).handle_error(|error: std::io::Error| async move {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Unhandled internal error: {}", error),
                )
            }),
        )
        .nest(
            "/index.html",
            get_service(ServeFile::new("index.html")).handle_error(
                |error: std::io::Error| async move {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Unhandled internal error: {}", error),
                    )
                },
            ),
        )
        .fallback(Router::new().nest(
            "/",
            get_service(ServeDir::new("dist")).handle_error(|error: std::io::Error| async move {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Unhandled internal error: {}", error),
                )
            }),
        ))
        .layer(TraceLayer::new_for_http());
    tracing::info!("Successfully bound server to {}", host_port);
    let http_server = axum::Server::bind(&sock_addr).serve(app.into_make_service());

    let index_loop_future = tokio::spawn(async move {
        r_ecipe_s_backend::search_indexer::index_loop(search_config, recipe_access).await
    });
    http_server.await.expect("Don't forget to map this error");
    index_loop_future.abort();
    Ok(())
}
