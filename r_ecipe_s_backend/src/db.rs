use crate::app_config::DBConfig;
use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
pub enum Error {
    #[error("Sqlx error {0}")]
    Postgres(#[from] sqlx::Error),
    #[error("Migration error {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
}

pub struct DBMigrator {
    pool: sqlx::PgPool,
}
type Result<T> = std::result::Result<T, Error>;

impl DBMigrator {
    pub async fn new(db_conf: &DBConfig) -> Result<Self> {
        let connection_string = format!(
            "postgresql://{}:{}@{}:{}/{}",
            db_conf.user, db_conf.password, db_conf.host, db_conf.port, db_conf.database
        );
        let pool = sqlx::PgPool::connect(&connection_string).await?;
        Ok(DBMigrator { pool })
    }

    pub async fn migrate(self) -> Result<DBAccess> {
        sqlx::migrate!("./migrations").run(&self.pool).await?;
        Ok(DBAccess { pool: self.pool })
    }
}

pub struct DBAccess {
    pool: sqlx::PgPool,
}

impl DBAccess {
    pub(crate) fn get_pool(&self) -> &sqlx::PgPool {
        &self.pool
    }
}
