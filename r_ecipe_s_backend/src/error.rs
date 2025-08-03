use crate::model::RecipeId;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("RecipeService failed")]
    Fail,
    #[error("Failed to serialise recipe: Serde Error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("Database Error: {0}")]
    DB(#[from] sqlx::Error),
    #[error("Parse in error: {0}")]
    ParseInt(#[from] std::num::ParseIntError),
    #[error("Missing item of type: {item_type} with id: {id}")]
    Missing { item_type: String, id: RecipeId },
    #[error("Incorrect page size: {0}. Must be between 1 and 100")]
    IncorrectPageSize(i64),
    #[error("Failed to authenticate")]
    Auth,
    #[error("Resource with Id {0} not found")]
    NotFoundId(RecipeId),
    #[error("Error with vector DB {0}.")]
    Vector(String),
    #[error("Error running model {0};")]
    Model(#[from] minilm::Error),
}

impl Error {
    pub(crate) fn qdrant(err: anyhow::Error) -> Error {
        Error::Vector(format!("{err}"))
    }
}
pub(crate) type Result<T> = std::result::Result<T, Error>;
