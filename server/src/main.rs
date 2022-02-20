use actix_web::{dev::*, http::header, middleware::Logger, web::Data, *};
use futures::executor::block_on;
use std::sync::Arc;

use env_logger;
use r_ecipe_s_backend::app_config;
use r_ecipe_s_backend::db;
use std::env;
use std::fs;
use thiserror::Error as ThisError;

use r_ecipe_s_backend::recipe_service::{RecipeAccess, RecipeService};

const DEFAULT_CONFIG_PATH: &str = "config/config.toml";
#[derive(Debug, ThisError)]
enum Error {
    #[error("r_ecipe_s failed to bind server with io error: {0}")]
    IO(#[from] std::io::Error),
    #[error("r_ecipe_s failed to load config from {DEFAULT_CONFIG_PATH}, Config Error {0}")]
    Confg(#[from] config::ConfigError),
    #[error("r_ecipe_s database error {0}")]
    DB(#[from] db::Error),
}
type Result<T> = std::result::Result<T, Error>;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let conf = app_config::AppConfig::load("config/config.toml")?;
    let db_access = Arc::new(
        db::DBMigrator::new(&conf.db_config)
            .await?
            .migrate()
            .await?,
    );
    env::set_current_dir("../frontend")?;

    std::env::set_var("RUST_LOG", "actix_web=info");
    let host_port = conf.http_config.connection_string();
    let http_server = HttpServer::new(move || {
        let recipe_access = Data::new(RecipeAccess::new(&db_access));
        App::new()
            .wrap(Logger::default())
            .bind_recipe_routes(recipe_access)
            .service(actix_files::Files::new("/static", "static"))
            .service(actix_files::Files::new("/", "dist").index_file("index.html"))
    })
    .bind(&host_port)?;

    println!("Successfully bound server to {}", host_port);

    http_server.run().await?;
    Ok(())
}

