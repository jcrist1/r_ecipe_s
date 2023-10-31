use config::{Config, ConfigError, File};

use serde::{Deserialize, Serialize};

use thiserror::Error as ThisError;
use tracing::log::info;

#[derive(Debug, ThisError)]
pub enum Error {
    #[error("Config Error {0}")]
    Config(#[from] ConfigError),
    #[error("Port parse error")]
    ParsePort,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DBConfig {
    pub host: String,
    pub port: u32, // u16 might be better?
    pub database: String,
    pub user: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HTTPConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchConfig {
    pub host: String,
    pub port: u16,
    pub api_key: String,
}

impl SearchConfig {
    pub fn http_url(&self) -> String {
        format!("http://{}:{}", self.host, self.port)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VectorSearchConfig {
    pub host: String,
    pub port: u16,
}

impl HTTPConfig {
    pub fn connection_string(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppConfig {
    pub http_config: HTTPConfig,
    pub db_config: DBConfig,
    pub search_config: SearchConfig,
    pub vector_search_config: VectorSearchConfig,
}

impl AppConfig {
    pub fn load(path_str: &str) -> Result<Self, ConfigError> {
        let mut conf = Config::default();
        let conf_file = File::new(path_str, config::FileFormat::Toml);
        conf.merge(conf_file)?;
        let mut db_config = conf.get::<DBConfig>("db")?;
        if let Ok(db_host) = std::env::var("R_ECIPE_S_DB_HOST") {
            info!("db host from env");
            db_config.host = db_host;
        } else {
            info!("db host from file")
        }
        if let Ok(db_pass) = std::env::var("R_ECIPE_S_DB_PASSWORD") {
            info!("password from env");
            db_config.password = db_pass;
        } else {
            info!("password from config file");
        }

        let mut http_config = conf.get::<HTTPConfig>("http")?;
        if let Ok(host) = std::env::var("R_ECIPE_S_SERVER_HOST") {
            info!("getting server host from env: {host}");
            http_config.host = host;
        } else {
            info!("getting server host from file");
        }

        let mut search_config = conf.get::<SearchConfig>("search")?;
        if let Ok(api_key) = std::env::var("R_ECIPE_S_SEARCH_API_KEY") {
            info!("getting search api key from env");
            search_config.api_key = api_key
        }
        if let Ok(host) = std::env::var("R_ECIPE_S_SEARCH_HOST") {
            info!("getting search host key from env");
            search_config.host = host
        }
        if let Ok(port) = std::env::var("R_ECIPE_S_SEARCH_PORT") {
            info!("getting search port key from env");
            search_config.port = port.parse::<u16>().map_err(|_| {
                ConfigError::Message("Failed to parse port for search config".to_string())
            })?;
        }

        let vector_search_config = conf.get::<VectorSearchConfig>("vector_search")?;

        Ok(AppConfig {
            http_config,
            db_config,
            search_config,
            vector_search_config,
        })
    }
}
