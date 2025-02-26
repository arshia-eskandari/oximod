use thiserror::Error;

#[derive(Debug, Error)]
pub enum MongoDbError {
    #[error("Failed to connect to db: {0}")] ConnectionError(String),

    #[error("Failed to set CLIENT")] SetClientError(String),

    #[error("CLIENT not found: {0}")] ClientNotFound(String),

    #[error("Failed to set DEFAULT_DB: {0}")] SetDefaultDb(String),

    #[error("DEFAULT_DB not found: {0}")] DefaultDbNotFound(String),
}
