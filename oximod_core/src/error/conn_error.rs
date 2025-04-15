use thiserror::Error;

#[derive(Debug, Error)]
pub enum MongoDbError {
    #[error("Failed to connect to db: {0}")] ConnectionError(String),

    #[error("Failed to set CLIENT")] SetClientError(String),

    #[error("CLIENT not found: {0}")] ClientNotFound(String),

    #[error("Serialization error: {0}")] SerializationError(String),
}
