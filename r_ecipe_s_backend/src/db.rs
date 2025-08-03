use std::future::Future;

use crate::app_config::DbConfig;
use sqlx::{PgConnection, PgExecutor, PgPool, Postgres, Transaction};
use tracing::info;

#[derive(Debug, thiserror::Error)]
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

pub(crate) struct CommitRequired<'a, T> {
    data: T,
    transaction: Transaction<'a, Postgres>,
}

impl DbMigrator {
    pub async fn new(db_conf: &DbConfig) -> Result<Self> {
        let connection_string = format!(
            "postgresql://{}:{}@{}:{}/{}",
            db_conf.user, db_conf.password, db_conf.db_host, db_conf.db_port, db_conf.database
        );
        let options = sqlx::pool::PoolOptions::new().max_connections(db_conf.max_connections);
        // sqlx::PgPool::connect(&connection_string).await?;
        let pool: sqlx::PgPool = options.connect(&connection_string).await?;
        Ok(DbMigrator { pool })
    }

    pub async fn migrate(self) -> Result<DbAccess> {
        info!("Running migrations");
        sqlx::migrate!("../r_ecipe_s_backend/migrations")
            .run(&self.pool)
            .await?;
        info!("Migrations run!");
        Ok(DbAccess { pool: self.pool })
    }
}

#[derive(Clone)]
pub struct DbAccess {
    pool: sqlx::PgPool,
}

impl DbAccess {
    pub(crate) fn get_pool(&self) -> &sqlx::PgPool {
        &self.pool
    }
}

impl<'a, T> CommitRequired<'a, T> {
    pub(crate) fn new(transaction: Transaction<'a, Postgres>, data: T) -> Self {
        CommitRequired { data, transaction }
    }
    pub(crate) async fn commit_and_get_data(self) -> Result<T> {
        self.transaction.commit().await?;
        Ok(self.data)
    }

    pub(crate) async fn and_then<
        T2,
        Fut: Future<Output = T2> + Send + Sync,
        F: for<'b> FnOnce(&mut PgConnection, T) -> Fut,
    >(
        self,
        f: F,
    ) -> CommitRequired<'a, T2> {
        let Self {
            mut transaction,
            data,
        } = self;
        let data = f(transaction.as_mut(), data).await;
        CommitRequired { transaction, data }
    }
}

#[cfg(test)]
pub(crate) mod test {
    use sqlx::PgPool;

    use super::DbAccess;

    impl DbAccess {
        pub(crate) fn for_test(pool: PgPool) -> Self {
            DbAccess { pool }
        }
    }
}
