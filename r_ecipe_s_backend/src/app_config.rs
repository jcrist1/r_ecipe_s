use config::{Config, ConfigError, File};

use serde::{Deserialize, Serialize};

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

impl HTTPConfig {
    pub fn connection_string(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppConfig {
    pub http_config: HTTPConfig,
    pub db_config: DBConfig,
}

impl AppConfig {
    pub fn load(path_str: &str) -> Result<Self, ConfigError> {
        let mut conf = Config::default();
        let conf_file = File::new(path_str, config::FileFormat::Toml);
        conf.merge(conf_file)?;
        let db_config = conf.get::<DBConfig>("db")?;
        let http_config = conf.get::<HTTPConfig>("http")?;
        Ok(AppConfig {
            http_config,
            db_config,
        })
    }
}
