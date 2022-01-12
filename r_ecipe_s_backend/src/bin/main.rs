use actix_web::{dev::*, http::header, web::Data, *};
use std::sync::Arc;
// use perseus_actix_web::configurer;

use r_ecipe_s_backend::app_config;
use r_ecipe_s_backend::db;
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

async fn greet(req: HttpRequest) -> impl Responder {
    let name = req.match_info().get("name").unwrap_or("World");
    format!("Hello {}", &name)
}

#[tokio::main]
async fn main() -> Result<()> {
    let conf = app_config::AppConfig::load("config/config.toml")?;
    let db_access = Arc::new(
        db::DBMigrator::new(&conf.db_config)
            .await?
            .migrate()
            .await?,
    );

    let host_port = conf.http_config.connection_string();
    let http_server = HttpServer::new(move || {
        let recipe_access = Data::new(RecipeAccess::new(&db_access));
        App::new()
            .route("/", web::get().to(greet))
            .bind_recipe_routes(recipe_access)
            .route("/{name}", web::get().to(greet))
        // .configure(block_on(configurer(todo!()))) // This could then be used to serve perseus
        // // stuff once figureed out
    })
    .bind(&host_port)?;

    println!("Successfully bound server to {}", host_port);

    http_server.run().await?;
    Ok(())
}
