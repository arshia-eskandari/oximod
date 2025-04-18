use thiserror::Error;

#[derive(Debug, Error)]
pub enum OximodError  {
    #[error("Failed to connect to db: {0}")] ConnectionError(String),

    #[error("Failed to set CLIENT")] GlobalClientInitError(String),

    #[error("CLIENT not found: {0}")] GlobalClientMissing(String),

    #[error("Serialization error: {0}")] SerializationError(String),
}
