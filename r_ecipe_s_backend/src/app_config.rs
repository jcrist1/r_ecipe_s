use config::{Config, ConfigError, File};

use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Config Error {0}")]
    Config(#[from] ConfigError),
    #[error("Port parse error")]
    ParsePort,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DbConfig {
    pub db_host: String,
    pub db_port: u32, // u16 might be better?
    pub database: String,
    pub user: String,
    pub password: String,
    pub max_connections: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HttpConfig {
    pub host: String,
    pub port: u16,
    pub api_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelConfig {
    pub model_path: String,
}

impl HttpConfig {
    pub fn connection_string(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppConfig {
    pub http_config: HttpConfig,
    pub db_config: DbConfig,
    pub model_config: ModelConfig,
    pub serving_directory: String,
}

impl AppConfig {
    pub fn load() -> std::result::Result<Self, Error> {
        let config: AppConfig = Config::builder()
            .add_source(config::File::with_name("config/default.toml").required(true))
            .add_source(config::File::with_name("config/config.toml").required(false))
            .add_source(config::Environment::with_prefix("R_ECIPE_S"))
            .build()?
            .try_deserialize()?;

        Ok(config)
    }
}
