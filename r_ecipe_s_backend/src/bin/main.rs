use actix_cors::Cors;
use actix_web::{dev::*, http::header, middleware::Logger, web::Data, *};
use futures::executor::block_on;
use perseus::internal::i18n::TranslationsManager;
use perseus::internal::serve::{ServerOptions, ServerProps};
use perseus::plugins::PluginAction;
use perseus::stores::{FsMutableStore, MutableStore};
use perseus::SsrNode;
use perseus_actix_web::configurer;
use std::sync::Arc;

use env_logger;
use perseus_engine::app::{
    get_app_root, get_error_pages_contained, get_immutable_store, get_locales, get_plugins,
    get_static_aliases, get_templates_map_atomic_contained, get_translations_manager,
};
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

async fn greet(req: HttpRequest) -> impl Responder {
    let name = req.match_info().get("name").unwrap_or("World");
    format!("Hello {}", &name)
}

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
    env::set_current_dir("../r_ecipe_s_frontend/.perseus").unwrap();

    std::env::set_var("RUST_LOG", "actix_web=info");
    let host_port = conf.http_config.connection_string();
    let http_server = HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_origin("https://localhost:8080/")
            .allowed_origin_fn(|origin, _req_head| origin.as_bytes().ends_with(b".localhost:8080"))
            .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
            .allowed_headers(vec![http::header::AUTHORIZATION, http::header::ACCEPT])
            .allowed_header(http::header::CONTENT_TYPE)
            .max_age(3600);
        let recipe_access = Data::new(RecipeAccess::new(&db_access));
        App::new()
            .wrap(Logger::default())
            .bind_recipe_routes(recipe_access)
            .configure(block_on(configurer(get_props())))
    })
    .bind(&host_port)?;

    println!("Successfully bound server to {}", host_port);

    http_server.run().await?;
    Ok(())
}

/// Gets the properties to pass to the server. This is copied from the perseus
/// generated program source code
fn get_props() -> ServerProps<impl MutableStore, impl TranslationsManager> {
    let plugins = get_plugins::<SsrNode>();

    plugins
        .functional_actions
        .server_actions
        .before_serve
        .run((), plugins.get_plugin_data());

    // This allows us to operate inside `.perseus/` and as a standalone binary in production
    let (html_shell_path, static_dir_path) = ("../index.html", "../static");

    let immutable_store = get_immutable_store(&plugins);
    let locales = get_locales(&plugins);
    let app_root = get_app_root(&plugins);
    let static_aliases = get_static_aliases(&plugins);

    let opts = ServerOptions {
        // We don't support setting some attributes from `wasm-pack` through plugins/`define_app!` because that would require CLI changes as well (a job for an alternative engine)
        index: html_shell_path.to_string(), // The user must define their own `index.html` file
        js_bundle: "dist/pkg/perseus_engine.js".to_string(),
        // Our crate has the same name, so this will be predictable
        wasm_bundle: "dist/pkg/perseus_engine_bg.wasm".to_string(),
        // It's a nightmare to get the templates map to take plugins, so we use a self-contained version
        // TODO reduce allocations here
        templates_map: get_templates_map_atomic_contained(),
        locales,
        root_id: app_root,
        snippets: "dist/pkg/snippets".to_string(),
        error_pages: get_error_pages_contained(),
        // The CLI supports static content in `../static` by default if it exists
        // This will be available directly at `/.perseus/static`
        static_dir: if fs::metadata(&static_dir_path).is_ok() {
            Some(static_dir_path.to_string())
        } else {
            None
        },
        static_aliases,
    };

    ServerProps {
        opts,
        immutable_store,
        mutable_store: FsMutableStore::new("dist/mutable".to_string()), // get_mutable_store(),
        translations_manager: block_on(get_translations_manager()),
    }
}
