use crate::app_config::DBConfig;
use thiserror::Error as ThisError;
use tracing::info;

#[derive(Debug, ThisError)]
pub enum Error {
    #[error("Sqlx error {0}")]
    Postgres(#[from] sqlx::Error),
    #[error("Migration error {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
}

pub struct DbMigrator {
    pool: sqlx::PgPool,
}
type Result<T> = std::result::Result<T, Error>;

impl DbMigrator {
    pub async fn new(db_conf: &DBConfig) -> Result<Self> {
        let connection_string = format!(
            "postgresql://{}:{}@{}:{}/{}",
            db_conf.user, db_conf.password, db_conf.host, db_conf.port, db_conf.database
        );
        let options = sqlx::pool::PoolOptions::new().max_connections(db_conf.max_connections);
        // sqlx::PgPool::connect(&connection_string).await?;
        let pool: sqlx::PgPool = options.connect(&connection_string).await?;
        Ok(DbMigrator { pool })
    }

    pub async fn migrate(self) -> Result<DbAccess> {
        info!("Running migrations");
        sqlx::migrate!("./migrations").run(&self.pool).await?;
        info!("Migrations run!");
        Ok(DbAccess { pool: self.pool })
    }
}

pub struct DbAccess {
    pool: sqlx::PgPool,
}

impl DbAccess {
    pub(crate) fn get_pool(&self) -> &sqlx::PgPool {
        &self.pool
    }
}
